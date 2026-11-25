// Moved out of `src/main.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod frames_tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// The drawing-call list has to be checked against the tables, not against itself.
///
/// `drawing_calls` counts by method name read out of a trace label, and the name list was written
/// by hand from those tables — so its two failure modes are silent in opposite directions: a name
/// that no interface actually has matches nothing and quietly shrinks the number, and a method
/// renamed in `src/abi/generated/` does the same. Asserting the count on an empty runtime would
/// only restate the implementation, so this asserts the one thing that can actually break: every
/// name in the list is a real method under one of the interfaces that can move pixels.
#[test]
fn every_name_the_drawing_count_matches_is_a_real_method_somewhere() {
    const DRAWING: &[&str] = &[
        "IDisplay",
        "IGraphics",
        "IBitmap",
        "IImage",
        "IImageGrid",
        "IFont",
        "IText",
        "IStatic",
        "IClip",
        "IWidget",
        "IRscPool",
    ];
    // Read from the same list the report uses, by re-listing it here rather than exporting it: a
    // private const inside a function cannot be seen by a test, and making it visible just for a
    // test would let a caller build the count by hand, which is the thing not to have.
    let names = [
        "BitBlt",
        "StretchBlt",
        "BltIn",
        "BltOut",
        "SetPixels",
        "DrawPixel",
        "DrawHScanline",
        "FillRect",
        "ClearRect",
        "ClearViewport",
        "DrawRect",
        "DrawRoundRectangle",
        "DrawFrame",
        "Draw",
        "DrawText",
        "DrawPoint",
        "DrawLine",
        "DrawPolyline",
        "DrawPolygon",
        "DrawTriangle",
        "DrawCircle",
        "DrawArc",
        "DrawPie",
        "DrawEllipse",
        "DrawEllipticalArc",
    ];
    for name in names {
        assert!(
            DRAWING
                .iter()
                .any(|iface| quoin::abi::method_of(iface, name).is_some()),
            "`{name}` is in the drawing-call list but in no interface that can draw: a name that \
                 matches nothing makes the count lie by omission"
        );
    }
}

#[test]
fn a_stack_word_inside_the_image_is_a_candidate_return_address() {
    let base = 0x1000_0000;
    let words = [
        0x2002_1234,       // heap: a local, not a frame
        base + 0x40,       // inside: a return address
        0,                 // a null somebody stored
        base - 4,          // below the image
        base + 0x1000 - 1, // the last byte of it
        base + 0x1000,     // one past the end
    ];
    assert_eq!(
        return_addresses(&words, base, 0x1000),
        vec![base + 0x40, base + 0x1000 - 1],
        "only the words inside the image count, and the end is exclusive"
    );
}

#[test]
fn a_stack_with_nothing_in_the_image_produces_no_frames() {
    assert!(return_addresses(&[0, 0x2000_0000, 0x0fff_fff0], 0x1000_0000, 0x1000).is_empty());
    assert!(return_addresses(&[], 0x1000_0000, 0x1000).is_empty());
}

/// What `--dump-audio` prints beside a clip, and the extension that clip is written out with, are now
/// one look at the same two authorities rather than two hand-written magic-number tables.
///
/// They had already drifted from the code that serves the formats: the label table knew a BMP clip
/// existed (the old line said so in words) but nothing else, and the extension table wrote out as
/// `.bin` every clip it could not name — so a picture whose bytes a real decoder would take got a
/// label of "unrecognised" and a file no tool opens by its own name.
///
/// What this does **not** cover is the picture arm: a decodable PNG or BMP would have to be rebuilt
/// here, and a second copy of a fixture the library already owns is a second thing to get wrong. The
/// MIDI arm is the one this file used to mislabel, and it needs no events at all — a header declaring
/// zero tracks still parses, which is also the shape that used to crash the host (see
/// `src/test/audio_out.rs`).
#[test]
fn a_clip_is_named_by_the_code_that_would_have_to_play_it() {
    // MThd, header length 6, format 0, **track count 0**, division 0.
    let score = [b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        sniff_audio(&score),
        quoin::audio::MIME_MIDI,
        "the listing says the authority's own word"
    );
    assert_eq!(
        clip_extension(&score),
        "mid",
        "and the file name is that word's subtype"
    );

    // A waveform container is named now that Symphonia decodes WAV/ADPCM, and the extension follows
    // the authority rather than a hand-written magic-number table.
    let riff = *b"RIFF\0\0\0\0WAVEfmt ";
    assert_eq!(sniff_audio(&riff), quoin::audio::MIME_WAV);
    assert_eq!(clip_extension(&riff), "wav");

    // The case that separates this from the table it replaced: a `MThd` header promising three tracks
    // with nothing behind them. The old extension table gave `.mid` to **anything** starting `MThd`,
    // so this input is exactly where "it looks like MIDI" and "an authority will take it" part ways —
    // the label still says what it sees, the file name says nothing, and revert the arm and this fails.
    let stub = [b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 3, 0x01, 0xE0];
    assert_eq!(
        sniff_audio(&stub),
        "MThd, but not a MIDI file this host can read",
        "the shape is still worth saying out loud"
    );
    assert_eq!(
        clip_extension(&stub),
        "bin",
        "and a shape is not a format someone can open"
    );

    assert_eq!(sniff_audio(&[]), "empty", "nothing to name");
    assert_eq!(clip_extension(&[]), "bin");
    let junk = [0xa5u8; 8];
    assert_eq!(sniff_audio(&junk), "an unrecognised format");
    assert_eq!(clip_extension(&junk), "bin");
}

/// The `--dump-wav` writer, end to end, with no sound card and no guest run.
///
/// Everything about this flag that could be wrong — that it skips the clips this host cannot play,
/// that it renders through the same mixer the window hears, that the file it writes has a header
/// describing the bytes behind it — sits behind a `Runtime` and a directory, both of which a test can
/// hand it. `Runtime::new()` builds an empty one, so this needs no pack and starts no guest: the only
/// thing the writer reads is the list of sources a run would have named.
///
/// The MIDI bytes are written out here rather than imported: the library's `one_note_midi` fixture is
/// `#[cfg(test)]` on the *library* side and cannot cross into the binary. What matters to this test is
/// that the payload parses as MIDI and sounds, not that the fixture is one file.
#[test]
fn dump_wav_writes_an_audible_file_for_the_clip_it_can_play_and_none_for_the_one_it_cannot() {
    use quoin::runtime::AudioSource;

    let dir = std::env::temp_dir().join(format!("quoin-dump-wav-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut rt = Runtime::new();
    // MThd, format 0, one track, 480 ticks: middle C on, off a quarter note later, end of track.
    let mut score: Vec<u8> = Vec::new();
    score.extend_from_slice(b"MThd");
    score.extend_from_slice(&6u32.to_be_bytes());
    score.extend_from_slice(&0u16.to_be_bytes());
    score.extend_from_slice(&1u16.to_be_bytes());
    score.extend_from_slice(&480u16.to_be_bytes());
    score.extend_from_slice(b"MTrk");
    score.extend_from_slice(&12u32.to_be_bytes());
    score.extend_from_slice(&[0x00, 0x90, 60, 100]);
    score.extend_from_slice(&[0x83, 0x60, 0x80, 60, 0]);
    score.extend_from_slice(&[0x00, 0xff, 0x2f, 0x00]);
    rt.audio.push(AudioSource {
        input: quoin::abi::generated::SDT_BUFFER,
        data: 0x2000_0000,
        size: score.len() as u32,
        name: "theme.mid".to_string(),
        bytes: score.clone(),
        problem: None,
    });
    // A second source the container called a sound and this host has nothing for: the writer must
    // count it, not hand back a file of silence.
    rt.audio.push(AudioSource {
        input: quoin::abi::generated::SDT_BUFFER,
        data: 0x2000_1000,
        size: 4,
        name: "jingle.mp3".to_string(),
        bytes: b"\xff\xfb\x90\x00".to_vec(),
        problem: None,
    });

    let dir_text = dir.display().to_string();
    write_wav(&rt, &dir_text, None).expect("the dump runs");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("the directory exists")
        .map(|e| {
            let n = e.expect("an entry").file_name();
            n.to_string_lossy().into_owned()
        })
        .collect();
    files.sort();
    assert_eq!(
        files,
        vec!["0001-theme-mid.wav"],
        "the clip this host can play is written, the one it cannot is not - and the numbering says \
         which source each file came from (`safe_stem` takes the dot out of the name, the same as \
         `--dump-audio`)"
    );
    let bytes = std::fs::read(dir.join(&files[0])).expect("the file is there");
    assert_eq!(&bytes[0..4], b"RIFF", "and what it wrote is a RIFF file");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize + 8,
        bytes.len(),
        "whose own size field is true of the bytes behind it"
    );
    let peak = bytes[44..]
        .chunks(2)
        .map(|b| i16::from_le_bytes(b.try_into().unwrap()).abs() as i32)
        .max()
        .unwrap_or(0);
    assert!(peak > 1_000, "the render is silent: peak {peak}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The row that answers a player's question has to say what it carried, not where it landed.
#[test]
fn an_event_delivery_row_names_the_event_and_the_key() {
    let slot = 0x2003_1000;
    // The row is built the way the report builds it: a whole `Downcall` is what the runtime records,
    // and the label is asked for its address and words.
    let row = |addr: u32, evt: u32, wp: u32| {
        let call = quoin::hle::Downcall {
            addr,
            args: [0x0400_0000, evt, wp, 0],
            retval: 1,
            steps: 10,
        };
        downcall_label(slot, call.addr, &call.args)
    };
    assert_eq!(
        row(slot, 0x100, 0xE036),
        "← IApplet::HandleEvent EVT_KEY AVK_SOFT1",
        "a named key is the whole point of the row"
    );
    // Another callee keeps its address: naming it would be a guess about who was called.
    assert_eq!(row(0x2003_2000, 0x100, 0xE036), "← guest 0x20032000");
    // A word with no name under its prefix keeps its number, because there the number is the finding.
    assert_eq!(
        row(slot, 0x100, 0x0006_0000),
        "← IApplet::HandleEvent EVT_KEY wp=0x00060000"
    );
    assert_eq!(
        row(slot, 0xdead_beef, 0),
        "← IApplet::HandleEvent evt=0xdeadbeef wp=0x00000000",
        "and an event word that is not an event is said plainly rather than renamed"
    );
}

/// A `--dump` aimed at a register reads that register; one aimed at a place reads the place, and says
/// which it was. A heap address copied out of one run's report and pasted into the next run's `--dump`
/// prints recycled artwork with full confidence — the same log's arena rows may show that block was
/// freed 200 M instructions earlier. A register cannot
/// go stale that way, which is why the form exists; what this pins is the resolution, since the header
/// that carries the name prints to stdout and nothing here captures it.
#[test]
fn a_dump_address_can_name_a_register_instead_of_a_place() {
    let mut rt = Runtime::new();
    rt.cpu.r[7] = 0x2000_1234;
    assert_eq!(
        dump_address(&rt, "r7"),
        Some((0x2000_1234, Some("r7"))),
        "the register's value, and the name it came from"
    );
    assert_eq!(
        dump_address(&rt, " 0x1000").map(|(at, from)| (at, from.is_some())),
        Some((0x1000, false)),
        "a place resolves as a place and does not claim to be a register"
    );
    assert!(
        dump_address(&rt, "lr").is_some(),
        "sp and lr are registers too"
    );
    assert!(
        dump_address(&rt, "r15").is_none(),
        "r15 is the program counter, which is not in the bank `--break` and the register dump read"
    );
    assert!(dump_address(&rt, "somewhere").is_none());
}

/// Both lines a `--dump` can print carry the register the address came from.
///
/// The failure line did not, and that lost a finding: a run stopped at the draw gate was asked about
/// with `--dump r7`, got back `--dump NULL: cannot read 64 byte(s)`, and nothing in that line said
/// whether `r7` had held nothing or whether the wrong register had been named — the two answers point
/// at different code, and only the first is a fact about the pack.
#[test]
fn a_dump_names_the_register_on_the_line_that_cannot_read_it() {
    let rt = Runtime::new();
    // A register that holds zero is still named: `NULL` alone cannot be told apart from a place.
    assert_eq!(dump_header(&rt, 0, Some("r7")), "r7 = NULL");
    // A place keeps its own name and borrows no register's.
    assert_eq!(dump_header(&rt, 0, None), "NULL");
}

/// A stop the caller asked for ends the run, including when it happens inside `AEEMod_Load`.
///
/// Neither instrument disarms when it fires — a breakpoint stays awaited once its arrival count is
/// reached, and a `--watch-stop` byte trips on the next access — so the applet phase after a load-phase
/// stop stops again at its first touch. Every report printed once at the end then describes *that*
/// machine, which is what made a `--dump` of a register report the wrong instruction while the line
/// above still read "stopped". Dropping the request has to be said, and dropping a request nobody made
/// must not invent a line about it.
#[test]
fn a_stop_in_the_load_phase_ends_the_run_before_the_applet() {
    assert_eq!(
        start_after_load(Some(Start::Auto), true),
        (None, true),
        "asked to start, stopped on purpose: the phase goes away and says why"
    );
    assert_eq!(
        start_after_load(None, true),
        (None, false),
        "nobody asked to start, so there is nothing to apologise for"
    );
    assert_eq!(
        start_after_load(Some(Start::Cls(0x1234_5678)), false),
        (Some(Start::Cls(0x1234_5678)), false),
        "and the ordinary run keeps the request it was given"
    );
}

/// A load that ran out of instructions is not a module that refused to publish an `IModule*`.
///
/// The sample set has self-unpacking loaders whose `AEEMod_Load` needs 4–18 million instructions. A
/// headless run with a small `--session-ceiling` cuts them off mid-unpack; the old report then said
/// "`--start` needs a module that published an IModule*", which sent the reader to the module's
/// own code instead of to the budget. The two skip reasons must stay distinguishable.
#[test]
fn a_load_that_spent_its_budget_is_not_blamed_on_the_module() {
    assert_eq!(
        start_after_load(Some(Start::Auto), true),
        (None, true),
        "the applet phase goes away and says why"
    );
    let unfinished = start_skip_reason(true);
    let stopped = start_skip_reason(false);
    assert!(
        unfinished.contains("never returned"),
        "name the missing return"
    );
    assert!(
        unfinished.contains("--session-ceiling"),
        "name the budget a person can raise"
    );
    assert!(
        stopped.contains("stopped where it was asked to"),
        "keep the deliberate stop's line"
    );
    assert_ne!(unfinished, stopped, "the two cases must not read the same");
}

#[test]
fn a_dumped_register_reads_the_fault_copy_the_panel_prints() {
    let mut rt = Runtime::new();
    let block = rt.cpu.mem.alloc(0x40, 1).unwrap();
    let at_fault = block + 8;
    // What the host's unwinding leaves in the live bank: a return value, not the pointer asked for.
    rt.cpu.r[0] = 0;
    let mut regs = [0u32; 15];
    regs[0] = at_fault;
    let stop = quoin::runtime::Crash {
        regs,
        pc: 0x1000_0230,
        cpsr: 0,
        thumb: true,
        steps: 2_891_571,
    };
    rt.crash = Some(stop);
    assert_eq!(
        dump_address(&rt, "r0"),
        Some((at_fault, Some("r0"))),
        "the fault's machine, not the one that moved on the way out"
    );
    // A watch-stop is the other copy that has to win over the live file.
    rt.crash = None;
    rt.watched = Some(stop);
    assert_eq!(dump_address(&rt, "r0"), Some((at_fault, Some("r0"))));
    // And the source the header says out loud follows the same order, or the line would lie.
    rt.crash = Some(stop);
    assert!(
        stop_machine_source(&rt).contains("fault"),
        "{}",
        stop_machine_source(&rt)
    );
    rt.crash = None;
    rt.watched = None;
    assert_eq!(
        dump_address(&rt, "r0"),
        Some((0, Some("r0"))),
        "with no stop to prefer, the live registers are the answer"
    );
    assert!(
        stop_machine_source(&rt).contains("live"),
        "{}",
        stop_machine_source(&rt)
    );
}

/// The panel and `--dump` must never read different machines for the same name.
///
/// One assertion per register, with a value that is different for every one of them: an index that
/// drifts by one between the two paths shows up here rather than in a report that looks plausible.
#[test]
fn the_register_panel_and_a_dump_answer_from_one_machine() {
    let mut rt = Runtime::new();
    let mut regs = [0u32; 15];
    for (i, slot) in regs.iter_mut().enumerate() {
        *slot = 0x005a_1000 + 4 * i as u32;
    }
    // The live bank deliberately holds something else for every register.
    for i in 0..15 {
        rt.cpu.r[i] = 0;
    }
    rt.crash = Some(quoin::runtime::Crash {
        regs,
        pc: 0x1000,
        cpsr: 0,
        thumb: false,
        steps: 1,
    });
    for (i, name) in REG_NAMES.iter().enumerate() {
        assert_eq!(
            dump_address(&rt, name).map(|(at, _)| at),
            Some(stop_machine(&rt).regs[i]),
            "{name} read from a different machine than the panel prints"
        );
    }
}

/// A `--watch-stop` is a stopped machine, not a live one.
///
/// Every report now keys off this one predicate: the register panel and `--dump` read `stop_machine`,
/// and `--disasm` with no address lists `stopped_machine`'s program counter. The panel already preferred
/// the copies, but `--disasm` looked only at `crash`, so a run stopped at a watchpoint was answered
/// "nothing to list" directly beside its own register panel. What is asserted here is the predicate —
/// the two callers are a `match` on it, and nothing in this binary captures stdout to test their words.
#[test]
fn a_watch_stop_is_a_stopped_machine_rather_than_a_live_one() {
    let mut rt = Runtime::new();
    assert!(
        stopped_machine(&rt).is_none(),
        "a run that neither faulted nor stopped has no copy to read"
    );
    let block = rt.cpu.mem.alloc(0x40, 1).unwrap();
    let mut regs = [0u32; 15];
    regs[13] = block;
    let stop = quoin::runtime::Crash {
        regs,
        pc: 0x1000_0230,
        cpsr: 0,
        thumb: true,
        steps: 7,
    };
    rt.watched = Some(stop);
    assert_eq!(stopped_machine(&rt).map(|c| c.pc), Some(0x1000_0230));
    assert_eq!(
        stop_machine(&rt).regs[13],
        block,
        "the panel's source and the predicate read the same copy"
    );
    assert!(
        stop_machine_source(&rt).contains("watch-stop")
            && stop_machine_source(&rt).contains("--break"),
        "a deliberate stop is produced by both instruments, and the sentence has to name both: {}",
        stop_machine_source(&rt)
    );
    // A fault outranks a watch-stop, because the report a reader is looking at is the one about how the
    // run died.
    rt.crash = Some(quoin::runtime::Crash {
        pc: 0x2000_0100,
        ..stop
    });
    assert_eq!(
        stopped_machine(&rt).map(|c| (c.pc, c.thumb)),
        Some((0x2000_0100, true)),
        "the newer of the two copies is not the point; the fault is"
    );
    assert!(
        stop_machine_source(&rt).contains("fault"),
        "{}",
        stop_machine_source(&rt)
    );
}

/// Where `--disasm` lists, decided as a value so the precedence can be asserted where it is used.
///
/// This is the sibling of the `--dump` defect: the report read `rt.crash` only, so a run stopped at a
/// watchpoint — whose register panel had just been printed — was answered "nothing to list". Every case
/// the old `match` had is here, including the state rule that keeps an even Thumb halfword from being
/// decoded as ARM.
#[test]
fn a_disasm_listing_with_no_address_lists_the_watch_stop_too() {
    let mut rt = Runtime::new();
    let stop = quoin::runtime::Crash {
        regs: [0; 15],
        pc: 0x0010_0230,
        cpsr: 0,
        thumb: true,
        steps: 12,
    };

    // Nothing named, nothing stopped: said out loud.
    assert!(matches!(disasm_listing(&rt, None), Listing::Nothing));

    // A watch-stop is a subject for the listing, not only a fault.
    rt.watched = Some(stop);
    match disasm_listing(&rt, None) {
        Listing::At { at, thumb, mark } => {
            assert_eq!(
                (at, thumb),
                (0x0010_0230, true),
                "the state the stop recorded"
            );
            assert_eq!(
                mark,
                Some(0x0010_0230),
                "and the arrow sits on that instruction"
            );
        }
        other => panic!("a watch-stop must be listable, got {other:?}"),
    }

    // A named address wins over the stop, and its own parity chooses the state...
    rt.crash = Some(quoin::runtime::Crash {
        pc: 0x0020_0000,
        ..stop
    });
    let named = String::from("0x300001");
    match disasm_listing(&rt, Some(&named)) {
        Listing::At { at, thumb, mark } => {
            assert_eq!((at, thumb), (0x300000, true), "an odd address is Thumb");
            assert_eq!(mark, Some(0x200000), "the fault's pc still gets the arrow");
        }
        other => panic!("an address this host parses must list, got {other:?}"),
    }
    // ...except at the stop's own halfword, where the stop's record outranks parity.
    let here = String::from("0x200000");
    assert!(
        matches!(
            disasm_listing(&rt, Some(&here)),
            Listing::At { thumb: true, .. }
        ),
        "an even address that *is* the fault must still be listed as the state it stopped in"
    );
    // A name this host cannot resolve comes back with the caller's own text, which is what its line says.
    let junk = String::from("somewhere");
    assert!(
        matches!(disasm_listing(&rt, Some(&junk)), Listing::Unparseable(t) if t == "somewhere")
    );
}

/// Every line a register `--dump` prints names the machine it read — the refusal included.
///
/// The help promises that, and the path that needs it most is the one that used to print only
/// `--dump r0 = NULL: cannot read 64 byte(s)`: that line reads as "this register held nothing" when the
/// truth can be "these are the registers after the way out". A place named directly borrows no
/// register, so it borrows no machine either — asserted, not assumed.
#[test]
fn every_dump_line_that_names_a_register_also_names_the_machine() {
    let mut rt = Runtime::new();
    let mut regs = [0u32; 15];
    regs[4] = 0x005a_1080;
    rt.crash = Some(quoin::runtime::Crash {
        regs,
        pc: 0x1000,
        cpsr: 0,
        thumb: true,
        steps: 3,
    });
    let line = dump_machine_line(&rt, Some("r4")).expect("a register dump says whose register");
    assert!(line.contains("r4"), "{line}");
    assert!(line.contains("fault"), "{line}");
    assert_eq!(
        dump_machine_line(&rt, None),
        None,
        "a place is not somebody's register"
    );
    rt.crash = None;
    let live = dump_machine_line(&rt, Some("r4")).unwrap();
    assert!(
        live.contains("no machine copy"),
        "a run that left no copy must not be described as one that stopped: {live}"
    );
}

/// There is no developer tier any more: every option is accepted on every run. The gate used to
/// turn a switch into a password — a command that named `--trace` had already said what it wanted,
/// and refusing it only taught people to prefix `--dev` until it stopped being a decision.
///
/// What this pins is that no flag left the gate half-removed: each one parses far enough that the
/// run stops for want of a pack, rather than being refused by name.
#[test]
fn diagnostics_no_longer_need_a_switch() {
    for args in [
        vec!["--trace"],
        vec!["--heap-mb", "64"],
        vec!["--heap-initial-kb", "512"],
        vec!["--ram-mb", "128"],
        vec!["--platform", "classic-1.2.3"],
        vec!["--carrier", "kddi"],
        vec!["--color-depth", "16"],
        vec!["--watch", "0x20000000,4"],
    ] {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let error = run(&args).expect_err("a run with no pack cannot succeed");
        assert!(
            !error.contains("--dev") && !error.contains("unknown"),
            "{} should parse on its own, got: {error}",
            args[0]
        );
        assert!(
            error.contains("no .mod"),
            "{} parsed, so the run should stop for want of a pack; got: {error}",
            args[0]
        );
    }
}

/// `--dev` is still accepted, as a no-op, because every command and script written against the old
/// surface carries it. It must also stay invisible: the same arguments say the same thing with it
/// and without it, so nobody has to wonder whether the run behaved differently.
#[test]
fn the_old_dev_switch_is_a_no_op() {
    let without = run(&["--trace".to_string()]).expect_err("no pack");
    let with = run(&["--dev".to_string(), "--trace".to_string()]).expect_err("no pack");
    assert_eq!(
        with, without,
        "--dev must not change what the rest of the line means"
    );
    assert!(
        !with.contains("--dev"),
        "and must not be reported as a problem: {with}"
    );
}

/// `--start auto`'s verdict, which used to require the running applet to be the one the probe found.
/// A launcher breaks that: Refresher builds its applet, closes it, and starts Monopoly's, so the
/// applet that ends up running belongs to another module and a working run was reported as
/// "created no applet".
#[test]
fn a_start_counts_when_something_is_running_afterwards() {
    assert!(
        start_counts(0x2000_00b0, 0x2000_0150),
        "a handoff to another applet is a start"
    );
    assert!(
        start_counts(0x2000_00b0, 0x2000_00b0),
        "and so is staying put"
    );
    assert!(
        !start_counts(0x2000_00b0, 0),
        "built and then closed is a start that led nowhere"
    );
    assert!(!start_counts(0, 0), "nothing built is not a start");
}

