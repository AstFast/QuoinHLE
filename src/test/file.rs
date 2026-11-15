// Moved out of `src/file.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// A directory of its own, so two runs mounting beside a `.mod` cannot see each other's files.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("quoin-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_save_directory_keeps_what_the_app_writes() {
    // The caller names a directory, and every write is mirrored into it as it happens — which is
    // what a handset's flash does, and what a game needs to remember a save between runs. A pack
    // writes its game once and asks `Test`/`GetInfo` about it before deciding to start a new one,
    // and a volume that forgot the write sends it down the
    // new-game path every time it was started.
    let pack = scratch("save-pack");
    std::fs::write(pack.join("app.mod"), b"image").unwrap();
    let home = scratch("save-home");

    let mut v = Volume::default();
    v.mount_beside(&pack.join("app.mod"));
    assert!(v.persist_to(&home).is_empty(), "nothing there yet");

    let mut f = v
        .open("progress", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut f, b"level=3"), Ok(7));
    assert_eq!(
        std::fs::read(home.join("progress")).unwrap(),
        b"level=3",
        "the write reached the host's disk, not only the volume"
    );

    // A nested name gets its directories, because a pack that files its data under `res/` writes
    // there too.
    let mut n = v
        .open("res/settings", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut n, b"sound=1"), Ok(7));
    assert_eq!(
        std::fs::read(home.join("res/settings")).unwrap(),
        b"sound=1"
    );

    // And the next run finds them, as *written* entries: a pack that also ships a file of that
    // name gets its own save back rather than the shipped template.
    let mut next = Volume::default();
    next.mount_beside(&pack.join("app.mod"));
    let loaded = next.persist_to(&home);
    assert_eq!(
        loaded.join(","),
        "progress,res,res/settings",
        "names, directories included, exactly as `mount_beside` reports them"
    );
    assert_eq!(next.test("progress"), g::AEE_SUCCESS);
    let info = next.info("progress").unwrap();
    assert_eq!((info.size, info.attrib), (7, FA_NORMAL));
    let r = next.open("progress", g::_OFM_READ).unwrap();
    assert_eq!(next.read(&r, 64), b"level=3");
}

#[test]
fn a_save_directory_is_where_removals_land_too() {
    let pack = scratch("save-rm-pack");
    std::fs::write(pack.join("app.mod"), b"image").unwrap();
    let home = scratch("save-rm-home");
    let mut v = Volume::default();
    v.mount_beside(&pack.join("app.mod"));
    v.persist_to(&home);
    let mut f = v
        .open("progress", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    v.write(&mut f, b"x").unwrap();
    assert!(home.join("progress").is_file());
    assert_eq!(v.remove("progress"), g::AEE_SUCCESS);
    assert!(
        !home.join("progress").exists(),
        "a run that deletes its save has deleted it, not hidden it"
    );
}

#[test]
fn without_a_save_directory_nothing_reaches_the_host() {
    // The default, and the reason this volume is memory at all: a run must not edit the tree it
    // was pointed at.
    let pack = scratch("no-home-pack");
    std::fs::write(pack.join("app.mod"), b"image").unwrap();
    let mut v = Volume::default();
    v.mount_beside(&pack.join("app.mod"));
    let mut f = v
        .open("progress", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    v.write(&mut f, b"x").unwrap();
    assert_eq!(
        std::fs::read_dir(&pack).unwrap().count(),
        1,
        "only the pack's own .mod is in its directory"
    );
}

#[test]
fn a_pack_beside_the_module_arrives_as_rom_until_the_app_writes_it() {
    let dir = scratch("mount");
    std::fs::write(dir.join("app.mod"), b"image").unwrap();
    std::fs::write(dir.join("res.bar"), b"container-bytes").unwrap();
    let mut v = Volume::default();
    let mounted = v.mount_beside(&dir.join("app.mod"));
    assert_eq!(
        mounted.join(","),
        "app.mod,res.bar",
        "sorted, and the .mod itself is in its own directory"
    );
    assert_eq!(v.test("res.bar"), g::AEE_SUCCESS);
    assert_eq!(
        v.test("RES.BAR"),
        g::AEE_EFAILED,
        "the volume does not invent a case rule a handset may not have"
    );
    let info = v.info("res.bar").unwrap();
    assert_eq!((info.size, info.attrib), (15, FA_READONLY | FA_CONST));
    // Reading it is what a pack is for, and from wherever the app's pointer is.
    let mut f = v.open("res.bar", g::_OFM_READ).unwrap();
    assert_eq!(v.read(&f, 64), b"container-bytes");
    f.pos = 10;
    assert_eq!(v.read(&f, 5), b"bytes", "and from wherever the pointer is");
    // Writing it takes a copy of itself. The bytes the run edits are its own and live in memory,
    // so the pack on disk still belongs to the next run; refusing the open instead would deny
    // every pack that stores its game in the directory it shipped in.
    let mut w = v.open("res.bar", g::_OFM_READWRITE).unwrap();
    assert_eq!(v.write(&mut w, b"x"), Ok(1));
    let r = v.open("res.bar", g::_OFM_READ).unwrap();
    assert_eq!(v.read(&r, 64), b"xontainer-bytes");
    assert_eq!(
        v.info("res.bar").unwrap().attrib,
        FA_NORMAL,
        "it is the app's file now, not a ROM one"
    );
    assert_eq!(
        std::fs::read(dir.join("res.bar")).unwrap(),
        b"container-bytes",
        "and the host's copy is exactly what it was"
    );
    // A shipped file the run never took over is still the pack's, and removing that is the pack
    // disappearing rather than being edited. The one it has written is the app's own, and goes.
    assert_eq!(v.remove("app.mod"), g::AEE_EFAILED);
    assert_eq!(
        v.last_error(),
        g::AEE_EREADONLY,
        "and the reason is not lost"
    );
    assert_eq!(v.remove("res.bar"), g::AEE_SUCCESS);
    assert!(v.info("res.bar").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pack_that_ships_its_art_in_a_subdirectory_mounts_it_too() {
    // What a pack may do before anything else: open `res\ma.pak` and walk the index inside it. A
    // volume that only knew the top level answered NULL, and the pack then spent its
    // run appending zero-length records to itself — a hang with no fault in it to report.
    let dir = scratch("nested");
    std::fs::write(dir.join("app.mod"), b"image").unwrap();
    std::fs::create_dir_all(dir.join("res/ui")).unwrap();
    std::fs::write(dir.join("res/ma.pak"), b"index").unwrap();
    std::fs::write(dir.join("res/ui/title.png"), b"png").unwrap();
    let mut v = Volume::default();
    v.mount_beside(&dir.join("app.mod"));

    // Spelled the way the pack spells it, with the separator its own source uses...
    let f = v
        .open("res\\ma.pak", g::_OFM_READ)
        .expect("the pak is mounted");
    assert_eq!(v.read(&f, 64), b"index");
    //...and spelled as a path the volume strips for itself.
    let f = v
        .open("fs:/~/res/ui/title.png", g::_OFM_READ)
        .expect("two levels down is still the app's own");
    assert_eq!(v.read(&f, 64), b"png");

    // The directory is an entry, which is what lets an app find its own subdirectory at all.
    assert_eq!(v.test("res"), g::AEE_SUCCESS);
    assert_eq!(v.enum_init("", true), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "res");
    assert!(v.enum_next().is_none(), "and only the root's own");
    // The root and the home directory are one app root here; a file manager enumerating either
    // gets the same bare leaves.
    for root in ["fs:/", "fs:/~/", "fs:/~"] {
        assert_eq!(v.enum_init(root, true), g::AEE_SUCCESS, "{root}");
        assert_eq!(v.enum_next().unwrap().name, "res", "{root}");
        assert!(v.enum_next().is_none(), "{root}");
    }
    // A listing of the subdirectory hands back **the path**, not the leaf: `CFileMgr_EnumInit` stores the
    // caller's own directory text (`aee_err_strdup`, leading `/` and `\` stripped) and `EnumNext` reports
    // `aee_makepath(that, basename(entry))`, so an app that listed `res` can open what it was just told.
    // Enumerating the root is the one case where the stored text is empty and the answer is a bare leaf.
    assert_eq!(v.enum_init("res", false), g::AEE_SUCCESS);
    let files: Vec<String> = (0..10)
        .filter_map(|_| v.enum_next())
        .map(|i| i.name)
        .collect();
    assert_eq!(files, ["res/ma.pak"]);
    assert_eq!(v.enum_init("res", true), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "res/ui");
    // … and a spelling that already ends in a separator does not get it doubled (`aee_makepath` adds a
    // `/` only when the stored directory neither is empty nor ends with one).
    assert_eq!(v.enum_init("res/", true), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "res/ui");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn enumeration_reports_the_text_it_was_given_and_a_failed_start_drops_the_walk() {
    let mut v = Volume::default();
    let mut f = v
        .open("logs/a.dat", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut f, b"xyz").unwrap(), 3);
    for (asked, want) in [
        ("logs", "logs/a.dat"),
        ("/logs", "logs/a.dat"),
        ("\\logs", "logs/a.dat"),
        ("logs/", "logs/a.dat"),
        // ⚠ Only the **leading** separators are dropped. `CFileMgr_EnumInit` stores the caller's text
        // after a loop that skips a leading `/` or `\` and nothing else, so a `./` the app typed comes
        // back in every name the walk reports. That is the shipped answer, odd as it reads.
        ("./logs", "./logs/a.dat"),
    ] {
        assert_eq!(
            v.enum_init(asked, false),
            g::AEE_SUCCESS,
            "{asked} is one directory"
        );
        assert_eq!(v.enum_next().unwrap().name, want, "reported as {asked:?}");
        assert!(v.enum_next().is_none(), "one entry either way");
    }
    // Half-walked, then a refused start: the listing has to be gone, not resumable. ⚠ A **second** file is
    // added for this and only for this: with one entry the walk answers `None` after consuming it whether
    // or not the teardown ran, so the assertion below would be reading the initial state rather than the
    // `CFileMgr_FinishEnum` it claims to test (a reset-class assertion).
    let mut second = v
        .open("logs/b.dat", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut second, b"q").unwrap(), 1);
    assert_eq!(v.enum_init("logs", false), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "logs/a.dat");
    assert_ne!(
        v.enum_init("fs:/card0/logs", false),
        g::AEE_SUCCESS,
        "another volume is not this one"
    );
    assert_eq!(v.last_error(), g::AEE_ESCHEMENOTSUPPORTED);
    assert!(
        v.enum_next().is_none(),
        "`logs/b.dat` was still there and `CFileMgr_FinishEnum` is what hid it"
    );
}

#[test]
fn a_starred_basename_filters_by_extension_rather_than_matching_like_a_glob() {
    let mut v = Volume::default();
    for name in [
        "ringers/song.mid",
        "ringers/shout.MID",
        "ringers/song.mp3",
        "ringers/notes",
        "ringers/hymn.mid.gz",
        "ringers/dual.b.mid",
        "ringers/songx.mid",
    ] {
        let mut f = v
            .open(name, g::_OFM_READWRITE | g::_OFM_CREATE)
            .expect("a ringer directory is the app's own");
        assert_eq!(v.write(&mut f, b"x").unwrap(), 1, "{name} is there");
    }
    let listed = |v: &mut Volume| -> Vec<String> {
        (0..12)
            .filter_map(|_| v.enum_next())
            .map(|i| i.name)
            .collect()
    };
    assert_eq!(v.enum_init("ringers/*.mid", false), g::AEE_SUCCESS);
    assert_eq!(
        listed(&mut v),
        [
            "ringers/dual.b.mid",
            "ringers/song.mid",
            "ringers/songx.mid"
        ],
        "the last dot decides, the case has to match, and nothing else about the name matters"
    );
    // The pattern filters leaves and nothing else: the same directory named without it reports all
    // seven, each with the text the caller passed in front of the leaf.
    assert_eq!(v.enum_init("ringers", false), g::AEE_SUCCESS);
    assert_eq!(listed(&mut v).len(), 7, "no pattern, no filtering");
    // ⚠ A `*` that is not at the very start of the basename is **not** a pattern — the body's test is
    // `aee_strbegins("*.", basename)` and nothing else — so the name goes to the file system as a name
    // of its own, and a directory has no entry spelled `song*.mid`.
    assert_eq!(v.enum_init("ringers/song*.mid", false), g::AEE_SUCCESS);
    assert_eq!(
        listed(&mut v),
        Vec::<String>::new(),
        "one literal name, which nothing in the directory is called"
    );
    // … and a bare `*` is not `*.` either, so it does not become "everything" as a glob would.
    assert_eq!(v.enum_init("ringers/*", false), g::AEE_SUCCESS);
    assert_eq!(
        listed(&mut v),
        Vec::<String>::new(),
        "no pattern, and no such entry"
    );
}

#[test]
fn create_alone_is_a_write_mode_because_that_is_how_a_pack_saves() {
    // Dragon's save path, and the reason `_OFM_CREATE` cannot be read as a bit that merely
    // permits creation: it opens `s.dat` for read, closes it, and reopens it with mode 4 and no
    // access bit at all. A host that finds no access in the mode answers `NULL`, and the pack's
    // very next Thumb instruction is `ldr r0, [r0]` on that answer — a data abort at zero, three
    // instructions after the call.
    let dir = scratch("create");
    std::fs::write(dir.join("app.mod"), b"image").unwrap();
    std::fs::write(dir.join("s.dat"), b"old save").unwrap();
    let mut v = Volume::default();
    v.mount_beside(&dir.join("app.mod"));
    let mut w = v.open("s.dat", g::_OFM_CREATE).unwrap();
    assert_eq!(v.write(&mut w, b"new"), Ok(3));
    let r = v.open("s.dat", g::_OFM_READ).unwrap();
    assert_eq!(
        v.read(&r, 64),
        b"new save",
        "the app's bytes over the shipped ones, which is a write at position zero"
    );
    // And the same mode on a name that does not exist yet is the creation the header names.
    let mut n = v.open("t.dat", g::_OFM_CREATE).unwrap();
    assert_eq!(v.write(&mut n, b"x"), Ok(1));
    assert_eq!(v.info("t.dat").unwrap().size, 1);
    // `_OFM_READWRITE` still does not create, which is the header's own comment and the one
    // difference between the two modes a save path can use.
    assert_eq!(
        v.open("u.dat", g::_OFM_READWRITE).err(),
        Some(AEEFS_EFILENOEXISTS)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_apps_own_file_grows_where_it_is_told_and_reads_back_as_zeros() {
    let mut v = Volume::default();
    let mut f = v
        .open("save.bin", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 6), g::AEE_SUCCESS);
    assert_eq!(v.write(&mut f, b"hi").unwrap(), 2);
    assert_eq!(
        v.info("save.bin").unwrap().size,
        8,
        "a write past the end extends the file, and the gap is not skipped"
    );
    let mut r = v.open("save.bin", g::_OFM_READ).unwrap();
    r.pos = 4;
    assert_eq!(
        v.read(&r, 4),
        [0, 0, b'h', b'i'],
        "the hole reads back as zeros, as it does on a card"
    );
    assert_eq!(v.seek(&mut r, g::_SEEK_START, -1), g::AEE_EFAILED);
    // `CFile_Truncate` answers a `bool` with TRUE meaning failure, and keeps the reason in the
    // record's last-error cell instead of returning it.
    assert!(
        v.truncate(&mut r, 1),
        "this handle was opened for reading, so the answer is TRUE = it did not happen"
    );
    assert_eq!(v.last_error(), g::AEE_EREADONLY, "the reason lives here");

    let mut w = v.open("save.bin", g::_OFM_READWRITE).unwrap();
    assert!(!v.truncate(&mut w, 5), "FALSE = the system call succeeded");
    assert_eq!(
        v.last_error(),
        g::AEE_SUCCESS,
        "and a success writes its own 0 into that cell, which is what \
         `*(DWORD*)(err_block + 12) = v8` does on every path"
    );
    assert_eq!(v.info("save.bin").unwrap().size, 5);
    let appended = v
        .open("save.bin", g::_OFM_APPEND | g::_OFM_READWRITE)
        .unwrap();
    assert_eq!(appended.pos, 5, "append means the write lands at the end");
    assert_eq!(
        v.open("gone.bin", g::_OFM_READ).err(),
        Some(AEEFS_EFILENOEXISTS),
        "reading without `CREATE` on a file that is not there is not a silent empty one"
    );
}

#[test]
fn a_const_view_is_answered_zero_with_the_file_systems_own_reason() {
    let mut v = Volume::default();
    v.entries
        .insert("theme.mid".into(), Entry::Shipped(vec![1, 2, 3, 4]));
    let mut f = v.open("theme.mid", g::_OFM_READ).unwrap();
    assert!(
        !f.writable,
        "a read-only handle is the only way a shipped name stays a const view here"
    );

    // The const branch: the answer is zero bytes, the reason is `AEEFS_EINVALIDOPERATION` (266), and
    // **no `AEE_E*` is written anywhere** — which is only visible because `GetLastError` shares the cell.
    assert_eq!(
        v.write(&mut f, b"xx").unwrap(),
        0,
        "`return 0;` is the body's whole answer"
    );
    assert_eq!(
        v.last_error(),
        AEEFS_EINVALIDOPERATION,
        "266 = AEEFS_ERROR + 0x0A, not a borrowed `AEE_EREADONLY`"
    );
    assert_eq!(
        v.entries.get("theme.mid").map(|e| e.len()),
        Some(4),
        "and nothing moved"
    );
    // `CFile_Truncate`'s first branch is the same view and the same number, and its answer is the
    // boolean TRUE ("it did not happen").
    assert!(
        v.truncate(&mut f, 2),
        "the const view shortens nothing, so the `bool` says failure"
    );
    assert_eq!(v.last_error(), AEEFS_EINVALIDOPERATION);
    assert_eq!(v.entries.get("theme.mid").map(|e| e.len()), Some(4));

    // A zero-length write is the *next* gate, and it is a flush: the reason from the branch above is
    // still what `GetLastError` says, because the body never reaches its own clearing.
    assert_eq!(v.write(&mut f, b"").unwrap(), 0);
    assert_eq!(
        v.last_error(),
        AEEFS_EINVALIDOPERATION,
        "the flush path does not touch the cell"
    );

    // And an accepted write clears the cell first, so a success answers 0 through `GetLastError`
    // rather than somebody else's old failure.
    let mut w = v
        .open("save.bin", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut w, b"abcd").unwrap(), 4);
    assert_eq!(v.last_error(), 0, "a normal write leaves the cell at 0");
    let mut ro = v.open("save.bin", g::_OFM_READ).unwrap();
    assert_eq!(
        v.write(&mut ro, b"z").err(),
        Some(g::AEE_EFAILED),
        "a read-only handle on the app's **own** file is not the const branch"
    );
    assert_eq!(v.last_error(), g::AEE_EREADONLY);
    assert_eq!(
        v.write(&mut ro, b"").unwrap(),
        0,
        "while the empty request is still the flush, even on a handle that cannot write"
    );
    assert_eq!(
        v.last_error(),
        g::AEE_EREADONLY,
        "and the flush leaves that reason standing"
    );
}

#[test]
fn seek_answers_a_position_to_one_question_and_zero_or_one_to_everything_else() {
    let mut v = Volume::default();
    let mut f = v
        .open("log.dat", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut f, b"0123456789").unwrap(), 10);

    assert_eq!(v.seek(&mut f, g::_SEEK_START, 4), g::AEE_SUCCESS);
    assert_eq!(f.pos, 4);
    // The one call that answers a number: and it does not move anything.
    assert_eq!(
        v.seek(&mut f, g::_SEEK_CURRENT, 0),
        4,
        "`_SEEK_CURRENT` with distance 0 is the body's `return FS_GETPOSITION`"
    );
    assert_eq!(f.pos, 4, "and the position is untouched by asking");
    // The other two bases, both answered as the body answers them.
    assert_eq!(v.seek(&mut f, g::_SEEK_CURRENT, -2), g::AEE_SUCCESS);
    assert_eq!(f.pos, 2);
    assert_eq!(v.seek(&mut f, g::_SEEK_END, 3), g::AEE_SUCCESS);
    assert_eq!(f.pos, 13, "a written file may be positioned past its end");

    // A target before the start: the answer is **1**, not -1, and the cell carries the FS's own code.
    assert_eq!(
        v.seek(&mut f, g::_SEEK_START, -5),
        g::AEE_EFAILED,
        "`return 1` — the body never hands the caller a -1"
    );
    assert_eq!(
        v.last_error(),
        AEEFS_EBADSEEKPOS,
        "260 is the code this member uses for a position outside the file"
    );
    assert_eq!(f.pos, 13, "and the refused seek left the position alone");
    // An unknown seek type is refused the same way, with the number the const branch names for it.
    assert_eq!(v.seek(&mut f, 9, 0), g::AEE_EFAILED);
    assert_eq!(v.last_error(), g::AEE_EBADPARM);
    assert_eq!(f.pos, 13);
}

#[test]
fn a_const_file_is_seeked_on_the_cursor_its_reads_use() {
    let mut v = Volume::default();
    v.entries
        .insert("theme.mid".into(), Entry::Shipped(vec![1, 2, 3, 4, 5, 6]));
    let mut f = v.open("theme.mid", g::_OFM_READ).unwrap();

    assert_eq!(v.seek(&mut f, g::_SEEK_START, 3), g::AEE_SUCCESS);
    assert_eq!(f.pos, 3, "the seek moved the cursor this handle reads from");
    assert_eq!(
        v.read(&f, 6),
        vec![4, 5, 6],
        "and the read starts there, bounded by the end as `v5[2] - v8` bounds it"
    );
    assert_eq!(
        v.seek(&mut f, g::_SEEK_CURRENT, 0),
        3,
        "the position query answers that same cursor"
    );

    assert_eq!(v.seek(&mut f, g::_SEEK_END, -1), g::AEE_SUCCESS);
    assert_eq!(f.pos, 5, "one short of the end");
    // The end itself is a legal address (`v7 <= v9`), and reading from it is the honest zero.
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 6), g::AEE_SUCCESS);
    assert_eq!(
        v.read(&f, 4),
        Vec::<u8>::new(),
        "nothing lies past a const file's end"
    );

    // One address further is refused: the bound is the size, not "no negative". A written handle takes
    // the same position, because there the file system would pay for the gap.
    assert_eq!(v.seek(&mut f, g::_SEEK_END, 1), g::AEE_EFAILED);
    assert_eq!(v.last_error(), AEEFS_EBADSEEKPOS);
    assert_eq!(f.pos, 6, "and the refused seek changed nothing");
    assert_eq!(v.seek(&mut f, g::_SEEK_CURRENT, -100), g::AEE_EFAILED);
    assert_eq!(v.last_error(), AEEFS_EBADSEEKPOS);
    assert_eq!(v.seek(&mut f, 7, 1), g::AEE_EFAILED);
    assert_eq!(
        v.last_error(),
        g::AEE_EBADPARM,
        "an unknown type is `14` here"
    );
    assert_eq!(f.pos, 6);

    // ⚠ And the `14` the refusal above left standing **survives a good seek**: this branch has no file
    // system call to flatten the cell. The refusal seeds it on purpose — a fresh volume starts at 0, and
    // then "it was cleared" and "it was never set" read the same.
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 2), g::AEE_SUCCESS);
    assert_eq!(f.pos, 2);
    assert_eq!(
        v.last_error(),
        g::AEE_EBADPARM,
        "a const seek that succeeded names no reason of its own"
    );
}

#[test]
fn a_nearly_full_volume_refuses_a_growing_seek_and_shortens_a_growing_write() {
    let mut v = Volume::default();
    v.entries
        .insert("big.dat".into(), Entry::Written(vec![0; VOLUME_LIMIT - 20]));
    let mut f = v
        .open("note.dat", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut f, b"123456").unwrap(), 6, "room is 14 now");
    assert_eq!(v.last_error(), 0, "an accepted write leaves the cell at 0");

    // Inside the room, and past the end: 14 bytes of growth are affordable.
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 9), g::AEE_SUCCESS);
    assert_eq!(f.pos, 9);
    assert_eq!(
        v.seek(&mut f, g::_SEEK_START, 40),
        g::AEE_EFAILED,
        "34 past the end does not fit, so the seek is refused — not clamped"
    );
    assert_eq!(v.last_error(), AEEFS_EFSFULL, "262 = AEEFS_ERROR + 6");
    assert_eq!(f.pos, 9, "and the refused seek left the position alone");
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 4), g::AEE_SUCCESS);
    assert_eq!(v.last_error(), 0, "a backward seek costs nothing");

    // The write from position 9 overwrites the 6 bytes it sits on and needs the rest: 14 + 6 - 9 = 11
    // of a 20-byte request can be honoured, and the answer says so.
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 9), g::AEE_SUCCESS);
    assert_eq!(
        v.write(&mut f, b"abcdefghijklmnopqrst").unwrap(),
        11,
        "`v5 = v20 + v12`: a short count, never a refusal"
    );
    assert_eq!(v.last_error(), AEEFS_EFSFULL);
    assert_eq!(
        v.info("note.dat").map(|i| i.size),
        Some(20),
        "the file is the 9 it had plus the 11 that fit"
    );

    // Now nothing is left: the answer is 0 bytes written, still not an error.
    assert_eq!(f.pos, 20, "the write left the position after what it wrote");
    assert_eq!(v.write(&mut f, b"xyz").unwrap(), 0, "no room, no bytes");
    assert_eq!(v.last_error(), AEEFS_EFSFULL);
    assert_eq!(v.info("note.dat").map(|i| i.size), Some(20));
    // And an overwrite of existing bytes is still free at a full volume.
    assert_eq!(v.seek(&mut f, g::_SEEK_START, 2), g::AEE_SUCCESS);
    assert_eq!(
        v.write(&mut f, b"ab").unwrap(),
        2,
        "two bytes laid over two bytes ask the volume for nothing"
    );
    assert_eq!(v.last_error(), 0, "and leave no refusal behind");
}

/// The three facts a `GetInfo` reports come out of the entry itself, which is how a handle over the
/// pack's shipped bytes answers what `FillConstInfo` puts in the record: `FA_READONLY|FA_CONST`
/// (`0x14`) and that view's own length (`*(a1[32] + 8)`), with no date. A file the app wrote is
/// `FA_NORMAL`. ⚠ The `AEE_FA_FIXED` bit the body ORs in for a view that cannot be overridden has no
/// answer here: [`Volume::open`] turns a shipped name into a written copy the moment a handle asks for
/// write access, which is exactly the overridability that bit says a file does **not** have.
#[test]
fn a_shipped_file_reports_the_attributes_its_own_view_carries() {
    let mut v = Volume::default();
    v.entries
        .insert("theme.mid".into(), Entry::Shipped(vec![1, 2, 3, 4, 5]));
    v.entries
        .insert("note.dat".into(), Entry::Written(vec![0; 9]));
    let shipped = v.open("theme.mid", g::_OFM_READ).unwrap();
    let written = v.open("note.dat", g::_OFM_READ).unwrap();
    assert_eq!(
        v.handle_info(&shipped.name)
            .map(|i| (i.attrib, i.size, i.name)),
        Some((FA_READONLY | FA_CONST, 5, "theme.mid".into())),
        "0x14 is `FillConstInfo`'s first answer, whatever the name is asked with"
    );
    assert_eq!(
        v.handle_info(&written.name)
            .map(|i| (i.attrib, i.size, i.name)),
        Some((FA_NORMAL, 9, "note.dat".into()))
    );

    // A name that has left the volume answers through the *other* channel: no facts at all, and no
    // reason put in the cell, because `CFile_ReOpen`'s failure branch writes nothing. `Volume::info`,
    // which the slots that ask by name use, does name a reason — that difference is why both exist.
    v.refuse(g::AEE_ENOSUCH);
    assert!(
        v.handle_info("gone.dat").is_none(),
        "a handle over a name that is gone"
    );
    assert_eq!(
        v.last_error(),
        g::AEE_ENOSUCH,
        "and this call is not the one that gets to say why"
    );
    assert!(v.info("gone.dat").is_none());
    assert_eq!(
        v.last_error(),
        AEEFS_EFILENOEXISTS,
        "the lookup by name names the file system's own code too"
    );
}

#[test]
fn enumeration_lists_one_level_and_a_directory_has_to_be_empty_first() {
    let mut v = Volume::default();
    for name in ["b.dat", "a.dat", "sub/c.dat"] {
        let mut f = v.open(name, g::_OFM_READWRITE | g::_OFM_CREATE).unwrap();
        assert_eq!(v.write(&mut f, b"x").unwrap(), 1);
    }
    assert_eq!(v.mkdir("logs"), g::AEE_SUCCESS);
    // AEEFile.h: SUCCESS "if the file is successfully created or if file already exists".
    assert_eq!(
        v.mkdir("logs"),
        g::AEE_SUCCESS,
        "already there is not an error"
    );
    assert_eq!(v.enum_init("", false), g::AEE_SUCCESS);
    let files: Vec<String> = (0..10)
        .filter_map(|_| v.enum_next())
        .map(|i| i.name)
        .collect();
    assert_eq!(
        files,
        ["a.dat", "b.dat"],
        "files one level down, sorted, with no directory and no `sub/c.dat` in the list"
    );
    assert_eq!(v.enum_init("", true), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "logs");
    assert!(
        v.enum_next().is_none(),
        "one directory was made, and a file's prefix is not"
    );

    assert_eq!(v.rmdir("sub"), g::AEE_EFAILED);
    assert_eq!(
        v.last_error(),
        AEEFS_EDIRNOEXISTS,
        "`sub` was made by a file's name rather than by `MkDir`, so there is no directory there"
    );
    assert_eq!(v.remove("sub/c.dat"), g::AEE_SUCCESS);
    assert_eq!(v.rmdir("logs"), g::AEE_SUCCESS);
    assert_eq!(v.mkdir(""), g::AEE_SUCCESS, "the root already is");
}

/// The volume's room is a real bound, and the two slots that hit it answer differently, as their
/// bodies do: `CFile_Seek` is **refused** (`262` in the cell, position unmoved) and `CFile_Write` is
/// **shortened** to zero or to what fits — a short count, never an error. ⚠ This test used to assert
/// the opposite for the write (`Err(AEE_EFAILED)` with `AEE_ENOMEMORY`), which is a code neither body
/// returns from these two slots.
#[test]
fn the_volume_room_refuses_a_seek_that_needs_more_and_shortens_a_write_that_does() {
    let mut v = Volume::default();
    let mut f = v
        .open("big.bin", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    assert_eq!(v.write(&mut f, b"x").unwrap(), 1, "a normal write works");
    let mut far = v
        .open("far.bin", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    // One byte is taken, so the last address that can be paid for is `VOLUME_LIMIT - 1`.
    assert_eq!(
        v.seek(&mut far, g::_SEEK_START, (VOLUME_LIMIT - 1) as i32),
        g::AEE_SUCCESS,
        "the boundary itself is affordable: growth of exactly the room is not more than the room"
    );
    assert_eq!(
        v.seek(&mut far, g::_SEEK_START, VOLUME_LIMIT as i32),
        g::AEE_EFAILED,
        "one address past it is refused, not clamped — this slot answers 0/1"
    );
    assert_eq!(v.last_error(), AEEFS_EFSFULL);
    assert_eq!(
        far.pos,
        (VOLUME_LIMIT - 1) as u32,
        "and the position stayed"
    );

    // Writing there needs one byte for itself and there are none left, so the answer is 0 bytes.
    assert_eq!(
        v.write(&mut far, b"tail").unwrap(),
        0,
        "a write that cannot be honoured at all is a short count of zero"
    );
    assert_eq!(v.last_error(), AEEFS_EFSFULL);
    assert_eq!(
        v.info("far.bin").map(|i| i.size),
        Some(0),
        "and no file was stretched to satisfy a request that did not fit"
    );
}

#[test]
fn a_directory_is_not_opened_as_a_file_and_a_rename_cannot_overwrite_the_pack() {
    let mut v = Volume::default();
    assert_eq!(v.mkdir("logs"), g::AEE_SUCCESS);
    assert_eq!(
        v.open("logs", g::_OFM_READ).err(),
        Some(g::AEE_EBADPARM),
        "an empty file and a directory are different answers, and reading a directory as a file \
             would report it as the first"
    );
    // The pack's own file. `Remove` refuses to take it away, and a rename onto it is the same
    // deletion by another route: the moved entry would be the only copy of the pack's bytes left.
    let dir = scratch("rename");
    std::fs::write(dir.join("app.mod"), b"x").unwrap();
    std::fs::write(dir.join("res.bar"), b"pack bytes").unwrap();
    v.mount_beside(&dir.join("app.mod"));
    let mut mine = v
        .open("mine.dat", g::_OFM_READWRITE | g::_OFM_CREATE)
        .unwrap();
    v.write(&mut mine, b"mine").unwrap();
    assert_eq!(v.rename("mine.dat", "res.bar"), g::AEE_EFAILED);
    assert_eq!(v.last_error(), g::AEE_EREADONLY);
    assert_eq!(v.info("res.bar").unwrap().size, 10, "the pack is untouched");
    assert_eq!(v.rename("mine.dat", "yours.dat"), g::AEE_SUCCESS);
    assert!(v.info("yours.dat").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_name_removed_midway_through_a_listing_does_not_end_it() {
    let mut v = Volume::default();
    for name in ["a", "b", "c"] {
        let mut f = v.open(name, g::_OFM_READWRITE | g::_OFM_CREATE).unwrap();
        v.write(&mut f, b"x").unwrap();
    }
    assert_eq!(v.enum_init("", false), g::AEE_SUCCESS);
    assert_eq!(v.enum_next().unwrap().name, "a");
    assert_eq!(v.remove("b"), g::AEE_SUCCESS);
    assert_eq!(
        v.enum_next().map(|i| i.name),
        Some("c".into()),
        "the walk goes past the hole instead of stopping at it"
    );
    assert!(v.enum_next().is_none());
}

#[test]
fn a_byte_in_the_middle_of_a_character_is_not_a_prefix_boundary() {
    // Names reach the volume as bytes the module chose, converted lossily. Slicing at a prefix's
    // byte length panics when that offset splits a character, so the classifier has to answer for
    // those names as well as the plain ones.
    let accented = "\u{e9}nstrument.bin";
    assert_eq!(
        normalize(accented),
        Resolved::Here(accented.into()),
        "a leading two-byte character is not a prefix"
    );
    assert_eq!(
        normalize("\u{e9}"),
        Resolved::Here("\u{e9}".into()),
        "and neither is a name that is only that character"
    );
    assert_eq!(
        normalize("~/\u{e9}"),
        Resolved::Here("\u{e9}".into()),
        "a real prefix followed by one still strips"
    );
}

#[test]
fn every_spelling_of_the_apps_own_directory_arrives_at_the_same_name() {
    for form in ["a.txt", "./a.txt", "fs:/~/a.txt", "~/a.txt", "fs:/~//a.txt"] {
        assert_eq!(
            normalize(form),
            Resolved::Here("a.txt".into()),
            "the app means one file however the handset spelled it: {form}"
        );
    }
    assert_eq!(normalize(""), Resolved::Here(String::new()), "the root");
    assert_eq!(normalize("~"), Resolved::Here(String::new()));
    assert_eq!(
        normalize(r"sub\a.txt"),
        Resolved::Here("sub/a.txt".into()),
        "a pack compiled for a Windows simulator uses the other separator"
    );
    assert_eq!(normalize("fs:/card0/x"), Resolved::OtherVolume);
    assert_eq!(
        normalize("fs:/x"),
        Resolved::Here("x".into()),
        "`AEEFS_ROOT_DIR` and `AEEFS_HOME_DIR` share the app root"
    );
    assert_eq!(normalize("fs:/"), Resolved::Here(String::new()));
    assert_eq!(normalize("fs:/~"), Resolved::Here(String::new()));
    assert_eq!(normalize("fs:/~/"), Resolved::Here(String::new()));
    assert_eq!(normalize("fs:/card0"), Resolved::OtherVolume);
    assert_eq!(normalize("xfs:/x"), Resolved::OtherVolume);
    assert_eq!(normalize("../x"), Resolved::Rejected);
    assert_eq!(normalize("fs:/~/a/../../b"), Resolved::Rejected);
    // Windows strips trailing spaces and dots from each component before resolving a path, so these
    // four all name `..` on the host even though none of them equals it as a string. Rejecting only
    // the literal would let a pack write outside its own directory on the platform this host runs on.
    for spelling in [".. ", "..  ", ".. .", "sub/.. /x", r"sub\.. \x"] {
        assert_eq!(
            normalize(spelling),
            Resolved::Rejected,
            "{spelling:?} names the parent once the host trims it"
        );
    }
    assert_eq!(
        normalize("a/../b"),
        Resolved::Rejected,
        "mid-path, not just leading"
    );
}

#[test]
fn a_module_directory_prefix_names_the_same_file_as_the_home_prefix() {
    // the header: `fs:/~/` is a shortcut for `fs:/mod/<modid>`, and `fs:/mod/` is where
    // module directories live. With one app volume both spellings reach a file in that volume. The
    // host cannot tell one module id from another — that is the approximation stated in `normalize`;
    // what matters here is that the `mod/<id>` pair is not looked for *inside* the app directory.
    assert_eq!(
        normalize("fs:/mod/XRead/res/gbk2uc.map"),
        Resolved::Here("res/gbk2uc.map".into())
    );
    assert_eq!(
        normalize("fs:/mod/gobaksodor/leveldat.dat"),
        Resolved::Here("leveldat.dat".into())
    );
    assert_eq!(
        normalize("fs:/~/res/gbk2uc.map"),
        Resolved::Here("res/gbk2uc.map".into())
    );
    assert_eq!(normalize("fs:/mod/"), Resolved::OtherVolume);
    assert_eq!(normalize("fs:/mod/XRead"), Resolved::OtherVolume);
    assert_eq!(
        normalize("fs:/mod/a:b/x"),
        Resolved::OtherVolume,
        "dropping the module id must not swallow a colon that names another scheme"
    );
    assert_eq!(normalize("fs:/card0/x"), Resolved::OtherVolume);
}

#[test]
fn the_volume_keeps_the_case_a_pack_asked_for() {
    // The header says names in the `fs:/` namespace are case-sensitive, and the volume
    // deliberately keeps that for names without the scheme too (see the mount test above). A
    // paragraph there about the pre-3.1 folding rules is not a licence to change the key a
    // create-mode open leaves behind: the ringer enumeration and the save-block format both depend
    // on that key, and a fold made three of those tests fail during this investigation.
    assert_eq!(normalize("Main.rlb"), Resolved::Here("Main.rlb".into()));
    assert_eq!(
        normalize("fs:/Main.rlb"),
        Resolved::Here("Main.rlb".into())
    );
    assert_eq!(
        normalize("fs:/~/Main.rlb"),
        Resolved::Here("Main.rlb".into())
    );
    assert_eq!(normalize(r"SUB\A.TXT"), Resolved::Here("SUB/A.TXT".into()));
    assert_eq!(
        normalize("\u{e9}A.TXT"),
        Resolved::Here("\u{e9}A.TXT".into())
    );
    assert_eq!(normalize("../x"), Resolved::Rejected);
}

#[test]
fn a_module_directory_prefix_opens_the_file_it_shipped() {
    let pack = scratch("mod-prefix");
    std::fs::write(pack.join("app.mod"), b"image").unwrap();
    std::fs::create_dir_all(pack.join("res")).unwrap();
    std::fs::write(pack.join("res/gbk2uc.map"), b"map").unwrap();
    std::fs::write(pack.join("main.rlb"), b"table").unwrap();

    let mut v = Volume::default();
    v.mount_beside(&pack.join("app.mod"));
    assert_eq!(
        v.test("fs:/mod/app/res/gbk2uc.map"),
        g::AEE_SUCCESS,
        "a module directory prefix reaches the shipped nested file"
    );
    let f = v
        .open("fs:/mod/app/res/gbk2uc.map", g::_OFM_READ)
        .unwrap();
    assert_eq!(v.read(&f, 3), b"map");
    assert_eq!(
        v.test("main.rlb"),
        g::AEE_SUCCESS,
        "the shipped spelling still opens"
    );
    assert_ne!(
        v.test("Main.rlb"),
        g::AEE_SUCCESS,
        "the volume does not fold case: the file is `main.rlb`, and a pack asking for `Main.rlb` \
         must keep missing just as it does on a handset"
    );
    assert_ne!(v.test("fs:/mod/"), g::AEE_SUCCESS);
}

/// `written_block`/`adopt_block` are the pair a front end with a *single* save buffer needs, and two
/// things have to hold: what goes out comes back, and a block from anywhere else is not read as a
/// save. The second matters more than it looks — a `.srm` is a file a person can pick up from a
/// directory full of other games' saves, and a block read as far as it happens to parse would start a
/// run from a state no handset ever produced.
#[test]
fn a_written_block_round_trips_and_a_foreign_one_is_refused_whole() {
    let mut volume = Volume::default();
    volume
        .entries
        .insert("save.dat".into(), Entry::Written(vec![1, 2, 3, 4]));
    volume
        .entries
        .insert("存档/一".into(), Entry::Written(vec![9]));
    // Neither of these belongs in a block: a shipped file comes back from the pack, and a directory
    // is not worth a name in a format that has to stay parseable.
    volume
        .entries
        .insert("shipped.dat".into(), Entry::Shipped(vec![7, 7]));
    volume.entries.insert("saves".into(), Entry::Dir);

    let block = volume.written_block();
    let mut back = Volume::default();
    assert_eq!(
        back.adopt_block(&block).unwrap(),
        2,
        "only the two written files are in it"
    );
    assert_eq!(
        back.written_block(),
        block,
        "and the round trip is byte for byte, names and order included"
    );

    // A block that is not one of ours, one that is short, and one whose name reaches out of the
    // volume: all three refused, and the volume that already parsed one is left as it was.
    assert!(back.adopt_block(b"a .srm from something else").is_err());
    assert!(
        back.adopt_block(&block[..block.len() - 1]).is_err(),
        "truncated"
    );
    assert!(
        back.adopt_block(&block[..6]).is_err(),
        "shorter than a header"
    );

    let mut outside = Vec::new();
    outside.extend_from_slice(&BLOCK_MAGIC);
    outside.extend_from_slice(&BLOCK_VERSION.to_le_bytes());
    outside.extend_from_slice(&1u16.to_le_bytes());
    outside.extend_from_slice(&4u16.to_le_bytes());
    outside.extend_from_slice(b"../x");
    outside.extend_from_slice(&1u32.to_le_bytes());
    outside.push(0);
    assert!(
        back.adopt_block(&outside).is_err(),
        "a name that would reach outside the app's directory is not a save"
    );
    assert_eq!(
        back.written_block(),
        block,
        "and none of the refusals changed what was already there"
    );
}
