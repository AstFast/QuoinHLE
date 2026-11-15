/// Bytes of heap header before the first node.
pub const HEADER: u32 = 20;

/// Bytes of header in front of every pointer the guest sees.
pub const NODE: u32 = 12;

/// The free bit in a node's size word.
const FREE: u32 = 1;

/// Offset of the sentinel's handle inside the heap header.
const SENTINEL: u32 = 8;

/// Offset of `high` inside the heap header.
const HIGH: u32 = 16;

/// The smallest remainder worth splitting off: a twelve-byte node with no payload.
const MIN_SPLIT: u32 = NODE;

/// How much of its span the heap is handed at start; `Sbrk` extends it to `limit` on demand.
///
/// `AEEkHeap_Init` does not choose this itself — it asks the owner's `sbrk(0)` for the current end.
/// Which size a handset's per-applet heap starts at is not on this box, so the number is this host's.
/// What has to be the delivery's is the *shape*: a heap that grows when nothing fits, rather than one
/// pinned at its ceiling from the first byte, which made `Sbrk` unreachable and turned a fragmented
/// heap into a refusal no handset would have answered. Growth only lengthens the last node, so the
/// starting size changes no address — only the point at which a request stops being a fit and becomes
/// a grow. A developer flag (`--heap-initial-kb`) may name a different first slice for one run; this
/// is the default.
pub const INITIAL: u32 = 256 * 1024;

/// The read side of a heap's memory. Split from [`Arena`] so a report can walk the heap through a
/// shared borrow — the run's own `Mem` is borrowed mutably for the whole of an allocation.
pub trait ReadArena {
    /// A word of the heap. An address the arena cannot read answers zero — the delivery trusts its
    /// own metadata, and a host that faults on a pack's scribbled node header would die where a
    /// handset merely misbehaves.
    fn read_word(&self, at: u32) -> u32;
    fn read_half(&self, at: u32) -> u16;
}

/// What the algorithm needs from the memory the heap lives in.
///
/// Addresses are absolute guest addresses; [`HeapState::base`] says where the heap starts, so the
/// same model serves a test's byte vector at zero and the guest's mapped pages at `0x2000_0000`.
pub trait Arena: ReadArena {
    fn write_word(&mut self, at: u32, value: u32);
    fn write_half(&mut self, at: u32, value: u16);
    /// `memmove`: overlap-safe, because a realloc that moves a block down into a hole it touches
    /// copies over itself.
    fn copy(&mut self, dst: u32, src: u32, len: u32);
    fn zero(&mut self, at: u32, len: u32);
    /// Make `[at, at + need)` the heap's own. False when the span cannot grow that far.
    fn grow(&mut self, at: u32, need: u32) -> bool;
}

/// The two numbers the model needs that the heap header does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeapState {
    pub base: u32,
    pub limit: u32,
    /// How much of the span [`HeapState::init`] hands over as its first free node. The rest is added
    /// by `Sbrk` as requests need it, so this changes when the heap starts growing, not how far.
    pub initial: u32,
}

impl Default for HeapState {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl HeapState {
    /// The delivery's own starting slice — see [`INITIAL`].
    pub fn new(base: u32, limit: u32) -> Self {
        Self::new_with_initial(base, limit, INITIAL)
    }

    /// A heap whose first free node is `initial` bytes rather than [`INITIAL`]'s. A developer flag
    /// uses this to move the `Sbrk` boundary without changing the ceiling.
    pub fn new_with_initial(base: u32, limit: u32, initial: u32) -> Self {
        Self {
            base,
            limit,
            initial,
        }
    }

    /// `AEEkHeap_Init`: the header, then one free node covering the first slice — [`HeapState::initial`]
    /// bytes, or the whole span when it is smaller. `Sbrk` extends that slice to `limit` as requests need.
    #[inline(never)]
    pub fn init<A: Arena>(&self, a: &mut A) {
        let span = self.limit.saturating_sub(self.base).min(self.initial);
        let end = self.base + (span & !3);
        // `AEEkHeap_Init` hands the heap one free node covering this whole slice, and on a
        // handset every byte of it is writable before the first allocation touches it. Bring
        // the pages in here rather than letting each allocator write map its own: a pack that
        // clears a region it owns with one `memset` writes bytes the allocator has not reached
        // yet, and that write must not be the one that faults.
        let _ = a.grow(self.base, end.saturating_sub(self.base));
        a.write_word(self.base, 0);
        a.write_word(self.base + 4, 0);
        a.write_word(self.base + SENTINEL, self.base + SENTINEL);
        a.write_word(self.base + 12, self.base + SENTINEL);
        a.write_word(self.base + HIGH, end);
        let size = end.saturating_sub(self.base + HEADER);
        if size >= MIN_SPLIT {
            self.free_node(a, self.base + HEADER, size);
        }
    }

    pub fn high<R: ReadArena>(&self, a: &R) -> u32 {
        a.read_word(self.base + HIGH)
    }

    fn set_high<A: Arena>(&self, a: &mut A, high: u32) {
        a.write_word(self.base + HIGH, high);
    }

    // ---- the node words ----

    fn round(len: u32) -> u32 {
        (len + 3) & !3
    }

    /// Total size of a node, header included. `2 * (word >> 1)`, which drops the free bit.
    fn size<R: ReadArena>(a: &R, node: u32) -> u32 {
        2 * (a.read_word(node) >> 1)
    }

    fn is_free<R: ReadArena>(a: &R, node: u32) -> bool {
        a.read_word(node) & FREE != 0
    }

    /// What a node carries: its size less the header and the slack a trim recorded. A free node's
    /// `+10` belongs to the link fields' word and is not an `extra`.
    fn payload<R: ReadArena>(a: &R, node: u32) -> u32 {
        let extra = if Self::is_free(a, node) {
            0
        } else {
            a.read_half(node + 10) as u32
        };
        Self::size(a, node).saturating_sub(NODE + extra)
    }

    /// What a node's payload was when it was handed out. Public for a caller that has a node address
    /// in hand — the free path records it for the watch log before the node goes back on the list.
    #[inline(never)]
    pub fn payload_of<R: ReadArena>(a: &R, node: u32) -> u32 {
        Self::payload(a, node)
    }

    /// A node's total size and whether it is on the free list, for a reader that is looking at an
    /// address a register held when something went wrong. `None` when the word there is not a node
    /// this heap could have written.
    pub fn node_at<R: ReadArena>(&self, a: &R, node: u32) -> Option<(u32, bool)> {
        if !self.sane(a, node) {
            return None;
        }
        Some((Self::size(a, node), Self::is_free(a, node)))
    }

    /// Whether a word is plausibly this heap's node rather than a pack's scribble: inside the
    /// used span, four-aligned, and big enough to hold its own header. `AEEkHeap_IsNode` checks all
    /// but the last; a zero-size node would walk for ever, so the host adds it.
    fn sane<R: ReadArena>(&self, a: &R, node: u32) -> bool {
        node >= self.base + HEADER
            && node < self.high(a)
            && (node - self.base) % 4 == 0
            && Self::size(a, node) >= NODE
            // Saturating like the walks below: the size word is guest memory and this predicate is
            // what rejects a scribble, so a sum that leaves u32 has to answer `false` rather than
            // overflow on the way to asking. The crash report reaches this with any register value.
            && node.saturating_add(Self::size(a, node)) <= self.high(a)
    }

    // ---- the free list ----

    /// `AEEkHeap_NextFreeNode`: the first free node at or above `node`, as a handle, or the
    /// sentinel's handle when there is none.
    #[inline(never)]
    fn next_free<R: ReadArena>(&self, a: &R, node: u32) -> u32 {
        let high = self.high(a);
        // Every size word on this walk is guest memory, and `push_node` reaches here without passing
        // through `sane`. Saturating ends the walk at the sentinel; wrapping would put `at` back
        // inside the heap and, since a scribbled size still clears the `size < NODE` gate, loop for ever.
        let mut at = node.saturating_add(Self::size(a, node));
        while at.saturating_add(NODE) <= high {
            if Self::is_free(a, at) {
                return at + 4;
            }
            let size = Self::size(a, at);
            if size < NODE {
                break;
            }
            at = at.saturating_add(size);
        }
        self.base + SENTINEL
    }

    /// Take a node off the list. `prev` and `next` are handles, and the sentinel is a handle like
    /// any other, so no branch is needed for the ends.
    #[inline(never)]
    fn unlink<A: Arena>(&self, a: &mut A, node: u32) {
        let handle = node + 4;
        let prev = a.read_word(handle);
        let next = a.read_word(handle + 4);
        a.write_word(prev + 4, next);
        a.write_word(next, prev);
        a.write_word(handle, handle);
        a.write_word(handle + 4, handle);
    }

    /// `AEEkHeap_FreeNode`: mark free and splice into the address-ordered list. The order is
    /// descending, because the node is inserted in front of the first free node above it.
    #[inline(never)]
    fn free_node<A: Arena>(&self, a: &mut A, node: u32, size: u32) {
        a.write_word(node, size | FREE);
        self.link(a, node);
    }

    /// The linking half of `AEEkHeap_FreeNode`, for a caller that wrote the size word itself.
    #[inline(never)]
    fn link<A: Arena>(&self, a: &mut A, node: u32) {
        let above = self.next_free(a, node);
        let after = a.read_word(above + 4);
        a.write_word(node + 8, after);
        a.write_word(node + 4, above);
        a.write_word(above + 4, node + 4);
        a.write_word(after, node + 4);
    }

    /// `AEEkHeap_CheckFreeNode`: is `node` (or the run it can swallow forward) big enough?
    ///
    /// Merging happens whether or not the answer is yes: each adjacent free node is unlinked and
    /// absorbed before the next test, which is why a scan compacts as it goes.
    #[inline(never)]
    fn check_free_node<A: Arena>(&self, a: &mut A, node: u32, need: u32) -> bool {
        let high = self.high(a);
        let at = node;
        loop {
            let size = Self::size(a, at);
            if size >= need {
                return true;
            }
            if size < NODE {
                return false;
            }
            // Both sums are of guest words: the step to the neighbour saturates so an absurd size
            // refuses the merge instead of wrapping back into the heap, and the accumulated size
            // word wraps for the same reason `push_node`'s absorb loop does.
            let next = at.saturating_add(size);
            if next.saturating_add(NODE) > high || !Self::is_free(a, next) {
                return false;
            }
            let absorbed = Self::size(a, next);
            if absorbed < NODE {
                return false;
            }
            self.unlink(a, next);
            a.write_word(at, a.read_word(at).wrapping_add(absorbed));
        }
    }

    /// `AEEkHeap_TrimNode`: split the tail off when it can pay for a header, then record the slack.
    #[inline(never)]
    fn trim<A: Arena>(&self, a: &mut A, node: u32, need: u32) {
        let want = Self::round(need) + NODE;
        let have = Self::size(a, node);
        if have >= want + MIN_SPLIT {
            a.write_word(node, want);
            self.free_node(a, node + want, have - want);
        }
        let extra = Self::size(a, node).saturating_sub(need + NODE);
        a.write_half(node + 10, extra as u16);
    }

    /// `AEEkHeap_Sbrk`: ask for `need` more bytes at the end and fold them into the last node.
    #[inline(never)]
    fn sbrk<A: Arena>(&self, a: &mut A, need: u32) -> bool {
        let old = self.high(a);
        if need == 0 || !a.grow(old, need) {
            return false;
        }
        self.set_high(a, old + need);
        // The node that owned the last byte. `AEEkHeap_Sbrk` walks it the same way.
        let mut at = self.base + HEADER;
        loop {
            let size = Self::size(a, at);
            if size < NODE {
                return false;
            }
            let next = at.saturating_add(size);
            if next >= old {
                break;
            }
            at = next;
        }
        let payload = Self::payload(a, at);
        let free = Self::is_free(a, at);
        a.write_word(at, a.read_word(at).wrapping_add(need));
        if !free {
            self.trim(a, at, payload);
        }
        true
    }

    // ---- the operations ----

    /// `AEEkHeap_Realloc(pHeap, 0, len)`: lowest-address first fit, split, grow, zero.
    #[inline(never)]
    pub fn alloc<A: Arena>(&self, a: &mut A, len: u32, clear: bool) -> Option<u32> {
        if len == 0 {
            return None;
        }
        let need = Self::round(len) + NODE;
        for _ in 0..64 {
            if let Some(node) = self.find(a, need) {
                self.take(a, node);
                self.trim(a, node, len);
                if clear {
                    a.zero(node + NODE, len);
                }
                return Some(node + NODE);
            }
            if self.sweep_right(a) {
                continue;
            }
            if !self.sbrk(a, need) {
                break;
            }
        }
        None
    }

    /// `AEEkHeap_Realloc(pHeap, p, 0)`: mark free and link. No merge, no clear.
    #[inline(never)]
    pub fn free<A: Arena>(&self, a: &mut A, ptr: u32) -> bool {
        let Some(node) = ptr.checked_sub(NODE) else {
            return false;
        };
        if !self.sane(a, node) || Self::is_free(a, node) {
            return false;
        }
        let size = Self::size(a, node);
        self.free_node(a, node, size);
        true
    }

    /// `AEEkHeap_Realloc(pHeap, p, len)`.
    ///
    /// The old node is put back on the list *before* the search, so the address a caller frees and
    /// immediately grows can come back to it. If nothing fits and the heap cannot grow, the block
    /// is restored where it was and the answer is null — `realloc` promises the old block survives.
    /// The answer is the pointer and whether the block was re-homed at all: the exact-size branch
    /// returns the old pointer untouched, and a caller recording handovers must not report one.
    #[inline(never)]
    pub fn realloc<A: Arena>(
        &self,
        a: &mut A,
        ptr: u32,
        len: u32,
        clear: bool,
    ) -> Option<(u32, bool)> {
        if ptr == 0 {
            return self.alloc(a, len, clear).map(|at| (at, true));
        }
        if len == 0 {
            self.free(a, ptr);
            return None;
        }
        let node = ptr.checked_sub(NODE)?;
        if !self.sane(a, node) {
            return None;
        }
        let old_payload = Self::payload(a, node);
        if old_payload == len {
            // The delivery compares the payload it is carrying with the request and returns the
            // pointer untouched: no trim, no split, no zeroing.
            return Some((ptr, false));
        }
        let need = Self::round(len) + NODE;
        let old_size = Self::size(a, node);
        let old_next = a.read_word(node + 4);
        let old_refs = a.read_word(node + 8);

        for _ in 0..2 {
            // Put the old node back on the list, exactly as `Realloc` does before searching.
            a.write_word(node, old_size | FREE);
            self.link(a, node);
            let victim = if self.check_free_node(a, node, need) {
                Some(node)
            } else {
                self.find(a, need)
            };
            if let Some(victim) = victim {
                let at = self.move_into(
                    a,
                    victim,
                    node,
                    ptr,
                    len,
                    old_payload,
                    old_next,
                    old_refs,
                    clear,
                );
                return Some((at, true));
            }
            // Nothing fits. Restore the block: either it is still a node of its own, or a merge
            // during the scan swallowed it and the run has to be split at its address.
            let owner = self.owner_of(a, node);
            match owner {
                Some(at) if at == node => {
                    a.write_word(node, Self::size(a, node));
                    self.unlink(a, node);
                }
                Some(at) => {
                    // Both operands come from guest words, so neither subtraction may underflow: a
                    // size that does not cover the node's offset is a corrupt chain, and refusing the
                    // request is the honest answer. Wrapping would write a size the walk cannot explain.
                    let Some(tail) = Self::size(a, at).checked_sub(node - at) else {
                        return None;
                    };
                    a.write_word(node, tail);
                    let Some(rest) = a.read_word(at).checked_sub(tail) else {
                        return None;
                    };
                    a.write_word(at, rest);
                }
                None => return None,
            }
            a.write_word(node + 4, old_next);
            a.write_word(node + 8, old_refs);
            self.trim(a, node, old_payload);
            if self.sweep_right(a) {
                continue;
            }
            if !self.sbrk(a, need) {
                return None;
            }
        }
        None
    }

    /// The tail of a move: unlink the node that will hold the payload, hand it the old node's
    /// bookkeeping, carry the bytes over, and trim.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn move_into<A: Arena>(
        &self,
        a: &mut A,
        victim: u32,
        from: u32,
        ptr: u32,
        len: u32,
        old_payload: u32,
        old_next: u32,
        old_refs: u32,
        clear: bool,
    ) -> u32 {
        self.unlink(a, victim);
        a.write_word(victim, Self::size(a, victim));
        a.write_half(victim + 8, old_refs as u16);
        a.write_word(victim + 4, old_next);
        if victim != from {
            a.copy(victim + NODE, ptr, old_payload);
        }
        self.trim(a, victim, len);
        if clear && old_payload < len {
            a.zero(victim + NODE + old_payload, len - old_payload);
        }
        victim + NODE
    }

    /// The node whose span covers `at`, found by walking the heap in address order.
    #[inline(never)]
    fn owner_of<R: ReadArena>(&self, a: &R, at: u32) -> Option<u32> {
        let high = self.high(a);
        if at < self.base + HEADER || at >= high {
            return None;
        }
        let mut node = self.base + HEADER;
        loop {
            let size = Self::size(a, node);
            if size < NODE {
                return None;
            }
            let next = node.saturating_add(size);
            if next > at {
                return Some(node);
            }
            if next >= high {
                return None;
            }
            node = next;
        }
    }

    /// Remove a free node from the list and hand it to a caller.
    #[inline(never)]
    fn take<A: Arena>(&self, a: &mut A, node: u32) {
        self.unlink(a, node);
        a.write_word(node, Self::size(a, node));
        // Newly allocated nodes have a lock count of 1 and no sentinel (`AEEkHeap.h`).
        a.write_half(node + 8, 1);
        a.write_word(node + 4, 0);
    }

    /// `AEEkHeap_PushNode`: move an unlocked, sentinel-marked node into a free run above it.
    /// The old node goes back on the list; the new node keeps the old lock count, sentinel word and
    /// payload, and any pointer stored in that sentinel is rebased by the move.
    #[inline(never)]
    fn push_node<A: Arena>(&self, a: &mut A, source: u32) -> u32 {
        if Self::is_free(a, source) {
            return source;
        }
        let source_size = Self::size(a, source);
        let payload = Self::payload(a, source);
        let need = Self::round(payload) + NODE;
        let high = self.high(a);
        let mut at = source.saturating_add(source_size);
        let mut candidate = None;
        while at.saturating_add(NODE) <= high {
            let size = Self::size(a, at);
            if size < NODE {
                break;
            }
            if Self::is_free(a, at) && self.check_free_node(a, at, need) {
                candidate = Some(at);
                break;
            }
            at = at.saturating_add(Self::size(a, at));
        }
        let Some(free) = candidate else {
            return source;
        };
        // `CheckFreeNode` may have stopped as soon as it had enough; the original then absorbs
        // every further free neighbour before splitting.
        loop {
            let end = free.saturating_add(Self::size(a, free));
            if end.saturating_add(NODE) > high || !Self::is_free(a, end) {
                break;
            }
            let extra = Self::size(a, end);
            self.unlink(a, end);
            a.write_word(free, a.read_word(free).wrapping_add(extra));
        }
        let have = Self::size(a, free);
        let new_node = if have < need.saturating_add(NODE) {
            self.unlink(a, free);
            a.write_word(free, have);
            free
        } else {
            let lower = have - need;
            a.write_word(free, lower | FREE);
            let new_node = free + lower;
            a.write_word(new_node, need);
            new_node
        };
        let lock = a.read_half(source + 8);
        let sentinel = a.read_word(source + 4);
        a.write_half(new_node + 8, lock);
        a.write_word(new_node + 4, sentinel);
        a.write_half(new_node + 10, (need - NODE - payload) as u16);
        a.copy(new_node + NODE, source + NODE, payload);
        if sentinel != 0 {
            let delta = new_node + NODE - (source + NODE);
            let pointer = a.read_word(sentinel);
            a.write_word(sentinel, pointer.wrapping_add(delta));
        }
        self.free_node(a, source, source_size);
        new_node
    }

    /// `AEEkHeap_SweepRight`: repeatedly move the last unlocked, sentinel-marked node that has a
    /// free run above it. This is the delivery's only compaction pass; it runs when allocation has
    /// already failed, not on a timer.
    #[inline(never)]
    pub fn sweep_right<A: Arena>(&self, a: &mut A) -> bool {
        let high = self.high(a);
        let mut last = 0u32;
        let mut moved = false;
        let mut guard = (high.saturating_sub(self.base) / NODE) + 2;
        loop {
            let mut at = self.base + HEADER;
            let mut candidate = 0u32;
            while at != last && at.saturating_add(NODE) <= high && guard > 0 {
                guard -= 1;
                let size = Self::size(a, at);
                if size < NODE {
                    break;
                }
                if !Self::is_free(a, at) && a.read_half(at + 8) == 0 && a.read_word(at + 4) != 0 {
                    candidate = at;
                }
                at = at.saturating_add(size);
            }
            if candidate == 0 || guard == 0 {
                break;
            }
            let to = self.push_node(a, candidate);
            last = to;
            moved |= to != candidate;
        }
        moved
    }

    /// The lowest-address free node that `CheckFreeNode` says can serve `need`.
    #[inline(never)]
    fn find<A: Arena>(&self, a: &mut A, need: u32) -> Option<u32> {
        let sentinel = self.base + SENTINEL;
        // The search starts at the sentinel's `prev` link — the list is kept descending, so that
        // is the lowest address — and follows `prev`, i.e. upward.
        let mut handle = a.read_word(sentinel);
        let mut steps = (self.high(a).saturating_sub(self.base) / NODE) + 1;
        while handle != sentinel && steps > 0 {
            steps -= 1;
            if handle < self.base + HEADER + 4 || (handle - self.base) % 4 != 0 {
                return None;
            }
            let node = handle - 4;
            if self.check_free_node(a, node, need) {
                return Some(node);
            }
            handle = a.read_word(handle);
        }
        None
    }

    /// An invariant check: every free-list handle names a node start, every free node is on the list
    /// exactly once, and the address walk reaches `high` without meeting a word it cannot explain.
    /// Returns the first disagreement, in words.
    ///
    /// A test runs it from `Runtime::drop`, so every test's traffic is checked. A run calls it when
    /// the heap refuses an allocation (below) and at a fault, which is when the question is worth its
    /// walk: "was this heap merely full, or has something written over a node header?"
    pub fn audit<R: ReadArena>(&self, a: &R) -> Result<(), String> {
        let high = self.high(a);
        let mut starts = std::collections::HashMap::new();
        let mut node = self.base + HEADER;
        let mut steps = (high.saturating_sub(self.base) / NODE) + 1;
        // The last few nodes, so a walk that stops on a word it cannot explain names where it had
        // been: the culprit is usually one of them, not the word it finally tripped over.
        let mut trail: Vec<(u32, u32, u32)> = Vec::new();
        let trail_text = |trail: &[(u32, u32, u32)]| {
            trail
                .iter()
                .map(|(at, word, size)| format!("{at:#x}:{word:#x}={size:#x}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        while node.saturating_add(NODE) <= high && steps > 0 {
            steps -= 1;
            let word = a.read_word(node);
            let size = 2 * (word >> 1);
            if size < NODE {
                return Err(format!(
                    "node {node:#x} has word {word:#x} (size {size:#x}); the walk before it was [{}]                      and high is {high:#x}",
                    trail_text(&trail)
                ));
            }
            if node.saturating_add(size) > high {
                return Err(format!(
                    "node {node:#x} size {size:#x} runs past {high:#x}; the walk before it was [{}]",
                    trail_text(&trail)
                ));
            }
            trail.push((node, word, size));
            if trail.len() > 8 {
                trail.remove(0);
            }
            starts.insert(node, (size, Self::is_free(a, node)));
            node = node.saturating_add(size);
        }
        if node != high {
            return Err(format!("the walk ended at {node:#x}, not {high:#x}"));
        }
        let sentinel = self.base + SENTINEL;
        let mut handle = a.read_word(sentinel + 4);
        let mut seen = std::collections::HashSet::new();
        let mut count = 0;
        while handle != sentinel {
            count += 1;
            if count > starts.len() + 1 {
                return Err("the free list cycles".into());
            }
            let Some(at) = handle.checked_sub(4) else {
                return Err(format!("handle {handle:#x} has no node"));
            };
            match starts.get(&at) {
                Some((_, true)) => {}
                Some((size, false)) => {
                    return Err(format!(
                        "node {at:#x} (size {size:#x}) is on the list but allocated"
                    ))
                }
                None => {
                    return Err(format!(
                        "handle {handle:#x} names {at:#x}, which is no node"
                    ))
                }
            }
            if !seen.insert(at) {
                return Err(format!("node {at:#x} is on the list twice"));
            }
            let next = a.read_word(handle + 4);
            if a.read_word(next) != handle {
                return Err(format!("link from {at:#x} is not back-linked"));
            }
            handle = next;
        }
        let free_in_chain = starts.values().filter(|(_, free)| *free).count();
        if free_in_chain != seen.len() {
            return Err(format!(
                "{free_in_chain} free node(s) in the heap, {} on the list",
                seen.len()
            ));
        }
        Ok(())
    }

    /// Every node the chain holds, as `(node address, total size, whether it is free)` — the walk
    /// `AEEHeap_DumpGroup` makes before it prints anything. Bounded the same way [`audit`](Self::audit)
    /// is: a chain whose size word has been written over stops the walk instead of spinning it, so a
    /// caller that dumps a broken heap gets the nodes up to the damage rather than never returning.
    ///
    /// Only a *validated* heap should be read with this; a walk that stops early is indistinguishable
    /// from a short heap, which is why `AEEkHeap_Validate` runs first in the delivered code too.
    pub fn walk<R: ReadArena>(&self, a: &R) -> Vec<(u32, u32, bool)> {
        let mut out = Vec::new();
        let mut node = self.base + HEADER;
        let high = self.high(a);
        let mut steps = (high.saturating_sub(self.base) / NODE) + 1;
        while node.saturating_add(NODE) <= high && steps > 0 {
            steps -= 1;
            let size = Self::size(a, node);
            if size < NODE {
                break;
            }
            out.push((node, size, Self::is_free(a, node)));
            node = node.saturating_add(size);
        }
        out
    }

    /// Total bytes a fresh walk finds on the free list. Used by the run report, not by the policy.
    pub fn spare<R: ReadArena>(&self, a: &R) -> u32 {
        let mut total: u32 = 0;
        let mut node = self.base + HEADER;
        let high = self.high(a);
        let mut steps = (high.saturating_sub(self.base) / NODE) + 1;
        while node.saturating_add(NODE) <= high && steps > 0 {
            steps -= 1;
            let size = Self::size(a, node);
            if size < NODE {
                break;
            }
            if Self::is_free(a, node) {
                total = total.saturating_add(size);
            }
            node = node.saturating_add(size);
        }
        total
    }

    /// How many blocks sit on the free list.
    pub fn free_blocks<R: ReadArena>(&self, a: &R) -> usize {
        let mut n = 0;
        let mut node = self.base + HEADER;
        let high = self.high(a);
        let mut steps = (high.saturating_sub(self.base) / NODE) + 1;
        while node.saturating_add(NODE) <= high && steps > 0 {
            steps -= 1;
            let size = Self::size(a, node);
            if size < NODE {
                break;
            }
            if Self::is_free(a, node) {
                n += 1;
            }
            node = node.saturating_add(size);
        }
        n
    }
}

#[cfg(test)]
#[path = "test/heap.rs"]
mod tests;
