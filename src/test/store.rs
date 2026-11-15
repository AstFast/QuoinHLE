// Moved out of `src/store.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::resource::{synthetic_container, RESTYPE_IMAGE};
use std::path::PathBuf;

/// A throwaway fixture directory. Named with this process's id because the suite runs tests in
/// parallel, and a shared name would have one test delete another's files mid-assertion.
fn scratch(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("quoin-store-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp fixture directory");
    dir
}

#[test]
fn a_bare_module_name_uses_the_current_directory() {
    // `Path::new("tank.mod").parent()` is `Some("")`, which is not a directory. Sibling lookup
    // must mean the current directory, or a run started inside a pack's folder mounts nothing.
    let cwd = std::env::current_dir().expect("a current directory");
    assert_eq!(dir_of(std::path::Path::new("tank.mod")), cwd);
    assert_eq!(
        dir_of(std::path::Path::new("pack/tank.mod")),
        std::path::Path::new("pack")
    );
}

#[test]
fn a_flat_layout_adopts_the_manifest_named_for_the_module() {
    // How the system-app collection is laid out — `app.mod` and `app.mif` in the same
    // directory, beside thirteen language `.bar`s, a `.sig`, the app's own saved `.ini` and loose
    // icons. `own` decides what a lookup that names no file reads, so it has to be *chosen by name*:
    // taking the first container in the directory is how an app gets handed another title's strings as
    // its own app-info.
    let root = scratch("flat-own");
    let app = root.join("app");
    std::fs::create_dir_all(&app).unwrap();
    // Without this, a fixture that grew an ancestor `root/app.mif` would quietly be testing
    // the ancestor rule instead of the fallback this is here to pin.
    assert!(
        !root.join("app.mif").exists() && !root.join("app.bar").exists(),
        "the ancestor rule must have nothing to find, or the fallback never runs"
    );
    std::fs::write(
        app.join("app.mod"),
        b"mounting reads no code, so these bytes are never executed",
    )
    .unwrap();
    // The app's own manifest, written the way the collection writes them: the table's final entry one
    // byte past the end of the file. With two records that costs only the last one — the final entry
    // is the *stop* of the last record and of nothing else — so id 7 still answers from here while the
    // neighbour's copy of the same id must not be preferred.
    std::fs::write(
        app.join("app.mif"),
        synthetic_container(
            &[(RESTYPE_IMAGE, 7, b"BMown"), (RESTYPE_IMAGE, 8, b"BMtail")],
            1,
        ),
    )
    .unwrap();
    std::fs::write(
        app.join("someone-else.mif"),
        synthetic_container(
            &[
                (RESTYPE_IMAGE, 7, b"BMneighbour"),
                (RESTYPE_IMAGE, 8, b"BMother8"),
            ],
            0,
        ),
    )
    .unwrap();

    let mut store = Store::default();
    let mounted = store.mount_beside(&app.join("app.mod"));
    assert!(
        mounted.iter().any(|n| n == "app.mif"),
        "a sentinel past the end no longer hides the container: {mounted:?}"
    );
    assert!(mounted.iter().any(|n| n == "someone-else.mif"));
    assert_eq!(
        store.own.as_deref(),
        Some("app.mif"),
        "named for the module, not the first file read_dir happened to give"
    );
    let found = store
        .find(None, RESTYPE_IMAGE, 7)
        .expect("the own container answers an unnamed lookup");
    assert_eq!(
        found.body, b"BMown",
        "this app's picture, not the neighbour's"
    );
    assert_eq!(found.from, "app.mif");
    // The record the sentinel bounds is not served *as a truncated one*, and asking for the file by
    // name says so: the answer is "not here", not the neighbour's bytes under this file's name.
    assert!(
        store.find(Some("app.mif"), RESTYPE_IMAGE, 8).is_none(),
        "the tail record stays unserved"
    );
    // A lookup that names no file still falls through the rest of the mount list — existing policy,
    // and what `own` buys is the order, not exclusivity.
    let fell_through = store
        .find(None, RESTYPE_IMAGE, 8)
        .expect("the neighbour still has an id 8");
    assert_eq!(fell_through.from, "someone-else.mif");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_flat_directory_with_no_same_named_manifest_gives_the_app_no_own() {
    // The fallback must not widen into "take whatever container is lying around". Those files are
    // mounted and remain reachable by name; what they must not do is become the app-info manifest.
    let root = scratch("flat-noown");
    std::fs::write(root.join("app.mod"), b"x").unwrap();
    std::fs::write(
        root.join("resource_en.bar"),
        synthetic_container(&[(RESTYPE_IMAGE, 7, b"BMart")], 0),
    )
    .unwrap();
    let mut store = Store::default();
    let mounted = store.mount_beside(&root.join("app.mod"));
    assert!(
        mounted.iter().any(|n| n == "resource_en.bar"),
        "mounted all the same: {mounted:?}"
    );
    assert!(store.own.is_none(), "and nothing adopted");
    std::fs::remove_dir_all(&root).unwrap();
}
