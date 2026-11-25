//! The app-private file volume: what `AEECLSID_FILEMGR` stands on.
//!
//! A handset gives every applet a directory of its own, and every name the app uses is relative to
//! it. Two things follow, and they are the reason this module exists: an app that cannot ask whether
//! its save file exists takes a code path that never happened on a real device, and an app that can
//! write anywhere can walk out of the sandbox. So the volume is that directory, sub-directories
//! included, with the prefix handling done here, and the only bytes on the host disk it can ever see
//! are the ones that were in the pack's own directory when the run started.
//!
//! Two kinds of entry, and the difference is not cosmetic. A *reference* file is the pack's own,
//! present before the app ran; a *written* file is one the run made or took over. Both live in
//! memory, and that is the point: a headless run that silently edited the user's tree would be worse
//! than one that loses the save, so the only bytes from the host disk an app ever sees are the ones
//! it started with, and an app that writes to a pack file gets a written copy of it.
//!
//! *Where* those written bytes go at the end of a run is the caller's decision, and there are two of
//! them. By default they go nowhere: the volume is memory, the save is lost with the run, and the
//! pack's directory on disk is left exactly as it was. A caller who wants a game to remember its save
//! names a directory with [`Volume::persist_to`], and then every write is mirrored there as it
//! happens — which is what a handset does when it writes its flash, and it is why a run with a home
//! directory is no longer reproducible from the same starting state: that is the point of asking.

use std::collections::BTreeMap;
use std::path::Path;

use crate::abi::generated as g;

/// `AEEFileMgr`'s attribute bits, from the header.
///
/// The extractor keeps a whitelist of `#define` name families and `AEE_FA_` is not in it (task #28),
/// so they are written out here. Nothing else should copy them a second time.
pub const FA_NORMAL: u8 = 0x00;
pub const FA_DIR: u8 = 0x02;
pub const FA_READONLY: u8 = 0x04;
pub const FA_CONST: u8 = 0x10;

/// A volume size in the only unit the API can carry. the header says it outright: space larger than a
/// 32-bit integer is reported as `0xFFFFFFFF`, so both the free and the total figure clamp here.
pub fn room(bytes: u64) -> u32 {
    bytes.min(u32::MAX as u64) as u32
}

/// `aee_makepath` (the delivered body, 338 bytes), read instruction by instruction: copy the directory,
/// append one `/` **only** when the directory is neither empty nor already ends with one, skip a
/// leading `/` on the leaf rather than doubling the separator, terminate. The capacity the caller gave
/// is applied by whoever writes the result, so it is not a concern here.
pub fn make_path(dir: &str, leaf: &str) -> String {
    let mut out = String::with_capacity(dir.len() + leaf.len() + 1);
    if !dir.is_empty() {
        out.push_str(dir);
        if !dir.ends_with('/') {
            out.push('/');
        }
    }
    out.push_str(leaf.strip_prefix('/').unwrap_or(leaf));
    out
}

/// `aee_basename` on its side of the split: the text after the last `/` or `\`, and everything up to and
/// including that separator. A name with no separator has the whole string as its basename and no head.
fn split_basename(name: &str) -> (String, &str) {
    match name.rfind(['/', '\\']) {
        Some(i) => (name[..=i].to_owned(), &name[i + 1..]),
        None => (String::new(), name),
    }
}

/// The gate `CFile_GetInfo` puts on its out-parameter: `AEE_CheckPtr(0, a2, 76, 1)` — the inspection
/// loads it as `movs r2, #0x4c`. 76 is `sizeof`: the three words at `+0/+4/+8` plus
/// `szName[AEE_MAX_FILE_NAME]`.
pub const INFO_SIZE: u32 = 12 + g::AEE_MAX_FILE_NAME;

/// How much a run may write. An app in a loop with no free-space check would otherwise grow the
/// host process until the operating system stops it; 16 MiB is far past anything the guest packs
/// hold and far below what would hurt.
pub const VOLUME_LIMIT: usize = 16 * 1024 * 1024;

/// The file system's own reason for refusing a request that is legal *as a call* but not on this
/// object: the header `AEEFS_EINVALIDOPERATION = AEEFS_ERROR + 0x0A` ("Invalid operation") —
/// 266, the number `CFile_Write` and `CFile_Truncate` put in the error cell for a const/ROM view.
/// The harvested table carries the family's base (`AEEFS_ERROR`) and not this member, so the name
/// lives beside the two callers rather than as a bare 266 in an arm.
pub const AEEFS_EINVALIDOPERATION: u32 = g::AEEFS_ERROR + 0x0A;

/// The header `AEEFS_EBADSEEKPOS = AEEFS_ERROR + 0x04` ("Bad seek position") — 260, the code
/// `CFile_Seek` puts in the error cell when a const view's target falls outside `0..=size`. On a file
/// the app wrote, the same range test lives in the file system and this store cannot ask it, so the
/// same number answers the same fact here.
pub const AEEFS_EBADSEEKPOS: u32 = g::AEEFS_ERROR + 0x04;

/// The header `AEEFS_EFSFULL = AEEFS_ERROR + 0x06` ("File system full") — 262. Two slots reach
/// for it and answer differently: `CFile_Seek` refuses (`return 1`, the position does not move) while
/// `CFile_Write` **clamps the length to the room** and reports the smaller count.
pub const AEEFS_EFSFULL: u32 = g::AEEFS_ERROR + 0x06;

/// The header `AEEFS_EFILENOEXISTS = AEEFS_ERROR + 0x01` — 0x101, the file system's answer for
/// a name it does not hold. `CFileMgr_OpenFile` writes the EFS result into the manager's error cell
/// (`*(a1 + 12) = v10`), so a pack that asks `GetLastError` sees this member. The generic
/// `AEE_ENOSUCH` (0x27) names a class or an object id, not a path, and telling a pack the wrong one
/// of the two changes which branch it takes on a missing resource.
pub const AEEFS_EFILENOEXISTS: u32 = g::AEEFS_ERROR + 0x01;

/// The header `AEEFS_EDIRNOEXISTS = AEEFS_ERROR + 0x09` — 0x109, the directory spelling of the
/// same answer. `RmDir` asks for a directory, so a name that is not one gets this.
pub const AEEFS_EDIRNOEXISTS: u32 = g::AEEFS_ERROR + 0x09;

pub const AEE_EBADFILENAME: u32 = g::AEEFS_ERROR + 0x03;

/// How many directories below the `.mod` [`Volume::mount_beside`] will go.
///
/// The deepest thing a pack in this sample set ships is `res/ui/`, so this is not a limit a real run
/// reaches; it is what stops a directory someone pointed a run at from being walked for as long as
/// it happens to nest.
///
/// Public because the library's save-directory listing walks the *same* tree on the *other* side of
/// the volume's lifetime, and a listing that went deeper than a run can reach would offer a file the
/// game itself cannot see.
pub const MOUNT_DEPTH: u8 = 4;

/// One entry in the volume.
#[derive(Debug)]
enum Entry {
    /// Bytes the app made this run.
    Written(Vec<u8>),
    /// A file the pack provides, copied in at mount time so the host file is never held open and
    /// cannot change under the run's feet.
    Shipped(Vec<u8>),
    Dir,
}

impl Entry {
    fn len(&self) -> usize {
        match self {
            Entry::Written(bytes) | Entry::Shipped(bytes) => bytes.len(),
            Entry::Dir => 0,
        }
    }

    fn attrib(&self) -> u8 {
        match self {
            Entry::Written(_) => FA_NORMAL,
            // ROM-resident and not writable, which is what `AEE_FA_CONST` means on a handset: the
            // pack is on the card and the app's own writes go somewhere else.
            Entry::Shipped(_) => FA_READONLY | FA_CONST,
            Entry::Dir => FA_DIR,
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Entry::Written(bytes) | Entry::Shipped(bytes) => bytes,
            Entry::Dir => &[],
        }
    }
}

/// How the app opened a file, and where its pointer is.
#[derive(Debug, Clone)]
pub struct Opened {
    /// The normalised name, so a handle keeps working after the app's own buffer has been reused.
    pub name: String,
    pub pos: u32,
    /// `_OFM_READWRITE` or `_OFM_APPEND`, and only for an entry the volume will accept writes to.
    pub writable: bool,
    pub append: bool,
}

/// One name and what the volume knows about it, for `AEEFileInfo`.
#[derive(Debug, Clone)]
pub struct Info {
    pub name: String,
    pub attrib: u8,
    pub size: u32,
}

/// The app-private volume, plus the single enumeration cursor `EnumInit`/`EnumNext` share.
#[derive(Default)]
pub struct Volume {
    /// Whether a lookup folds ASCII case, as a FAT-backed handset does. The default is the BREW
    /// header's case-sensitive namespace; a device profile whose packs rely on folding turns it on.
    case_insensitive: bool,
    /// Sorted, so `EnumNext` has an order worth asserting on. A handset's directory order is
    /// unspecified; a run's should not be.
    entries: BTreeMap<String, Entry>,
    /// What `EnumInit` picked — each entry's key and its **leaf** — and how far `EnumNext` has got
    /// through them. The name a caller is handed is the leaf with [`Volume::enum_dir`] put back in
    /// front, which is what the reference enumeration does at report time rather than at list time.
    listing: Vec<(String, String)>,
    at: usize,
    /// The directory string the running enumeration was started with, with only its leading separators
    /// removed. `CFileMgr_EnumInit` stores `aee_err_strdup` of **the caller's text** — not the resolved
    /// path — and `EnumNext`/`EnumNextEx` prefix every leaf with it, so enumerating `res` reports
    /// `res/ui`. Enumerating the root stores an empty string (the body turns a null or empty name into
    /// `"/"`, then strips it), which is why that one reports bare leaves.
    enum_dir: String,
    enum_pattern: Option<String>,
    /// `IFILEMGR_GetLastError`: the detail behind a returned `AEE_EFAILED`.
    last_error: u32,
    /// The host directory this volume's *writes* are mirrored into, when the caller named one.
    ///
    /// `None` is every ordinary run, and it is the interesting case: what the app writes is kept
    /// faithfully while the run lasts and lost when it ends, so a run never edits the tree it was
    /// pointed at. A caller who wants a game to remember its save across runs names a directory with
    /// [`Volume::persist_to`], and that directory is then the only place besides the pack's own files
    /// this volume ever touches.
    home: Option<std::path::PathBuf>,
    /// The canonical form of [`Volume::home`], fixed when the save directory was opened.
    ///
    /// A normalized guest name cannot climb out lexically, but a link already inside the save
    /// directory can carry a write or a delete to wherever it points. Every host path is resolved
    /// and tested against this root before it is touched, so the link decides nothing by itself.
    home_real: Option<std::path::PathBuf>,
    /// `IFILEMGR_SetDescription`: name → description, as `AECHAR` units.
    ///
    /// The header says `pszDescription` is a wide-string description whose capacity comes in from
    /// `nDescriptionSize` in bytes. On a handset it lives in the
    /// file's EFS attributes; this machine has no such table, so it is kept here and handed back by
    /// `GetInfoEx` — which is the whole of what the pair is for: an app writes one and reads it back.
    descriptions: BTreeMap<String, Vec<u16>>,
    /// `IFILEMGR_Use`/`IFILEMGR_UnUse`: name → how many holders have said they are using it.
    ///
    /// The header defines `AEEFileUseInfo::wFilesUsed` as "Number of Files currenty used by this
    /// Module" and `Use` is the action that marks one, so this is what
    /// `GetFileUseInfo` counts. It is also the reason `Remove` can answer `AEE_EBADSTATE` rather than
    /// deleting a file somebody has open on purpose.
    in_use: BTreeMap<String, u32>,
}

/// Where a name asks to live, relative to the app's own directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A name relative to the app's directory — possibly empty, which is the root itself.
    Here(String),
    /// A name carrying another volume's scheme, such as `fs:/card0/`: a real thing to ask for, and
    /// not this one. the header publishes `AEE_ESCHEMENOTSUPPORTED` for exactly this.
    OtherVolume,
    /// A name that would reach outside the app's directory. `AEE_EBADPARM`, because the handset has
    /// no better code for it and the caller's path is genuinely not usable.
    Rejected,
}

/// Strip the volume prefixes a handset maps onto the app's own directory, and classify a name that
/// does not fit inside it.
///
/// Apps pass the whole space of strings here, so this is a boundary check rather than tidiness, and
/// `".."` is the case that has to fail rather than be normalised away: a pack that could reach
/// through its parent would read the next app's files.
pub fn normalize(name: &str) -> Resolved {
    let mut rest = name.trim();
    // `AEEFS_HOME_DIR` and `AEEFS_ROOT_DIR` are unified here: this host has one app volume, so
    // `fs:/` and `fs:/~/` both name the same root and a file manager can enumerate it. `fs:/card0/`
    // and its neighbours are still other devices, so those are refused rather than answered from
    // the app's own directory.
    if let Some(tail) = strip_prefix_any(rest, "fs:/") {
        let tail = tail.trim_start_matches(['/', '\\']);
        let first = tail.split(['/', '\\']).next().unwrap_or("");
        if first.starts_with("card") {
            return Resolved::OtherVolume;
        }
        if first == "mod" {
            // the header says `fs:/~/` is a shortcut for `fs:/mod/<modid>` and that
            // `fs:/mod/` is where module directories live. A name below a module id therefore names
            // a file in the app's own directory, not a `mod/` directory inside it: drop the
            // `mod/<id>` pair. This is an approximation with a stated cost — a request aimed at
            // *another* module cannot be told apart here, and would answer from this app's files if
            // the names collided. There is no second volume to answer from, and the alternative is
            // refusing every `fs:/mod/` path a pack uses to reach its own resources. A name that
            // stops at the module id names a directory this host does not have.
            // A colon in the discarded part would vanish along with it, and the check further down
            // only sees what is left, so the whole tail is tested before the id is dropped. A
            // `fs:/mod/a:b/x` is not a file in this app's directory, and answering `x` would widen
            // the volume.
            if tail.contains(':') {
                return Resolved::OtherVolume;
            }
            let after = tail
                .get(first.len()..)
                .unwrap_or("")
                .trim_start_matches(['/', '\\']);
            let Some(sep) = after.find(['/', '\\']) else {
                return Resolved::OtherVolume;
            };
            rest = &after[sep + 1..];
        } else {
            rest = if let Some(after) = strip_prefix_any(tail, "~/") {
                after
            } else if strip_prefix_any(tail, "~").is_some() {
                ""
            } else {
                tail
            };
        }
    } else if let Some(tail) = strip_prefix_any(rest, "~/") {
        rest = tail;
    } else if strip_prefix_any(rest, "~").is_some() {
        rest = "";
    }
    // Any scheme left over is a volume this host does not have.
    if rest.contains(':') {
        return Resolved::OtherVolume;
    }
    let mut out = String::new();
    for part in rest.split(['/', '\\']) {
        match part {
            "" | "." => continue,
            ".." => return Resolved::Rejected,
            p => {
                // Windows strips trailing spaces and dots from every component before it resolves
                // a path, so `".. "`, `".. ."` and `"..."` all collapse towards `..` — none of them
                // equals it as a string, and `Path::join` hands the untrimmed name to the
                // filesystem, which does the trimming this check would otherwise have missed.
                // A component that *starts* with `..` and holds nothing but dots and spaces after
                // it can only collapse into the parent (or into nothing, which names no file), so
                // it is refused; `..name` is an ordinary file name and stays.
                let collapses_to_parent = p
                    .strip_prefix("..")
                    .is_some_and(|tail| tail.chars().all(|c| c == '.' || c == ' '));
                if collapses_to_parent {
                    return Resolved::Rejected;
                }
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(p);
            }
        }
    }
    Resolved::Here(out)
}

fn strip_prefix_any<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let len = prefix.len();
    // Byte-compare the prefix and take the remainder through `get`, which yields `None` at a
    // non-boundary instead of panicking. Names reach here as whatever bytes the module held, run
    // through a lossy conversion, so an offset that splits a character is reachable input.
    match s.get(..len) {
        Some(head) if head.eq_ignore_ascii_case(prefix) => s.get(len..),
        _ => None,
    }
}

/// Where a host path may be touched: its parent, resolved, must still be inside the save root.
///
/// `Ok(None)` means the parent is not there, which for a deletion is the same answer as the target
/// already being gone; a write resolves this through [`ensure_parent_inside`] instead.
fn parent_inside(root: &Path, path: &Path) -> std::io::Result<Option<std::path::PathBuf>> {
    let Some(parent) = path.parent() else {
        return Ok(None);
    };
    match std::fs::canonicalize(parent) {
        Ok(real) if real.starts_with(root) => Ok(Some(real)),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "path resolves outside the save directory",
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Create the parent of a write's target, after proving that the nearest part of it which exists
/// still resolves inside the save root. Creating first and checking afterwards would let a link in
/// the middle of the path receive the new directories.
fn ensure_parent_inside(root: &Path, path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let mut probe = parent;
    loop {
        match std::fs::canonicalize(probe) {
            Ok(real) => {
                if !real.starts_with(root) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "path resolves outside the save directory",
                    ));
                }
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let Some(next) = probe.parent() else {
                    return Err(e);
                };
                probe = next;
            }
            Err(e) => return Err(e),
        }
    }
    std::fs::create_dir_all(parent)?;
    let real = std::fs::canonicalize(parent)?;
    if !real.starts_with(root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "path resolves outside the save directory",
        ));
    }
    Ok(())
}

/// A link Windows reports as a directory (a junction) rather than as a symbolic link.
///
/// `remove_dir_all` on one of these follows it. The save directory's links are removed as links.
#[cfg(windows)]
fn is_reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse(_meta: &std::fs::Metadata) -> bool {
    false
}

fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink() || is_reparse(meta)
}

/// Remove a directory and everything below it without ever following a link.
///
/// `std::fs::remove_dir_all` is left alone because a link inside the tree decides for itself
/// whether it is a directory on some platforms; a save directory the guest asked to delete must
/// not become a way to delete whatever such a link points at.
fn remove_tree(path: &Path) -> std::io::Result<()> {
    let mut dirs = Vec::new();
    let mut todo = vec![path.to_path_buf()];
    while let Some(dir) = todo.pop() {
        dirs.push(dir.clone());
        for entry in std::fs::read_dir(&dir)? {
            let child = entry?.path();
            let meta = std::fs::symlink_metadata(&child)?;
            if is_link(&meta) {
                std::fs::remove_file(&child).or_else(|_| std::fs::remove_dir(&child))?;
            } else if meta.is_dir() {
                todo.push(child);
            } else {
                std::fs::remove_file(&child)?;
            }
        }
    }
    for dir in dirs.into_iter().rev() {
        std::fs::remove_dir(&dir)?;
    }
    Ok(())
}

/// What a failed host file-system call means to the guest, in the header's own EFS family.
///
/// The family has one member for "no room" and none for the rest; a permission, lock or device
/// error is reported as an operation the file system would not perform.
fn host_failure(err: &std::io::Error) -> u32 {
    #[cfg(windows)]
    let full = matches!(err.raw_os_error(), Some(39 | 112));
    #[cfg(not(windows))]
    let full = err.raw_os_error() == Some(28);
    if full {
        AEEFS_EFSFULL
    } else {
        AEEFS_EINVALIDOPERATION
    }
}

impl Volume {
    /// Copy every file the pack ships into the volume, which is where a handset keeps the app's own
    /// directory.
    ///
    /// The `.mod` itself is included: an app that opens its own image to read a blob out of it is
    /// doing something a real device lets it do.
    ///
    /// Sub-directories count as the app's own too, because to an app they are: a pack may open
    /// `res\ma.pak` as its very first act and then walk the index inside it, and a volume that only
    /// knows the top level answers `NULL` and leaves the pack appending zero-length records to
    /// itself forever. A nested name is keyed exactly as [`normalize`] renders it — `res/ma.pak` —
    /// and the directory itself is an entry, so `EnumFiles` on the root can name it.
    pub fn mount_beside(&mut self, mod_path: &Path) -> Vec<String> {
        let mut mounted = Vec::new();
        let dir = crate::store::dir_of(mod_path);
        // Every entry is resolved before it is used, and one that resolves outside the pack's own
        // directory is skipped: a link beside a `.mod` must not mount the rest of the host.
        let Ok(root) = std::fs::canonicalize(&dir) else {
            return mounted;
        };
        // A work list rather than recursion, because the depth is what is bounded and a directory
        // tree is not naturally a stack of frames the host has to pay for.
        let mut todo = vec![(root.clone(), String::new(), 0u8)];
        // What the pack holds is copied into memory so the host files are never held open, so the
        // volume's own ceiling bounds the copy as well: a directory someone pointed a run at is
        // allowed to be enormous, and mounting all of it would end the run before it started.
        let mut room = VOLUME_LIMIT;
        while let Some((path, prefix, depth)) = todo.pop() {
            let Ok(read) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in read.flatten() {
                let child = entry.path();
                let Some(name) = child.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let key = if prefix.is_empty() {
                    name.to_owned()
                } else {
                    format!("{prefix}{name}")
                };
                let Ok(real) = std::fs::canonicalize(&child) else {
                    continue;
                };
                if !real.starts_with(&root) {
                    eprintln!(
                        "quoin: pack directory: skipping {} because it resolves outside {}",
                        child.display(),
                        root.display()
                    );
                    continue;
                }
                if real.is_dir() {
                    if depth >= MOUNT_DEPTH {
                        continue;
                    }
                    self.entries.insert(key.clone(), Entry::Dir);
                    mounted.push(key.clone());
                    todo.push((real, format!("{key}/"), depth + 1));
                    continue;
                }
                if !real.is_file() {
                    continue;
                }
                // Asked before read, because the thing being kept out is a file large enough to
                // matter, and reading it to find that out is the cost.
                let Ok(meta) = std::fs::metadata(&real) else {
                    continue;
                };
                if meta.len() > room as u64 {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&real) else {
                    continue;
                };
                room -= bytes.len().min(room);
                mounted.push(key.clone());
                self.entries.insert(key, Entry::Shipped(bytes));
            }
        }
        mounted.sort();
        mounted
    }

    /// Name the directory this volume's writes are mirrored into, and load what an earlier run left
    /// there.
    ///
    /// The directory is created if it is not there, and every file already in it becomes a *written*
    /// entry: these are the app's own files from last time, so they are writable, and a name the pack
    /// also ships resolves to the save rather than to the reference copy. That last part is what makes a
    /// game remember — a pack asks `Test`/`GetInfo` about its save before it decides to start a new
    /// game, and a volume that answers "not there" for a file an earlier run wrote sends it down the
    /// new-game path every time it is started.
    ///
    /// The returned names are what was loaded, for the caller to report.
    /// Point the volume's writes at the directory the module came from, the way a handset keeps a
    /// module's files and its saves in one place.
    ///
    /// Unlike [`Volume::persist_to`], nothing is loaded: [`Volume::mount_beside`] already put the
    /// pack files into the volume, and they stay read-only until the app opens one for writing.
    /// What it then writes is mirrored back into that directory.
    pub fn write_back_to(&mut self, dir: &Path) -> bool {
        self.home = Some(dir.to_path_buf());
        match std::fs::canonicalize(dir) {
            Ok(root) => {
                self.home_real = Some(root);
                true
            }
            Err(e) => {
                eprintln!(
                    "quoin: pack directory {} could not be resolved: {e}",
                    dir.display()
                );
                self.home_real = None;
                false
            }
        }
    }

    pub fn persist_to(&mut self, dir: &Path) -> Vec<String> {
        self.home = Some(dir.to_path_buf());
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!(
                "quoin: save directory {} could not be created: {e}",
                dir.display()
            );
            self.home_real = None;
            return Vec::new();
        }
        let Ok(root) = std::fs::canonicalize(dir) else {
            eprintln!(
                "quoin: save directory {} could not be resolved",
                dir.display()
            );
            self.home_real = None;
            return Vec::new();
        };
        self.home_real = Some(root.clone());
        let mut loaded = Vec::new();
        let mut todo = vec![(root.clone(), String::new(), 0u8)];
        let mut room = VOLUME_LIMIT;
        while let Some((path, prefix, depth)) = todo.pop() {
            let Ok(read) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in read.flatten() {
                let child = entry.path();
                let Some(name) = child.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let key = format!("{prefix}{name}");
                // A link that leads out of the save directory is left alone rather than loaded:
                // what it points at is not the guest's file, and a write through it would edit
                // something the guest never owned.
                let Ok(real) = std::fs::canonicalize(&child) else {
                    continue;
                };
                if !real.starts_with(&root) {
                    eprintln!(
                        "quoin: save directory: skipping {} because it resolves outside {}",
                        child.display(),
                        root.display()
                    );
                    continue;
                }
                if real.is_dir() {
                    if depth >= MOUNT_DEPTH {
                        continue;
                    }
                    self.entries.insert(key.clone(), Entry::Dir);
                    loaded.push(key.clone());
                    todo.push((real, format!("{key}/"), depth + 1));
                    continue;
                }
                if !real.is_file() {
                    continue;
                }
                let Ok(meta) = std::fs::metadata(&real) else {
                    continue;
                };
                if meta.len() > room as u64 {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&real) else {
                    continue;
                };
                room -= bytes.len().min(room);
                loaded.push(key.clone());
                self.entries.insert(key, Entry::Written(bytes));
            }
        }
        loaded.sort();
        loaded
    }

    /// Bring the host directory in line with one entry of the volume — or with its absence.
    ///
    /// Called after every change, so the directory is never behind the run: a save that is on disk at
    /// the moment the app wrote it is still on disk if the process is killed a second later, which is
    /// the same promise a handset's flash makes. A volume the caller gave no home does nothing here,
    /// which is every ordinary run.
    fn mirror(&self, key: &str) -> std::io::Result<()> {
        let Some(home) = &self.home else {
            return Ok(());
        };
        if key.is_empty() {
            return Ok(());
        }
        let Some(root) = &self.home_real else {
            let why = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("save directory {} is not usable", home.display()),
            );
            eprintln!("quoin: save: {why}");
            return Err(why);
        };
        let path = root.join(key);
        match self.entries.get(key) {
            Some(Entry::Written(bytes)) => {
                if let Err(e) = ensure_parent_inside(root, &path).and_then(|()| {
                    if std::fs::symlink_metadata(&path).is_ok_and(|m| is_link(&m)) {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "refusing to write through a link in the save directory",
                        ))
                    } else {
                        std::fs::write(&path, bytes)
                    }
                }) {
                    eprintln!("quoin: save: could not write {}: {e}", path.display());
                    return Err(e);
                }
                Ok(())
            }
            // The bytes are the pack's, and a run does not put the pack's own files anywhere.
            Some(Entry::Shipped(_)) => Ok(()),
            Some(Entry::Dir) => {
                if let Err(e) = ensure_parent_inside(root, &path).and_then(|()| {
                    if std::fs::symlink_metadata(&path).is_ok_and(|m| is_link(&m)) {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "refusing to create a directory over a link in the save directory",
                        ))
                    } else {
                        std::fs::create_dir_all(&path)
                    }
                }) {
                    eprintln!("quoin: could not create {}: {e}", path.display());
                    return Err(e);
                }
                Ok(())
            }
            None => {
                if parent_inside(root, &path)
                    .map_err(|e| {
                        eprintln!("quoin: could not remove {}: {e}", path.display());
                        e
                    })?
                    .is_none()
                {
                    return Ok(());
                }
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    // The caller asked for the name not to be there, and it is not.
                    return Ok(());
                };
                if is_link(&meta) {
                    return std::fs::remove_file(&path)
                        .or_else(|_| std::fs::remove_dir(&path))
                        .map_err(|e| {
                            eprintln!("quoin: could not remove {}: {e}", path.display());
                            e
                        });
                }
                if meta.is_dir() {
                    eprintln!(
                        "quoin: save: deleting {} recursively because the guest removed the directory",
                        path.display()
                    );
                    return remove_tree(&path).map_err(|e| {
                        eprintln!("quoin: could not remove {}: {e}", path.display());
                        e
                    });
                }
                std::fs::remove_file(&path).map_err(|e| {
                    eprintln!("quoin: could not remove {}: {e}", path.display());
                    e
                })
            }
        }
    }

    /// Record the detail behind an `AEE_EFAILED`, which is what `GetLastError` exists to report.
    fn fail(&mut self, why: u32) -> u32 {
        self.last_error = why;
        g::AEE_EFAILED
    }

    /// What a refusal of [`Volume::open`] meant, in words a report can print.
    ///
    /// `IFILEMGR_OpenFile` answers with an `IFile*` or `NULL` and has no out-parameter for a reason,
    /// so the pack's only way to find out is to ask `GetLastError` — and a pack that does not ask, or
    /// asks and ignores the answer, leaves the host knowing something nobody can see. "Which name did
    /// it want, and why did we not have it" is the question a run that never loads its next scene is
    /// really asking, and these are the only three codes [`Volume::open`] can return.
    pub fn refusal(code: u32) -> &'static str {
        match code {
            g::AEE_ESCHEMENOTSUPPORTED => {
                "a volume this host does not have (a card, or another app's directory)"
            }
            g::AEE_ENOSUCH => "no file of that name",
            AEEFS_EFILENOEXISTS => "no file of that name",
            AEEFS_EDIRNOEXISTS => "no directory of that name",
            g::AEE_EBADPARM => {
                "not a name or mode this volume can open (empty, a directory, no access mode)"
            }
            _ => "refused",
        }
    }

    /// A name, or the reason it is not one. Every method takes its `EFAILED` from here so that
    /// asking for the card and asking to escape the directory are distinguishable afterwards, which
    /// is the only thing `GetLastError` is good for.
    fn resolve(&mut self, name: &str) -> Option<String> {
        match normalize(name) {
            Resolved::Here(n) => Some(self.canonical(n)),
            Resolved::OtherVolume => {
                self.last_error = g::AEE_ESCHEMENOTSUPPORTED;
                None
            }
            Resolved::Rejected => {
                self.last_error = g::AEE_EBADPARM;
                None
            }
        }
    }

    /// The key an existing entry is filed under, folding ASCII case when the device says its
    /// filesystem does. A name that is not there keeps the spelling the caller asked for, so a
    /// create-mode open still leaves a key with that case.
    fn canonical(&self, name: String) -> String {
        if !self.case_insensitive || name.is_empty() || self.entries.contains_key(&name) {
            return name;
        }
        self.entries
            .keys()
            .find(|key| key.eq_ignore_ascii_case(&name))
            .cloned()
            .unwrap_or(name)
    }

    /// Turn on the ASCII-case folding a FAT-backed handset performs.
    pub fn set_case_insensitive(&mut self, fold: bool) {
        self.case_insensitive = fold;
    }

    fn refused(&mut self) -> u32 {
        // `resolve` has already recorded which of the two it was.
        g::AEE_EFAILED
    }

    /// `IFILEMGR_Test`: `AEE_SUCCESS` for a file or directory that is there. `""` names the app's
    /// root directory, which always is.
    pub fn test(&mut self, name: &str) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        if n.is_empty() || self.entries.contains_key(&n) {
            return g::AEE_SUCCESS;
        }
        self.fail(AEEFS_EFILENOEXISTS)
    }

    /// `IFILEMGR_GetInfo` and `IFile_GetInfo` both report through this.
    pub fn info(&mut self, name: &str) -> Option<Info> {
        let n = self.resolve(name)?;
        if n.is_empty() {
            return Some(Info {
                name: String::new(),
                attrib: FA_DIR,
                size: 0,
            });
        }
        self.info_of(&n)
    }

    pub fn resolved(&mut self, name: &str) -> Option<String> {
        self.resolve(name)
    }

    /// The same for a name that is already normalised, which is what a handle holds.
    fn info_of(&mut self, name: &str) -> Option<Info> {
        let Some(entry) = self.entries.get(name) else {
            self.last_error = AEEFS_EFILENOEXISTS;
            return None;
        };
        Some(Info {
            size: entry.len() as u32,
            attrib: entry.attrib(),
            name: name.to_owned(),
        })
    }

    /// `IFile::GetInfo`/`GetInfoEx` on a handle this volume handed out, asked by the name the handle
    /// carries. This is `CFile_GetInfo`'s branch list: the const view answers from the record
    /// (`FillConstInfo`, whose attrib is [`Entry::attrib`]'s `FA_READONLY|FA_CONST`), a written file
    /// answers from the file system, and a name that has left the volume is the body's
    /// `CFile_ReOpen(a1, 1) != 0` — which returns TRUE **before writing a byte** of the struct and
    /// **without touching the error cell**. `Volume::info` puts a code in the cell because the slots
    /// that reach it by *name* are supposed to; this one must not, so it borrows the volume read-only.
    pub fn handle_info(&self, name: &str) -> Option<Info> {
        let entry = self.entries.get(name)?;
        Some(Info {
            size: entry.len() as u32,
            attrib: entry.attrib(),
            name: name.to_owned(),
        })
    }

    /// `IFILEMGR_OpenFile`. The handset returns an `IFile*` or `NULL`, so the reason is carried in
    /// [`Volume::last_error`] for the caller to report.
    pub fn open(&mut self, name: &str, mode: u32) -> Result<Opened, u32> {
        let Some(n) = self.resolve(name) else {
            return Err(self.last_error);
        };
        if n.is_empty() {
            // The root is a directory, and opening it as a file is a caller bug.
            self.last_error = g::AEE_EBADPARM;
            return Err(g::AEE_EBADPARM);
        }
        let append = mode & g::_OFM_APPEND != 0;
        // `_OFM_CREATE` is a write mode as surely as `_OFM_READWRITE` is. The header's one sentence
        // about it — that it "will only create a file if it did not exist prior to the
        // `IFILEMGR_OpenFile` call" — is about the creation, not the access, and what a pack does
        // with the handle it gets is `Seek` and `Write`: a save path may open a file for read, close
        // it, and reopen it with mode 4 to write the game back.
        let writable = mode & (g::_OFM_READWRITE | g::_OFM_APPEND | g::_OFM_CREATE) != 0;
        if mode & (g::_OFM_READ | g::_OFM_READWRITE | g::_OFM_APPEND | g::_OFM_CREATE) == 0 {
            self.last_error = g::AEE_EBADPARM;
            return Err(g::AEE_EBADPARM);
        }
        if matches!(self.entries.get(&n), Some(Entry::Dir)) {
            self.last_error = g::AEE_EBADPARM;
            return Err(g::AEE_EBADPARM);
        }
        // A pack file the app wants to edit becomes a written copy of itself. The bytes on the
        // host disk are already out of reach — `mount_beside` copied them in at the start — so this
        // changes the run and nothing that comes after it, which is the same bargain the module makes
        // for a save file the app invents: kept faithfully while it runs, lost when it does. Refusing
        // instead was the old reading, and it costs any pack that stores its game in its own
        // directory, because a handset's app directory is writable and these files came out of one.
        if writable && matches!(self.entries.get(&n), Some(Entry::Shipped(_))) {
            let bytes = match self.entries.remove(&n) {
                Some(Entry::Shipped(bytes)) => bytes,
                _ => Vec::new(),
            };
            self.entries.insert(n.clone(), Entry::Written(bytes));
        }
        if !self.entries.contains_key(&n) {
            if mode & g::_OFM_CREATE != 0 {
                self.entries.insert(n.clone(), Entry::Written(Vec::new()));
            } else {
                // `_OFM_READWRITE` and `_OFM_APPEND` do not create, which is the header's own
                // comment, and so the answer for a name that is not there is the same as for a
                // reader that asks for it.
                self.last_error = AEEFS_EFILENOEXISTS;
                return Err(AEEFS_EFILENOEXISTS);
            }
        }
        let pos = if append {
            self.entries.get(&n).map(Entry::len).unwrap_or(0) as u32
        } else {
            0
        };
        Ok(Opened {
            name: n,
            pos,
            writable,
            append,
        })
    }

    /// The bytes `IFile::Map` may hand out: a read-only view of a file the pack **reference**.
    ///
    /// `None` is the header's own failure answer, and it covers each of its conditions
    ///: only a const file can be mapped, so a file the app wrote this run is
    /// not mappable, and a range that does not fit between `offset` and the end is not the mapping
    /// that was asked for. A directory has no bytes either way.
    ///
    /// Keyed by the normalised name rather than by an `&Opened`, because the caller cannot keep a
    /// borrow on the volume's bookkeeping across the arena allocation it is about to make. It is asked
    /// twice — once for the length, once for the bytes — rather than holding the view over that call.
    pub fn const_bytes_at(&self, name: &str, offset: u32, size: u32) -> Option<&[u8]> {
        let bytes = match self.entries.get(name) {
            Some(Entry::Shipped(bytes)) => bytes,
            _ => return None,
        };
        let start = offset as usize;
        let stop = start.checked_add(size as usize)?;
        (start <= stop && stop <= bytes.len()).then(|| &bytes[start..stop])
    }

    /// As many bytes as exist from `pos`, up to `want`.
    pub fn read(&self, open: &Opened, want: u32) -> Vec<u8> {
        let Some(entry) = self.entries.get(&open.name) else {
            return Vec::new();
        };
        let bytes = entry.bytes();
        let start = (open.pos as usize).min(bytes.len());
        let stop = (start as u64 + u64::from(want)).min(bytes.len() as u64) as usize;
        bytes[start..stop].to_vec()
    }

    pub fn write(&mut self, open: &mut Opened, bytes: &[u8]) -> Result<u32, u32> {
        if matches!(self.entries.get(&open.name), Some(Entry::Shipped(_))) {
            self.last_error = AEEFS_EINVALIDOPERATION;
            return Ok(0);
        }
        if bytes.is_empty() {
            if let Err(e) = self.mirror(&open.name) {
                self.last_error = host_failure(&e);
                return Err(g::AEE_EFAILED);
            }
            return Ok(0);
        }
        if !open.writable {
            self.last_error = g::AEE_EREADONLY;
            return Err(g::AEE_EFAILED);
        }
        self.last_error = 0;
        let held = self
            .entries
            .get(&open.name)
            .map(|e| e.len() as i64)
            .unwrap_or(0);
        let start = if open.append { held } else { open.pos as i64 };
        let asked = bytes.len() as i64;
        let total = self.entries.values().map(Entry::len).sum::<usize>() as i64;
        let free = (VOLUME_LIMIT as i64) - total;
        let room = (free + held - start).max(0);
        let keep = if asked > room {
            self.last_error = AEEFS_EFSFULL;
            room as usize
        } else {
            asked as usize
        };
        let bytes = &bytes[..keep.min(bytes.len())];
        if bytes.is_empty() {
            // Nothing could be honoured, so nothing happens at all: no bytes, no position, and in
            // particular **no hole** stretched out to the request's length. The cell already says why.
            return Ok(0);
        }
        let end = start + bytes.len() as i64;
        let old_len = match self.entries.get(&open.name) {
            Some(Entry::Written(data)) => data.len(),
            _ => 0,
        };
        // The part of the old file this write replaces, so a host failure can put it back rather
        // than leave the guest's memory and the save directory disagreeing about what is on disk.
        let replaced = match self.entries.get(&open.name) {
            Some(Entry::Written(data)) if (start as usize) < old_len => {
                data[start as usize..(end as usize).min(old_len)].to_vec()
            }
            _ => Vec::new(),
        };
        // The mutable borrow ends with this block: what follows reads the entry back out of the volume
        // to mirror it, and a save that has been written but not yet put on the host's disk is a save
        // the next run looks for and does not find.
        {
            let Entry::Written(data) = self.entries.get_mut(&open.name).ok_or(g::AEE_EFAILED)?
            else {
                self.last_error = g::AEE_EREADONLY;
                return Err(g::AEE_EFAILED);
            };
            if (end as usize) > data.len() {
                data.resize(end as usize, 0);
            }
            data[start as usize..end as usize].copy_from_slice(bytes);
        }
        if let Err(e) = self.mirror(&open.name) {
            if let Some(Entry::Written(data)) = self.entries.get_mut(&open.name) {
                data.truncate(old_len);
                if !replaced.is_empty() {
                    data[start as usize..start as usize + replaced.len()]
                        .copy_from_slice(&replaced);
                }
            }
            self.last_error = host_failure(&e);
            return Err(g::AEE_EFAILED);
        }
        open.pos = end as u32;
        Ok(bytes.len() as u32)
    }

    pub fn seek(&mut self, open: &mut Opened, kind: u32, distance: i32) -> u32 {
        let size = self.entries.get(&open.name).map(|e| e.len()).unwrap_or(0) as i64;
        // The const view is tested first, and its branch never reaches the file system — so it never
        // rewrites the error cell with a success either. A const seek, accepted or refused, leaves the
        // previous reason standing; only its own refusals put a number there.
        if matches!(self.entries.get(&open.name), Some(Entry::Shipped(_))) {
            return self.seek_const(open, kind, distance, size);
        }
        // Every other seek ends in a file-system call, and that call rewrites the error cell — so a
        // success leaves 0 there rather than an older refusal. This store has no such call to do it,
        // and clears it here instead.
        self.last_error = 0;
        if kind == g::_SEEK_CURRENT && distance == 0 {
            return open.pos;
        }
        let base = match kind {
            s if s == g::_SEEK_START => 0,
            s if s == g::_SEEK_END => size,
            s if s == g::_SEEK_CURRENT => open.pos as i64,
            _ => {
                self.last_error = g::AEE_EBADPARM;
                return g::AEE_EFAILED;
            }
        };
        let target = base + distance as i64;
        if target < 0 {
            self.last_error = AEEFS_EBADSEEKPOS;
            return g::AEE_EFAILED;
        }
        // Whatever lies past the current end is what has to be paid for; the body spells this out per
        // seek type (`case 0: v7 - size`, `case 1: distance`, `case 2: distance - (size - pos)`) and the
        // three reduce to it.
        let growth = (target - size).max(0);
        if growth > 0 {
            let held = self.entries.values().map(Entry::len).sum::<usize>() as i64;
            if growth > (VOLUME_LIMIT as i64) - held {
                self.last_error = AEEFS_EFSFULL;
                return g::AEE_EFAILED;
            }
        }
        open.pos = target as u32;
        g::AEE_SUCCESS
    }

    fn seek_const(&mut self, open: &mut Opened, kind: u32, distance: i32, size: i64) -> u32 {
        let target = match kind {
            s if s == g::_SEEK_START => distance as i64,
            s if s == g::_SEEK_END => size + distance as i64,
            s if s == g::_SEEK_CURRENT => {
                if distance == 0 {
                    return open.pos;
                }
                open.pos as i64 + distance as i64
            }
            _ => {
                self.last_error = g::AEE_EBADPARM;
                return g::AEE_EFAILED;
            }
        };
        if !(0..=size).contains(&target) {
            self.last_error = AEEFS_EBADSEEKPOS;
            return g::AEE_EFAILED;
        }
        open.pos = target as u32;
        g::AEE_SUCCESS
    }

    /// `IFile::Truncate`. Answers the reference `bool`, where **TRUE means failure**, and leaves the
    /// file-system result in [`Volume::last_error`] instead of returning it — that is what
    /// `CFile_Truncate` does: it calls the system and then writes `*(a1[3] + 12) = v8`, the cell
    /// `CFileMgr_GetLastError` reads, so a *successful* truncate leaves **0** there rather than the
    /// previous failure. The return is `v8 != 0` and nothing else.
    ///
    /// Two steps of that body have no counterpart in this store, and are named rather than invented:
    ///
    /// - the cached-write branch, `if ( a1[32] != 0 ) { *(a1[3] + 12) = 266; return true; }`. That
    ///   word is the record's read-only view of the file (`FindConstFile` installs it at `+128`), and
    ///   266 = `0x100 + 0x0A` = `AEEFS_ERROR + 0x0A` = [`AEEFS_EINVALIDOPERATION`]. This host's
    ///   counterpart is a handle over an [`Entry::Reference`] name — the pack's own bytes, mounted read-only —
    ///   so the branch **is** reachable and is taken first, exactly as the body takes it before any
    ///   cache or quota work. What this store does not have is a cache still holding a *previous*
    ///   write, which is the other thing that word can mean on a handset.
    /// - `Mod_UpdateFileSpaceUsed(a1[23], len - info.dwSize)`, which adjusts the owning module's used
    ///   space by the delta. It needs a per-module counter; this store's quota is one number for the
    ///   whole volume ([`Volume::room`]), so there is nothing to adjust.
    pub fn truncate(&mut self, open: &mut Opened, len: u32) -> bool {
        let fs = self.truncate_handle(open, len);
        self.last_error = fs;
        fs != g::AEE_SUCCESS
    }

    /// The system call `CFile_Truncate` makes, as this store can make it: 0 is success, anything
    /// else is the reason kept in the error cell rather than the answer given to the pack.
    ///
    /// Its first branch is the const/ROM view, which [`Volume::write`] now answers the same way:
    /// `if ( a1[32] != 0 ) { *(a1[3] + 12) = 266; return true; }` — 266 is
    /// [`AEEFS_EINVALIDOPERATION`], and the `bool` this slot returns means "it did not happen".
    fn truncate_handle(&mut self, open: &mut Opened, len: u32) -> u32 {
        if matches!(self.entries.get(&open.name), Some(Entry::Shipped(_))) {
            return AEEFS_EINVALIDOPERATION;
        }
        if !open.writable {
            return g::AEE_EREADONLY;
        }
        let (old_len, tail) = {
            let Some(Entry::Written(data)) = self.entries.get_mut(&open.name) else {
                return g::AEE_EREADONLY;
            };
            let old_len = data.len();
            let tail = if (len as usize) < old_len {
                data[len as usize..].to_vec()
            } else {
                Vec::new()
            };
            data.truncate(len as usize);
            (old_len, tail)
        };
        if let Err(e) = self.mirror(&open.name) {
            if let Some(Entry::Written(data)) = self.entries.get_mut(&open.name) {
                data.truncate(old_len);
                if !tail.is_empty() {
                    data[len as usize..].copy_from_slice(&tail);
                }
            }
            return host_failure(&e);
        }
        g::AEE_SUCCESS
    }

    /// `IFILEMGR_Remove`. A pack file cannot be removed, because that would be a run editing the
    /// pack rather than its own data.
    pub fn remove(&mut self, name: &str) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        // A file somebody has said they are using is not a file to delete — that is what `Use`
        // exists to hold. The reason given is `AEE_EBADSTATE` ("not in the state to do that") rather
        // than a permission error: the caller is not forbidden, it is early. Like every other reason
        // in this family it goes into `GetLastError` — `fail` keeps the return value at
        // `AEE_EFAILED`, so a caller that reads only that sees a refusal while one that asks why
        // gets the difference.
        if self.in_use.contains_key(&n) {
            return self.fail(g::AEE_EBADSTATE);
        }
        match self.entries.get(&n) {
            Some(Entry::Written(_)) => {
                let Some(entry) = self.entries.remove(&n) else {
                    return self.fail(AEEFS_EFILENOEXISTS);
                };
                if let Err(e) = self.mirror(&n) {
                    self.entries.insert(n, entry);
                    return self.fail(host_failure(&e));
                }
                g::AEE_SUCCESS
            }
            Some(Entry::Shipped(_)) => self.fail(g::AEE_EREADONLY),
            // A directory is `RmDir`'s job, and the caller needs that distinction in `GetLastError`.
            Some(Entry::Dir) => self.fail(g::AEE_EBADPARM),
            None => self.fail(AEEFS_EFILENOEXISTS),
        }
    }

    pub fn mkdir(&mut self, name: &str) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        // the header: SUCCESS "if the file is successfully created or if file already exists", so an
        // existing directory is the common case rather than an error - and the root always exists.
        if n.is_empty() || self.entries.contains_key(&n) {
            return g::AEE_SUCCESS;
        }
        self.entries.insert(n.clone(), Entry::Dir);
        if let Err(e) = self.mirror(&n) {
            self.entries.remove(&n);
            return self.fail(host_failure(&e));
        }
        g::AEE_SUCCESS
    }

    /// `IFILEMGR_RmDir`, which the header says needs the directory empty first.
    pub fn rmdir(&mut self, name: &str) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        if !matches!(self.entries.get(&n), Some(Entry::Dir)) {
            return self.fail(AEEFS_EDIRNOEXISTS);
        }
        let prefix = format!("{n}/");
        if self.entries.keys().any(|k| k.starts_with(&prefix)) {
            return self.fail(g::AEE_EALREADY);
        }
        let Some(entry) = self.entries.remove(&n) else {
            return self.fail(AEEFS_EDIRNOEXISTS);
        };
        if let Err(e) = self.mirror(&n) {
            self.entries.insert(n, entry);
            return self.fail(host_failure(&e));
        }
        g::AEE_SUCCESS
    }

    pub fn rename(&mut self, from: &str, to: &str) -> u32 {
        let (Some(a), Some(b)) = (self.resolve(from), self.resolve(to)) else {
            return self.refused();
        };
        let moved = match self.entries.get(&a) {
            Some(Entry::Written(bytes)) => Some(Entry::Written(bytes.clone())),
            Some(Entry::Shipped(_)) => return self.fail(g::AEE_EREADONLY),
            Some(Entry::Dir) => None,
            None => return self.fail(AEEFS_EFILENOEXISTS),
        };
        // A directory rename would have to move everything below it, which the name space here can
        // express but nothing on disk has needed yet, so it is refused rather than half done.
        let Some(moved) = moved else {
            return self.fail(g::AEE_EBADPARM);
        };
        // Landing on a pack file would overwrite the pack through a side door that `OpenFile`
        // and `Remove` both refuse, so the target gets the same check.
        if matches!(self.entries.get(&b), Some(Entry::Shipped(_))) {
            return self.fail(g::AEE_EREADONLY);
        }
        let old_b = self.entries.insert(b.clone(), moved);
        let old_a = self.entries.remove(&a);
        let result = self.mirror(&b).and_then(|()| self.mirror(&a));
        if let Err(e) = result {
            // Put the name space back before reporting: a host rename that only half happened must
            // not leave the volume pointing at a file the disk does not have.
            self.entries.remove(&b);
            self.entries.remove(&a);
            if let Some(entry) = old_a {
                self.entries.insert(a.clone(), entry);
            }
            if let Some(entry) = old_b {
                self.entries.insert(b.clone(), entry);
            }
            let _ = self.mirror(&b);
            let _ = self.mirror(&a);
            return self.fail(host_failure(&e));
        }
        g::AEE_SUCCESS
    }

    /// `IFILEMGR_EnumInit`: take the list now, so a file created mid-enumeration does not shift what
    /// the app is walking past. `bDirs` asks for directories instead of files. A name whose basename
    /// begins with `*.` is a directory plus an extension, and the extension is kept for `EnumNext`.
    pub fn enum_init(&mut self, dir: &str, dirs: bool) -> u32 {
        let stripped = dir.trim_start_matches(['/', '\\']);
        let (head, base) = split_basename(stripped);
        let pattern = match base.strip_prefix('*') {
            Some(ext) if ext.starts_with('.') => Some(ext.to_owned()),
            _ => None,
        };
        let (walk, prefix_text) = match &pattern {
            Some(_) => (head.as_str(), head.clone()),
            None => (dir, stripped.to_owned()),
        };
        let Some(n) = self.resolve(walk) else {
            self.listing.clear();
            self.at = 0;
            self.enum_dir.clear();
            self.enum_pattern = None;
            return self.refused();
        };
        self.enum_pattern = pattern;
        self.enum_dir = if n.is_empty() {
            String::new()
        } else {
            prefix_text
        };
        let prefix = if n.is_empty() {
            String::new()
        } else {
            format!("{n}/")
        };
        self.listing = self
            .entries
            .iter()
            .filter(|(k, e)| {
                let Some(tail) = k.strip_prefix(&prefix) else {
                    return false;
                };
                // One level only: a file the app wrote as `sub/deep/x` belongs to `sub`'s listing,
                // not to this one.
                !tail.is_empty() && !tail.contains('/') && matches!(e, Entry::Dir) == dirs
            })
            .map(|(k, _)| (k.clone(), k[prefix.len()..].to_owned()))
            .collect();
        self.at = 0;
        g::AEE_SUCCESS
    }

    /// `IFILEMGR_EnumNext`: the next entry, or `None` once the list is spent — which the caller
    /// answers as `AEE_ENOMORE`.
    pub fn enum_next(&mut self) -> Option<Info> {
        // The listing was frozen by `EnumInit`, so a name the app removed since then is simply not
        // there any more. Stopping there instead would end the walk early and hide the files after
        // it, which an app would read as a shorter directory.
        while self.at < self.listing.len() {
            let (key, leaf) = self.listing[self.at].clone();
            self.at += 1;
            if let Some(pattern) = &self.enum_pattern {
                if !leaf.rfind('.').is_some_and(|i| &leaf[i..] == pattern) {
                    continue;
                }
            }
            if let Some(mut info) = self.info_of(&key) {
                // `aee_makepath(dir, leaf)`: the directory the caller enumerated, a separator only when
                // the stored text does not already end in one, and a leading separator on the leaf
                // skipped rather than doubled.
                info.name = make_path(&self.enum_dir, &leaf);
                return Some(info);
            }
        }
        None
    }

    /// `IFILEMGR_CheckPathAccess(IFileMgr*, const char *cpszIn, uint32 dwDesiredRights,
    /// uint32 *pdwActualRights)` — which of the rights the caller named this volume can grant.
    ///
    /// The bits are `AEEFP_*` (the header: `READ`=1, `WRITE`=2, `CREATE`=4,
    /// `DELETE`=8). The answer is not what this host feels like doing but what the volume *is*:
    /// what the pack provides a read-only copy taken at mount time (`Entry::Reference`), and only
    /// what the app wrote this run is `Entry::Written`. A name outside `AEEFS_HOME_DIR` does not
    /// belong to this volume at all, and that is a different answer from "not permitted" — the two
    /// stay separate here for the same reason `Resolved` keeps them apart.
    pub fn access(&mut self, name: &str, desired: u32) -> (u32, u32) {
        let Some(n) = self.resolve(name) else {
            // `resolve` has already recorded which of the two it was.
            return (g::AEEFP_NONE, g::AEE_EFAILED);
        };
        let root = n.is_empty();
        let entry = self.entries.get(&n);
        let mut rights = g::AEEFP_READ;
        if entry.is_some_and(|e| matches!(e, Entry::Written(_))) {
            rights |= g::AEEFP_WRITE | g::AEEFP_DELETE;
        }
        // Creating here means putting something inside: the root always can, and so can a
        // directory that exists. `MkDir`/`RmDir` answer for themselves further down.
        if root || entry.is_some_and(|e| matches!(e, Entry::Dir)) {
            rights |= g::AEEFP_CREATE;
        }
        let granted = desired & !rights == 0;
        (
            rights,
            if granted {
                g::AEE_SUCCESS
            } else {
                g::AEE_EPRIVLEVEL
            },
        )
    }

    /// The two *used* readings `IFILEMGR_GetFileUseInfo` has to fill, as `(files, bytes)`.
    ///
    /// The header defines `AEEFileUseInfo` as a **quota**:
    /// `wFilesUsed`/`dwSpaceUsed` are what this module has used, `wMaxFiles`/`dwMaxSpace` what it
    /// is allowed to use. This machine has no EFS quota policy, so the two *used* numbers are real
    /// readings (the non-directory entries of this volume and their bytes); of the two ceilings,
    /// only the byte one is a ceiling this store actually enforces, and the file-count question is
    /// reported the way the delivered body spells "no quota" — its `0xFFFF` sentinel.
    pub fn use_info(&self) -> (u32, u64) {
        let mut files = 0u32;
        let mut bytes = 0u64;
        for entry in self.entries.values() {
            if !matches!(entry, Entry::Dir) {
                files += 1;
                bytes += entry.len() as u64;
            }
        }
        (files, bytes)
    }

    /// `IFILEMGR_SetDescription(IFileMgr*, const char *pszName, AECHAR *pszDesc)` — remember a
    /// file's description. `None` is "forget it", which is what
    /// `IFILEMGR_UnUse`'s sibling does for a null description.
    ///
    /// Only a file that is here can be described: the header's answer for a name the volume does not
    /// have is the file system's own [`AEEFS_EFILENOEXISTS`], read back through `GetLastError`.
    pub fn set_description(&mut self, name: &str, desc: Option<Vec<u16>>) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        if !matches!(
            self.entries.get(&n),
            Some(Entry::Written(_)) | Some(Entry::Shipped(_))
        ) {
            return self.fail(AEEFS_EFILENOEXISTS);
        }
        match desc {
            Some(units) => {
                self.descriptions.insert(n, units);
            }
            None => {
                self.descriptions.remove(&n);
            }
        }
        g::AEE_SUCCESS
    }

    /// The description `set_description` kept, for a name already normalised. `GetInfoEx` is the
    /// only reader: the header's struct is where a description comes back out.
    pub fn description(&self, name: &str) -> Option<&[u16]> {
        self.descriptions.get(name).map(|units| units.as_slice())
    }

    /// `IFILEMGR_Use(IFileMgr*, const char *pszName, boolean bUse)` — mark a file as being used by
    /// this module, or release it (the header; `IFILEMGR_UnUse` is the same call with
    /// `FALSE`). What it buys is a refusal: `Remove` will not delete a file somebody has said they
    /// are using.
    ///
    /// ⚠ The header does not say whether `Use` is what makes a file count in
    /// `AEEFileUseInfo::wFilesUsed` — it defines that field only as "Number of Files currenty used by
    /// this Module". This host has no second record for that, so `GetFileUseInfo`
    /// keeps answering with what the volume holds, and `Use` is used for the `Remove` guard alone.
    pub fn use_file(&mut self, name: &str, on: bool) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        if !self.entries.contains_key(&n) {
            return self.fail(AEEFS_EFILENOEXISTS);
        }
        let holders = self.in_use.entry(n.clone()).or_insert(0);
        if on {
            *holders += 1;
        } else {
            *holders = holders.saturating_sub(1);
        }
        if *holders == 0 {
            self.in_use.remove(&n);
        }
        g::AEE_SUCCESS
    }

    /// `IFileSystem::Truncate(po, pszPath, nLength)` — the same job as `IFile::Truncate`, but the
    /// file is named rather than held.
    ///
    /// The two differ in exactly one place: what "may I write this" is asked of. A handle carries
    /// the flag `OpenFile` decided from its mode; here there is no handle, so it is asked of the
    /// entry itself — only what the app wrote this run is writable.
    pub fn truncate_at(&mut self, name: &str, len: u32) -> u32 {
        let Some(n) = self.resolve(name) else {
            return self.refused();
        };
        let (old_len, tail) = match self.entries.get_mut(&n) {
            Some(Entry::Written(data)) => {
                let old_len = data.len();
                let tail = if (len as usize) < old_len {
                    data[len as usize..].to_vec()
                } else {
                    Vec::new()
                };
                data.truncate(len as usize);
                (old_len, tail)
            }
            Some(Entry::Shipped(_)) => return self.fail(g::AEE_EREADONLY),
            Some(Entry::Dir) => return self.fail(g::AEE_EBADPARM),
            None => return self.fail(AEEFS_EFILENOEXISTS),
        };
        if let Err(e) = self.mirror(&n) {
            if let Some(Entry::Written(data)) = self.entries.get_mut(&n) {
                data.truncate(old_len);
                if !tail.is_empty() {
                    data[len as usize..].copy_from_slice(&tail);
                }
            }
            return self.fail(host_failure(&e));
        }
        g::AEE_SUCCESS
    }

    /// `FSStat::dwMode` for one name — the `FS_S_IFMT` family, a table of its own.
    ///
    /// ⚠ It is a **different family** from `Entry::attrib`'s `FA_*` (`AEEFileAttrib`, which is what
    /// `AEEFileInfo::attrib` wants). One entry, two tables, and they are not interchangeable: a pack
    /// that reads `dwMode` and is handed an `FA_*` value sees a directory as a plain file, and one
    /// that reads `attrib` and is handed an `FS_S_IF*` value sees a read-only file as something else.
    pub fn mode_bits(&mut self, name: &str) -> Option<u32> {
        let n = self.resolve(name)?;
        let entry = self.entries.get(&n)?;
        Some(match entry {
            // A directory is readable and writable; `Stat` on the root answers the same, which is
            // why `resolve("")` giving an empty name is not special-cased out.
            Entry::Dir => g::FS_S_IFDIR | g::FS_R_OK | g::FS_W_OK,
            // What the pack provides read-only here for the same reason `Entry::attrib` says
            // `AEE_FA_CONST`: it is the card, and the app's writes go elsewhere.
            Entry::Shipped(_) => g::FS_S_IFREG | g::FS_S_IFRDONLY | g::FS_R_OK,
            Entry::Written(_) => g::FS_S_IFREG | g::FS_R_OK | g::FS_W_OK,
        })
    }

    /// How many bytes this volume holds, for `FSStatVFS`: the same reading `GetFileUseInfo` and
    /// `GetFreeSpace` answer from, so a pack that asks twice gets one number.
    pub fn bytes_held(&self) -> u64 {
        self.entries.values().map(|e| e.len() as u64).sum()
    }

    /// Record a reason the volume did not itself discover, for the caller that refused a request
    /// before reaching here. `GetLastError` is the only channel any of these slots have.
    pub fn refuse(&mut self, why: u32) {
        self.last_error = why;
    }

    /// The success half of the same store. `CFile_GetInfo`, `CFile_GetInfoEx`, `CFileMgr_GetInfo` and
    /// `CFileMgr_GetInfoEx` all end with `*(a1[3] + 12) = status; return status != 0;`, so a slot that
    /// answered FALSE also **wrote 0 into the cell** — the previous refusal is gone. A body that reaches
    /// the file system has that store done by the call; this store does it here, and says so.
    pub fn accept(&mut self) {
        self.last_error = 0;
    }

    pub fn last_error(&self) -> u32 {
        self.last_error
    }

    /// Every file this run has *written*, as one block of bytes.
    ///
    /// This is the shape a front end needs when its save slot is one buffer rather than a directory: a
    /// libretro core hands its front end `retro_get_memory_data` and the front end writes those bytes
    /// out as a `.srm`. Only *written* entries are in it — a pack file is the pack's own and comes
    /// back from the pack — and directories are not, because an empty directory is not worth a name in
    /// a format that has to stay parseable.
    ///
    /// Layout, little-endian throughout: `"QSVR"`, u16 version, u16 count, then per entry a u16 name
    /// length, that many bytes of name (the normalised names `IFileMgr` takes), a u32 data length and
    /// the data. The lengths are what make a short block detectable: a count promising more bytes than
    /// are there is refused rather than read as far as it goes.
    pub fn written_block(&self) -> Vec<u8> {
        let written: Vec<(&str, &[u8])> = self
            .entries
            .iter()
            .filter_map(|(name, entry)| match entry {
                Entry::Written(bytes) => Some((name.as_str(), bytes.as_slice())),
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        out.extend_from_slice(&BLOCK_MAGIC);
        out.extend_from_slice(&BLOCK_VERSION.to_le_bytes());
        out.extend_from_slice(&(written.len() as u16).to_le_bytes());
        for (name, bytes) in written {
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
        }
        out
    }

    /// Put back the files in a block made by [`Volume::written_block`], returning how many were
    /// adopted.
    ///
    /// A block that is not one of ours, or one that is short, is refused **whole**: the parse happens
    /// into a list first and nothing is touched until every entry has been read, because a
    /// half-adopted save is a run starting from a state no handset ever produced.
    ///
    /// Names go through the same [`normalize`] every `IFileMgr` call does, so a block carrying `../`
    /// or an absolute path is refused like a malformed one. A block arrives from outside the run — a
    /// `.srm` picked up from anywhere — and a save file that writes outside the app's own directory is
    /// what this volume exists to stop.
    ///
    /// What arrives here **wins over** what `mount_beside`/`persist_to` put in the volume, and that
    /// order is deliberate: this block is the newer of the two by construction, because a front end
    /// writes it out at the end of a session.
    pub fn adopt_block(&mut self, block: &[u8]) -> Result<usize, String> {
        let mut at = 0usize;
        let magic =
            read_bytes(block, &mut at, 4).ok_or("a save block shorter than its own magic")?;
        if magic != BLOCK_MAGIC {
            return Err("not a save block this host wrote".into());
        }
        let version = read_u16(block, &mut at).ok_or("a save block with no version")?;
        if version != BLOCK_VERSION {
            return Err(format!(
                "a save block of version {version}, and this host writes {BLOCK_VERSION}"
            ));
        }
        let count = read_u16(block, &mut at).ok_or("a save block with no entry count")?;
        let mut files: Vec<(String, Vec<u8>)> = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let len = read_u16(block, &mut at).ok_or("an entry with no name length")? as usize;
            let name = read_bytes(block, &mut at, len).ok_or("a truncated file name")?;
            let name = std::str::from_utf8(name).map_err(|_| "a file name that is not UTF-8")?;
            let name = match normalize(name) {
                Resolved::Here(name) if !name.is_empty() => name,
                _ => return Err(format!("a name no app could own: {name:?}")),
            };
            let len = read_u32(block, &mut at).ok_or("an entry with no data length")? as usize;
            let data = read_bytes(block, &mut at, len).ok_or("a truncated file")?;
            files.push((name, data.to_vec()));
        }
        let adopted = files.len();
        for (name, data) in files {
            self.entries.insert(name, Entry::Written(data));
        }
        Ok(adopted)
    }
}

/// The magic that opens a block [`Volume::written_block`] wrote, so that a file which is not one is
/// refused rather than read as an empty save.
const BLOCK_MAGIC: [u8; 4] = *b"QSVR";
/// The layout's version. A block saying anything else is refused: reading a newer layout as this one
/// is how a save comes back as nonsense.
const BLOCK_VERSION: u16 = 1;

/// Take `n` bytes at `at`, moving it past them, or `None` when the block ends first.
fn read_bytes<'a>(block: &'a [u8], at: &mut usize, n: usize) -> Option<&'a [u8]> {
    let end = at.checked_add(n)?;
    let slice = block.get(*at..end)?;
    *at = end;
    Some(slice)
}

fn read_u16(block: &[u8], at: &mut usize) -> Option<u16> {
    let bytes = read_bytes(block, at, 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(block: &[u8], at: &mut usize) -> Option<u32> {
    let bytes = read_bytes(block, at, 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
#[path = "test/file.rs"]
mod tests;
