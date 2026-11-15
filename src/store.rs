//! The resources a run was given, looked up the way a handset looks them up.
//!
//! A shipped module names resources by file and id and never by the container's internal kind
//! ranges, so a lookup here is by id first. That leniency is deliberate and bounded: the kind a
//! container records is what its author wrote, and a module asking `ISHELL_LoadResString` for an id
//! its own manifest files as an image is a question about *the id*, not about our taxonomy.

use std::path::{Path, PathBuf};

use crate::file::VOLUME_LIMIT;
use crate::par::{self, ParResource};
use crate::resource::{self, Container, RESTYPE_STRING};

/// The directory a module's siblings live in.
///
/// `Path::new("tank.mod").parent()` is `Some("")`, and an empty path is not a directory: every
/// sibling lookup then silently finds nothing. A bare file name means "the current directory", so
/// the module's `.bar`/`.mif`/`.par` files are mounted exactly as they are when the caller passes
/// an absolute path.
pub(crate) fn dir_of(mod_path: &Path) -> PathBuf {
    match mod_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

/// A file this much bigger is not a pack's resources, and reading it whole to find that out is not
/// worth the run.
pub const CONTAINER_LIMIT: u64 = VOLUME_LIMIT as u64;

/// One resource out of a mounted container, copied so the caller owns it.
///
/// [`Store::find`] hands the same bytes to one lookup; this exists for the caller that has to walk
/// **everything** — a probe describing a pack — where tying the answer to the store's lifetime would
/// buy nothing.
#[derive(Debug)]
pub struct Record {
    /// The table the container filed it under. Not a type: see [`Store::inventory`]'s note on `mime`.
    pub kind: u16,
    pub id: u32,
    /// The declared type out of the container's property block. `None` for a `.par` leaf, which stores
    /// no block at all, and for a block that names nothing.
    pub mime: Option<String>,
    pub body: Vec<u8>,
}

/// One resource, copied out of its container so the caller owns it.
#[derive(Debug)]
pub struct Found {
    /// The kind the container filed it under.
    pub kind: u16,
    /// The file it came from, for a run to report.
    pub from: String,
    /// Payload after the property block.
    pub body: Vec<u8>,
    /// The property block itself, which is where the payload's own declared type lives — see
    /// [`resource::mime`]. The kind above is only the table the container filed it under, and the
    /// corpus files MIDI and text under the image kind.
    pub props: Vec<u8>,
    /// The resource as the container stores it, property block included. `ISHELL_LoadResData`
    /// answers with these: the app is handed the bytes and skips the block itself.
    pub stored: Vec<u8>,
}

/// Every container a run was pointed at.
#[derive(Default)]
pub struct Store {
    files: Vec<(String, Vec<u8>)>,
    /// The `"PAR "` packs, already inflated and kept apart from `files`.
    ///
    /// They cannot share a list: a `.bar`'s resources are borrowed slices of the file, while a
    /// `.par` stores its payloads deflated, so reading one produces owned bytes and there is
    /// nothing left to point at. Mounting inflates them once, where the run is already doing file
    /// I/O, rather than on every lookup.
    packs: Vec<(String, Vec<ParResource>)>,
    /// The container the app's *own* resources live in: the `<appid>.mif`/`.bar` — named for the
    /// directory the module was expanded into, or, in the flat layout the system applets use, for the
    /// module itself. A lookup that names no file is a question about this one, and every pack also
    /// mounts a sibling art `.bar` whose id 7 is a different text entirely.
    own: Option<String>,
}

impl Store {
    /// Read every container beside `dir`, then work out which container is the app's *own* manifest —
    /// the one above the module's directory in the game layout, or the same-named one beside it in the
    /// flat layout the system applets use.
    ///
    /// The extension decides nothing: a pack's pictures are in a `.bar` and its manifest strings in a
    /// `.mif`, and the two are the same container, so every sibling file is offered to
    /// [`Container::parse`] and the ones that answer are mounted. A file that is not a container is
    /// skipped, not fatal: a pack carries blobs alongside its resources.
    pub fn mount_beside(&mut self, mod_path: &Path) -> Vec<String> {
        let mut mounted = Vec::new();
        let dir = dir_of(mod_path);
        let Ok(read) = std::fs::read_dir(&dir) else {
            return mounted;
        };
        for entry in read.flatten() {
            self.mount_file(&entry.path(), &mut mounted);
        }
        // The app-info manifest is named for the directory the module was expanded into and sits one
        // level *above* it — `<appid>.mif` beside the `<appid>/` that holds the `.mod` — which is how
        // all ten builds in this corpus are laid out. It is the file the handset answers
        // `ISHELL_LoadResString(NULL, 8, …)` from, the version every AppGen applet asks for as it is
        // created, so a store that only looks beside the `.mod` cannot serve it for any pack.
        //
        // Named, not enumerated: the directory above is wherever a person put the game, and reading
        // everything in it would mount another title's art as this one's resources.
        if let (Some(parent), Some(name)) = (dir.parent(), dir.file_name()) {
            for extension in ["mif", "bar"] {
                let mut manifest = std::ffi::OsString::from(name);
                manifest.push(".");
                manifest.push(extension);
                let path = parent.join(manifest);
                let before = mounted.len();
                self.mount_file(&path, &mut mounted);
                // The first manifest that is a container at all is the app's own, and `.mif` is
                // offered before `.bar` because that is the order the handset's own registry reads
                // them in.
                if self.own.is_none() && mounted.len() > before {
                    self.own = mounted.get(before).cloned();
                }
            }
        }
        // The other layout the collection ships: the manifest **beside** the module, named for the
        // module itself — `app.mod` next to `app.mif`, which is
        // how every one of the 257 system applets is laid out. Those files were already offered to the
        // readers above, so what this decides is only which mounted container is the app's *own*.
        //
        // Still by name, not by taking the first thing in the directory: a flat app folder also holds
        // thirteen language `.bar`s, a `.sig`, the app's own saved `.ini`, and loose `.png` icons, and
        // serving one of those as "the app's manifest" is how a pack ends up comparing another title's
        // copyright line against its own.
        if self.own.is_none() {
            let Some(stem) = mod_path.file_stem() else {
                return mounted;
            };
            for extension in ["mif", "bar"] {
                let mut name = stem.to_os_string();
                name.push(".");
                name.push(extension);
                // Compared case-insensitively against what mounted, because the ancestor rule above
                // asks the filesystem instead — and on this host that means `MUSICPLAYER.MIF` would be
                // found there and missed here if this were an exact match. `candidates()` already
                // compares mounted names this way. The name kept is the one that mounted, so the
                // report quotes the file as it is spelled on disk.
                let want = name.to_string_lossy().into_owned();
                if let Some(found) = mounted
                    .iter()
                    .find(|mounted| mounted.eq_ignore_ascii_case(&want))
                {
                    self.own = Some(found.clone());
                    break;
                }
            }
        }
        mounted
    }

    /// Offer one file to both container readers, mounting it under its own name if either answers.
    fn mount_file(&mut self, path: &Path, mounted: &mut Vec<String>) {
        if !path.is_file() {
            return;
        }
        // Reading a file to find out whether it is a container is only worth it below the cap:
        // the parse needs the whole file, because its own table has to add up to its length.
        if path
            .metadata()
            .map(|m| m.len() > CONTAINER_LIMIT)
            .unwrap_or(true)
        {
            return;
        }
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_owned();
        if Container::parse(&bytes).is_ok() {
            mounted.push(name.clone());
            self.files.push((name, bytes));
            return;
        }
        // The other container a pack ships its art in, which the `0x11` reader above never accepts
        // because the two formats are unrelated. A file that answers neither reader is not a container
        // at all, and that is normal: a pack carries blobs next to its resources — save files, an
        // index, its own compressed code.
        if par::is_par(&bytes) {
            if let Ok(resources) = par::read(&bytes) {
                if !resources.is_empty() {
                    mounted.push(name.clone());
                    self.packs.push((name, resources));
                }
            }
        }
    }

    /// The containers a lookup may answer from, in the order it should try them.
    ///
    /// A name in the call is part of the question: `ISHELL_LoadResString(p, "brewapp.bar", 7, …)` is
    /// not asking about `35345.mif`, and searching every mounted file for the first hit answers with
    /// another container's text. A pack that compares that text against its own copyright line then
    /// concludes it is a pirated copy and closes — which is what `35345` does.
    ///
    /// A name that matches nothing mounted is *not* treated as "absent", because a file over the
    /// mount cap or under a spelling this reader does not know would then answer as a missing
    /// resource; the search falls back to the ordinary order and the miss is still recorded by the
    /// caller. With no name at all, the app's own manifest goes first and the rest only answer for
    /// an id it does not hold.
    fn candidates<'a, T>(
        &'a self,
        named: Option<&str>,
        list: &'a [(String, T)],
    ) -> Vec<&'a (String, T)> {
        let matches = |name: &str| {
            named.is_some_and(|want| {
                let base = want.rsplit(['/', '\\']).next().unwrap_or(want);
                name.eq_ignore_ascii_case(base)
            })
        };
        let mut out: Vec<&(String, T)> = match named {
            Some(_) if list.iter().any(|(name, _)| matches(name)) => {
                list.iter().filter(|(name, _)| matches(name)).collect()
            }
            _ => list.iter().collect(),
        };
        if named.is_none() {
            if let Some(own) = &self.own {
                out.sort_by_key(|(name, _)| usize::from(!name.eq_ignore_ascii_case(own)));
            }
        }
        out
    }

    /// Everything mounted, in mount order: `(file name, its records)`.
    ///
    /// Built from what [`Store::mount_file`] accepted, so a caller listing this describes the same set
    /// a lookup can answer from — no second policy about which sibling file counts. A record's `kind`
    /// is only the table it was filed under, and the corpus files MIDI and text under the image kind,
    /// which is why [`Record::mime`] travels with it.
    pub fn inventory(&self) -> Vec<(String, Vec<Record>)> {
        let mut out = Vec::new();
        for (name, bytes) in &self.files {
            let Ok(c) = Container::parse(bytes) else {
                continue;
            };
            let records = c
                .all()
                .unwrap_or_default()
                .into_iter()
                .map(|r| Record {
                    kind: r.kind,
                    id: u32::from(r.id),
                    mime: resource::mime(r.props).map(str::to_owned),
                    body: r.body.to_vec(),
                })
                .collect();
            out.push((name.clone(), records));
        }
        for (name, resources) in &self.packs {
            out.push((
                name.clone(),
                resources
                    .iter()
                    // A `.par` leaf carries no property block, so its type has to come from its bytes
                    // and this reports it as unnamed rather than inventing a MIME for it.
                    .map(|r| Record {
                        kind: r.kind,
                        id: r.id,
                        mime: None,
                        body: r.payload.clone(),
                    })
                    .collect(),
            ));
        }
        out
    }

    /// The manifest the app's own records — `AEEAppInfo`, its title, its version — are read from.
    pub fn manifest(&self) -> Option<String> {
        self.own.clone()
    }

    /// The resource of this kind and id, in the order [`Store::candidates`] says to try.
    pub fn find(&self, named: Option<&str>, kind: u16, id: u16) -> Option<Found> {
        for (name, bytes) in self.candidates(named, &self.files) {
            let Ok(c) = Container::parse(bytes) else {
                continue;
            };
            // Look the id up rather than expanding the whole container to find it. `all()` walks every
            // map entry and then every id each one covers, which on a container with a wide `extra` is
            // orders of magnitude more work than the single record being asked for — and this ran per
            // `LoadResData`, `LoadResString` and `LoadResObject`, so a pack that loads an image per
            // level paid it every time. A record that cannot be framed is the same "not here" as an id
            // the container does not carry, which is the tolerance `all()` had by skipping it.
            let Ok(Some(r)) = c.find(kind, id) else {
                continue;
            };
            return Some(Found {
                kind: r.kind,
                from: name.clone(),
                body: r.body.to_vec(),
                props: r.props.to_vec(),
                stored: r.stored.to_vec(),
            });
        }
        for (name, resources) in self.candidates(named, &self.packs) {
            if let Some(r) = resources
                .iter()
                .find(|r| r.kind == kind && r.id == u32::from(id))
            {
                return Some(Found {
                    kind: r.kind,
                    from: name.clone(),
                    body: r.payload.clone(),
                    // A `.par` writes no property block: the payload stands alone, so there is no
                    // declared type to report, and `LoadResData` is handed the payload itself.
                    props: Vec::new(),
                    stored: r.payload.clone(),
                });
            }
        }
        None
    }

    /// The app's own `AEEAppInfo`, which its manifest carries as a `RESTYPE_BINARY` record of id 2.
    ///
    /// This is the record a handset's shell reads to know what applet is installed at all: the class
    /// id, the resource id its title and icon sit above, and its flags. Every manifest in this corpus
    /// holds one, and for one pack — whose `.mod` is 568 bytes of loader for a compressed image it
    /// inflates into the heap — the class id exists *nowhere else*, so a host that does not read
    /// this file cannot know what the pack is.
    ///
    /// Returned as stored, without checking its length: whether twenty bytes is the shape this host
    /// asked for is a question about `AEEAppInfo`, and the ABI tables are where that is known.
    pub fn applet_info(&self) -> Option<Vec<u8>> {
        let own = self.own.as_deref()?;
        self.find(Some(own), resource::RESTYPE_BINARY as u16, 2)
            .map(|found| found.body)
    }

    /// The characters a module expects back from `ISHELL_LoadResString`.
    ///
    /// Only a `RESTYPE_STRING` answers that question. Handing back an image's bytes as text was tried
    /// and is worse than a miss: the caller cannot tell a wall of glyph bytes from its own string, and
    /// the app then continues in a state that never existed on a handset.
    pub fn string(&self, named: Option<&str>, id: u16) -> Option<Vec<u16>> {
        let found = self.find(named, RESTYPE_STRING, id)?;
        let mut languages = resource::strings(&found.body).ok()?;
        let mut first = languages.drain(..).next()?;
        if first.iter().all(|c| *c != 0) {
            first.push(0);
        }
        Some(first)
    }
}

#[cfg(test)]
#[path = "test/store.rs"]
mod tests;
