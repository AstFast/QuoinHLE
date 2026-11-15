//! The guest's 32-bit address space.
//!
//! Page-backed and sparse, little-endian, and carrying the guest heap exactly as the delivery does:
//! The delivered body
//! and [`crate::heap`] is the body that writes them. A real handset carves that arena out of
//! `IShell`'s private memory; here [`Mem::set_heap`] is where it is handed over.
//! Page 0 is deliberately left unmapped so a null `PSHELL` or a bogus vtable pointer faults
//! loudly instead of quietly reading zeros.

use std::cell::{Cell, RefCell};
use std::fmt;

use crate::heap::{self, Arena, HeapState, ReadArena};

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: u32 = (PAGE_SIZE as u32) - 1;

/// Super-page shift: 1 MiB, so 256 4 KiB pages share one contiguous run of storage (see [`Super`]).
const SUPER_SHIFT: u32 = 20;
const SUPER_SIZE: usize = 1 << SUPER_SHIFT;
const SUPER_PAGES: usize = SUPER_SIZE / PAGE_SIZE;

pub type MResult<T> = Result<T, Fault>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Touching an address that was never mapped.
    Unmapped { addr: u32 },
    /// The heap ran out: `MALLOC` would answer NULL, which a handset also does here.
    HeapExhausted { want: u32 },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Unmapped { addr } => write!(f, "unmapped guest address {addr:#010x}"),
            Fault::HeapExhausted { want } => {
                write!(f, "guest heap exhausted while requesting {want:#x} bytes")
            }
        }
    }
}

impl std::error::Error for Fault {}

/// How many 1 MiB super pages one allocation reserves for a region (= 16 MiB of contiguous host
/// memory).
///
/// This number decides how many spans the JIT windows can see. The heap grows a page at a time, and
/// if every super page were allocated on its own a 5 MiB heap would be **five runs of host memory
/// that are not adjacent**: five window slots gone, and the stack pushed to the end of the
/// comparison chain — measured, that dropped stack access from 1.12 ns/instruction to 1.99
/// (`benches/jit_frame.rs`). One span, allocated in one go and cut into 1 MiB super pages, puts the
/// heap back to a single span.
///
/// The 16 MiB is **virtual address space**, not resident memory: on a block this size `vec![0u8; n]`
/// goes through the operating system's lazy zero page, so only pages actually written cost physical
/// memory. Being generous here is therefore free — a heap grown to 16 MiB is still one span, and the
/// surplus that a 351 KiB module never touches costs not one physical page.
const SPAN_SUPERS: usize = 16;

/// One contiguous run of host memory, cut into `SPAN_SUPERS` super pages of 1 MiB.
///
/// `bytes` is a `Box<[u8]>`, whose contents never move once allocated (growing `Vec<Span>` moves
/// only the `Span` shells), so the bases recorded by the super pages stay valid for the life of the
/// span — the invariant that `page_table` and the JIT window contract both rest on.
struct Span {
    /// The super-page index range covered, `[first, first + count)`.
    first: usize,
    count: usize,
    bytes: Box<[u8]>,
}

impl Span {
    fn new(first: usize) -> Self {
        let count = SPAN_SUPERS;
        Self {
            first,
            count,
            bytes: vec![0u8; count * SUPER_SIZE].into_boxed_slice(),
        }
    }

    /// If `si` falls inside this span, its 1 MiB base.
    #[inline]
    fn slot(&self, si: usize) -> Option<usize> {
        (si >= self.first && si < self.first + self.count)
            .then(|| self.bytes.as_ptr() as usize + (si - self.first) * SUPER_SIZE)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The 16 MiB themselves are not printed: what is wanted here is which super pages this span
        // covers.
        f.debug_struct("Span")
            .field("first_super", &self.first)
            .field("supers", &self.count)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

/// Find the 1 MiB base of `si` in a span table. A free function because `read_state` has to call
/// it on a local span table that has not been handed to `Mem` yet.
fn span_slot(spans: &[Span], si: usize) -> Option<usize> {
    spans.iter().find_map(|s| s.slot(si))
}

/// One 1 MiB run of **contiguous** storage, covering 256 consecutive 4 KiB pages.
///
/// The memory itself is held by [`Span`]; this only records where in the span it starts. Saving,
/// watchpoints and fault semantics are unchanged — what changed is that adjacent super pages now
/// land in the same run of host memory, so the JIT can fold them into one window instead of one per
/// super page.
struct Super {
    /// This super page's 1 MiB base. A `usize` rather than a bare pointer, for the same reason as
    /// `page_table`: `Runtime` is moved into a worker thread by `thread::spawn`, and a bare pointer
    /// would infect it with `!Send`.
    base: usize,
    /// One bit per page: 1 = that 4 KiB page is mapped. An unmapped page **must** report
    /// [`Fault::Unmapped`] (leaving page 0 unmapped is how a null pointer gets caught), so the bitmap
    /// is the authority and whatever bytes are still lying around do not count.
    mapped: [u64; SUPER_PAGES / 64],
}

impl fmt::Debug for Super {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mapped = self
            .mapped
            .iter()
            .map(|w| w.count_ones() as usize)
            .sum::<usize>();
        f.debug_struct("Super")
            .field("mapped_pages", &mapped)
            .finish()
    }
}

impl Super {
    fn new_at(base: usize) -> Self {
        Self {
            base,
            mapped: [0; SUPER_PAGES / 64],
        }
    }

    /// The byte view of this 1 MiB.
    ///
    /// SAFETY: `base` comes from `Span::new` and points at one whole contiguous 1 MiB inside that
    /// span; the span is owned by `Mem`, and a `Box<[u8]>`'s contents do not move before it is
    /// dropped.
    #[inline]
    fn bytes(&self) -> &[u8; SUPER_SIZE] {
        unsafe { &*(self.base as *const [u8; SUPER_SIZE]) }
    }

    /// Mutable view, on the same terms.
    #[inline]
    fn bytes_mut(&mut self) -> &mut [u8; SUPER_SIZE] {
        unsafe { &mut *(self.base as *mut [u8; SUPER_SIZE]) }
    }

    #[inline]
    fn mapped_bit(&self, page: usize) -> bool {
        self.mapped[page >> 6] >> (page & 63) & 1 != 0
    }

    #[inline]
    fn set_mapped_bit(&mut self, page: usize) {
        self.mapped[page >> 6] |= 1 << (page & 63);
    }

    #[inline]
    fn clear_mapped_bit(&mut self, page: usize) {
        self.mapped[page >> 6] &= !(1 << (page & 63));
    }
}

/// What wrote a watched byte. The distinction is the whole point of the watchpoint: "this field
/// was never filled in" has exactly two possible culprits, and they want different fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchBy {
    /// Guest code at this address, with the encoding that did it.
    Guest { pc: u32, insn: u32 },
    /// A guest instruction that trapped into a host service, and the access happened inside that
    /// service — `AEEHelperFuncs::memmove` is the case that matters. Attributed to the instruction
    /// that ordered the copy, because a watched buffer whose every write reads back as bare `host`
    /// cannot say who asked for it, and a pack that reassembles its own archives does all of its
    /// moving through here.
    Helper { pc: u32, insn: u32 },
    /// The host, with no guest instruction behind the access: the loader's own copies, and whatever
    /// the emulator writes between one downcall and the next.
    Host,
}

/// The host call an access happened inside: what the pack asked for, in its own words.
///
/// A watchpoint answers "who wrote these bytes" twice over — the trapped guest instruction, and the host
/// method that wrote once it was in; only the second says whether content, length or destination was the
/// wrong one, and it cannot be joined afterwards: the two records lose **opposite ends** (the call log
/// drops its oldest half, `hle.rs:662`; this one is first-kept, `mem.rs:464`).
#[derive(Debug, Clone, PartialEq)]
pub struct WatchCall {
    /// Interned by `abi::label`, so stamping the in-flight call onto a watched store costs no
    /// allocation even when a watchpoint is armed over a busy blit.
    pub label: &'static str,
    pub args: Vec<u32>,
}

impl WatchCall {
    /// The argument words as one field, in the caller's own order (`this` first on an interface
    /// call). The report prints it in two places — the byte record and the arena record — and those
    /// two must not drift into different shapes, or a reader cannot tell them apart.
    pub fn words(&self) -> String {
        self.args
            .iter()
            .map(|a| format!("{a:#010x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// One access that touched the watched bytes.
#[derive(Debug, Clone)]
pub struct WatchHit {
    pub addr: u32,
    /// The byte before the access and the byte after it. A read leaves the two equal, which is what
    /// lets one pair of fields serve both directions instead of a second struct.
    pub old: u8,
    pub new: u8,
    pub by: WatchBy,
    /// Which of the `--watch` spans this access fell inside. With more than one address armed, a log
    /// that cannot say which one was touched is a log of two questions with one answer.
    pub span: (u32, u32),
    /// Which way the access went. A watchpoint answers two different questions — "did anything write
    /// here" and "did anything read here" — and a log that cannot tell them apart answers neither.
    pub read: bool,
    /// The host call in flight, if this happened inside one. `None` for guest code and for the
    /// emulator's own writes, which have no call to name. The one name that reaches the report is
    /// the one no window can drop: a watch line that says `IFile::Read(0x200635e8, 0xac,...)` names
    /// the method, the destination and the length in the same breath as the value it wrote.
    pub call: Option<WatchCall>,
}

impl WatchHit {
    /// The call to record for an access attributed to `by`. A downcall runs guest instructions while a
    /// host call is still in flight, and the CPU re-asserts [`WatchBy::Guest`] for each of them — so
    /// the call in flight is only a true witness for the accesses the *host* made. Naming the outer
    /// `IFile::Read` on a store the pack's own code issued would blame the wrong party outright.
    pub(crate) fn call_for(by: WatchBy, in_flight: &Option<WatchCall>) -> Option<WatchCall> {
        match by {
            WatchBy::Guest { .. } => None,
            _ => in_flight.clone(),
        }
    }
}

/// An arena event that landed inside an armed `--watch` span: space handed out, space given back.
///
/// A watchpoint on a pointer slot answers "who wrote these bytes", and that is where a watchpoint
/// stops: a pack that releases a block it has already handed back needs the *other* half of the question,
/// and `alloc`/`free` move no bytes, so no access record can ever name them. The handover is
/// therefore recorded beside the accesses, in the record that does not drop its oldest entries.
#[derive(Debug, Clone)]
pub struct WatchArena {
    /// `"alloc"`, `"free"`, or `"free-refused"` — the last being a `FREE(p)` for space this arena
    /// never owned, including the second release of a block already released.
    pub event: &'static str,
    pub start: u32,
    /// Always the real size except for `"free-refused"`, which has none to claim and is recorded
    /// with `len = 0` so it is matched by its address alone.
    pub len: u32,
    /// Where the bump cursor stood at the moment of the event. With the delivered allocator the span
    /// is fixed, so this only ever moves when the heap grows; it places the event on the arena's own
    /// timeline and is what makes `returned_at` meaningful.
    pub cursor: u32,
    /// For an `alloc` served from a block that had been given back: the instruction count when that
    /// block was freed. Its presence is the answer to "was this recycled space or space the heap had
    /// not handed out before", which a byte watch cannot distinguish at all — both look like a write
    /// of new bytes.
    pub returned_at: Option<u64>,
    /// Guest instructions retired when it happened. `None` when none had retired yet — the loader's
    /// own allocations, and any host call taken before the interpreter ran its first instruction.
    /// The counter is only kept while a watchpoint is armed, in the same branch that sets
    /// [`Mem::watch_by`].
    pub steps: Option<u64>,
    /// The host call responsible, when there was one: `AEEHelperFuncs::free` (the pack's own `FREE`)
    /// and an internal release the host made on its own initiative are different parties, and the
    /// whole question is which of the two handed this space back.
    pub call: Option<WatchCall>,
    /// What the heap's own bookkeeping said **right after** this handover, when it disagreed with
    /// itself: the first event whose chain is broken is the one that dates the damage, and every
    /// allocation that found no room afterwards is a symptom of it.
    pub chain: Option<String>,
}

/// How many hits to keep. A watched word inside a framebuffer would otherwise be logged per pixel.
const WATCH_LIMIT: usize = 4096;

#[derive(Debug)]
pub struct Mem {
    /// The super-page table, indexed by `addr >> SUPER_SHIFT`.
    ///
    /// The super pages no longer hold bytes themselves: they point into a run in [`Mem::spans`]. A
    /// `Box<[u8]>`'s contents never move once allocated (growing a `Vec` moves the shells), so a
    /// pointer handed out by `page_ptr` stays valid for the whole life of the super page — exactly
    /// the stability the JIT window contract needs.
    ///
    /// The bitmap (`Super::mapped`) is the authority on "is this page mapped"; `map`/`unmap` both
    /// advance [`Mem::map_generation`], and only events that can invalidate a **published JIT window**
    /// advance [`Mem::window_generation`].
    supers: Vec<Option<Super>>,
    /// The unit of host allocation: one contiguous run of [`SPAN_SUPERS`] super pages.
    ///
    /// Append-only and as long-lived as `supers` — `unmap` clears the bitmap but keeps the span, so a
    /// remap gets the same memory back (semantics identical to the old "zero the bytes and clear the
    /// bit"), and the pointers baked into JIT windows therefore never dangle.
    spans: Vec<Span>,
    /// Software page table: page number → page byte base (0 = unmapped). The bitmap is still the
    /// authority on "mapped"; this table is its projection, kept in step by `map`/`unmap`/
    /// `read_state`.
    ///
    /// Why it exists: after the super-page change every access walked six checks (`Vec` bound, `Option`
    /// unwrap, bit test, two slice bounds, `try_into`) and the interpreter fell to 56.4/41.8
    /// (66.2/59.4 before super pages). A single-entry memo does not save it — the fetch page and the
    /// data page alternate in a data/stack loop, so the hit rate approaches zero (measured 56.4 →
    /// 56.3, no gain). A flat table is one index plus one zero test, the same order as the old
    /// per-page `Box` table.
    ///
    /// Pointers are stored as `usize` for `Send`: `Runtime` is moved by `thread::spawn` into the
    /// audio/session threads, and raw pointer types would infect it with `!Send`. The base does not
    /// change while the page lives (a super page's `Box` contents never move; growing a `Vec` moves
    /// the shells), so dereferencing is sound.
    page_table: Vec<usize>,
    /// Layout generation: every `map`/`unmap` that really adds or removes pages bumps it, describing
    /// "how many times the layout changed". The JIT does **not** use it as an invalidation gate —
    /// see [`Mem::window_generation`].
    map_generation: u64,
    /// JIT window invalidation generation: bumped only when a **published window may have gone
    /// stale** — `unmap` digging pages out, `read_state` replacing a whole super page's storage, or
    /// arming a watchpoint that changes the window gating.
    ///
    /// Mapping a new page deliberately does **not** advance it: the JIT freezes the window set at
    /// publish time (the `Bridge` cache in `cpu/jit.rs`), pages inside a compiled span can only grow
    /// in number and never disappear, and the host pointers do not move; a new page falls outside the
    /// frozen windows, so its accesses drop back to the `Memory` callback on their own. If `map`
    /// advanced it too, every new heap page would invalidate the whole code cache (about 21 times in
    /// 300 ms of PvZ virtual time, each one recompiling every hot block).
    window_generation: u64,
    /// The delivered heap's two constants. Everything else about the heap — the header, every node,
    /// the free list and the high-water — lives in the guest's own pages, because that is where
    /// `AEEkHeap` keeps it and a pack that peeks at `p - 12` sees the same words a handset wrote.
    heap: HeapState,
    /// When each block was given back, by node address, so a report can say whether the space a
    /// later allocation landed on is fresh or recycled. Report-only: nothing in the policy reads it.
    freed_at: std::collections::HashMap<u32, u64>,
    /// Highest byte written, used to size a `.mod`'s data area when dumping state.
    pub touch_high: u32,
    /// The node headers of the chain as it stands, ascending. Test builds only, and only so
    /// [`Mem::guard_header`] can answer "does this store land on one" in a binary search: a node
    /// header is twelve bytes of the heap's own metadata, and a guest or host store into one breaks
    /// the chain in a way the *next* allocation reports from somewhere else entirely.
    #[cfg(test)]
    heap_heads: RefCell<Vec<u32>>,

    /// How many allocations the arena refused over the run. See [`Mem::alloc`].
    pub alloc_fails: u32,
    /// The largest single request among those refusals, which is what says whether the arena is merely
    /// tight or something asked for the moon.
    pub alloc_fail_max: u32,
    /// `--watch`: the half-open byte spans whose accesses are logged. A single word is the usual ask;
    /// a span answers the different question "does anything write into this region at all", which is
    /// how one asks whether the guest treats a loaded image as data. More than one may be armed,
    /// because the question that needs a watchpoint is usually "which of these three writers touched
    /// this", and arming them one run at a time cannot tell a shared cause from a coincidence.
    pub watch: Vec<(u32, u32)>,
    /// What the run was executing at the moment of the last write, refreshed by the CPU before each
    /// instruction and cleared by the HLE layer while it services a trap.
    pub watch_by: WatchBy,
    /// `--watch-stop`: whether the run should stop the first time a watched byte is touched, and
    /// whether it has. A watchpoint that only keeps a log answers "did anything happen" some time
    /// later, by which point the registers that made the access interesting have been overwritten;
    /// stopping where it happened turns the same watchpoint into a place to stand and look.
    pub watch_stop: bool,
    /// `--watch-chain`: whether each recorded arena handover also walks the free chain. The walk is
    /// the only thing that can date the damage (`chain` names the first node that disagrees), but it
    /// covers the whole arena, and a pack allocates on every frame — so with a wide `--watch` span the
    /// recording costs more than the run it is recording. Off by default; the record is the same
    /// apart from that one verdict.
    pub watch_chain: bool,
    /// Where the watched access that stopped the run happened, or `None` while it has not. A `Cell`
    /// because the read path is `&self`, and a read can be the very thing that trips it.
    pub watch_tripped: Cell<Option<u32>>,
    /// The accesses observed, oldest first. A `RefCell` because reads arrive through `&self` — the
    /// fetch of every instruction is one — and making them `&mut self` would be a change to the
    /// hottest path in the emulator for the sake of a debugging flag.
    pub watch_hits: RefCell<Vec<WatchHit>>,
    /// The host call in flight, set by the dispatch around the handler so a hit recorded inside it
    /// can say which method and with what argument words. A `RefCell` because the read path is
    /// `&self`, like [`Mem::watch_hits`].
    pub watch_call: RefCell<Option<WatchCall>>,
    /// Arena events inside an armed span. [`Mem::note_arena`] is the only writer.
    pub watch_arena: RefCell<Vec<WatchArena>>,
    /// Guest instructions retired, mirrored here so an arena record can be placed on the same
    /// timeline as the byte records. Only ever written while a watchpoint is armed, in the same
    /// branch that sets [`Mem::watch_by`].
    pub watch_steps: Cell<u64>,
    /// Set once the arena record reached [`WATCH_LIMIT`] and started dropping events.
    pub watch_arena_capped: Cell<bool>,
}

impl Default for Mem {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem {
    pub fn new() -> Self {
        Mem {
            supers: Vec::new(),
            spans: Vec::new(),
            page_table: Vec::new(),
            map_generation: 0,
            window_generation: 0,
            heap: HeapState::default(),
            freed_at: std::collections::HashMap::new(),
            touch_high: 0,
            #[cfg(test)]
            heap_heads: RefCell::new(Vec::new()),

            alloc_fails: 0,
            alloc_fail_max: 0,
            watch: Vec::new(),
            watch_by: WatchBy::Host,
            watch_stop: false,
            watch_chain: false,
            watch_tripped: Cell::new(None),
            watch_hits: RefCell::new(Vec::new()),
            watch_call: RefCell::new(None),
            watch_arena: RefCell::new(Vec::new()),
            watch_steps: Cell::new(0),
            watch_arena_capped: Cell::new(false),
        }
    }

    /// Give the guest heap its span, and `AEEkHeap_Init` it: a twenty-byte header and one free node
    /// covering everything after it. The initial slice's pages are brought in by `init` itself,
    /// because the whole slice is writable on a handset; growth past it (`Sbrk`) maps what it adds.
    pub fn set_heap(&mut self, base: u32, limit: u32) {
        self.set_heap_with_initial(base, limit, heap::INITIAL);
    }

    /// As [`Mem::set_heap`], for a run that names how much of the span the first free node covers.
    /// Only the `Sbrk` boundary moves: the ceiling is still `limit`, and every address the heap can
    /// hand out is the same.
    pub fn set_heap_with_initial(&mut self, base: u32, limit: u32, initial: u32) {
        let heap = HeapState::new_with_initial(base, limit, initial);
        self.map(base, heap::HEADER);
        {
            let mut arena = GuestArena(self);
            heap.init(&mut arena);
        }
        self.heap = heap;
        #[cfg(test)]
        self.refresh_heads();
    }

    /// The top of the heap's span, as the run was given it. `AEEHeap_GetRAMFree`-style answers and
    /// the end-of-run report both read this rather than the compile-time constant, so a developer
    /// flag that moves it is visible in both.
    pub fn heap_limit(&self) -> u32 {
        self.heap.limit
    }

    /// How far the heap has been grown: `AEEkHeap`'s own high-water, read out of the header.
    pub fn heap_high(&self) -> u32 {
        self.heap.high(self)
    }

    /// Where the arena starts.
    pub fn heap_floor(&self) -> u32 {
        self.heap.base
    }

    /// How many watched writes are kept before the log stops. `--watch` reports it so a reader knows
    /// whether they are looking at the whole record or the start of one.
    pub fn watch_limit(&self) -> usize {
        WATCH_LIMIT
    }

    /// Watch the four bytes of one word, which is what `--watch ADDR` means. Repeatable, and a second
    /// word is not a replacement for the first.
    pub fn watch_word(&mut self, addr: u32) {
        let at = addr & !3;
        self.watch_span(at, 4);
    }

    /// Watch a span of bytes. Arming this before a load would fill the log with the loader's own
    /// copy, so the CLI arms it once the image is in place.
    pub fn watch_span(&mut self, start: u32, len: u32) {
        let span = (start, start.saturating_add(len));
        if !self.watch.contains(&span) {
            self.watch.push(span);
            // Arming drops any window overlapping that span from the JIT's set; the published set
            // changed, so blocks have to be recompiled (on a normal run the gate stops a watch from
            // being armed in the first place — this pins down the "arm one while it runs" path too).
            self.window_generation = self.window_generation.wrapping_add(1);
        }
    }

    /// Whether a byte address falls inside any watched span.
    pub fn watch_contains(&self, addr: u32) -> bool {
        self.watch_spans(addr).is_some()
    }

    /// The watched span a byte address falls inside, which is the one worth naming in a report.
    pub fn watch_spans(&self, addr: u32) -> Option<(u32, u32)> {
        self.watch
            .iter()
            .copied()
            .find(|&(lo, hi)| addr >= lo && addr < hi)
    }

    /// The layout generation: `map`/`unmap` bump it. It describes "how many times the layout
    /// changed", and it is **not** the JIT's invalidation gate (that one is
    /// [`Mem::window_generation`]).
    pub fn map_generation(&self) -> u64 {
        self.map_generation
    }

    /// Invalidation generation of the published JIT windows. The JIT compares it before executing
    /// and, when it moved, calls `invalidate_code_cache` and republishes the windows.
    ///
    /// The difference from [`Mem::map_generation`] is deliberate: mapping a new page changes no page
    /// or pointer inside any published window, the frozen set is still correct and needs no
    /// invalidation; only events that invalidate a window's description or its pointers — `unmap`,
    /// `read_state`, arming a watchpoint — advance it.
    pub fn window_generation(&self) -> u64 {
        self.window_generation
    }

    pub fn is_mapped(&self, addr: u32) -> bool {
        self.page(addr).is_some()
    }

    pub fn mapped_pages(&self) -> usize {
        self.supers
            .iter()
            .flatten()
            .map(|sup| {
                sup.mapped
                    .iter()
                    .map(|w| w.count_ones() as usize)
                    .sum::<usize>()
            })
            .sum()
    }

    /// How many of the `len` bytes at `addr` are actually there, stopping at the first byte of a page
    /// that was never mapped.
    ///
    /// A caller that reads a fixed-size header out of a buffer it was never told the length of is
    /// reading past the end whenever the container held less than a header — and whether that turns out
    /// harmless or fatal then depends on where the allocator happened to place the block, which is a
    /// fact about this host and not about the pack. Answering "the bytes run out here" lets the caller
    /// refuse on the shape of what it was given instead of faulting on an address.
    pub fn mapped_run(&self, addr: u32, len: u32) -> u32 {
        let mut run = 0u32;
        while run < len {
            let at = addr.wrapping_add(run);
            if self.page(at).is_none() {
                break;
            }
            // Everything to the end of this page is present, so step to the next one rather than
            // testing a byte at a time.
            run = (at & !PAGE_MASK) + PAGE_SIZE as u32 - addr;
        }
        run.min(len)
    }

    /// The page an address lives in, or nothing when it has never been mapped.
    ///
    /// One `Vec` bound check and one zero test, the same order as the old per-page `Box` table; the
    /// bitmap is still used by [`Self::fast_spans`] and by state dumps.
    #[inline]
    fn page(&self, addr: u32) -> Option<&[u8; PAGE_SIZE]> {
        let base = *self.page_table.get((addr >> PAGE_SHIFT) as usize)?;
        if base == 0 {
            return None;
        }
        // SAFETY: a non-zero entry means the page is mapped and the base is valid; the `Box` the
        // base points into does not move while the page lives (see the `page_table` docs).
        Some(unsafe { &*(base as *const [u8; PAGE_SIZE]) })
    }

    #[inline]
    fn page_mut(&mut self, addr: u32) -> Option<&mut [u8; PAGE_SIZE]> {
        let base = *self.page_table.get((addr >> PAGE_SHIFT) as usize)?;
        if base == 0 {
            return None;
        }
        // SAFETY: as for `page`; `&mut self` is exclusive, so promoting to mutable cannot alias.
        Some(unsafe { &mut *(base as *const [u8; PAGE_SIZE] as *mut [u8; PAGE_SIZE]) })
    }

    /// The byte an address lives in, or the fault the guest would take.
    #[inline]
    fn at(&self, addr: u32) -> MResult<&[u8; PAGE_SIZE]> {
        self.page(addr).ok_or(Fault::Unmapped { addr })
    }

    /// Cap on the page spans handed to the JIT for inlining (aligned with Rynarmic's `MAX_WINDOWS`).
    ///
    /// Eight rather than four: spans are cut at 1 MiB super pages, and a 350 KiB image, a heap of a
    /// few MiB and a 512 KiB stack already make seven — with four slots it is the **stack** that gets
    /// dropped (highest addresses, so last in line). A span without a window pays a Rust callback on
    /// every access: measured 1.88 ns/instruction against 0.79 (`benches/jit_frame.rs`).
    pub const MAX_FAST_SPANS: usize = 8;

    /// Page spans that are fully mapped and **contiguous in host memory**, at most eight, **longest
    /// first** (length takes precedence over address: a heap of a few MiB folds into several 1 MiB
    /// spans, and truncating by address would push the stack out of the windows). Each element is
    /// `(span start guest address, first page number, page count)`.
    ///
    /// Two hard constraints, both required:
    /// - **every page in the span is mapped**: the JIT's inlined path checks bounds only, not whether
    ///   the page exists, so handing over a span with a hole makes unmapped accesses read zero
    ///   silently and `Fault::Unmapped` never happens again (leaving page 0 unmapped is how a null
    ///   pointer gets caught);
    /// - **the span is contiguous in host memory**: adjacent super pages inside one span are
    ///   contiguous and can be joined; two different spans sit wherever the allocator put them and
    ///   must be kept apart.
    ///
    /// A read-only query that changes no state; it runs once per JIT window publication, not on the
    /// hot path.
    pub fn fast_spans(&self) -> [(u32, usize, usize); Self::MAX_FAST_SPANS] {
        const NONE: (u32, usize, usize) = (0, 0, 0);
        let mut out = [NONE; Self::MAX_FAST_SPANS];
        // Only reached when the windows are republished, so this `Vec` is not a hot-path allocation.
        // A `Vec` rather than a fixed-size array because "merge first, then take the longest N" has to
        // see every candidate: the earlier implementation stopped as soon as it had enough and sorted
        // afterwards, so a 5 MiB heap folding into five 1 MiB spans filled every slot and the
        // **stack**, at the highest addresses, never got in.
        let mut runs: Vec<(u32, usize, u32)> = Vec::new();
        for (si, sup) in self.supers.iter().enumerate() {
            let Some(sup) = sup else { continue };
            let base_page = si << (SUPER_SHIFT - PAGE_SHIFT);
            let mut pi = 0usize;
            while pi < SUPER_PAGES {
                if !sup.mapped_bit(pi) {
                    pi += 1;
                    continue;
                }
                // Extend from this page onward: contiguous bits mean contiguous guest addresses
                let start = pi;
                while pi + 1 < SUPER_PAGES && sup.mapped_bit(pi + 1) {
                    pi += 1;
                }
                let pages = (pi - start + 1) as u32;
                let guest_page = (base_page + start) as u32;
                let host = sup.base + start * PAGE_SIZE;
                // Merge across super pages: consecutive guest page numbers, and the host memory joins
                // up as well (adjacent slots of one span). Both are checked, because two spans that
                // differ can still happen to be adjacent in guest addresses.
                match runs.last_mut() {
                    Some((g, h, n))
                        if *g + *n == guest_page && *h + *n as usize * PAGE_SIZE == host =>
                    {
                        *n += pages;
                    }
                    _ => runs.push((guest_page, host, pages)),
                }
                pi += 1;
            }
        }
        // A one-page span is not worth a window: an extra comparison on every access to save one
        // callback does not pay.
        runs.retain(|run| run.2 >= 2);
        runs.sort_unstable_by_key(|run| std::cmp::Reverse(run.2));
        for (slot, &(page, _, count)) in runs.iter().take(Self::MAX_FAST_SPANS).enumerate() {
            out[slot] = (page << PAGE_SHIFT, page as usize, count as usize);
        }
        out
    }

    /// The host pointer of a page (the companion query to `fast_spans`).
    pub fn page_ptr(&self, page: usize) -> Option<*const u8> {
        let sup = self
            .supers
            .get(page >> (SUPER_SHIFT - PAGE_SHIFT))?
            .as_ref()?;
        let pi = page & (SUPER_PAGES - 1);
        if !sup.mapped_bit(pi) {
            return None;
        }
        Some((sup.base + pi * PAGE_SIZE) as *const u8)
    }

    #[inline]
    fn at_mut(&mut self, addr: u32) -> MResult<&mut [u8; PAGE_SIZE]> {
        self.page_mut(addr).ok_or(Fault::Unmapped { addr })
    }

    /// Make every page covering `[base, base + len)` present. Existing contents survive.
    ///
    /// The super-page version: a page that does not exist is zeroed (the old "a new page is all
    /// zeroes" semantics), an existing one keeps its contents. [`Mem::map_generation`] advances only
    /// when a page is **really newly mapped**; an idempotent repeated map changes nothing.
    ///
    /// [`Mem::window_generation`] deliberately does **not** advance here: the JIT freezes its window
    /// set at publish time, a new page can only fall outside the frozen windows and go through the
    /// callback, and neither the pages nor the pointers baked into compiled blocks have changed.
    pub fn map(&mut self, base: u32, len: u32) {
        if len == 0 {
            // Nothing to bring in, and the arithmetic below would ask for the *previous* page: a
            // zero length makes `end` come out one below `start`, which is an empty range that
            // `..=` refuses rather than an empty loop. A pack that names a bitmap of no height
            // reaches here through `alloc_zeroed(0)`.
            return;
        }
        let start = base >> PAGE_SHIFT;
        let end = (base.saturating_add(len).saturating_sub(1)) >> PAGE_SHIFT;
        let end_super = (end >> (SUPER_SHIFT - PAGE_SHIFT)) as usize;
        if end_super >= self.supers.len() {
            // Sized to the highest super ever asked for — a handful of MiB for the regions a
            // guest actually touches, and an address near the top of the 32-bit space stays
            // the unmapped fault it was, because nothing indexes that far.
            self.supers.resize_with(end_super + 1, || None);
            self.page_table
                .resize((end_super + 1) << (SUPER_SHIFT - PAGE_SHIFT), 0);
        }
        // Advanced only when a page really was newly mapped: at run time there are idempotent maps
        // ("confirm once on every entry" — the halt trap, the heap remapping pages it already has),
        // and advancing unconditionally would inflate the layout generation.
        let mut changed = false;
        for page in start..=end {
            let si = (page >> (SUPER_SHIFT - PAGE_SHIFT)) as usize;
            self.ensure_super(si);
            let sup = self.supers[si].as_mut().expect("ensure_super has just created it");
            let pi = (page & (SUPER_PAGES as u32 - 1)) as usize;
            if !sup.mapped_bit(pi) {
                // A first mapping clears; mapping the same page again keeps the contents (old
                // semantics: existing contents survive).
                let off = pi * PAGE_SIZE;
                sup.bytes_mut()[off..off + PAGE_SIZE].fill(0);
                sup.set_mapped_bit(pi);
                self.page_table[page as usize] = sup.base + off;
                changed = true;
            }
        }
        if changed {
            self.map_generation = self.map_generation.wrapping_add(1);
        }
    }

    /// Make sure the `si`th super page has its own 1 MiB.
    ///
    /// Prefer reusing a slot in the span it belongs to (that is where "super pages of one region are
    /// adjacent in host memory" comes from); open a new span when none covers it. Spans are
    /// append-only, so the number of calls is bounded by "how many regions were ever touched + one per
    /// 16 MiB", independent of how often `map` is called.
    fn ensure_super(&mut self, si: usize) {
        if self.supers[si].is_some() {
            return;
        }
        let base = match span_slot(&self.spans, si) {
            Some(base) => base,
            None => {
                let span = Span::new(si);
                let base = span.bytes.as_ptr() as usize;
                self.spans.push(span);
                base
            }
        };
        self.supers[si] = Some(Super::new_at(base));
    }

    pub fn unmap(&mut self, base: u32, len: u32) {
        if len == 0 {
            return;
        }
        let start = base >> PAGE_SHIFT;
        let end = (base.saturating_add(len).saturating_sub(1)) >> PAGE_SHIFT;
        let mut changed = false;
        for page in start..=end {
            let Some(sup) = self
                .supers
                .get_mut((page >> (SUPER_SHIFT - PAGE_SHIFT)) as usize)
                .and_then(|s| s.as_mut())
            else {
                continue;
            };
            let pi = (page & (SUPER_PAGES as u32 - 1)) as usize;
            if sup.mapped_bit(pi) {
                // The old implementation dropped the page entirely, so a remap got a fresh zero page;
                // here the bytes are zeroed and the bit cleared, which keeps remap semantics identical
                // without disturbing the super page's own pointer.
                sup.clear_mapped_bit(pi);
                let off = pi * PAGE_SIZE;
                sup.bytes_mut()[off..off + PAGE_SIZE].fill(0);
                self.page_table[page as usize] = 0;
                changed = true;
            }
        }
        if changed {
            // `unmap` digs pages out of published windows: the inlined path does not consult the
            // bitmap, so until the cache is invalidated those addresses read zero instead of
            // `Fault::Unmapped`. After a whole-cache invalidation the windows are republished and hold
            // only spans that are still fully mapped.
            self.map_generation = self.map_generation.wrapping_add(1);
            self.window_generation = self.window_generation.wrapping_add(1);
        }
    }

    /// Public because the HLE layer copies guest bytes one at a time on purpose: an unmapped
    /// destination must fault the way a data abort would, not materialise memory.
    #[inline]
    pub fn read_u8(&self, addr: u32) -> MResult<u8> {
        let val = self.at(addr)?[(addr & PAGE_MASK) as usize];
        // That branch is the entire cost of watching reads, and it is not taken unless a watchpoint is
        // armed: this function is the interpreter's own instruction fetch. An empty watch list makes
        // `watch_spans` a walk over nothing, so it costs the same as the guard it replaces.
        if let Some(span) = self.watch_spans(addr) {
            self.record_read(addr, val, span);
        }
        Ok(val)
    }

    /// The watch record for a read — out of line on purpose. [`Mem::read_u8`] is the interpreter's own
    /// instruction fetch, and a `RefCell` borrow plus a `Vec` push is not worth a register spill on
    /// every instruction that runs. None of this can happen unless a watchpoint is armed.
    #[cold]
    #[inline(never)]
    fn record_read(&self, addr: u32, val: u8, span: (u32, u32)) {
        if self.watch_stop {
            self.watch_tripped.set(Some(addr));
        }
        let mut hits = self.watch_hits.borrow_mut();
        if hits.len() < WATCH_LIMIT {
            hits.push(WatchHit {
                addr,
                old: val,
                new: val,
                by: self.watch_by,
                span,
                read: true,
                call: WatchHit::call_for(self.watch_by, &self.watch_call.borrow()),
            });
        }
    }

    #[inline]
    pub fn write_u8(&mut self, addr: u32, val: u8) -> MResult<()> {
        #[cfg(test)]
        self.guard_header(addr, 1);
        let page = self.at_mut(addr)?;
        let at = (addr & PAGE_MASK) as usize;
        let old = page[at];
        page[at] = val;
        if addr > self.touch_high {
            self.touch_high = addr;
        }
        // One check in one place covers every width of store, because `write_u16`, `write_u32`,
        // `write_bytes` and `fill` all come through here. A byte of a halfword or a word lands in
        // the same watched cell, and seeing which width did it is itself a clue.
        let watched = self.watch_spans(addr);
        if let Some(span) = watched {
            self.record_write(addr, old, val, span);
        }
        Ok(())
    }

    /// [`Mem::record_read`]'s twin, for stores.
    #[cold]
    #[inline(never)]
    fn record_write(&mut self, addr: u32, old: u8, val: u8, span: (u32, u32)) {
        if self.watch_stop {
            self.watch_tripped.set(Some(addr));
        }
        let mut hits = self.watch_hits.borrow_mut();
        if hits.len() < WATCH_LIMIT {
            hits.push(WatchHit {
                addr,
                old,
                new: val,
                by: self.watch_by,
                span,
                read: false,
                call: WatchHit::call_for(self.watch_by, &self.watch_call.borrow()),
            });
        }
    }

    /// Whether a byte range overlaps any armed span. A zero-length event — the refusal of a `free`
    /// this arena never owned — counts as the one byte at its address, or it would never be visible
    /// at all to the watchpoint that was armed to catch it.
    fn watch_covers(&self, start: u32, len: u32) -> bool {
        let end = start.saturating_add(len.max(1));
        self.watch.iter().any(|&(lo, hi)| start < hi && end > lo)
    }

    /// Record an arena event that landed inside an armed span.
    ///
    /// [`Mem::alloc`] and [`Mem::free`] move no bytes, so a watchpoint that only kept byte accesses
    /// could never answer the question this exists for: is the block under this pointer fresh space,
    /// or space somebody handed back and this call filled with file data? Out of line and guarded —
    /// every pack allocates on every frame, and with nothing armed the caller does not get here.
    #[cold]
    #[inline(never)]
    fn note_arena(&mut self, event: &'static str, start: u32, len: u32, returned_at: Option<u64>) {
        if !self.watch_covers(start, len) {
            return;
        }
        // The walk that can say *when* the chain broke rather than that it is broken — and the only
        // part of this record that costs a full arena scan, so a run pays for it only when asked.
        let chain = if self.watch_chain {
            self.heap_audit().err()
        } else {
            None
        };
        // The high-water the heap stood on at this event, which is the same figure a `free` is
        // recorded with.
        let cursor = self.heap_high();
        let steps = self.watch_steps.get();
        let call = self.watch_call.borrow().clone();
        let mut events = self.watch_arena.borrow_mut();
        if events.len() >= WATCH_LIMIT {
            // Keep the oldest, like the byte record does — which means the *end* of the run is what
            // goes missing here, and a report that kept saying "no allocation landed across this
            // span" after that would be reading a truncated record as if it were the whole one.
            self.watch_arena_capped.set(true);
            return;
        }
        events.push(WatchArena {
            event,
            start,
            len,
            cursor,
            returned_at,
            steps: (steps != 0).then_some(steps),
            call,
            chain,
        });
    }

    /// A `FREE(p)` for an address this arena never handed out, or the second release of a block
    /// already released — the case [`crate::hle::Hle::dealloc`] refuses.
    ///
    /// Refusing is right (a double free must not hand the same space to two callers), and silence
    /// about it is not: "the pack released this block twice" is the sentence this exists to find,
    /// and the refusal is the only place it becomes visible. Recorded with no length, so the report
    /// matches it by its address alone.
    pub fn note_unowned_free(&mut self, addr: u32) {
        if self.watch.is_empty() {
            return;
        }
        self.note_arena("free-refused", addr, 0, None);
    }

    /// Whether the arena record filled up and stopped keeping events.
    pub fn watch_arena_capped(&self) -> bool {
        self.watch_arena_capped.get()
    }

    /// Instruction fetch for the quiet interpreter: the watch list is empty by construction, so the
    /// per-read watch test that [`Mem::read_u16`] must keep for guest loads can be omitted.
    #[inline(always)]
    pub(crate) fn fetch_u16(&self, addr: u32) -> MResult<u32> {
        if self.one_page(addr, 2) {
            let page = self.at(addr)?;
            let at = (addr & PAGE_MASK) as usize;
            return Ok(u16::from_le_bytes([page[at], page[at + 1]]) as u32);
        }
        self.read_u16_slow(addr)
    }

    /// [`Mem::fetch_u16`] for an ARM instruction word.
    #[inline(always)]
    pub(crate) fn fetch_u32(&self, addr: u32) -> MResult<u32> {
        if self.one_page(addr, 4) {
            let page = self.at(addr)?;
            let at = (addr & PAGE_MASK) as usize;
            return Ok(u32::from_le_bytes([
                page[at],
                page[at + 1],
                page[at + 2],
                page[at + 3],
            ]));
        }
        self.read_u32_slow(addr)
    }

    #[inline(always)]
    pub fn read_u16(&self, addr: u32) -> MResult<u32> {
        // `watch.is_empty` is the same guard the wide *stores* take, and for the same reason: the
        // byte-wise path is where the record is kept, so a watched read has to go through it. Without
        // it, "who read this word" would be answerable only for the readers that happen to be
        // byte-wise, which is not a property anyone can rely on.
        if self.one_page(addr, 2) && self.watch.is_empty() {
            let page = self.at(addr)?;
            let at = (addr & PAGE_MASK) as usize;
            return Ok(u16::from_le_bytes([page[at], page[at + 1]]) as u32);
        }
        self.read_u16_slow(addr)
    }

    /// The straddling or watched case, out of line so that the table-lookup fast path above is what
    /// the interpreter's fetch inlines to (`read_u16` already did and [`Mem::read_u32`] below is the
    /// asymmetric half).
    #[cold]
    #[inline(never)]
    fn read_u16_slow(&self, addr: u32) -> MResult<u32> {
        Ok(u16::from_le_bytes([self.read_u8(addr)?, self.read_u8(addr.wrapping_add(1))?]) as u32)
    }

    /// Every access is byte-wise, so an unaligned word read yields the four bytes starting at
    /// `addr` — which is what an ARMv5 core's two aligned reads and a rotate amount to.
    ///
    /// The one-page case takes one table access rather than four, and this is the hottest function
    /// in the emulator: the interpreter's own instruction fetch comes through it.
    #[inline(always)]
    pub fn read_u32(&self, addr: u32) -> MResult<u32> {
        if self.one_page(addr, 4) && self.watch.is_empty() {
            let page = self.at(addr)?;
            let at = (addr & PAGE_MASK) as usize;
            return Ok(u32::from_le_bytes([
                page[at],
                page[at + 1],
                page[at + 2],
                page[at + 3],
            ]));
        }
        self.read_u32_slow(addr)
    }

    /// [`Mem::read_u16_slow`] for four bytes. The halves of a straddling word can be independently
    /// unmapped, so the byte path is what reports the fault against the right address.
    #[cold]
    #[inline(never)]
    fn read_u32_slow(&self, addr: u32) -> MResult<u32> {
        let a = self.read_u8(addr)?;
        let b = self.read_u8(addr.wrapping_add(1))?;
        let c = self.read_u8(addr.wrapping_add(2))?;
        let d = self.read_u8(addr.wrapping_add(3))?;
        Ok(u32::from_le_bytes([a, b, c, d]))
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) -> MResult<()> {
        let b = (val as u16).to_le_bytes();
        if self.one_page(addr, 2) && self.watch.is_empty() {
            return self.write_word(addr, &[b[0], b[1]]);
        }
        self.write_u16_slow(addr, b)
    }

    #[cold]
    #[inline(never)]
    fn write_u16_slow(&mut self, addr: u32, b: [u8; 2]) -> MResult<()> {
        self.write_u8(addr, b[0])?;
        self.write_u8(addr.wrapping_add(1), b[1])
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) -> MResult<()> {
        let b = val.to_le_bytes();
        if self.one_page(addr, 4) && self.watch.is_empty() {
            return self.write_word(addr, &b);
        }
        self.write_u32_slow(addr, b)
    }

    #[cold]
    #[inline(never)]
    fn write_u32_slow(&mut self, addr: u32, b: [u8; 4]) -> MResult<()> {
        for (i, byte) in b.iter().enumerate() {
            self.write_u8(addr.wrapping_add(i as u32), *byte)?;
        }
        Ok(())
    }

    /// Do `len` bytes from `addr` lie inside one page? The fast paths can only index one page's
    /// array, and anything that straddles belongs to the byte-wise path.
    #[inline]
    fn one_page(&self, addr: u32, len: u32) -> bool {
        addr & PAGE_MASK <= PAGE_MASK - len
    }

    /// A run of bytes known to be inside one mapped page, written with a single table access.
    #[inline]
    fn write_word(&mut self, addr: u32, bytes: &[u8]) -> MResult<()> {
        #[cfg(test)]
        self.guard_header(addr, bytes.len() as u32);
        let page = self.at_mut(addr)?;
        let at = (addr & PAGE_MASK) as usize;
        page[at..at + bytes.len()].copy_from_slice(bytes);
        let high = addr.wrapping_add(bytes.len() as u32 - 1);
        if high > self.touch_high {
            self.touch_high = high;
        }
        Ok(())
    }

    /// The two pages a page-wise copy needs, one immutable and one mutable. Same-superpage pairs
    /// split the byte array itself; a cross-superpage pair splits the superpage table, so a copy
    /// across pages does not allocate a staging buffer.
    fn page_pair_mut(
        &mut self,
        src: u32,
        dst: u32,
    ) -> MResult<(&[u8; PAGE_SIZE], &mut [u8; PAGE_SIZE])> {
        let s_super = (src >> SUPER_SHIFT) as usize;
        let d_super = (dst >> SUPER_SHIFT) as usize;
        // The table is split below by super-page index, and `split_at_mut`'s cut has to land inside
        // it. A wild address from the guest can name a super page that was never mapped; that should
        // report `Fault::Unmapped`, not let the split run off the end and take the host process down
        // — a missing page is the guest's answer to give, a panic is not.
        if s_super >= self.supers.len() {
            return Err(Fault::Unmapped { addr: src });
        }
        if d_super >= self.supers.len() {
            return Err(Fault::Unmapped { addr: dst });
        }
        if s_super != d_super {
            // Different super pages: split the table. The two `Box`es do not overlap, so each side
            // takes its own page.
            let (lo, hi) = if s_super < d_super {
                self.supers.split_at_mut(d_super)
            } else {
                self.supers.split_at_mut(s_super)
            };
            let (s_slot, d_slot) = if s_super < d_super {
                (&lo[s_super], &mut hi[0])
            } else {
                (&hi[0], &mut lo[d_super])
            };
            let s = s_slot.as_ref().ok_or(Fault::Unmapped { addr: src })?;
            let d = d_slot.as_mut().ok_or(Fault::Unmapped { addr: dst })?;
            return Ok((
                Self::page_of(s, src).ok_or(Fault::Unmapped { addr: src })?,
                Self::page_mut_of(d, dst).ok_or(Fault::Unmapped { addr: dst })?,
            ));
        }
        // Same super page: the two pages are two non-overlapping 4 KiB ranges of one 1 MiB block, so
        // split at the page start of the lower address and each side gets one range.
        let sup = self
            .supers
            .get_mut(s_super)
            .and_then(|s| s.as_mut())
            .ok_or(Fault::Unmapped { addr: src })?;
        let s_pi = ((src >> PAGE_SHIFT) & (SUPER_PAGES as u32 - 1)) as usize;
        let d_pi = ((dst >> PAGE_SHIFT) & (SUPER_PAGES as u32 - 1)) as usize;
        debug_assert_ne!(s_pi, d_pi, "the caller guarantees distinct pages");
        if !sup.mapped_bit(s_pi) {
            return Err(Fault::Unmapped { addr: src });
        }
        if !sup.mapped_bit(d_pi) {
            return Err(Fault::Unmapped { addr: dst });
        }
        let (s_off, d_off) = (s_pi * PAGE_SIZE, d_pi * PAGE_SIZE);
        let (lo, hi) = if s_off < d_off {
            sup.bytes_mut().split_at_mut(d_off)
        } else {
            sup.bytes_mut().split_at_mut(s_off)
        };
        // After `split_at_mut(k)`, `lo`'s base is 0 and `hi`'s is k — both indices are **relative to
        // their own slice**: a source page at the cut takes `hi[0..PAGE_SIZE]`, a destination page
        // before the cut takes `lo[d_off..]`. Applying an absolute offset straight to `hi` reads a
        // different page, and applying `s_off` to `lo` is guaranteed to run off the end (`lo`'s length
        // is exactly `s_off`).
        let (s_bytes, d_bytes) = if s_off < d_off {
            (&lo[s_off..s_off + PAGE_SIZE], &mut hi[0..PAGE_SIZE])
        } else {
            (&hi[0..PAGE_SIZE], &mut lo[d_off..d_off + PAGE_SIZE])
        };
        Ok((
            s_bytes.try_into().expect("a page slice is always 4 KiB"),
            d_bytes.try_into().expect("a page slice is always 4 KiB"),
        ))
    }

    /// The free-function form of [`Self::page`]: take a page out of one super page by global
    /// address.
    #[inline]
    fn page_of(sup: &Super, addr: u32) -> Option<&[u8; PAGE_SIZE]> {
        let pi = ((addr >> PAGE_SHIFT) & (SUPER_PAGES as u32 - 1)) as usize;
        if !sup.mapped_bit(pi) {
            return None;
        }
        let off = ((addr as usize) & (SUPER_SIZE - 1)) & !(PAGE_MASK as usize);
        sup.bytes()[off..off + PAGE_SIZE].try_into().ok()
    }

    /// The free-function form of [`Self::page_mut`].
    #[inline]
    fn page_mut_of(sup: &mut Super, addr: u32) -> Option<&mut [u8; PAGE_SIZE]> {
        let pi = ((addr >> PAGE_SHIFT) & (SUPER_PAGES as u32 - 1)) as usize;
        if !sup.mapped_bit(pi) {
            return None;
        }
        let off = ((addr as usize) & (SUPER_SIZE - 1)) & !(PAGE_MASK as usize);
        <&mut [u8; PAGE_SIZE] as TryFrom<&mut [u8]>>::try_from(
            &mut sup.bytes_mut()[off..off + PAGE_SIZE],
        )
        .ok()
    }

    /// `memmove`: copy `len` bytes without allocating. Non-overlapping copies move page runs
    /// directly; an overlap within one page uses `copy_within`. The rare cross-page overlap still
    /// takes the snapshot path, which is what makes it a true `memmove` rather than `memcpy`.
    pub fn copy_bytes(&mut self, dst: u32, src: u32, len: usize) -> MResult<()> {
        if len == 0 {
            return Ok(());
        }
        if !self.watch.is_empty() {
            let bytes = self.read_bytes(src, len)?;
            return self.write_bytes(dst, &bytes);
        }
        let src_end = src as u64 + len as u64;
        let dst_end = dst as u64 + len as u64;
        let overlap = (dst as u64) < src_end && (src as u64) < dst_end;
        if overlap {
            let soff = (src & PAGE_MASK) as usize;
            let doff = (dst & PAGE_MASK) as usize;
            // The fast path is only for a run that stays inside the one page. Two addresses in the
            // same page say nothing about where the *end* of the run lands: `copy_within` is bounded
            // by the page, so a copy that starts on one page and runs past its end has to take the
            // staged route below rather than index off the end of the page.
            if (src >> PAGE_SHIFT) == (dst >> PAGE_SHIFT)
                && len <= PAGE_SIZE - soff
                && len <= PAGE_SIZE - doff
            {
                let page = self.at_mut(dst)?;
                page.copy_within(soff..soff + len, doff);
                let high = dst.wrapping_add(len as u32 - 1);
                if high > self.touch_high {
                    self.touch_high = high;
                }
                return Ok(());
            }
            let bytes = self.read_bytes(src, len)?;
            return self.write_bytes(dst, &bytes);
        }
        let (mut at_src, mut at_dst, mut left) = (src, dst, len);
        while left > 0 {
            let soff = (at_src & PAGE_MASK) as usize;
            let doff = (at_dst & PAGE_MASK) as usize;
            let run = (PAGE_SIZE - soff).min(PAGE_SIZE - doff).min(left);
            #[cfg(test)]
            self.guard_header(at_dst, run as u32);
            if (at_src >> PAGE_SHIFT) == (at_dst >> PAGE_SHIFT) {
                let page = self.at_mut(at_dst)?;
                page.copy_within(soff..soff + run, doff);
            } else {
                let (src_page, dst_page) = self.page_pair_mut(at_src, at_dst)?;
                dst_page[doff..doff + run].copy_from_slice(&src_page[soff..soff + run]);
            }
            let high = at_dst.wrapping_add(run as u32 - 1);
            if high > self.touch_high {
                self.touch_high = high;
            }
            at_src = at_src.wrapping_add(run as u32);
            at_dst = at_dst.wrapping_add(run as u32);
            left -= run;
        }
        Ok(())
    }

    /// `memcmp` without allocating either side: compare one page run at a time and stop at the
    /// first differing byte.
    pub fn compare_bytes(&self, a: u32, b: u32, len: usize) -> MResult<std::cmp::Ordering> {
        if !self.watch.is_empty() {
            let x = self.read_bytes(a, len)?;
            let y = self.read_bytes(b, len)?;
            return Ok(x.cmp(&y));
        }
        let (mut at_a, mut at_b, mut left) = (a, b, len);
        while left > 0 {
            let aoff = (at_a & PAGE_MASK) as usize;
            let boff = (at_b & PAGE_MASK) as usize;
            let run = (PAGE_SIZE - aoff).min(PAGE_SIZE - boff).min(left);
            let pa = self.at(at_a)?;
            let pb = self.at(at_b)?;
            let ord = pa[aoff..aoff + run].cmp(&pb[boff..boff + run]);
            if ord != std::cmp::Ordering::Equal {
                return Ok(ord);
            }
            at_a = at_a.wrapping_add(run as u32);
            at_b = at_b.wrapping_add(run as u32);
            left -= run;
        }
        Ok(std::cmp::Ordering::Equal)
    }

    /// Copy `len` bytes out of the guest.
    ///
    /// No capacity is reserved up front. `len` reaches most callers from a guest register, and
    /// reserving it first meant one bad length asked the host for gigabytes before a single address
    /// had been checked — the emulator process died where the guest should have taken a data abort.
    /// Reading stops at the first unmapped byte, so the work is bounded by what is mapped rather
    /// than by the number the guest happened to pass.
    pub fn read_bytes(&self, addr: u32, len: usize) -> MResult<Vec<u8>> {
        // The same guard the wide readers take: while a watchpoint is armed the byte-wise path is the
        // one that keeps the record, so the block reader defers to it.
        if !self.watch.is_empty() {
            let mut out = Vec::new();
            for i in 0..len as u32 {
                out.push(self.read_u8(addr.wrapping_add(i))?);
            }
            return Ok(out);
        }
        let mut out = Vec::new();
        let (mut at, mut left) = (addr, len);
        while left > 0 {
            let off = (at & PAGE_MASK) as usize;
            let run = (PAGE_SIZE - off).min(left);
            // The run stops at the page boundary, so an unmapped second page faults here on its own
            // first address — the same report the byte-wise loop gave.
            let page = self.at(at)?;
            out.extend_from_slice(&page[off..off + run]);
            at = at.wrapping_add(run as u32);
            left -= run;
        }
        Ok(out)
    }

    /// [`Mem::read_bytes`] into a buffer the caller already has.
    ///
    /// The difference matters on the paths that run once per *row* of a blit: a `Vec` per row is the
    /// cost of the copy rather than a rounding error on it. Same paging and the same watch rule —
    /// while a watchpoint is armed the byte-wise path is the one that keeps the record.
    pub fn read_into(&self, addr: u32, dst: &mut [u8]) -> MResult<()> {
        if !self.watch.is_empty() {
            for (i, byte) in dst.iter_mut().enumerate() {
                *byte = self.read_u8(addr.wrapping_add(i as u32))?;
            }
            return Ok(());
        }
        let (mut at, mut rest) = (addr, dst);
        while !rest.is_empty() {
            let off = (at & PAGE_MASK) as usize;
            let run = (PAGE_SIZE - off).min(rest.len());
            let page = self.at(at)?;
            rest[..run].copy_from_slice(&page[off..off + run]);
            at = at.wrapping_add(run as u32);
            rest = &mut rest[run..];
        }
        Ok(())
    }

    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) -> MResult<()> {
        if !self.watch.is_empty() {
            for (i, byte) in data.iter().enumerate() {
                self.write_u8(addr.wrapping_add(i as u32), *byte)?;
            }
            return Ok(());
        }
        let (mut at, mut rest) = (addr, data);
        while !rest.is_empty() {
            let off = (at & PAGE_MASK) as usize;
            let run = (PAGE_SIZE - off).min(rest.len());
            self.write_word(at, &rest[..run])?;
            at = at.wrapping_add(run as u32);
            rest = &rest[run..];
        }
        Ok(())
    }

    /// Unlike a handset's `MEMSET`, this faults instead of growing the address space: the
    /// caller owns the mapping.
    pub fn fill(&mut self, addr: u32, len: u32, val: u8) -> MResult<()> {
        if !self.watch.is_empty() {
            for i in 0..len {
                self.write_u8(addr.wrapping_add(i), val)?;
            }
            return Ok(());
        }
        let (mut at, mut left) = (addr, len);
        while left > 0 {
            let off = (at & PAGE_MASK) as usize;
            let run = (PAGE_SIZE as u32 - off as u32).min(left) as usize;
            #[cfg(test)]
            self.guard_header(at, run as u32);
            let page = self.at_mut(at)?;
            page[off..off + run].fill(val);
            let high = at.wrapping_add(run as u32 - 1);
            if high > self.touch_high {
                self.touch_high = high;
            }
            at = at.wrapping_add(run as u32);
            left -= run as u32;
        }
        Ok(())
    }

    /// `malloc(len)`: `AEEkHeap_Realloc(pHeap, 0, len)`.
    ///
    /// `align` is not honoured, and that is the delivery's behaviour, not an omission: the allocator
    /// has no alignment parameter. Every payload starts four bytes after a four-aligned node, and
    /// nodes are sized in multiples of four, so every block it hands out is four-aligned and no
    /// caller can ask for more.
    ///
    /// A block freed earlier is reused here exactly when the delivery's search reaches it first —
    /// lowest address first, merging touching holes on the way. See [`crate::heap`] for the body this
    /// follows and the crash that made following it necessary.
    pub fn alloc(&mut self, len: u32, _align: u32) -> MResult<u32> {
        // A zero-length request still gets an address that reads, because every caller hands it to
        // something that will index it: `alloc_zeroed(0)` returning a pointer into an unmapped page
        // turns a degenerate size into a data abort later.
        self.heap_block(len.max(1), true)
    }

    /// `AEEkHeap_Realloc(pHeap, 0, len)` with the high bit of the size set: the delivery skips the
    /// clear in that case, and a caller that says so gets whatever the bytes held.
    pub fn alloc_unzeroed(&mut self, len: u32) -> MResult<u32> {
        self.heap_block(len.max(1), false)
    }

    /// `AEEkHeap_Realloc(pHeap, p, len)`. `None` when the heap could not serve it, in which case the
    /// old block is still the caller's.
    pub fn realloc(&mut self, ptr: u32, len: u32, clear: bool) -> Option<u32> {
        if ptr == 0 {
            return self.heap_block(len.max(1), clear).ok();
        }
        if len == 0 {
            self.free(ptr);
            return None;
        }
        let heap = self.heap;
        let at = {
            let mut arena = GuestArena(self);
            heap.realloc(&mut arena, ptr, len, clear)
        };
        if let Some((at, handed)) = at {
            // An exact-size realloc hands nothing over, and the watch record must not say it did.
            if handed {
                self.note_alloc(at, len);
            }
        }
        #[cfg(test)]
        self.refresh_heads();
        at.map(|(at, _)| at)
    }

    /// Bring a block the allocator just handed out into the guest's pages and note it for the watch
    /// log — the allocator writes its own metadata, never the payload.
    fn note_alloc(&mut self, at: u32, len: u32) {
        let then = self.freed_at.remove(&(at - heap::NODE));
        self.map(at, len);
        if !self.watch.is_empty() {
            self.note_arena("alloc", at, len, then);
        }
    }

    /// The one entry point the delivered allocator needs, with refusal bookkeeping around it.
    fn heap_block(&mut self, len: u32, clear: bool) -> MResult<u32> {
        let heap = self.heap;
        let at = {
            let mut arena = GuestArena(self);
            heap.alloc(&mut arena, len, clear)
        };
        match at {
            Some(at) => {
                self.note_alloc(at, len);
                #[cfg(test)]
                self.refresh_heads();
                Ok(at)
            }
            None => {
                // A refusal is `MALLOC` answering NULL, which is the pack's own problem to handle —
                // and it is also the only way a pack that has run for three hundred million
                // instructions suddenly dereferences a null. A run that never says it happened leaves
                // that as an invisible cause, so the count and the largest refused request are kept.
                self.alloc_fails += 1;
                self.alloc_fail_max = self.alloc_fail_max.max(len);
                Err(Fault::HeapExhausted { want: len })
            }
        }
    }

    /// `free(p)`, which the delivery spells `AEEkHeap_Realloc(pHeap, p, 0)`.
    ///
    /// The size comes from the node's own header, not from the caller: `FREE` is handed a pointer and
    /// nothing else, and a host that believed a length it was given would free the wrong span. A
    /// pointer into this heap that is already free is refused — the delivery breaks there, and a
    /// second release must not put one run of space on the list twice.
    ///
    /// Unlinking and merging are the allocator's business on the next allocation: `free` here marks
    /// the node and links it, exactly as `AEEkHeap_FreeNode` does.
    pub fn free(&mut self, addr: u32) {
        if addr < self.heap.base + heap::HEADER + heap::NODE {
            return;
        }
        let node = addr - heap::NODE;
        let heap = self.heap;
        let len = HeapState::payload_of(self, addr - heap::NODE);
        let freed = {
            let mut arena = GuestArena(self);
            heap.free(&mut arena, addr)
        };
        if freed {
            if !self.watch.is_empty() {
                // The block as its owner handed it back. The node's span is what went back on the
                // list, and it is the range that has to line up with the `alloc` that handed it out.
                self.note_arena("free", addr, len, None);
            }
            self.freed_at.insert(node, self.watch_steps.get());
        }
        #[cfg(test)]
        self.refresh_heads();
    }

    /// Whether the heap's own bookkeeping still agrees with itself: the address walk reaches `high`
    /// exactly, every free-list handle names a free node on that walk, and the links are symmetrical.
    ///
    /// The delivery trusts its metadata, so a run only notices a broken heap when something later
    /// asks it for space. This is the question a test asks at the end of every one (`Runtime::drop`),
    /// the one the reports ask when an allocation was refused or the guest faulted, and — under
    /// `--watch-chain` — the one each recorded arena handover answers, because a broken chain is dated
    /// by the first event that reports it. With the nodes living in guest memory, a single byte written
    /// over a neighbour's size word is enough to break it.
    pub fn heap_audit(&self) -> Result<(), String> {
        self.heap.audit(self)
    }

    /// Test builds: refuse a store that lands on a node header, at the store.
    ///
    /// The heap keeps its bookkeeping in guest memory, so one unit written past a block silently
    /// rewrites the next node's size word. Nothing notices until a later allocation cannot find room,
    /// and the report then names the wrong call — this is the assertion that stops at the store that
    /// did it, with the address of the node and of the caller's buffer.
    ///
    /// The allocator's own header stores are exempt ([`Mem::heap_writing`]), and a heap whose chain is
    /// already broken is left to [`Mem::heap_audit`], which says so properly.
    #[cfg(test)]
    fn guard_header(&self, start: u32, len: u32) {
        if HeapWriting::depth() != 0 {
            return;
        }
        let heads = self.heap_heads.borrow();
        if heads.is_empty() {
            return;
        }
        let end = start.saturating_add(len.max(1));
        // The walk that filled this is in address order, so the first header that could overlap is
        // the first whose end is past `start`.
        let at = heads.partition_point(|&h| h + heap::NODE <= start);
        if let Some(&head) = heads.get(at) {
            if head < end {
                panic!(
                    "a write of {len} byte(s) at {start:#010x} lands on the node header at {head:#010x} (+{:#x} into it): the heap keeps twelve bytes of its own metadata there, and this store is not the allocator's",
                    start - head
                );
            }
        }
    }

    /// Rebuild [`Mem::heap_heads`] from the chain. Called after every allocation, release and
    /// reallocation in a test build — one walk over a few dozen nodes.
    #[cfg(test)]
    fn refresh_heads(&mut self) {
        let high = self.heap_high();
        let mut heads = Vec::new();
        let mut node = self.heap.base + heap::HEADER;
        while node + heap::NODE <= high {
            let size = self.read_u32(node).map(|w| 2 * (w >> 1)).unwrap_or(0);
            if size < heap::NODE {
                break;
            }
            heads.push(node);
            node += size;
        }
        *self.heap_heads.borrow_mut() = heads;
    }

    /// Overwrite a node header on purpose, bypassing the write guard.
    ///
    /// A test is the only caller, and it exists because the alternative is a report path nothing can
    /// exercise: the guard refuses every *accidental* store into the metadata, so the "the chain is
    /// broken" verdict would otherwise be untestable code.
    #[cfg(test)]
    pub fn corrupt_node_header(&mut self, node: u32, word: u32) {
        let _allocator = HeapWriting::new();
        let _ = self.write_u32(node, word);
    }

    /// What the heap makes of a pointer a register held: the node it names, its total size and whether
    /// it is free. `None` when the pointer is not a node start in this heap — a field that never held a
    /// pointer, an interior address, or something else entirely.
    ///
    /// This is the question a data abort raises and the one a report has to answer: "was this block
    /// still the caller's, or had the heap already taken it back and handed it on?". The two want
    /// different fixes, and a host that keeps the heap in guest memory knows.
    pub fn node_at(&self, ptr: u32) -> Option<(u32, bool)> {
        let node = ptr.checked_sub(heap::NODE)?;
        self.heap.node_at(self, node)
    }

    /// Bytes sitting on the free list. A node counts with its header, which is what the heap spends.
    /// Every node in the heap, as `AEEkHeap::walk` gives them (`node`, total size, free or not) — the
    /// listing `DUMPHEAP` prints. The chain is walked, not remembered, so this is only as good as the
    /// heap's own bookkeeping; `dumpheap` validates first for exactly that reason.
    pub fn heap_walk(&self) -> Vec<(u32, u32, bool)> {
        self.heap.walk(self)
    }

    pub fn heap_spare(&self) -> u32 {
        self.heap.spare(self)
    }

    /// How many free nodes the heap holds: adjacent holes are separate until an allocation walks
    /// past and merges them, so this says how chopped up the free space is.
    pub fn heap_free_blocks(&self) -> usize {
        self.heap.free_blocks(self)
    }

    pub fn alloc_zeroed(&mut self, len: u32) -> MResult<u32> {
        // `alloc` already zeroes, because the delivery does ("the memory returned is
        // zero-initialized, unless the high bit of uSize is set").
        self.alloc(len, 8)
    }

    /// Read a NUL-terminated single-byte string, capped so a wild pointer cannot hang us.
    ///
    /// A null pointer reads as the empty string rather than faulting, for the reason
    /// [`Mem::read_u16str`] gives: a pack's own unchecked null is not this host's fault to die on.
    pub fn read_cstr(&self, addr: u32, cap: usize) -> MResult<Vec<u8>> {
        if addr == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for i in 0..cap as u32 {
            let c = self.read_u8(addr.wrapping_add(i))?;
            if c == 0 {
                return Ok(out);
            }
            out.push(c);
        }
        Ok(out)
    }

    /// Read an `AECHAR` (UTF-16LE) string, capped at `cap` code units.
    ///
    /// A null pointer reads as the empty string rather than faulting. That is not what a C library does,
    /// and not what a handset does either — but a pack that hands an unchecked null to `wstrcmp` would
    /// have crashed on the handset too, and ending the run there says nothing about this emulator while
    /// costing the reader everything the pack would have done next — a pack that compares a lookup
    /// result it never checked reaches exactly that.
    pub fn read_u16str(&self, addr: u32, cap: usize) -> MResult<Vec<u16>> {
        if addr == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for i in 0..cap as u32 {
            let c = self.read_u16(addr.wrapping_add(i * 2))? as u16;
            if c == 0 {
                return Ok(out);
            }
            out.push(c);
        }
        Ok(out)
    }

    /// Write `units` and the `AECHAR` terminator after them. The slice is the **text**, without its
    /// NUL: a caller that pushes one as well writes a unit past the end of its own buffer, and the
    /// delivered heap keeps the next node's size word exactly there.
    pub fn write_u16str(&mut self, addr: u32, units: &[u16]) -> MResult<()> {
        for (i, u) in units.iter().enumerate() {
            self.write_u16(addr.wrapping_add(i as u32 * 2), *u as u32)?;
        }
        self.write_u16(addr.wrapping_add(units.len() as u32 * 2), 0)
    }

    /// Every mapped page, ascending. Used by the state dumper.
    pub fn pages(&self) -> Vec<(u32, &[u8])> {
        let mut out = Vec::new();
        for (si, sup) in self.supers.iter().enumerate() {
            let Some(sup) = sup else { continue };
            for pi in 0..SUPER_PAGES {
                if sup.mapped_bit(pi) {
                    let off = pi * PAGE_SIZE;
                    let addr = (((si << (SUPER_SHIFT - PAGE_SHIFT)) + pi) as u32) << PAGE_SHIFT;
                    out.push((addr, &sup.bytes()[off..off + PAGE_SIZE]));
                }
            }
        }
        out
    }

    /// Write this memory's whole state into `out`.
    ///
    /// The pages go out as a bitmap of which slots exist plus the bytes of the ones that do, which is
    /// what makes a state the size of what the run touched rather than the size of the address space:
    /// the page table reaches `layout::STACK_TOP`, and a state that carried every slot would be
    /// hundreds of megabytes of mostly-absent pages.
    #[allow(dead_code)]
    pub(crate) fn write_state(&self, out: &mut Vec<u8>) {
        // The external format is exactly what it was in the per-page table days: a count of 4 KiB
        // page slots, a presence bitmap, then the bytes of the mapped pages in address order. Super
        // pages are only how those pages are stored internally.
        let slots = self.supers.len() << (SUPER_SHIFT - PAGE_SHIFT);
        put_u32(out, slots as u32);
        let mut bits = vec![0u8; slots.div_ceil(8)];
        for si in 0..self.supers.len() {
            if let Some(sup) = &self.supers[si] {
                for (word, bits_set) in sup.mapped.iter().enumerate() {
                    let mut rest = *bits_set;
                    while rest != 0 {
                        let bit = rest.trailing_zeros() as usize;
                        rest &= rest - 1;
                        let page = (si << (SUPER_SHIFT - PAGE_SHIFT)) + word * 64 + bit;
                        bits[page / 8] |= 1 << (page % 8);
                    }
                }
            }
        }
        out.extend_from_slice(&bits);
        for si in 0..self.supers.len() {
            let Some(sup) = &self.supers[si] else {
                continue;
            };
            for pi in 0..SUPER_PAGES {
                if sup.mapped_bit(pi) {
                    let off = pi * PAGE_SIZE;
                    out.extend_from_slice(&sup.bytes()[off..off + PAGE_SIZE]);
                }
            }
        }
        put_u32(out, self.heap.base);
        put_u32(out, self.heap.limit);
        // The heap's own header and every node are in the pages above, because that is where the
        // delivered allocator keeps them; there is no second copy to write.
        put_u32(out, self.touch_high);
        put_u32(out, self.alloc_fails);
        put_u32(out, self.alloc_fail_max);
    }

    /// Read back what [`Mem::write_state`] wrote.
    ///
    /// Everything is validated before anything is stored: a block that ends early is refused with a
    /// reason rather than leaving the machine half-way into a state no handset ever produced.
    #[allow(dead_code)]
    pub(crate) fn read_state(&mut self, st: &mut State) -> Result<(), String> {
        let slots = st.u32()? as usize;
        let bits = st.take(slots.div_ceil(8))?.to_vec();
        let mut supers: Vec<Option<Super>> = Vec::new();
        let mut spans: Vec<Span> = Vec::new();
        // Bitmap and byte stream are both in address order: when a mapped page comes up, make sure
        // the super page exists and copy the bytes in.
        for page in 0..slots {
            if bits[page / 8] & (1 << (page % 8)) == 0 {
                continue;
            }
            let si = page >> (SUPER_SHIFT - PAGE_SHIFT);
            if si >= supers.len() {
                supers.resize_with(si + 1, || None);
            }
            if supers[si].is_none() {
                // A contiguous run in the archive has to come back as a contiguous run of host memory,
                // or the restored machine has every 4 KiB page in a span of its own and the JIT window
                // cannot hold even one.
                let base = match span_slot(&spans, si) {
                    Some(base) => base,
                    None => {
                        let span = Span::new(si);
                        let base = span.bytes.as_ptr() as usize;
                        spans.push(span);
                        base
                    }
                };
                supers[si] = Some(Super::new_at(base));
            }
            let sup = supers[si].as_mut().expect("just created");
            let pi = page & (SUPER_PAGES - 1);
            let off = pi * PAGE_SIZE;
            let bytes: [u8; PAGE_SIZE] = st.take(PAGE_SIZE)?.try_into().expect("the page is always 4 KiB");
            sup.bytes_mut()[off..off + PAGE_SIZE].copy_from_slice(&bytes);
            sup.set_mapped_bit(pi);
        }
        let heap_base = st.u32()?;
        let heap_limit = st.u32()?;
        let touch_high = st.u32()?;
        let alloc_fails = st.u32()?;
        let alloc_fail_max = st.u32()?;

        self.supers = supers;
        self.spans = spans;
        self.page_table.clear();
        self.page_table
            .resize(self.supers.len() << (SUPER_SHIFT - PAGE_SHIFT), 0);
        for (si, sup) in self.supers.iter().enumerate() {
            let Some(sup) = sup else { continue };
            let base = sup.base;
            for pi in 0..SUPER_PAGES {
                if sup.mapped_bit(pi) {
                    self.page_table[(si << (SUPER_SHIFT - PAGE_SHIFT)) + pi] =
                        base + pi * PAGE_SIZE;
                }
            }
        }
        // The whole super-page storage was replaced: every host pointer baked into the old windows
        // now dangles, so they must be invalidated and republished.
        self.map_generation = self.map_generation.wrapping_add(1);
        self.window_generation = self.window_generation.wrapping_add(1);
        self.heap = HeapState::new(heap_base, heap_limit);
        self.touch_high = touch_high;
        self.alloc_fails = alloc_fails;
        self.alloc_fail_max = alloc_fail_max;
        // The chain came back inside `pages`, so the header table the write guard searches is now
        // describing the *previous* machine's layout. Rebuild it or every store into the restored
        // heap is checked against addresses that are no longer headers.
        #[cfg(test)]
        self.refresh_heads();
        Ok(())
    }
}

/// The read side the heap model needs, over the guest's own pages.
impl ReadArena for Mem {
    fn read_word(&self, at: u32) -> u32 {
        self.read_u32(at).unwrap_or(0)
    }

    fn read_half(&self, at: u32) -> u16 {
        self.read_u16(at).map(|v| v as u16).unwrap_or(0)
    }
}

// How deep inside the allocator's own stores this thread is. A counter, because a split's
// `free_node` runs inside the `trim` of an allocation: a boolean would be cleared by the inner guard
// while the outer one still had header writes to do.
thread_local! {
    #[cfg(test)]
    static HEAP_WRITING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Marks the allocator's own stores, so the test-build header guard does not blame it for writing
/// the metadata it owns. A drop guard rather than a pair of calls: an early return inside the arena
/// must not leave the count raised, or the *next* guest store would be excused.
#[cfg(test)]
struct HeapWriting;

#[cfg(test)]
impl HeapWriting {
    fn new() -> Self {
        HEAP_WRITING.with(|d| d.set(d.get() + 1));
        HeapWriting
    }

    fn depth() -> u32 {
        HEAP_WRITING.with(|d| d.get())
    }
}

#[cfg(test)]
impl Drop for HeapWriting {
    fn drop(&mut self) {
        HEAP_WRITING.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// The heap model's view of the memory it is working on.
///
/// Metadata writes bring their page in on demand: the heap's span is 240 MiB and only the pages the
/// allocator has actually written exist. `read_word` answering zero for a page that does not exist is
/// deliberate — `AEEkHeap` trusts its own metadata, and a host that faulted on a pack's scribbled node
/// header would end the run where a handset merely behaves badly.
struct GuestArena<'a>(&'a mut Mem);

impl ReadArena for GuestArena<'_> {
    fn read_word(&self, at: u32) -> u32 {
        self.0.read_u32(at).unwrap_or(0)
    }

    fn read_half(&self, at: u32) -> u16 {
        self.0.read_u16(at).map(|v| v as u16).unwrap_or(0)
    }
}

impl Arena for GuestArena<'_> {
    fn write_word(&mut self, at: u32, value: u32) {
        #[cfg(test)]
        let _heap = HeapWriting::new();
        if !self.0.is_mapped(at) {
            self.0.map(at, 4);
        }
        let _ = self.0.write_u32(at, value);
    }

    fn write_half(&mut self, at: u32, value: u16) {
        #[cfg(test)]
        let _heap = HeapWriting::new();
        if !self.0.is_mapped(at) {
            self.0.map(at, 2);
        }
        let _ = self.0.write_u16(at, value as u32);
    }

    fn copy(&mut self, dst: u32, src: u32, len: u32) {
        #[cfg(test)]
        let _heap = HeapWriting::new();
        if len == 0 {
            return;
        }
        let bytes: Vec<u8> = (0..len)
            .map(|i| self.0.read_u8(src.wrapping_add(i)).unwrap_or(0))
            .collect();
        self.0.map(dst, len);
        let _ = self.0.write_bytes(dst, &bytes);
    }

    fn zero(&mut self, at: u32, len: u32) {
        #[cfg(test)]
        let _heap = HeapWriting::new();
        if len == 0 {
            return;
        }
        self.0.map(at, len);
        let _ = self.0.fill(at, len, 0);
    }

    fn grow(&mut self, at: u32, need: u32) -> bool {
        if need == 0 {
            return false;
        }
        if self.0.heap.limit != 0 && at.saturating_add(need) > self.0.heap.limit {
            return false;
        }
        self.0.map(at, need);
        true
    }
}

/// A cursor over a state block, and the writers that make one.
///
/// It lives here because memory is the bottom of the tree: `Runtime`, `Cpu` and `Hle` all serialize
/// through these two, and a reader defined above them would have them depending on their own caller.
///
/// Every read is a `Result` and every one is checked, so a truncated or foreign block is refused where
/// it is read rather than being stored as a field of zeroes: a machine restored from half a state
/// faults somewhere the pack never went, and blaming the pack for that is the wrong answer.
#[allow(dead_code)]
pub(crate) struct State<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> State<'a> {
    #[allow(dead_code)]
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        State { bytes, at: 0 }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .ok_or("a state block that asks for more than it holds")?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| format!("a state block that ends {n} byte(s) early"))?;
        self.at = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    #[allow(dead_code)]
    pub(crate) fn u64(&mut self) -> Result<u64, String> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub(crate) fn small(&mut self) -> Result<usize, String> {
        Ok(self.u32()? as usize)
    }

    pub(crate) fn string(&mut self) -> Result<String, String> {
        let len = self.small()?;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| "a name in the state that is not UTF-8".into())
    }

    /// Whether every byte has been consumed. A block with bytes left over was written by a version
    /// that knew more than this one, and reading it as if it did not is how a save comes back wrong.
    pub(crate) fn spent(&self) -> bool {
        self.at == self.bytes.len()
    }
}

#[allow(dead_code)]
pub(crate) fn put_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

#[allow(dead_code)]
pub(crate) fn put_bool(out: &mut Vec<u8>, v: bool) {
    put_u8(out, u8::from(v));
}

#[allow(dead_code)]
pub(crate) fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[allow(dead_code)]
pub(crate) fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[allow(dead_code)]
pub(crate) fn put_string(out: &mut Vec<u8>, s: &str) {
    put_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

/// A `HashMap`'s iteration order is not state, so a map that has one is written in key order: two
/// snapshots of the same machine have to be the same bytes, or a baseline could never be compared
/// with a run.
#[allow(dead_code)]
pub(crate) fn sorted_u32<V>(map: &std::collections::HashMap<u32, V>) -> Vec<(u32, &V)> {
    let mut rows: Vec<(u32, &V)> = map.iter().map(|(k, v)| (*k, v)).collect();
    rows.sort_by_key(|(k, _)| *k);
    rows
}

#[cfg(test)]
#[path = "test/mem.rs"]
mod tests;
