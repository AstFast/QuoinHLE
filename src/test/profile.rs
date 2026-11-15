//! The folded call tree, tested against readings a test can state exactly.
//!
//! Every test here drives [`Tree`] directly rather than through a run: the tree's inputs are two
//! integers (steps and nanoseconds at each enter and exit) and a name, and a test that went through
//! a pack would be measuring the pack. The runtime's own wiring — that a dispatch opens a frame and
//! that a downcall opens one too — is what `src/test/runtime.rs` covers with a synthetic module.

use super::*;

/// A reading, so a test can say what the clock was.
fn at(steps: u64, nanos: u64) -> Cost {
    Cost { steps, nanos }
}

#[test]
fn a_callee_entered_twice_from_one_parent_is_one_node_and_two_calls() {
    let mut t = Tree::new();
    for _ in 0..2 {
        t.enter(Who::Span("frame"), at(100, 1_000));
        t.enter(Who::Dispatch(7), at(110, 1_100));
        t.exit(at(130, 1_900));
        t.exit(at(160, 2_500));
    }
    let kids = t.children(ROOT, Weight::Steps);
    assert_eq!(kids.len(), 1, "two turns of the same frame are one node");
    let frame = kids[0];
    assert_eq!(t.calls(frame), 2);
    assert_eq!(t.total(frame), at(120, 3_000));
    let disp = t.children(frame, Weight::Steps)[0];
    assert_eq!(t.calls(disp), 2);
    assert_eq!(t.total(disp), at(40, 1_600));
}

#[test]
fn a_nodes_own_cost_is_its_total_minus_its_children() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(100, 1_000));
    t.enter(Who::Dispatch(7), at(110, 1_100));
    t.exit(at(130, 1_900));
    t.exit(at(160, 2_500));
    let frame = t.children(ROOT, Weight::Steps)[0];
    let disp = t.children(frame, Weight::Steps)[0];
    // 60 steps under the frame, 20 of them under the dispatch: the other 40 are the frame's own.
    assert_eq!(t.total(frame), at(60, 1_500));
    assert_eq!(t.own(frame), at(40, 700));
    assert_eq!(t.own(disp), t.total(disp));
    assert_eq!(
        t.own(frame).steps + t.total(disp).steps,
        t.total(frame).steps,
        "own + children must add back up to the total"
    );
}

#[test]
fn frames_nest_as_a_stack_so_a_re_entrant_call_lands_under_its_parent() {
    // The shape the runtime really produces: the host calls into the module, the module calls back
    // into the host, and that call calls into the module again — `AEEApplet_New` does exactly this.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Downcall(0x459), at(10, 100));
    t.enter(Who::Dispatch(3), at(20, 200));
    t.enter(Who::Downcall(0x511), at(30, 300));
    t.exit(at(40, 400));
    t.exit(at(70, 700));
    t.exit(at(90, 900));
    t.exit(at(100, 1_000));

    let frame = t.children(ROOT, Weight::Steps)[0];
    let outer = t.children(frame, Weight::Steps)[0];
    let disp = t.children(outer, Weight::Steps)[0];
    let inner = t.children(disp, Weight::Steps)[0];
    assert_eq!(t.total(inner), at(10, 100));
    assert_eq!(
        t.own(outer),
        at(30, 300),
        "outer's own is what closed outside the dispatch under it"
    );
    assert_eq!(t.own(disp), at(40, 400));
    assert_eq!(t.own(frame), at(20, 200));
}

#[test]
fn an_unarmed_profile_folds_nothing_and_says_it_recorded_nothing() {
    let mut p = Profile::new();
    assert!(!p.on(), "off unless someone asks");
    p.enter(Who::Span("frame"), 0);
    p.exit(10);
    assert!(p.tree().is_empty());
    assert_eq!(p.tree().entered(), 0);

    p.set(true);
    p.enter(Who::Span("frame"), 10);
    p.exit(20);
    assert!(!p.tree().is_empty());
    assert_eq!(p.tree().entered(), 1);
}

#[test]
fn turning_the_switch_off_again_stops_recording_but_keeps_the_tree() {
    let mut p = Profile::new();
    p.set(true);
    p.enter(Who::Span("frame"), 0);
    p.exit(10);
    p.set(false);
    p.enter(Who::Span("frame"), 100);
    p.exit(200);
    assert_eq!(p.tree().entered(), 1, "the unarmed pair is not recorded");
    assert!(!p.tree().is_empty(), "and what was folded stays readable");
}

#[test]
fn a_frame_past_the_depth_ceiling_is_dropped_and_the_report_can_say_so() {
    let mut t = Tree::new();
    let rounds = MAX_DEPTH + 8;
    for _ in 0..rounds {
        t.enter(Who::Span("deep"), at(0, 0));
    }
    for _ in 0..rounds {
        t.exit(at(1, 1));
    }
    assert!(
        t.dropped() > 0,
        "silently stopping at the ceiling is the bug"
    );
    assert!(
        t.nodes() <= MAX_DEPTH + 2,
        "the refused frames must not become nodes"
    );
    // The pairs still balanced: one enter per exit, refused or not.
    assert_eq!(t.entered(), rounds as u64);
}

#[test]
fn a_callee_past_the_node_ceiling_is_refused_rather_than_merged_into_a_neighbour() {
    let mut t = Tree::new();
    for i in 0..(MAX_NODES as u32 + 16) {
        t.enter(Who::Downcall(i), at(0, 0));
        t.exit(at(1, 1));
    }
    assert!(t.dropped() > 0);
    assert!(t.nodes() <= MAX_NODES);
    // Every distinct callee that fit kept its own node: the first and the last are not the same.
    let kids = t.children(ROOT, Weight::Steps);
    assert_eq!(kids.len(), t.nodes() - 1);
}

#[test]
fn a_reading_that_goes_backwards_is_clamped_and_never_wraps() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(1_000, 1_000));
    t.exit(at(900, 400));
    let frame = t.children(ROOT, Weight::Steps)[0];
    assert_eq!(t.total(frame), at(0, 0));
    assert_eq!(t.own(frame), at(0, 0));
    assert_eq!(t.total(ROOT), at(0, 0));
}

#[test]
fn children_are_ordered_by_width_under_the_weight_the_report_chose() {
    let mut t = Tree::new();
    // Three callees under one frame with different step and nanosecond widths, so the two weights
    // disagree about the order and the test can tell which one was used.
    let plan = [
        ("narrow-steps", 10u64, 900u64),
        ("wide", 50, 100),
        ("middle", 30, 400),
    ];
    t.enter(Who::Span("frame"), at(0, 0));
    for (name, steps, nanos) in plan {
        t.enter(Who::Span(name), at(0, 0));
        t.exit(at(steps, nanos));
    }
    t.exit(at(100, 1_000));
    let frame = t.children(ROOT, Weight::Steps)[0];

    let by_steps: Vec<&str> = t
        .children(frame, Weight::Steps)
        .iter()
        .map(|i| match t.node(*i).who {
            Who::Span(s) => s,
            _ => "?",
        })
        .collect();
    assert_eq!(by_steps, vec!["wide", "middle", "narrow-steps"]);

    let by_nanos: Vec<&str> = t
        .children(frame, Weight::Nanos)
        .iter()
        .map(|i| match t.node(*i).who {
            Who::Span(s) => s,
            _ => "?",
        })
        .collect();
    assert_eq!(by_nanos, vec!["narrow-steps", "middle", "wide"]);
}

#[test]
fn lines_cut_narrow_and_deep_rows_out_loudly_rather_than_silently() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Span("wide"), at(0, 0));
    t.exit(at(900, 0));
    t.enter(Who::Span("sliver"), at(0, 0));
    t.exit(at(1, 0));
    t.exit(at(1000, 0));

    let (lines, elided) = t.lines(Options {
        weight: Weight::Steps,
        max_depth: 4,
        min_share: 0.02,
    });
    let names: Vec<&str> = lines
        .iter()
        .map(|l| match t.node(l.node).who {
            Who::Span(s) => s,
            _ => "?",
        })
        .collect();
    assert_eq!(names, vec!["run", "frame", "wide"]);
    assert_eq!(elided, 1, "the 0.1% row is counted, not lost");
    assert!((lines[2].share - 0.9).abs() < 1e-9);
}

#[test]
fn a_tree_with_nothing_but_its_root_is_empty_and_draws_no_rows() {
    let t = Tree::new();
    assert!(t.is_empty());
    let (lines, elided) = t.lines(Options::default());
    assert!(
        lines.is_empty(),
        "a width of zero is not a row of zero-width bars"
    );
    assert_eq!(elided, 0);
}

#[test]
fn clearing_keeps_the_mount_point_and_forgets_what_was_under_it() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.exit(at(10, 0));
    assert!(!t.is_empty());
    t.clear();
    assert!(t.is_empty());
    assert_eq!(t.nodes(), 1);
    assert_eq!(t.calls(ROOT), 1);
    assert_eq!(t.dropped(), 0);
    assert_eq!(t.entered(), 0);
    t.enter(Who::Span("frame"), at(0, 0));
    t.exit(at(10, 0));
    assert!(!t.is_empty(), "the tree still works after a reset");
}

#[test]
fn a_name_is_what_the_reader_would_call_it() {
    assert_eq!(Who::Span("frame").name(0), "frame");
    assert_eq!(Who::Dispatch(0).kind(), "call");
    assert_eq!(Who::Downcall(0x99).kind(), "down");

    // The applet's own event handler is the one downcall address with a name; every other slot is
    // the module's private code and keeps its number.
    assert_eq!(Who::Downcall(0x459).name(0x459), "IApplet::HandleEvent");
    assert_eq!(Who::Downcall(0x459).name(0x511), "guest 0x00000459");

    // A dispatch is named by the ABI table, which is in the repository, so this is a fact about the
    // table and not about a pack.
    let id = abi::method_of("IShell", "CreateInstance").expect("published by the ABI table");
    let label = Who::Dispatch(id).name(0);
    assert!(
        label.contains("CreateInstance"),
        "expected a method name, got {label:?}"
    );
}

#[test]
fn the_root_is_as_wide_as_what_it_directly_holds() {
    let mut t = Tree::new();
    t.enter(Who::Span("a"), at(0, 0));
    t.exit(at(30, 0));
    t.enter(Who::Span("b"), at(0, 0));
    t.exit(at(70, 0));
    assert_eq!(t.total(ROOT), at(100, 0));
    assert_eq!(t.own(ROOT), at(0, 0));
    let (lines, _) = t.lines(Options {
        weight: Weight::Steps,
        max_depth: 4,
        min_share: 0.0,
    });
    assert_eq!(lines[0].share, 1.0, "the root is the whole of the width");
    assert!((lines[1].share - 0.7).abs() < 1e-9, "widest child first");
}

#[test]
fn a_bar_is_as_long_as_the_share_it_stands_for() {
    assert_eq!(bar(0.0), " ".repeat(BAR));
    assert_eq!(bar(1.0), "#".repeat(BAR));
    assert_eq!(bar(0.5).matches('#').count(), BAR / 2);
    // A share outside the range is drawn as one of the ends, not as a panic or a negative count.
    assert_eq!(bar(-0.5), " ".repeat(BAR));
    assert_eq!(bar(1.5), "#".repeat(BAR));
}

#[test]
fn a_report_names_every_row_and_says_how_wide_it_was() {
    // One frame of a run: a branch into the module, and one host call inside it.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(100, 1_000_000));
    t.enter(Who::Downcall(0x459), at(110, 1_100_000));
    let bitblt = abi::method_of("IDisplay", "BitBlt").expect("published by the ABI table");
    t.enter(Who::Dispatch(bitblt), at(120, 1_200_000));
    t.exit(at(130, 3_000_000));
    t.exit(at(900, 4_000_000));
    t.exit(at(950, 5_000_000));

    let text = t.report(0x459, Options::default());
    // The heading says which clock drew the bars, because the same tree reads differently under the
    // other one.
    assert!(text.contains("call tree (drawn by wall clock)"), "{text}");
    // Every row is named: the frame, the one downcall address that has a name, and the host method.
    assert!(text.contains("frame"), "{text}");
    assert!(text.contains("IApplet::HandleEvent"), "{text}");
    assert!(text.contains("BitBlt"), "{text}");
    // Widths, in wall clock: the handler held 2.9 of the run's 4.0 ms, the blit 1.8 of them.
    assert!(text.contains("100.0%"), "{text}");
    assert!(text.contains("72.5%"), "{text}");
    assert!(text.contains("45.0%"), "{text}");
    // The instruction column travels beside it: the guest retired 780 while the handler ran, and
    // only 10 of those during the blit — which is the reason host code is invisible under that
    // weight and why the bar is drawn by the clock.
    assert!(text.contains("790 instr"), "{text}");
    assert!(text.contains("10 instr"), "{text}");
    // A row whose children accounted for nothing says so, rather than leaving a reader to subtract.
    assert!(text.contains("(all its own)"), "{text}");
    assert!(text.contains("2x") || text.contains(" 1x"), "{text}");
}

#[test]
fn a_report_says_when_it_cut_rows_instead_of_ending_like_a_complete_one() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    // Three callees of different widths under one frame: the narrowest is below the cut. They must
    // have different names — the same name would fold into one node, which is the property the test
    // above already covers and would hide the cut here.
    for (name, nanos) in [
        ("wide", 900_000_000u64),
        ("middle", 99_000_000),
        ("sliver", 1_000_000),
    ] {
        t.enter(Who::Span(name), at(0, 0));
        t.exit(at(0, nanos));
    }
    t.exit(at(0, 1_000_000_000));
    let text = t.report(
        0,
        Options {
            weight: Weight::Nanos,
            max_depth: 4,
            min_share: 0.05,
        },
    );
    assert!(text.contains("not shown"), "{text}");
    assert!(text.contains("narrower than 5.0%"), "{text}");
}

#[test]
fn a_rows_self_share_is_of_its_own_width_and_not_of_the_runs() {
    // The distinction the `% own` column and the developer page's hover read-out both rest on. A row
    // worth 10 % of the run can still be "25 % its own", and it is the second number that says whether
    // the width is this frame's work or its children's. The two numbers only differ below the first
    // level, so this tree is three deep: `frame` is the whole run, `child` is 40 % of it, and `leaf`
    // takes three quarters of `child`.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Span("child"), at(0, 0));
    t.enter(Who::Span("leaf"), at(0, 0));
    t.exit(at(0, 300));
    t.exit(at(0, 400));
    t.exit(at(0, 1_000));

    let f = t.flame(
        0,
        Options {
            weight: Weight::Nanos,
            max_depth: 8,
            min_share: 0.0,
        },
    );
    let share = |name: &str| {
        f.rows
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.share)
            .unwrap_or_else(|| panic!("no {name} row"))
    };
    let own = |name: &str| {
        f.rows
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.self_share)
            .unwrap_or_else(|| panic!("no {name} row"))
    };
    // Of the run: 100 %, 40 %, 30 %.
    assert!((share("frame") - 1.0).abs() < 1e-9);
    assert!((share("child") - 0.4).abs() < 1e-9);
    assert!((share("leaf") - 0.3).abs() < 1e-9);
    // Of the row itself: 600/1000, 100/400, 300/300 — the middle one is where a row-relative share and
    // a run-relative one part company (a run-relative reading of the same row would say 10 %).
    assert!((own("frame") - 0.6).abs() < 1e-9);
    assert!(
        (own("child") - 0.25).abs() < 1e-9,
        "child's own is a quarter of *its* width: {}",
        own("child")
    );
    assert!((own("leaf") - 1.0).abs() < 1e-9);
}

#[test]
fn a_saved_report_can_keep_every_row_while_a_printed_one_cuts_them() {
    // `min_share` belongs to the caller and not to the renderer: a terminal is somebody reading and
    // wants the table short, while a file the developer page saves is worth having whole. Both come out
    // of the same renderer, which is what keeps them from disagreeing about the numbers.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Span("wide"), at(0, 0));
    t.exit(at(0, 999_000_000));
    t.enter(Who::Span("sliver"), at(0, 0));
    t.exit(at(0, 1_000_000));
    t.exit(at(0, 1_000_000_000));
    let f = t.flame(
        0,
        Options {
            weight: Weight::Nanos,
            max_depth: 8,
            min_share: 0.0,
        },
    );

    let whole = f.report(0.0, Weight::Nanos);
    assert!(whole.contains("sliver"), "{whole}");
    assert!(!whole.contains("not shown"), "{whole}");
    assert!(whole.contains("4 node(s)"), "{whole}");

    let cut = f.report(0.05, Weight::Nanos);
    assert!(!cut.contains("sliver"), "{cut}");
    assert!(cut.contains("not shown"), "{cut}");
    // The tree still holds the row the cut report did not print, which is the reason a saved file is
    // worth keeping at all: it is the picture plus every row, not just the picture's rows.
    assert_eq!(f.rows.len(), 4, "run, frame, wide, sliver");
}

#[test]
fn a_report_over_a_run_that_recorded_nothing_says_zero_rather_than_nothing() {
    let t = Tree::new();
    let text = t.report(0, Options::default());
    assert!(text.contains("1 node(s)"), "{text}");
    assert!(text.contains("0.00 ms"), "{text}");
}

#[test]
fn rows_are_a_strict_depth_first_walk_so_indentation_can_be_trusted() {
    // What this caught, in a file saved from the developer page: `a`'s child used to be emitted after
    // `b`'s row, so a table indented by depth read as though that child belonged to `b`. Every number in
    // that file was self-consistent — the *order* was the lie, and indentation is how a call tree is
    // read.
    let mut t = Tree::new();
    t.enter(Who::Span("a"), at(0, 0));
    t.enter(Who::Span("a-child"), at(0, 0));
    t.exit(at(0, 300));
    t.exit(at(0, 600));
    t.enter(Who::Span("b"), at(0, 600));
    t.exit(at(0, 1_000));

    let (lines, _) = t.lines(Options {
        weight: Weight::Nanos,
        max_depth: 8,
        min_share: 0.0,
    });
    let names: Vec<&str> = lines
        .iter()
        .map(|l| match t.node(l.node).who {
            Who::Span(s) => s,
            _ => "?",
        })
        .collect();
    let depths: Vec<usize> = lines.iter().map(|l| l.depth).collect();
    assert_eq!(
        names,
        vec!["run", "a", "a-child", "b"],
        "a's child comes before b, or the indentation lies"
    );
    assert_eq!(
        depths,
        vec![0, 1, 2, 1],
        "each row is its parent's depth plus one"
    );
}

#[test]
fn a_flame_row_knows_where_it_starts_so_children_line_up_under_their_parents() {
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Span("wide"), at(0, 0));
    t.exit(at(0, 800));
    t.enter(Who::Span("narrow"), at(0, 0));
    t.exit(at(0, 200));
    t.exit(at(0, 1_000));

    let opt = Options {
        weight: Weight::Nanos,
        max_depth: 8,
        min_share: 0.0,
    };
    let f = t.flame(0, opt);
    let row = |name: &str| {
        f.rows
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("no {name} row in {:?}", f.rows))
    };
    assert_eq!(row("run").start, 0.0);
    assert_eq!(row("frame").start, 0.0);
    assert!((row("wide").start - 0.0).abs() < 1e-9);
    assert!(
        (row("narrow").start - 0.8).abs() < 1e-9,
        "the second child starts where the first one ended"
    );
    assert!((row("narrow").share - 0.2).abs() < 1e-9);
    assert_eq!(f.depth, 2, "run, frame, and the two callees under it");
    assert_eq!(f.nodes, 4);
    assert_eq!(f.folded, 3);
    assert_eq!(f.dropped, 0);
    assert!((f.total_ms - 0.001).abs() < 1e-12, "1000 ns is 0.001 ms");
}

#[test]
fn a_flame_never_cuts_a_row_by_width_because_a_gap_would_move_what_is_after_it() {
    // The narrow row sits between two wide ones, and a cut would pull the third one left into the
    // space the cut left behind — a picture that is wrong about where something is, which is worse
    // than one that is crowded.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    for (name, nanos) in [("wide", 600u64), ("sliver", 50), ("right", 350)] {
        t.enter(Who::Span(name), at(0, 0));
        t.exit(at(0, nanos));
    }
    t.exit(at(0, 1_000));

    let f = t.flame(
        0,
        Options {
            weight: Weight::Nanos,
            max_depth: 8,
            min_share: 0.5,
        },
    );
    assert_eq!(f.elided, 0, "a width cut is not a cut this call may make");
    // Widest first, so the rows are `wide`, `right`, `sliver` — and each begins where the one before
    // it ended, which is the property that makes this an icicle.
    let start = |name: &str| {
        f.rows
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("no {name} row in {:?}", f.rows))
            .start
    };
    assert!((start("wide") - 0.0).abs() < 1e-9);
    assert!((start("right") - 0.60).abs() < 1e-9);
    assert!(
        (start("sliver") - 0.95).abs() < 1e-9,
        "the narrowest row is drawn too, and holds its own width"
    );
}

#[test]
fn a_flame_that_was_cut_at_depth_still_lines_the_rest_of_the_rows_up() {
    // Cutting by depth is the cut that is safe: the rows that remain are still where they were.
    let mut t = Tree::new();
    t.enter(Who::Span("frame"), at(0, 0));
    t.enter(Who::Span("left"), at(0, 0));
    t.enter(Who::Span("deep"), at(0, 0));
    t.exit(at(0, 600));
    t.exit(at(0, 600));
    t.enter(Who::Span("right"), at(0, 0));
    t.exit(at(0, 400));
    t.exit(at(0, 1_000));

    let f = t.flame(
        0,
        Options {
            weight: Weight::Nanos,
            max_depth: 2,
            min_share: 0.0,
        },
    );
    assert!(f.elided > 0, "the deep row was cut, and the count says so");
    let right = f
        .rows
        .iter()
        .find(|r| r.name == "right")
        .expect("a row at the cut depth is still drawn");
    assert!(
        (right.start - 0.6).abs() < 1e-9,
        "its place is its own, cut or not: {}",
        right.start
    );
}
