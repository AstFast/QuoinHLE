// Moved out of `src/main.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod coverage_tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// What a run's tally looks like: each label once, with how many times it was reached. Built here
/// rather than by dispatching a guest, because the fold is the thing under test.
fn tally(calls: &[&str]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for label in calls {
        match out.iter_mut().find(|(seen, _)| seen == label) {
            Some((_, n)) => *n += 1,
            None => out.push(((*label).to_string(), 1)),
        }
    }
    out
}

#[test]
fn the_fold_counts_methods_per_interface_busiest_first() {
    let folded = coverage(&tally(&[
        "IShell::CreateInstance",
        "AEEHelperFuncs::malloc",
        "IShell::CreateInstance",
        "AEEHelperFuncs::malloc",
        "AEEHelperFuncs::malloc",
        "IShell::GetDeviceInfo",
    ]));
    assert_eq!(folded.len(), 2);
    assert_eq!(folded[0].0, "AEEHelperFuncs", "the busiest interface leads");
    assert_eq!(folded[0].1, vec![("malloc".to_string(), 3)]);
    assert_eq!(folded[0].2, 3, "the total is every call it took");
    assert_eq!(folded[1].0, "IShell");
    assert_eq!(folded[1].2, 3);
    assert_eq!(
        folded[1].1,
        vec![
            ("CreateInstance".to_string(), 2),
            ("GetDeviceInfo".to_string(), 1)
        ],
        "within an interface the busiest method leads too"
    );
}

#[test]
fn the_interface_census_adds_up_to_every_dispatch_the_run_took() {
    // `interfaces touched` is the line a reader counts interfaces from, so
    // its numbers have to sum to the dispatch total. It used to be folded out of the capped history
    // window, which reported a 16 013-dispatch run as 3 725 — and a census that silently is a window
    // is exactly how "this pack never asked for X" ends up written from what the list happens to hold.
    let line = interfaces_touched(&tally(&[
        "IShell::CreateInstance",
        "IShell::CreateInstance",
        "IShell::GetDeviceInfo",
        "AEEHelperFuncs::malloc",
        "IDisplay::Update",
    ]));
    assert_eq!(line, "IShell×3, AEEHelperFuncs×1, IDisplay×1", "{line}");
}

#[test]
fn a_label_with_no_interface_keeps_a_heading_of_its_own() {
    let folded = coverage(&tally(&["something-odd"]));
    assert_eq!(folded[0].0, "(unlabelled)");
    assert_eq!(folded[0].1, vec![("something-odd".to_string(), 1)]);
    assert!(declared_methods("something-odd").is_none());
}

#[test]
fn the_denominators_come_from_the_abi_tables() {
    // These two numbers are what make "7 of 117" sayable, and they are the tables' own counts
    // rather than a constant this report keeps — a constant here would be the copy that goes stale.
    assert_eq!(declared_methods(abi::HELPER_STRUCT), Some(117));
    assert!(
        declared_methods("IShell").unwrap_or(0) >= 50,
        "the shell's published method count"
    );
}

#[test]
fn nothing_reached_is_an_empty_fold_rather_than_a_panic() {
    assert!(coverage(&tally(&[])).is_empty());
}

/// The report shows both ends of a pack's debug lines: its own introduction and its last words.
///
/// Both halves are pinned. A head-only window hid the twelve `ScriptRunner-Command(N)` lines that
/// place a null-vtable fault inside a script loop; a tail-only window hid `Create filemgr:0`,
/// `My ClassID:0x…` and `ISHELL_CreateInstance Err`.
#[test]
fn the_debug_window_keeps_a_packs_first_lines_and_its_last_ones() {
    let said: Vec<String> = (0..41).map(|n| format!("line {n}")).collect();
    let (head, tail, skipped) = debug_window(&said, 24);
    assert_eq!(head.len(), 8, "a third of the budget is the introduction");
    assert_eq!(tail.len(), 16, "two thirds is where it stopped");
    assert_eq!(skipped, 17, "and the middle is counted, not silently lost");
    assert_eq!(head[0], "line 0");
    assert_eq!(head[7], "line 7");
    assert_eq!(tail[0], "line 25", "the gap is exactly what is not printed");
    assert_eq!(
        tail[15], "line 40",
        "and the last thing said always survives"
    );
    assert_eq!(
        head.len() + tail.len() + skipped,
        said.len(),
        "no line is both uncounted and unprinted"
    );

    // One line over the budget still leaves a real gap, and the two slices never overlap.
    let edge: Vec<String> = (0..25).map(|n| format!("line {n}")).collect();
    let (head, tail, skipped) = debug_window(&edge, 24);
    assert_eq!(skipped, 1);
    assert_eq!(head.len() + tail.len(), 24);
    assert_eq!(head[0], "line 0");
    assert_eq!(tail[tail.len() - 1], "line 24");

    // A short list is printed whole, with nothing claimed as missing: announcing a gap over
    // three lines would be the same lie in the other direction.
    let few: Vec<String> = (0..3).map(|n| format!("line {n}")).collect();
    let (head, tail, skipped) = debug_window(&few, 24);
    assert_eq!(skipped, 0);
    assert_eq!(head.len(), 3);
    assert!(
        tail.is_empty(),
        "nothing is held back, so there is nothing to append"
    );

    // Exactly one window's worth is not an overflow either, and neither is an empty list
    // (the caller guards on it, but the helper makes no such promise).
    let exact: Vec<String> = (0..24).map(|n| format!("line {n}")).collect();
    let (head, tail, skipped) = debug_window(&exact, 24);
    assert_eq!(skipped, 0);
    assert_eq!(head.len(), 24);
    assert!(tail.is_empty());
    let none: Vec<String> = Vec::new();
    let (head, tail, skipped) = debug_window(&none, 24);
    assert!(head.is_empty() && tail.is_empty() && skipped == 0);
}
