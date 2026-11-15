use super::*;

/// The test arena: a byte vector with the same span rules the guest heap has.
struct Bytes {
    bytes: Vec<u8>,
    cap: u32,
}

impl Bytes {
    fn new(cap: u32) -> Self {
        Self {
            bytes: vec![0u8; cap as usize],
            cap,
        }
    }
}

impl ReadArena for Bytes {
    fn read_word(&self, at: u32) -> u32 {
        let i = at as usize;
        u32::from_le_bytes([
            self.bytes[i],
            self.bytes[i + 1],
            self.bytes[i + 2],
            self.bytes[i + 3],
        ])
    }

    fn read_half(&self, at: u32) -> u16 {
        let i = at as usize;
        u16::from_le_bytes([self.bytes[i], self.bytes[i + 1]])
    }
}

impl Arena for Bytes {
    fn write_word(&mut self, at: u32, value: u32) {
        let i = at as usize;
        self.bytes[i..i + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_half(&mut self, at: u32, value: u16) {
        let i = at as usize;
        self.bytes[i..i + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn copy(&mut self, dst: u32, src: u32, len: u32) {
        let bytes: Vec<u8> = (0..len).map(|i| self.bytes[(src + i) as usize]).collect();
        for (i, b) in bytes.into_iter().enumerate() {
            self.bytes[dst as usize + i] = b;
        }
    }

    fn zero(&mut self, at: u32, len: u32) {
        for i in 0..len {
            self.bytes[(at + i) as usize] = 0;
        }
    }

    fn grow(&mut self, at: u32, need: u32) -> bool {
        at + need <= self.cap
    }
}

fn heap(cap: u32) -> (HeapState, Bytes) {
    let state = HeapState::new(0, cap);
    let mut bytes = Bytes::new(cap);
    state.init(&mut bytes);
    (state, bytes)
}

/// What the walk finds: `(node address, size, free)` in address order.
fn nodes(h: &HeapState, m: &Bytes) -> Vec<(u32, u32, bool)> {
    let mut out = Vec::new();
    let mut node = HEADER;
    while node + NODE <= h.high(m) {
        let size = HeapState::size(m, node);
        if size < NODE {
            break;
        }
        out.push((node, size, HeapState::is_free(m, node)));
        node += size;
    }
    out
}

#[test]
fn the_first_node_starts_after_the_twenty_byte_heap_header() {
    let (h, m) = heap(1 << 16);
    assert_eq!(h.high(&m), 1 << 16);
    assert_eq!(
        m.read_word(SENTINEL),
        HEADER + 4,
        "the sentinel's prev link names the one free node's handle"
    );
    assert_eq!(m.read_word(12), HEADER + 4, "and so does its next link");
    assert_eq!(
        m.read_word(HEADER + 4),
        SENTINEL,
        "whose own links are the sentinel"
    );
    assert_eq!(m.read_word(HEADER + 8), SENTINEL);
    assert_eq!(
        nodes(&h, &m),
        vec![(HEADER, (1 << 16) - HEADER, true)],
        "one free node covers the span after the header"
    );
}

#[test]
fn a_request_costs_its_size_rounded_up_plus_a_twelve_byte_header() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 1, true).unwrap();
    assert_eq!(a, HEADER + NODE);
    assert_eq!(HeapState::size(&m, HEADER), 16, "1 input byte rounds to 4");
    let b = h.alloc(&mut m, 0x7a, true).unwrap();
    assert_eq!(b - a, 16);
    assert_eq!(HeapState::size(&m, b - NODE), 0x88, "0x7a rounds to 0x7c");
    assert_eq!(m.read_half(b - NODE + 10), 2, "0x7c - 0x7a is the slack");
    assert_eq!(m.read_half(b - NODE + 8), 1, "a new node is locked once");
}

#[test]
fn a_new_block_reads_as_zeroes() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x20, true).unwrap();
    for i in 0..0x20 {
        m.bytes[(a + i) as usize] = 0xAA;
    }
    h.free(&mut m, a);
    let again = h.alloc(&mut m, 0x20, true).unwrap();
    assert_eq!(again, a, "the hole it just freed is the lowest one");
    assert!(
        (0..0x20).all(|i| m.bytes[(again + i) as usize] == 0),
        "the delivery zeroes a block as it hands it out"
    );
}

#[test]
fn the_lowest_hole_that_fits_is_taken_before_the_tail() {
    let (h, mut m) = heap(1 << 16);
    let wide = h.alloc(&mut m, 0x100, true).unwrap();
    let snug = h.alloc(&mut m, 0x40, true).unwrap();
    h.free(&mut m, snug);
    h.free(&mut m, wide);
    let answer = h.alloc(&mut m, 0x30, true).unwrap();
    assert_eq!(
        answer, wide,
        "first fit from the lowest address, not best fit"
    );
    assert_ne!(answer, snug);
}

#[test]
fn the_free_list_is_kept_descending_and_the_search_runs_upward() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x40, true).unwrap();
    let _b = h.alloc(&mut m, 0x40, true).unwrap();
    let c = h.alloc(&mut m, 0x40, true).unwrap();
    h.free(&mut m, a);
    h.free(&mut m, c);
    // The sentinel's `next` is the highest address, its `prev` the lowest, and following
    // `next` walks the list downward — the order `FreeNode` inserts into.
    let mut seen = Vec::new();
    let mut handle = m.read_word(SENTINEL + 4);
    while handle != SENTINEL {
        seen.push(handle - 4);
        handle = m.read_word(handle + 4);
        assert!(seen.len() < 8, "the list must not cycle");
    }
    assert_eq!(seen.len(), 3, "the two holes and the tail");
    assert!(seen.windows(2).all(|w| w[0] > w[1]), "{seen:?} descending");
    assert_eq!(seen.last(), Some(&(a - NODE)), "a is the lowest of them");
    assert_eq!(
        m.read_word(SENTINEL) - 4,
        a - NODE,
        "the sentinel's prev is the lowest node, which is where the search starts"
    );
    assert_eq!(seen[0], h.high(&m) - HeapState::size(&m, seen[0]));
}

#[test]
fn a_free_does_not_merge_and_an_allocation_that_walks_does() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x40, true).unwrap();
    let b = h.alloc(&mut m, 0x40, true).unwrap();
    let c = h.alloc(&mut m, 0x40, true).unwrap();
    h.free(&mut m, a);
    h.free(&mut m, c);
    h.free(&mut m, b);
    assert_eq!(
        h.free_blocks(&m),
        4,
        "free marks and links only; three holes stay three nodes"
    );
    // A request for the whole run fails on `a` alone, merges `b` and `c` in as it looks, and
    // then fits — which is `CheckFreeNode` doing the compaction the free did not.
    let merged = h.alloc(&mut m, 3 * (0x40 + NODE) - NODE, true).unwrap();
    assert_eq!(merged, a);
    assert_eq!(h.free_blocks(&m), 1, "and the run is spent exactly");
}

#[test]
fn a_split_needs_twelve_bytes_and_a_smaller_tail_stays_as_slack() {
    // 20-byte header + 0x1C node + a twelve-byte tail: the tail can pay for a header, so it is
    // split off even though it is too small to serve a request afterwards.
    let (h, mut m) = heap(0x3C);
    let a = h.alloc(&mut m, 0x10, true).unwrap();
    assert_eq!(a, HEADER + NODE);
    assert_eq!(
        nodes(&h, &m),
        vec![(HEADER, 0x1Cu32, false), (HEADER + 0x1C, 0xCu32, true)]
    );
    assert!(
        h.alloc(&mut m, 0x10, true).is_none(),
        "0xC cannot hold a node"
    );

    // An eight-byte tail cannot, so the node keeps it as slack.
    let (h, mut m) = heap(0x38);
    h.alloc(&mut m, 0x10, true).unwrap();
    assert_eq!(HeapState::size(&m, HEADER), 0x24);
    assert_eq!(m.read_half(HEADER + 10), 8);
    assert_eq!(nodes(&h, &m).len(), 1);
}

#[test]
fn a_realloc_of_the_same_size_does_nothing_at_all() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x20, true).unwrap();
    let before = HeapState::size(&m, a - NODE);
    let word_before = m.read_word(a - NODE);
    assert_eq!(h.realloc(&mut m, a, 0x20, true).unwrap().0, a);
    assert_eq!(HeapState::size(&m, a - NODE), before);
    assert_eq!(m.read_word(a - NODE), word_before, "not even a trim");
}

#[test]
fn a_realloc_that_fits_stays_put_and_one_that_does_not_moves() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x20, true).unwrap();
    for i in 0..0x20u32 {
        m.bytes[(a + i) as usize] = i as u8 + 1;
    }
    let (shrunk, moved_any) = h.realloc(&mut m, a, 0x10, true).unwrap();
    assert!(moved_any);
    assert_eq!(shrunk, a, "the old node still has room");
    assert_eq!(HeapState::size(&m, a - NODE), 0x1C);
    let _b = h.alloc(&mut m, 0x400, true).unwrap();
    let (moved, _) = h.realloc(&mut m, a, 0x100, true).unwrap();
    assert_ne!(
        moved, a,
        "the node could not hold it and the tail is far away"
    );
    for i in 0..0x10u32 {
        assert_eq!(
            m.bytes[(moved + i) as usize],
            i as u8 + 1,
            "payload carried"
        );
    }
    assert!(!HeapState::is_free(&m, moved - NODE));
}

#[test]
fn a_second_free_is_refused_and_the_heap_keeps_its_shape() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 0x40, true).unwrap();
    assert!(h.free(&mut m, a));
    assert!(!h.free(&mut m, a));
    assert_eq!(h.free_blocks(&m), 2, "the block and the tail");
}

#[test]
fn a_long_sequence_of_operations_keeps_the_heap_consistent() {
    // Deterministic pseudo-random traffic, because the shapes that break an allocator are rarely
    // the ones a test writes by hand: a free, a grow that only just does not fit, a shrink that
    // leaves a twelve-byte tail. The audit after every step checks the invariants the walk and
    // the list have to agree on; the canary checks that no live block lost a byte to a
    // neighbour's header.
    let (h, mut m) = heap(1 << 16);
    let mut live: Vec<(u32, u32, u8)> = Vec::new();
    let mut seed = 0x1234_5678u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 8) & 0xffff
    };
    for step in 0..400 {
        let roll = next() % 100;
        if roll < 45 || live.is_empty() {
            let len = 1 + next() % 0x300;
            if let Some(at) = h.alloc(&mut m, len, true) {
                let canary = (step % 251) as u8 + 1;
                for i in 0..len {
                    m.bytes[(at + i) as usize] = canary;
                }
                live.push((at, len, canary));
            }
        } else if roll < 75 {
            let pick = (next() as usize) % live.len();
            let (at, len, _) = live.remove(pick);
            h.free(&mut m, at);
            assert!(len > 0);
        } else {
            let pick = (next() as usize) % live.len();
            let (at, len, canary) = live[pick];
            let want = 1 + next() % 0x500;
            if let Some((at, _)) = h.realloc(&mut m, at, want, true) {
                for i in 0..len.min(want) {
                    assert_eq!(
                        m.bytes[(at + i) as usize],
                        canary,
                        "step {step}: a live block lost byte {i} of its payload"
                    );
                }
                for i in len.min(want)..want {
                    m.bytes[(at + i) as usize] = canary;
                }
                live[pick] = (at, want, canary);
            }
        }
        h.audit(&m)
            .unwrap_or_else(|why| panic!("step {step}: {why}"));
        for &(at, len, canary) in &live {
            for i in 0..len {
                assert_eq!(
                    m.bytes[(at + i) as usize],
                    canary,
                    "step {step}: block {at:#x} byte {i} was overwritten"
                );
            }
        }
    }
    assert!(!live.is_empty(), "the run allocated something");
}

#[test]
fn a_request_that_fits_nowhere_grows_the_heap_before_it_answers() {
    // `AEEkHeap_Realloc`'s fourth step: look, find nothing, `Sbrk`, look again. The first request
    // spends the initial slice, so the second one can only be served by growing the last node —
    // and it lands exactly where a pre-extended tail would have put it.
    let (h, mut m) = heap(1 << 20);
    assert_eq!(
        h.high(&m),
        INITIAL,
        "init hands over a slice, not the whole span"
    );
    let wide = INITIAL - HEADER - NODE;
    let a = h.alloc(&mut m, wide, true).unwrap();
    let b = h.alloc(&mut m, 0x100, true).unwrap();
    assert!(h.high(&m) > INITIAL, "the heap had to grow for it");
    assert_eq!(
        b,
        a + wide + NODE,
        "and served it right after the first block"
    );
}

#[test]
fn growth_is_refused_once_the_span_is_spent() {
    let (h, mut m) = heap(0x100);
    let mut got = 0;
    while h.alloc(&mut m, 0x40, true).is_some() {
        got += 1;
        if got > 100 {
            break;
        }
    }
    assert!(got >= 1, "the span serves what fits: {got} blocks");
    assert!(h.high(&m) <= 0x100);
}

#[test]
fn sweep_right_moves_an_unlocked_sentinel_node_into_a_free_run() {
    let (h, mut m) = heap(1 << 16);
    let a = h.alloc(&mut m, 8, true).unwrap();
    let b = h.alloc(&mut m, 8, true).unwrap();
    let sentinel = h.alloc(&mut m, 4, true).unwrap();
    for i in 0..2 {
        m.write_word(a + i * 4, 0x1111_1111 + i);
    }
    h.free(&mut m, b);
    m.write_half(a - NODE + 8, 0); // unlock the node
    m.write_word(a - NODE + 4, sentinel); // and give it a sentinel
    m.write_word(sentinel, a);

    assert!(h.sweep_right(&mut m), "the node had a free run above it");
    assert_eq!(
        m.read_word(sentinel),
        b,
        "the sentinel is rebased by the move"
    );
    assert_eq!(m.read_word(b), 0x1111_1111);
    h.audit(&m).unwrap();
}

/// `sane` exists to reject a pack's scribble, so its own arithmetic has to survive one. The size
/// word is guest memory and `2 * (word >> 1)` reaches 0xfffffffe, which leaves u32 when added to a
/// node address. This is the crash-report path: `main` asks what each register points at.
#[test]
fn a_scribbled_node_size_does_not_overflow_the_plausibility_check() {
    let (h, mut m) = heap(1 << 16);
    m.write_word(HEADER, 0xffff_ffff);
    assert_eq!(h.node_at(&m, HEADER), None);
}

/// The rejection above must not become a blanket one: a node whose last byte is the heap's last
/// byte sums exactly to `high` and is still a node.
#[test]
fn a_node_ending_exactly_at_high_is_still_a_node() {
    let (h, mut m) = heap(1 << 16);
    let high = h.high(&m);
    let size = high - HEADER;
    m.write_word(HEADER, size);
    assert_eq!(h.node_at(&m, HEADER), Some((size, false)));
    m.write_word(HEADER, size + 4);
    assert_eq!(h.node_at(&m, HEADER), None, "one byte past the end is not a node");
}

/// The free-list walk reads the same guest-writable size words, and `push_node` reaches it without
/// passing through `sane`. Saturating is the convention the rest of this file already uses.
#[test]
fn the_free_list_walk_survives_a_scribbled_size_word() {
    let (h, mut m) = heap(1 << 16);
    m.write_word(HEADER, 0xffff_ffff);
    assert_eq!(h.next_free(&m, HEADER), h.base + SENTINEL);
}

/// `check_free_node` absorbs forward neighbours, so both the step to the neighbour and the accumulated
/// size word are sums of guest values. `need` larger than the scribbled size is what reaches the step.
#[test]
fn merging_free_nodes_survives_a_scribbled_size_word() {
    let (h, mut m) = heap(1 << 16);
    m.write_word(HEADER, 0xffff_ffff);
    assert!(!h.check_free_node(&mut m, HEADER, 0xffff_ffff));
}

/// The run report walks the heap with `audit`, `walk`, `spare` and `free_blocks`; `audit` also runs
/// at a fault. All four read the same guest-writable size words, and `walk`'s own doc promises it
/// "stops the walk instead of spinning it" — which a wrapping step broke in debug builds.
#[test]
fn the_report_walks_survive_a_scribbled_size_word() {
    let (h, mut m) = heap(1 << 16);
    m.write_word(HEADER, 0xffff_ffff);
    assert!(h.audit(&m).is_err(), "a broken chain is a disagreement, not a crash");
    assert_eq!(h.walk(&m).len(), 1, "the walk stops at the node it cannot step past");
    let _ = h.spare(&m);
    let _ = h.free_blocks(&m);
    // And the fix must not reject a heap that is fine.
    let (ok, mut bytes) = heap(1 << 16);
    let _ = ok.alloc(&mut bytes, 64, true);
    assert!(ok.audit(&bytes).is_ok(), "{}", ok.audit(&bytes).unwrap_err());
}

/// `owner_of` walks the same guest-written chain and had neither a saturating step nor a step budget:
/// a scribbled size wrapped `next` back below `node` and the loop walked backwards for ever.
#[test]
fn finding_the_owner_of_a_node_survives_a_scribbled_size_word() {
    let (h, mut m) = heap(1 << 16);
    m.write_word(HEADER, 0xffff_ffff);
    assert_eq!(h.owner_of(&m, HEADER + 8), Some(HEADER));
}
