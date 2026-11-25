// Moved out of `src/mem.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn page_zero_stays_unmapped() {
    let mem = Mem::new();
    assert_eq!(mem.read_u32(0), Err(Fault::Unmapped { addr: 0 }));
}

#[test]
fn a_same_page_memmove_that_runs_past_the_page_end_is_copied() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    mem.map(base, 0x2000);
    let src = base + 0xf00;
    let dst = base + 0xe00;
    for i in 0..0x400u32 {
        mem.write_u8(src + i, (i & 0xff) as u8).unwrap();
    }
    // The two addresses are in one page, but the run ends at `base + 0x1300`: `copy_within` is
    // bounded by the page, so the same-page fast path must not take a run that crosses its end.
    mem.copy_bytes(dst, src, 0x400).unwrap();
    for i in 0..0x400u32 {
        assert_eq!(mem.read_u8(dst + i).unwrap(), (i & 0xff) as u8);
    }
}

#[test]
fn spans_pages_and_survives_unaligned_reads() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8040_0000);
    // Eight bytes short of a page boundary, so the later reads cross it.
    let base = 0x1000_fff8;
    mem.map(base, 16);
    mem.write_u32(base, 0xdead_beef).unwrap();
    assert_eq!(mem.read_u32(base).unwrap(), 0xdead_beef);
    assert_eq!(mem.read_u16(base + 2).unwrap(), 0xdead);
    // ARMv5 permits an unaligned word access: the result is the four bytes starting at the
    // address, not a rotation of the aligned word that contains it.
    assert_eq!(mem.read_u32(base + 2).unwrap(), 0x0000_dead);
    mem.write_u32(base + 8, 0x1122_3344).unwrap(); // lands in the next page
    assert_eq!(mem.read_u32(base + 5).unwrap(), 0x4400_0000);
    assert_eq!(mem.read_u8(base + 8).unwrap(), 0x44);
}

#[test]
fn an_unmapped_high_address_faults_instead_of_growing_the_table() {
    // The page table is indexed, which is only safe while the guest cannot make it big. Mapping
    // is the host's act — the loader and the bump allocator — so an address out of a wild
    // register has to fault where it stands, not resize anything.
    let mut mem = Mem::new();
    mem.map(0x0100_0000, 0x1000);
    let before = mem.mapped_pages();
    for addr in [0, 0x8000_0000, 0xf000_0004, 0xffff_fff0] {
        assert_eq!(
            mem.write_u32(addr, 1).unwrap_err(),
            Fault::Unmapped { addr },
            "{addr:#010x} is unmapped and a store must say so"
        );
        assert_eq!(mem.read_u32(addr).unwrap_err(), Fault::Unmapped { addr });
        assert!(!mem.is_mapped(addr));
    }
    assert_eq!(
        mem.mapped_pages(),
        before,
        "the attempts allocated no page at all"
    );
}

#[test]
fn alloc_grows_and_zeroes() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_0100);
    let a = mem.alloc(7, 8).unwrap();
    let b = mem.alloc(4, 8).unwrap();
    // `AEEkHeap_Realloc` has no alignment parameter: the first node follows the twenty-byte heap
    // header, every payload follows a twelve-byte node header, and every size is a multiple of four.
    // A block is therefore four-aligned and a caller cannot ask for more.
    assert_eq!(a, 0x8000_0020);
    assert_eq!(
        b,
        a + 0x14,
        "7 bytes round to 8, plus the twelve-byte header"
    );
    mem.write_u32(b, 7).unwrap();
    let c = mem.alloc_zeroed(4).unwrap();
    assert_eq!(mem.read_u32(c).unwrap(), 0);
    assert!(mem.alloc(0x1000, 8).is_err());
}

#[test]
fn a_declared_newline_slack_stops_at_the_requested_length() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_4000);
    let plain = mem.alloc(0x8ad, 8).unwrap();
    assert_eq!(
        mem.read_u8(plain + 0x8ad).unwrap(),
        0,
        "without a declaration the spare bytes are the arena's"
    );

    mem.set_slack_newline(true);
    let declared = mem.alloc(0x8ad, 8).unwrap();
    for i in 0..3 {
        assert_eq!(
            mem.read_u8(declared + 0x8ad + i).unwrap(),
            0x0a,
            "the three bytes between 0x8ad and the next node read as a newline"
        );
    }
    assert_eq!(
        mem.read_u8(declared + 0x8ad - 1).unwrap(),
        0,
        "only the spare bytes are painted; the payload keeps what the allocator gave it"
    );
    assert!(
        mem.heap_audit().is_ok(),
        "the next node header is untouched"
    );
}

#[test]
fn a_request_for_nothing_still_names_a_page_it_can_be_read_from() {
    // `map(base, 0)` used to compute an end page one *below* the start and slice `start..=end`,
    // which is not an empty range but a panic. A degenerate size reaches it from anywhere a guest
    // supplies a height of zero, so both halves are pinned.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.map(0x8000_0000, 0);
    mem.map(0x8000_0ff8, 0);
    let before = mem.mapped_pages();
    let at = mem.alloc(0, 8).unwrap();
    assert_eq!(at, 0x8000_0020, "a degenerate size still gets a real block");
    mem.write_u8(at, 0x5a).unwrap();
    assert_eq!(
        mem.read_u8(at).unwrap(),
        0x5a,
        "and the address reads, instead of faulting"
    );
    let _ = before;
}

#[test]
fn a_request_for_more_than_exists_faults_without_reserving_it_first() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.map(0x8000_0000, 0x1000);
    mem.write_u32(0x8000_0000, 1).unwrap();
    // One mapped page, and a request for the guest's whole 32-bit space. The answer is the
    // fault at the first unmapped byte; the point is that the host never reserves four
    // gigabytes to find out, which is what a guest register alone should not be able to ask for.
    assert!(matches!(
        mem.read_bytes(0x8000_0000, u32::MAX as usize),
        Err(Fault::Unmapped { .. })
    ));
    assert_eq!(
        mem.mapped_pages(),
        1,
        "and no further page was brought in on the way"
    );
}

#[test]
fn a_refused_allocation_is_counted_rather_than_only_returned_as_null() {
    // `MALLOC()` answering NULL is the pack's own problem to check. It is also the only way a pack
    // that has run for three hundred million instructions suddenly walks a pointer it had used all
    // along, and until the refusal was tallied the run ended without ever saying one had happened.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_0100);
    assert!(
        mem.alloc(0x80, 4).is_ok(),
        "the 256-byte span serves one 0x80 request"
    );
    assert!(mem.alloc(0x80, 4).is_err(), "and nothing else fits");
    assert!(
        mem.alloc(0x9000, 4).is_err(),
        "and neither does one far larger than the whole arena"
    );
    assert_eq!(mem.alloc_fails, 2, "both refusals, not just the last");
    assert_eq!(
        mem.alloc_fail_max, 0x9000,
        "the largest request is the one worth naming: it says whether the arena is tight or \\
         something asked for the moon"
    );
}

#[test]
fn a_freed_block_is_reused_as_soon_as_the_search_reaches_it() {
    // The delivery is not a bump allocator with a quarantine: `free` marks the node and links it, and
    // the very next request starts its search at the lowest free node. A pack that frees a buffer and
    // keeps the pointer therefore sees that address again — and a host that holds the space back is
    // running a different allocator, which is what took a guest pack down.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8100_0000);
    let first = mem.alloc(0x40, 8).unwrap();
    let second = mem.alloc(0x40, 8).unwrap();
    assert_ne!(first, second, "nothing has been given back yet");
    mem.write_u32(second, 0xdead_beef).unwrap();
    mem.free(second);
    assert_eq!(
        mem.alloc(0x40, 8).unwrap(),
        second,
        "the hole it just freed is the lowest one"
    );
    assert_eq!(
        mem.read_u32(second).unwrap(),
        0,
        "and the delivery clears what it hands out"
    );
    assert_eq!(
        mem.heap_high(),
        0x8000_0000 + crate::heap::INITIAL,
        "reuse spent no new space — the heap never had to grow — which is the difference between a          reuse and a leak"
    );
}

#[test]
fn a_small_request_lands_at_the_head_of_the_lowest_freed_block() {
    // The shape that took a guest pack down — and it is the *delivery's* shape, not a bug in it.
    // Lowest-address first fit hands a small object the head of a large freed buffer, so a stale
    // pointer to that buffer now names a live object. This host used to refuse that on purpose; the
    // rule was ours, and running it meant answering addresses no handset would have.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    let buffer = mem.alloc(0x21de, 8).unwrap();
    mem.free(buffer);
    let object = mem.alloc(0x1c, 8).unwrap();
    assert_eq!(
        object, buffer,
        "the lowest hole is the one it was freed from"
    );
}

#[test]
fn free_marks_and_links_while_an_allocation_that_walks_merges() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8100_0000);
    let (a, b, c) = (
        mem.alloc(0x40, 8).unwrap(),
        mem.alloc(0x40, 8).unwrap(),
        mem.alloc(0x40, 8).unwrap(),
    );
    mem.free(a);
    mem.free(c);
    assert_eq!(mem.heap_free_blocks(), 3, "two holes and the tail");
    mem.free(b);
    assert_eq!(
        mem.heap_free_blocks(),
        4,
        "`AEEkHeap_FreeNode` marks and links; it never merges, unlike this host's old list"
    );
    mem.alloc(3 * (0x40 + 12) - 12, 8).unwrap();
    assert_eq!(
        mem.heap_free_blocks(),
        1,
        "`CheckFreeNode` merged b and c into a while looking, which is the merge free did not do"
    );
    assert_eq!(
        mem.heap_spare(),
        mem.heap_high() - (0x8000_0000 + 20 + 3 * 0x4C),
        "and everything above the run it served is free space again"
    );
}

#[test]
fn a_realloc_that_fits_stays_put_and_one_that_cannot_moves() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    let a = mem.alloc(0x20, 8).unwrap();
    let _keeper = mem.alloc(0x20, 8).unwrap();
    mem.write_u32(a, 0x1122_3344).unwrap();
    assert_eq!(
        mem.realloc(a, 0x20, true).unwrap(),
        a,
        "the payload is already that size, and the delivery returns it untouched"
    );
    assert_eq!(
        mem.realloc(a, 0x10, true).unwrap(),
        a,
        "the node still has room"
    );
    let moved = mem.realloc(a, 0x100, true).unwrap();
    assert_ne!(
        moved, a,
        "the block after it is live, so the node cannot grow"
    );
    assert_eq!(
        mem.read_u32(moved).unwrap(),
        0x1122_3344,
        "the payload was carried over"
    );
}

#[test]
fn a_realloc_of_the_same_size_does_not_even_trim() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8100_0000);
    let a = mem.alloc(6, 8).unwrap();
    mem.watch_span(a, 6);
    let before = mem.watch_arena.borrow().len();
    assert_eq!(mem.realloc(a, 6, true).unwrap(), a);
    assert_eq!(
        mem.watch_arena.borrow().len(),
        before,
        "the delivery's exact-size branch is a bare return of p: no free, no trim, no split"
    );
}

#[test]
fn a_free_this_arena_never_saw_changes_nothing() {
    // A pack that frees a pointer into its own statics, or frees the same block twice, is a pack
    // that has to keep running — and a second free must not hand the same space to two callers.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_0100);
    let (blocks, spare, high) = (mem.heap_free_blocks(), mem.heap_spare(), mem.heap_high());
    mem.free(0x1234);
    mem.free(0);
    mem.free(0x8000_0010);
    assert_eq!(
        (mem.heap_free_blocks(), mem.heap_spare(), mem.heap_high()),
        (blocks, spare, high),
        "no space appeared from an address that was never handed out"
    );
    let block = mem.alloc(0x40, 8).unwrap();
    let after_alloc = mem.heap_spare();
    mem.free(block);
    mem.free(block);
    assert_eq!(
        mem.heap_spare(),
        after_alloc + 0x4C,
        "the double free is the same extent once, not the block twice"
    );
}

#[test]
fn a_watched_span_catches_writes_anywhere_inside_it_and_nothing_outside() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.map(0x8000_0000, 0x1000);
    // The question a span answers is "does anything touch this region", so a store at either
    // end has to be caught and one just past it must not be.
    mem.watch_span(0x8000_0010, 0x20);
    assert!(mem.watch_contains(0x8000_0010));
    assert!(mem.watch_contains(0x8000_002f));
    assert!(!mem.watch_contains(0x8000_0030));
    mem.write_u32(0x8000_0028, 0x1122_3344).unwrap();
    mem.write_u32(0x8000_0040, 0xdead_beef).unwrap();
    let addrs: Vec<u32> = mem.watch_hits.borrow().iter().map(|h| h.addr).collect();
    assert_eq!(
        addrs,
        vec![0x8000_0028, 0x8000_0029, 0x8000_002a, 0x8000_002b],
        "the store inside the span is logged byte by byte, the one outside is not"
    );
}

#[test]
fn two_watchpoints_stay_armed_together_and_each_hit_says_which_one_it_hit() {
    // A second `--watch` used to replace the first, so the question "which of these did the guest
    // write" took one run per address — and separate runs cannot tell a shared cause from a
    // coincidence. All of them stay armed now, and every hit carries the span it fell inside, so
    // the silent one is reportable rather than merely absent.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.map(0x8000_0000, 0x1000);
    let word = (0x8000_0010, 0x8000_0014);
    let span = (0x8000_0040, 0x8000_0050);
    let quiet = (0x8000_0200, 0x8000_0208);
    mem.watch_word(word.0);
    mem.watch_span(span.0, 0x10);
    mem.watch_span(quiet.0, 8);
    assert_eq!(
        mem.watch.len(),
        3,
        "arming a third address did not retire the first two"
    );

    mem.write_u32(span.0 + 4, 0x1122_3344).unwrap();
    mem.write_u32(word.0, 0xdead_beef).unwrap();
    let hits = mem.watch_hits.borrow();
    let of = |want: (u32, u32)| hits.iter().filter(|h| h.span == want).count();
    assert_eq!(
        (of(word), of(span), of(quiet)),
        (4, 4, 0),
        "each store was charged to its own address, and the untouched one has a zero to report"
    );
    assert!(
        hits.iter().all(|h| h.span == word || h.span == span),
        "no hit was filed under an address it did not fall inside"
    );
}

#[test]
fn a_watchpoint_sees_every_width_and_says_who_wrote() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_0100);
    let w = mem.alloc(8, 4).unwrap();
    mem.watch_word(w);
    // Nothing is executing, so these are host writes. The CPU claims the guest role instruction
    // by instruction, which is what makes the distinction mean anything.
    mem.write_u32(w, 0xdead_beef).unwrap();
    mem.write_u16(w + 2, 0x1234).unwrap();
    mem.write_u8(w + 3, 0xab).unwrap();
    mem.watch_by = WatchBy::Guest {
        pc: 0x1000_1234,
        insn: 0xe584_6024,
    };
    mem.write_u32(w, 0xffff_ffff).unwrap();
    // A store to the neighbouring word is not this one's business.
    mem.write_u32(w + 4, 1).unwrap();

    let hits: Vec<(u32, u32, WatchBy)> = mem
        .watch_hits
        .borrow()
        .iter()
        .map(|h| (h.old as u32, h.new as u32, h.by))
        .collect();
    assert_eq!(
        hits.len(),
        11,
        "four bytes for each word store, two for the halfword, one for the byte"
    );
    assert_eq!(hits[0], (0x00, 0xef, WatchBy::Host));
    assert_eq!(hits[3], (0x00, 0xde, WatchBy::Host));
    assert_eq!(
        hits[4],
        (0xad, 0x34, WatchBy::Host),
        "the halfword store replaces the low two bytes"
    );
    assert_eq!(hits[6], (0x12, 0xab, WatchBy::Host), "the byte store");
    assert_eq!(
        hits[7],
        (
            0xef,
            0xff,
            WatchBy::Guest {
                pc: 0x1000_1234,
                insn: 0xe584_6024
            }
        ),
        "and the next word store is credited to the instruction that made it"
    );
}

#[test]
fn a_guest_store_under_a_live_host_call_is_not_blamed_on_that_call() {
    // A downcall runs the pack's own instructions while the host call that made it is still in
    // flight, and the CPU re-asserts `Guest` for each of them. If the record stamped the live call
    // onto those stores too, the watch would blame `IFile::Read` for arithmetic the pack did itself.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.map(0x8000_0000, 0x1000);
    mem.watch_span(0x8000_0010, 4);
    *mem.watch_call.borrow_mut() = Some(WatchCall {
        label: "IFile::Read",
        args: vec![0x0400_1e48, 0x8000_0010, 4],
    });
    mem.watch_by = WatchBy::Guest {
        pc: 0x1000_1234,
        insn: 0xdf07,
    };
    mem.write_u32(0x8000_0010, 1).unwrap();
    let blamed: Vec<Option<String>> = mem
        .watch_hits
        .borrow()
        .iter()
        .map(|h| h.call.as_ref().map(|c| c.label.to_string()))
        .collect();
    assert_eq!(
        blamed,
        vec![None; 4],
        "the guest's own store carries no host call, though one is open"
    );
    // The same address written by the host under that open call *is* blamed on it — otherwise the
    // guard above would merely be a watchpoint that lost the information.
    drop(blamed);
    mem.watch_hits.borrow_mut().clear();
    mem.watch_by = WatchBy::Host;
    mem.write_u32(0x8000_0010, 2).unwrap();
    let names: Vec<Option<String>> = mem
        .watch_hits
        .borrow()
        .iter()
        .map(|h| h.call.as_ref().map(|c| c.label.to_string()))
        .collect();
    assert_eq!(
        names,
        vec![Some("IFile::Read".to_string()); 4],
        "a host store under the same open call keeps the attribution"
    );
}

#[test]
fn a_watched_span_reports_the_arena_events_across_it() {
    // `alloc` and `free` move no bytes, so a byte watchpoint cannot say whether the space under a
    // watched word is fresh or recycled. This is the record that says it: the three states, in order,
    // with the instruction count each one stood on.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    let block = 0x8000_0020;
    mem.watch_span(block, 0x40);
    // The instruction count is mirrored in by the interpreter once per retired instruction, on the
    // same branch that sets `watch_by`; here it stands for "the run had moved on".
    mem.watch_steps.set(1234);
    let first = mem.alloc(0x40, 4).unwrap();
    assert_eq!(first, block);
    mem.free(block);
    let again = mem.alloc(0x40, 4).unwrap();
    assert_eq!(again, block, "the recycled space is the same address");

    let events: Vec<(&str, u32, u32, Option<u64>, Option<u64>)> = mem
        .watch_arena
        .borrow()
        .iter()
        .map(|e| (e.event, e.start, e.len, e.returned_at, e.steps))
        .collect();
    assert_eq!(
        events,
        vec![
            ("alloc", block, 0x40, None, Some(1234)),
            ("free", block, 0x40, None, Some(1234)),
            ("alloc", block, 0x40, Some(1234), Some(1234)),
        ],
        "fresh, handed back, then recycled — and the recycle names the instruction the free stood on"
    );
    assert_eq!(
        mem.watch_arena.borrow()[0].cursor,
        mem.heap_high(),
        "the record carries the heap's high-water, which starts at the slice it was given"
    );
}

#[test]
fn a_handover_that_finds_the_chain_broken_says_so_on_its_own_line() {
    // The verdict is taken *after* each handover, so the first event whose chain disagrees dates the
    // damage. `corrupt_node_header` is the test's way past the write guard — the guard exists to stop
    // the accident this test is deliberately staging.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    mem.watch_span(0x8000_0020, 0x40);
    mem.watch_chain = true;
    let a = mem.alloc(16, 4).unwrap();
    let b = mem.alloc(16, 4).unwrap();
    let c = mem.alloc(16, 4).unwrap();
    assert_eq!(
        mem.watch_arena
            .borrow()
            .last()
            .and_then(|e| e.chain.clone()),
        None,
        "a healthy handover says nothing"
    );
    mem.corrupt_node_header(a - crate::heap::NODE, 0);
    mem.free(b);
    let verdict = mem
        .watch_arena
        .borrow()
        .last()
        .and_then(|e| e.chain.clone());
    assert!(
        verdict.as_deref().unwrap_or("").contains("has word 0x0"),
        "the free that followed the damage carries the verdict: {verdict:?}"
    );
    // …and the walk is the only part of the record that costs an arena scan, so not asking for it has
    // to leave the same event on the list with that one line missing — not a different event, and not
    // no event at all.
    mem.watch_chain = false;
    mem.free(c);
    let events = mem.watch_arena.borrow();
    let last = events.last().expect("the second free was recorded");
    assert_eq!(last.event, "free");
    assert_eq!(last.len, 16, "the handover itself is still recorded");
    assert_eq!(
        last.chain, None,
        "with the walk not asked for, nothing is paid for it"
    );
}

#[test]
fn an_arena_event_outside_the_watched_span_is_not_recorded() {
    // The span is what the reader asked about; recording the whole arena would answer a question
    // nobody asked and bury the one line that matters.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    mem.watch_span(0x8010_0000, 4);
    let elsewhere = mem.alloc(0x40, 4).unwrap();
    mem.free(elsewhere);
    assert_eq!(
        elsewhere, 0x8000_0020,
        "positive control: this is the lowest block there is, far below the watched span"
    );
    assert!(
        mem.watch_arena.borrow().is_empty(),
        "nothing that did not touch the span belongs in its record: {:?}",
        mem.watch_arena.borrow()
    );
}

#[test]
fn a_full_arena_record_says_so_instead_of_going_quiet() {
    // The byte record keeps its first `WATCH_LIMIT` entries, and so does this one — which means the
    // end of a long run is missing, and a span could then read as untouched. The flag is the only
    // thing standing between that record and a false negative.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    let limit = mem.watch_limit();
    mem.watch_span(0x8000_0000, 0x10_0000);
    // One `alloc` and one `free` per round, so the cap counts events rather than requests.
    for _ in 0..limit / 2 {
        let at = mem.alloc(8, 8).unwrap();
        mem.free(at);
    }
    assert_eq!(
        mem.watch_arena.borrow().len(),
        limit,
        "the record is exactly full at half the round count"
    );
    assert!(
        !mem.watch_arena_capped(),
        "exactly full is not yet overflow"
    );
    let at = mem.alloc(8, 8).unwrap();
    mem.free(at);
    assert!(
        mem.watch_arena_capped(),
        "the event that did not fit has to leave the record marked short"
    );
    assert_eq!(mem.watch_arena.borrow().len(), limit);
}

#[test]
fn a_split_remainder_is_not_charged_to_the_free_that_made_the_space() {
    // A recycled block split by a smaller request derives two nodes from one freed address, and the
    // record is keyed by node address: the head is named by the `free`, the remainder is not. Pinned
    // as behaviour so that "no earlier free names this address" is never read as "never handed out".
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8400_0000);
    mem.watch_span(0x8000_0000, 0x400);
    mem.watch_steps.set(7);
    let wide = mem.alloc(0x400, 8).unwrap();
    mem.free(wide);
    mem.watch_steps.set(99);
    let head = mem.alloc(0x100, 8).unwrap();
    assert_eq!(head, wide, "the reuse takes the head of the freed span");
    let tail = mem.alloc(0x100, 8).unwrap();
    assert_eq!(
        tail,
        wide + 0x10C,
        "and the next request takes the remainder"
    );

    let recycled: Vec<(u32, Option<u64>)> = mem
        .watch_arena
        .borrow()
        .iter()
        .filter(|e| e.event == "alloc" && e.returned_at.is_some())
        .map(|e| (e.start, e.returned_at))
        .collect();
    assert_eq!(
        recycled,
        vec![(head, Some(7))],
        "only the node the `free` named carries its instruction count: {recycled:?}"
    );
}

#[test]
fn a_wide_string_write_stops_at_its_own_terminator() {
    // The helper terminates; the slice is the text. A caller that passes a slice with the NUL in it
    // and sizes the block for that slice writes one unit past the block — and the delivered heap keeps
    // the next node's size word exactly there, so the heap is what pays for the extra unit.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    let at = mem.alloc(8, 2).unwrap();
    mem.fill(at, 8, 0xAA).unwrap();
    mem.write_u16str(at, &[0x41, 0x42]).unwrap();
    assert_eq!(mem.read_u16(at).unwrap(), 0x41);
    assert_eq!(mem.read_u16(at + 2).unwrap(), 0x42);
    assert_eq!(
        mem.read_u16(at + 4).unwrap(),
        0,
        "the terminator follows the text"
    );
    assert_eq!(
        mem.read_u16(at + 6).unwrap(),
        0xAAAA,
        "and nothing past the terminator is written"
    );
}

#[test]
#[should_panic(expected = "lands on the node header")]
fn a_store_into_the_heap_s_own_metadata_is_refused_where_it_happens() {
    // The whole reason the guard exists: this store is what a one-unit overrun looks like from the
    // heap's side, and without the guard nothing notices until a later allocation finds the chain
    // broken and reports from somewhere else.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    let a = mem.alloc(8, 4).unwrap();
    let header = a - crate::heap::NODE;
    mem.write_u32(header, 0).unwrap();
}

#[test]
#[should_panic(expected = "lands on the node header")]
fn a_bulk_fill_that_reaches_a_node_header_is_refused_too() {
    // The byte-wise path is not the only one: `fill` and `write_bytes` write a page at a time, and a
    // guard that only watched `write_u8` would miss them.
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8000_1000);
    let a = mem.alloc(8, 4).unwrap();
    let header = a - crate::heap::NODE;
    mem.fill(header + 2, 4, 0).unwrap();
}

/// A state is only worth having if it comes back: the mapped bytes, the heap cursors and the free
/// list, byte for byte — and a block that ends early is refused rather than half-read, because a
/// machine restored from half a state faults somewhere the pack never went.
#[test]
fn a_memory_state_round_trips_and_a_short_one_is_refused() {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    mem.map(0x1000_0000, (PAGE_SIZE * 3) as u32);
    mem.write_u32(0x1000_0100, 0xdead_beef).unwrap();
    let block = mem.alloc(64, 4).unwrap();
    mem.write_bytes(block, &[7u8; 64]).unwrap();

    let mut before = Vec::new();
    mem.write_state(&mut before);

    let mut fresh = Mem::new();
    fresh.read_state(&mut State::new(&before)).unwrap();
    assert_eq!(
        fresh.read_u32(0x1000_0100).unwrap(),
        0xdead_beef,
        "a page's bytes came back"
    );
    assert_eq!(
        fresh.read_bytes(block, 64).unwrap(),
        vec![7u8; 64],
        "and the block the allocator handed out"
    );
    let mut after = Vec::new();
    fresh.write_state(&mut after);
    assert_eq!(after, before, "the state is byte for byte the same");

    assert!(
        fresh
            .read_state(&mut State::new(&before[..before.len() - 1]))
            .is_err(),
        "one byte short is refused"
    );
    assert!(
        fresh
            .read_state(&mut State::new(b"a state from somewhere else"))
            .is_err(),
        "and so is anything that is not one"
    );
}

/// `AEEkHeap_Init` hands the heap one free node covering the whole initial slice, and on a
/// handset every byte of that slice is writable before the first allocation touches it. A pack
/// that clears the region it owns with one `memset` writes pages the allocator has not reached
/// yet; on this host those pages must be present too, or the clear faults where a handset would
/// simply zero its own memory.
#[test]
fn the_whole_initial_heap_is_writable_before_any_allocation() {
    let mut mem = Mem::new();
    mem.set_heap(0x8000_0000, 0x8040_0000);
    let start = 0x8000_0000 + crate::heap::HEADER + crate::heap::NODE;
    let len = crate::heap::INITIAL - crate::heap::HEADER - crate::heap::NODE;
    mem.fill(start, len, 0)
        .expect("the initial slice is the heap's own, page by page");
    assert_eq!(
        mem.read_u8(0x8000_0000 + crate::heap::INITIAL - 1),
        Ok(0),
        "the last byte of the slice was written, not faulted"
    );
}

#[test]
fn copy_bytes_handles_overlap_within_one_page() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    mem.map(base, 0x1000);
    mem.write_bytes(base, b"abcdefgh").unwrap();
    mem.copy_bytes(base + 2, base, 6).unwrap();
    assert_eq!(mem.read_bytes(base, 8).unwrap(), b"ababcdef");
}

#[test]
fn copy_bytes_moves_a_run_across_pages() {
    let mut mem = Mem::new();
    let base = 0x1000_0ffc;
    mem.map(base, 0x2000);
    mem.write_bytes(base, b"abcdefgh").unwrap();
    mem.copy_bytes(base + 0x1000, base, 8).unwrap();
    assert_eq!(mem.read_bytes(base + 0x1000, 8).unwrap(), b"abcdefgh");
}

/// A pattern that differs from page to page. A copy test has to prove both that the destination
/// got the source bytes and that a `page_pair_mut` which indexes the wrong half of the split did
/// not quietly read some other page: a wrong-but-plausible byte is worse than a panic.
fn fill_pattern(mem: &mut Mem, base: u32, len: usize) {
    let bytes: Vec<u8> = (0..len)
        .map(|i| {
            let page = (i / PAGE_SIZE) as u8;
            (i as u8)
                .wrapping_mul(31)
                .wrapping_add(page.wrapping_mul(97))
                .wrapping_add(0x5a)
        })
        .collect();
    mem.write_bytes(base, &bytes).unwrap();
}

#[test]
fn copy_bytes_downwards_between_pages_of_one_superpage() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    let span = 16 * PAGE_SIZE;
    mem.map(base, span as u32);
    fill_pattern(&mut mem, base, span);
    // Page 10 -> page 6 of the same 1 MiB superpage: the `s_off > d_off` half of
    // `page_pair_mut`. A pack's memmove reached this and took `lo[s_off..s_off + PAGE_SIZE]`
    // out of a slice only `s_off` bytes long. The old tests all copied *upwards*.
    let src = base + 10 * PAGE_SIZE as u32 + 0x30;
    let dst = base + 6 * PAGE_SIZE as u32 + 0x40;
    let len = 0x100;
    let before = mem.read_bytes(base, span).unwrap();
    let expected = mem.read_bytes(src, len).unwrap();

    mem.copy_bytes(dst, src, len).unwrap();

    assert_eq!(
        mem.read_bytes(dst, len).unwrap(),
        expected,
        "the source bytes arrived"
    );
    let after = mem.read_bytes(base, span).unwrap();
    let d = (dst - base) as usize;
    for (i, (was, now)) in before.iter().zip(&after).enumerate() {
        if !(d..d + len).contains(&i) {
            assert_eq!(*was, *now, "byte {i:#x} outside the destination changed");
        }
    }
}

#[test]
fn copy_bytes_upwards_between_pages_of_one_superpage() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    let span = 16 * PAGE_SIZE;
    mem.map(base, span as u32);
    fill_pattern(&mut mem, base, span);
    let src = base + 3 * PAGE_SIZE as u32 + 0x30;
    let dst = base + 9 * PAGE_SIZE as u32 + 0x40;
    let len = 0x100;
    let expected = mem.read_bytes(src, len).unwrap();

    mem.copy_bytes(dst, src, len).unwrap();

    assert_eq!(mem.read_bytes(dst, len).unwrap(), expected);
}

#[test]
fn copy_bytes_downwards_walks_several_pages() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    let span = 20 * PAGE_SIZE;
    mem.map(base, span as u32);
    fill_pattern(&mut mem, base, span);
    // Long enough that `copy_bytes`'s loop visits several page pairs, each with a different
    // `(s_pi, d_pi)` but every one of them source-above-destination.
    let src = base + 12 * PAGE_SIZE as u32 + 0x100;
    let dst = base + 2 * PAGE_SIZE as u32 + 0x300;
    let len = 3 * PAGE_SIZE + 0x200;
    let expected = mem.read_bytes(src, len).unwrap();

    mem.copy_bytes(dst, src, len).unwrap();

    assert_eq!(mem.read_bytes(dst, len).unwrap(), expected);
}

#[test]
fn copy_bytes_across_superpages_in_both_directions() {
    let mut mem = Mem::new();
    let a = 0x100f_f000u32; // last page of superpage 0x100 (page 0x100ff)
    let b = 0x1010_0000u32; // first page of superpage 0x101 (page 0x10100)
    mem.map(a, 2 * PAGE_SIZE as u32);
    fill_pattern(&mut mem, a, 2 * PAGE_SIZE);

    let up = mem.read_bytes(a + 0x10, 0x100).unwrap();
    mem.copy_bytes(b + 0x20, a + 0x10, 0x100).unwrap();
    assert_eq!(
        mem.read_bytes(b + 0x20, 0x100).unwrap(),
        up,
        "low superpage -> high superpage"
    );

    let down = mem.read_bytes(b + 0x30, 0x100).unwrap();
    mem.copy_bytes(a + 0x40, b + 0x30, 0x100).unwrap();
    assert_eq!(
        mem.read_bytes(a + 0x40, 0x100).unwrap(),
        down,
        "high superpage -> low superpage"
    );
}

#[test]
fn copy_bytes_to_a_never_mapped_superpage_faults_instead_of_panicking() {
    let mut mem = Mem::new();
    let base = 0x1000_0000;
    let far = 0xf000_0000u32;
    mem.map(base, PAGE_SIZE as u32);
    mem.write_bytes(base, b"abcdefgh").unwrap();
    // Neither half of the split may be indexed with a superpage slot that was never allocated.
    // An unmapped guest address is a guest fault, not a host panic.
    for (dst, src) in [(far, base), (base, far)] {
        assert_eq!(
            mem.copy_bytes(dst, src, 8),
            Err(Fault::Unmapped { addr: far }),
            "copy {src:#010x} -> {dst:#010x}"
        );
    }
}

#[test]
fn compare_bytes_reports_the_first_difference() {
    let mut mem = Mem::new();
    let a = 0x1000_2000;
    let b = 0x1000_3000;
    mem.map(a, 0x1000);
    mem.map(b, 0x1000);
    mem.write_bytes(a, &[1, 2, 3, 4]).unwrap();
    mem.write_bytes(b, &[1, 2, 9, 4]).unwrap();
    assert_eq!(
        mem.compare_bytes(a, b, 4).unwrap(),
        std::cmp::Ordering::Less
    );
    mem.write_bytes(b, &[1, 2, 3, 4]).unwrap();
    assert_eq!(
        mem.compare_bytes(a, b, 4).unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        mem.compare_bytes(b, a, 4).unwrap(),
        std::cmp::Ordering::Equal
    );
}

/// The split between the two generations: `map_generation` records layout changes,
/// `window_generation` records only "a published JIT window may be stale". The heap maps once for
/// every new page; if the latter advanced too, the JIT would invalidate and recompile every hot block
/// (about 21 times in 300 ms of PvZ virtual time).
#[test]
fn mapping_new_pages_advances_the_layout_generation_but_not_the_window_one() {
    let mut mem = Mem::new();
    let base = 0x1000_0000u32;
    mem.map(base, 0x3000);
    let layout = mem.map_generation();
    assert!(layout > 0);
    assert_eq!(mem.window_generation(), 0);

    // An idempotent map: neither generation moves.
    mem.map(base, 0x3000);
    assert_eq!(mem.map_generation(), layout);
    assert_eq!(mem.window_generation(), 0);

    // Heap growth crossing into a new page: the layout changed, the windows did not.
    mem.map(base + 0x3000, 0x1000);
    assert!(mem.map_generation() > layout);
    assert_eq!(mem.window_generation(), 0);

    // `unmap` digs a page out of a window: the inlined path does not consult the bitmap, so the
    // cache must be invalidated.
    mem.unmap(base + 0x1000, 0x1000);
    assert_eq!(mem.window_generation(), 1);
    assert!(mem.map_generation() > layout);

    // Mapping it back must not invalidate a second time (the windows were already republished on
    // `unmap`).
    let after_unmap = mem.window_generation();
    mem.map(base + 0x1000, 0x1000);
    assert_eq!(mem.window_generation(), after_unmap);

    // Arming a watchpoint changes the window gating; arming the same span twice does not advance
    // again.
    mem.watch_span(base + 0x10, 4);
    assert_eq!(mem.window_generation(), after_unmap + 1);
    mem.watch_span(base + 0x10, 4);
    assert_eq!(mem.window_generation(), after_unmap + 1);
}

/// The heap and the stack each live in their own contiguous run, so windows are counted in **runs**,
/// not in 1 MiB units.
///
/// This test recorded two things, and the order matters:
/// 1. When every super page was allocated on its own, a 5 MiB heap folded into five runs, exhausted
///    the four slots and pushed the stack, at the highest address, out; raising the slot count from 4
///    to 8 only relieved it — the stack landed fifth in the comparison chain and was still 0.87
///    ns/instruction dearer than first.
/// 2. Now `Span` hands out 16 MiB of contiguous memory at a time, the heap is **one run**, and the
///    stack is back at the head of the chain.
///
/// So the asserted number went from 7 to 3; what did not change is the sentence "the stack must get a
/// window".
#[test]
fn the_stack_keeps_a_window_when_the_heap_needs_several_spans() {
    let mut mem = Mem::new();
    let module = 0x1000_0000u32;
    let heap = 0x2000_0000u32;
    let stack = 0x2FF8_0000u32;
    mem.map(module, 0x5_7000);
    mem.map(heap, 0x50_0000); // 5 MiB, grown page by page but inside one contiguous run
    mem.map(stack, 0x8_0000); // 512 KiB, the highest addresses

    let spans = mem.fast_spans();
    let present = |base: u32| spans.iter().any(|span| span.0 == base && span.2 > 0);
    assert!(present(stack), "the stack must get a window: {spans:?}");
    assert!(
        present(module),
        "the image segment must be kept too: {spans:?}"
    );
    assert_eq!(
        spans.iter().filter(|s| *s != &(0, 0, 0)).count(),
        3,
        "image + heap + stack, one segment each: {spans:?}"
    );
    assert_eq!(
        spans[0],
        (heap, (heap >> PAGE_SHIFT) as usize, 0x50_0000 / PAGE_SIZE),
        "a 5 MiB heap must be one continuous window, not five 1 MiB segments: {spans:?}"
    );
}

/// A heap wider than 16 MiB opens a second span — a boundary, not a defect, written down so it is not
/// mistaken for a regression next time.
///
/// One span is [`SPAN_SUPERS`] super pages of 1 MiB. A heap that crosses it is necessarily broken in
/// host memory, so the JIT sees two spans and the stack is pushed back one place. Removing that would
/// mean letting a span grow on demand, and then every growth has to reallocate and move the bytes
/// already there (every pointer in `page_table` would dangle).
#[test]
fn a_heap_wider_than_one_span_becomes_two_windows() {
    let mut mem = Mem::new();
    let heap = 0x2000_0000u32;
    mem.map(heap, 0x140_0000); // 20 MiB: wider than one 16 MiB span
    let spans = mem.fast_spans();
    assert_eq!(
        spans.iter().filter(|s| *s != &(0, 0, 0)).count(),
        2,
        "a 20 MiB heap splits into two segments: {spans:?}"
    );
    assert_eq!(
        spans[0].2,
        16 * 0x10_0000 / PAGE_SIZE,
        "the first segment is a full span"
    );
    assert_eq!(
        spans[1].2,
        4 * 0x10_0000 / PAGE_SIZE,
        "the second segment is the remaining 4 MiB"
    );
    assert_eq!(
        spans[0].0 + (spans[0].2 * PAGE_SIZE) as u32,
        spans[1].0,
        "the two segments must be adjacent in guest address space, or a hole appears that the JIT cannot see"
    );
}

/// The precondition a frozen window's correctness rests on: every page a published window covers is
/// still mapped after a later `map`, and the new pages necessarily fall outside the window (so they go
/// through the callback instead of being inlined and silently read as 0).
#[test]
fn a_published_window_stays_fully_mapped_when_new_pages_arrive() {
    let mut mem = Mem::new();
    let base = 0x1000_0000u32;
    mem.map(base, 0x8000);
    let spans = mem.fast_spans();
    let (win_base, start_page, pages) = spans[0];
    assert!(pages >= 2, "the test needs a multi-page window");
    let win_end = win_base + (pages * PAGE_SIZE) as u32;

    mem.map(base + 0x8000, 0x4000);
    for p in 0..pages {
        assert!(
            mem.is_mapped(win_base + (p * PAGE_SIZE) as u32),
            "page {p} inside the frozen window must not disappear"
        );
        assert!(mem.page_ptr(start_page + p).is_some());
    }
    assert!(
        base + 0x8000 >= win_end,
        "the new page must land outside the frozen window"
    );

    // Republishing really would produce a longer window — and because it would, the JIT has to freeze
    // it rather than have each new block ask `fast_spans` again (block linking shares one primary
    // window base).
    assert!(mem.fast_spans()[0].2 > pages);
}

/// `--compat unmapped-read` answers an unmapped read with zero: a handset's map is continuous RAM,
/// so a pack that reads past the end of its own image gets whatever was there rather than a data
/// abort. This host maps exactly what it was asked to, so the same read faults — right about the
/// address, wrong about the machine.
///
/// What it must not do is invent memory to *write* into. A store past the map is a bug on the
/// handset too, and turning it into a silent drop would move one corruption's evidence somewhere
/// else entirely, which is the failure shape this repository treats as the worst one.
#[test]
fn compat_unmapped_read_answers_zero_and_stores_still_fault() {
    let mut mem = Mem::new();
    mem.map(0x1000_0000, 0x1000);

    assert_eq!(
        mem.read_u32(0x1000_4000),
        Err(Fault::Unmapped { addr: 0x1000_4000 }),
        "unmapped reads fault by default"
    );

    mem.set_compat_read_zero(true);
    assert_eq!(mem.read_u32(0x1000_4000).unwrap(), 0, "zeros under compat");
    assert_eq!(mem.read_u8(0).unwrap(), 0, "page zero is included");
    assert_eq!(
        mem.read_bytes(0x1000_4000, 8).unwrap(),
        vec![0u8; 8],
        "and a run of bytes reads as zeros"
    );

    assert!(
        mem.write_u8(0x1000_4000, 1).is_err(),
        "a store past the map is still a fault"
    );
}
