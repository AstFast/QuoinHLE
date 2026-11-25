//! The `AEEHelperFuncs` table: BREW's libc, one `SVC` per function pointer.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    pub(crate) fn proc_method(&mut self, id: u32) -> Result<u32, Error> {
        let slot = abi::slot_of_method_id(id) as usize;
        let name = crate::gl::PROC_NAMES.get(slot).copied().unwrap_or("");
        let argc = if name.starts_with("glDrawTex") || name == "glFramebufferTexture2DOES" {
            5
        } else {
            4
        };
        let mut args = [0u32; 5];
        for (i, arg) in args.iter_mut().take(argc).enumerate() {
            *arg = self.arg(i).map_err(Error::Hle)?;
        }
        let answer = if let Some(gl_name) = name.strip_prefix("gl") {
            self.gl
                .method(
                    "IGL",
                    gl_name,
                    &args[..argc],
                    &mut self.cpu.mem,
                    &mut self.egl,
                )
                .map_err(Error::Hle)?
        } else {
            self.egl
                .proc_call(name, &args[..argc])
                .map_err(Error::Hle)?
        };
        self.apply_egl_texture_bind();
        Ok(answer)
    }

    /// One `AEEHelperFuncs` slot. This is the module's libc: `MALLOC`, `MEMSET`,
    /// `AEESTRLEN` and the rest of the 117 pointers all arrive here.
    pub(crate) fn helper(&mut self, id: u32, label: &str) -> Result<u32, Error> {
        let name = abi::helper_field(id).map(|f| f.name).unwrap_or("");
        let this = self.cpu.r[0];
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            "SetupNativeImage" => {
                // `CONVERTBMP(cls, pBuffer, pii, pbRealloc)`, from the header. `AEEHelperFuncs` is a
                // plain function table rather than a COM object, so the first argument is in `r0`.
                // The header settles what the answer is: from BREW 1.2 on this "returns an IBitmap
                // interface pointer", which is the same object `ISHELL_LoadResObject` builds for a
                // picture, and the app hands it to `IDISPLAY_BitBlt`.
                let (cls, buf, info, prealloc) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                if cls != g::AEECLSID_WINBMP {
                    self.display.note(format!(
                        "SetupNativeImage for class {cls:#010x}: only AEECLSID_WINBMP is decoded here"
                    ));
                    return Ok(0);
                }
                // No length travels with the pointer, so the header the buffer carries is the only
                // answer, and it is capped by how much of that buffer is actually mapped. A container
                // may file an id with a property block and no picture behind it — many image records
                // are exactly that, twelve bytes of `image/bmp` and nothing after — and
                // then whether the read ran off the end or landed in a neighbouring block's slack was a
                // fact about where this host had put the twelve bytes, not about the pack. Short of a
                // header, `file_size` says so and the note below is the answer, which is the same
                // outcome a handset reaches when its own heap gives it garbage to look at.
                let room = self.cpu.mem.mapped_run(buf, BMP_HEADER as u32) as usize;
                let head = self.cpu.mem.read_bytes(buf, room)?;
                let preview = || {
                    head.iter()
                        .take(12)
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let Some(total) = resource::file_size(&head) else {
                    self.display.note(format!(
                        "SetupNativeImage: {:x} is not a BMP this host can size ({} bytes: {})",
                        buf,
                        head.len(),
                        preview()
                    ));
                    return Ok(0);
                };
                // The size the header claims, checked against the bytes that are there: a header that
                // describes a picture longer than its own buffer is the same lie whether it was told
                // by a truncated record or by a random one, and the second case is why this is not
                // trusted.
                let whole = self.cpu.mem.mapped_run(buf, total as u32) as usize;
                if whole < total {
                    self.display.note(format!(
                        "SetupNativeImage: {:x} claims a {total}-byte BMP with {whole} mapped \
                         byte(s) behind it",
                        buf
                    ));
                    return Ok(0);
                }
                let bytes = self.cpu.mem.read_bytes(buf, total)?;
                let Ok(img) = resource::bmp(&bytes) else {
                    self.display.note(format!(
                        "SetupNativeImage: a {total}-byte BMP in a shape this host cannot decode \
                         ({} bytes: {})",
                        head.len(),
                        preview()
                    ));
                    return Ok(0);
                };
                let colors = img.palette.as_ref().map_or(0, |p| p.len() / 3);
                if img.height <= 1 {
                    // A strip is legal, but it is also what a misread BMP header looks like. The
                    // fault context names the bitmap; this names the decode, and the header words
                    // beside it say whether the file itself claimed one row.
                    let word32 = |at: usize| {
                        head.get(at..at + 4)
                            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    };
                    let word16 = |at: usize| {
                        head.get(at..at + 2)
                            .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    };
                    self.display.note(format!(
                        "SetupNativeImage decoded a {}x{} BMP at {buf:#010x} (header {}x{}, {} bpp, \
                         compression {})",
                        img.width,
                        img.height,
                        word32(18).unwrap_or(0),
                        word32(22).unwrap_or(0),
                        word16(28).unwrap_or(0),
                        word32(30).unwrap_or(0)
                    ));
                }
                let obj = self.raster(&img)?;
                self.put_image_info(info, img.width, img.height, colors, &ImageParm::default())?;
                if prealloc != 0 {
                    // TRUE: the returned buffer was allocated separately, which it was - a handset
                    // that could convert in place would answer FALSE. The app then releases it with
                    // `SYSFREE`, which this arena cannot honour because it never gives memory back.
                    self.cpu.mem.write_u8(prealloc, g::TRUE as u8)?;
                }
                if self.device.ktf && self.oem_native_images && img.height == 1 {
                    if img.palette.is_some() {
                        let bmp = self.display.new_bitmap(
                            &mut self.cpu.mem,
                            &mut self.hle,
                            img.width as u16,
                            1,
                            8,
                        )?;
                        if let Some(b) = self.display.bitmap(bmp) {
                            for (i, ix) in img.pixels.iter().enumerate() {
                                self.cpu.mem.write_u8(b.px + i as u32, *ix)?;
                            }
                            return Ok(b.px);
                        }
                    }
                }
                Ok(obj)
            }
            "malloc" => {
                // Through `heap_alloc`, so the documented `ALLOC_NO_ZMEM` flag inside the size
                // argument is split off here too — this is the entry point packs actually OR it into.
                let request = self.arg(0)?;
                self.heap_alloc(request).map_err(Error::from)
            }
            // `REALLOC(p, size)` is the helper twin of `IHeap::Realloc`, and both are `heap_realloc`.
            // A pack may be reference built against a debug heap and ask for this one directly, which is
            // how a pack ends up wanting both spellings of the same call.
            "realloc" => {
                let (block, size) = (self.arg(0)?, self.arg(1)?);
                self.heap_realloc(block, size).map_err(Error::from)
            }
            // The block goes back to the arena, but the answer is still the pointer that came in:
            // the function is `void`, so a stub leaves r0 alone, and a caller that goes on using the
            // word it passed is relying on exactly that. A pack can free a buffer, keep the register,
            // and hand it to `wstrcmp` a million instructions later — answering null here turns that
            // into a read of address 0.
            "free" | "sysfree" => {
                let block = self.arg(0)?;
                self.heap_free(block);
                Ok(block)
            }
            // `ERR_REALLOC(nlen, pp)` and `ERR_STRDUP(pszSrc, pp)` (the header,) are
            // the two helpers that answer with a *code* and hand the block back through the caller's own
            // pointer slot. Both bodies are a few lines, both leave the caller's word holding the new
            // block, and the ordering of the free in `err_strdup` is the part that is not decoration.
            "err_realloc" => {
                let (request, slot) = (self.arg(0)?, self.arg(1)?);
                let current = self.cpu.mem.read_u32(slot)?;
                let block = self.heap_realloc(current, request)?;
                // The doubled-and-halved test is the compiler's way of asking "are the low 31 bits not
                // zero" — that is, a *real* request failed. `aee_realloc(p, 0)` is a `FREE` that answers
                // NULL on success, so a zero-length request must not be reported as an outage.
                if block == 0 && request.wrapping_mul(2) >> 1 != 0 {
                    return Ok(g::AEE_EFAILED);
                }
                self.cpu.mem.write_u32(slot, block)?;
                Ok(0)
            }
            "err_strdup" => {
                let (src, slot) = (self.arg(0)?, self.arg(1)?);
                // This scan ceiling is the host's, not the body's (`aee_strlen` has none): a source with
                // no terminator inside it cannot be measured here, and the answer given is the one a
                // failed allocation already gives — code 2, and the caller's slot left holding its block.
                const SCAN: usize = 4096;
                let bytes = if src == 0 {
                    Vec::new()
                } else {
                    self.cpu.mem.read_cstr(src, SCAN)?
                };
                let mut copy = 0u32;
                if src != 0 {
                    copy = self.heap_alloc(bytes.len() as u32 + 1)?;
                    if copy == 0 {
                        return Ok(g::AEE_EFAILED);
                    }
                    self.cpu.mem.write_bytes(copy, &bytes)?;
                    self.cpu.mem.write_u8(copy + bytes.len() as u32, 0)?;
                }
                // Given back *before* the new word is written, and only on the path that reached here: a
                // failed copy returns 2 above without touching the slot, so the caller keeps what it had.
                let previous = self.cpu.mem.read_u32(slot)?;
                if previous != 0 {
                    self.heap_free(previous);
                }
                self.cpu.mem.write_u32(slot, copy)?;
                Ok(0)
            }
            "dumpheap" => {
                if let Err(why) = self.cpu.mem.heap_audit() {
                    self.display.note("Validating Heap Failed!");
                    self.display.note(format!("  {why}"));
                    return Ok(0);
                }
                let nodes = self.cpu.mem.heap_walk();
                self.display.note("------ Heap Info ------");
                let mut alloc = 0u32;
                let mut wasted = 0u32;
                let mut free_total = 0u32;
                let mut largest = 0u32;
                for (node, size, is_free) in &nodes {
                    self.display.note(format!(
                        "  {node:#010x} {size} byte(s) {}",
                        if *is_free { "free" } else { "used" }
                    ));
                    if *is_free {
                        free_total += size;
                        largest = largest.max(*size);
                    } else {
                        alloc += size.saturating_sub(crate::heap::NODE);
                        wasted += crate::heap::NODE;
                    }
                }
                self.display.note(format!("{alloc} Alloc - Total"));
                self.display.note(format!("{wasted} Wasted"));
                self.display.note(format!("{free_total} Free - Total"));
                self.display.note(format!("{largest} Largest"));
                self.display.note("no memory groups in this heap model");
                self.display.note("-------------------------");
                Ok(0)
            }
            // `void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *,const void *))`,
            // through the eight-byte `aee_qsort` tail call into libc. The gate is on arithmetic, not on
            // trust: `nmemb * size` is the caller's claim about memory, and an overflowed product would
            // otherwise send this host reading — or the pack's own comparator dereferencing — wherever the
            // wrap lands. Refusing with a word is cheaper than a wrong sort, and the delivered libc has no
            // such check because it dies instead.
            "qsort" => {
                let (base, nmemb, size, compar) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let span = u64::from(nmemb).saturating_mul(u64::from(size));
                if nmemb > 1 && size > 0 {
                    if base as u64 + span > u64::from(u32::MAX) {
                        self.display.note(format!(
                            "qsort({base:#010x}, {nmemb}, {size}): {span} byte(s) of array does not fit \
                             in the address space, so nothing was sorted"
                        ));
                    } else {
                        self.guest_qsort(base, nmemb, size, compar)?;
                    }
                }
                // The slot is `void`; the answer word is whatever the call leaves, which is nothing a
                // caller can read, so nothing is claimed here.
                Ok(0)
            }
            // The two slots whose **names** swap between the reference tables: BREW 3.1.2 has
            // `AEEOS_IsBadPtr` at index 100 and `dbgevent` at 101, while 4.0.2 has `dbgevent` at 100
            // and `aee_IsBadPtr` at 101 — and no delivered object contains the table instance itself
            // (the largest data section any delivered object carries is 84 bytes, which cannot hold
            // 114 pointers), so nothing on disk says which order the pack in hand was built against. ⇒ One
            // arm serves both readings at both offsets and says so in the report.
            //
            // The answer word is `0` for both, and that is a position rather than a shrug: a `void`
            // leaves it undefined, and `boolean FALSE` out of `IsBadPtr` means "this pointer is not
            // bad" — which is what this host already answers for `AEE_CheckPtr`, whose shell-side gate
            // passes every pointer because the interpreter, not the ABI, is what bounds a guest access.
            // Answering TRUE would steer the caller into its failure branch over memory it does own.
            "dbgevent" | "aee_IsBadPtr" => {
                self.display.note(format!(
                    "Helper {id:#06x}: the 3.1.2 and 4.0.2 `AEEHelperFuncs` tables swap these two \
                     offsets (`AEEOS_IsBadPtr`/`dbgevent` against `dbgevent`/`aee_IsBadPtr`), so this \
                     call is served both ways — the debug event records nothing, and the pointer check \
                     answers FALSE (not bad), because this host does not police guest pointers; the \
                     arguments are in the call log"
                ));
                Ok(0)
            }
            "memset" => {
                let dst = self.arg(0)?;
                let byte = (self.arg(1)? & 0xff) as u8;
                let len = self.arg(2)?;
                self.cpu.mem.fill(dst, len, byte)?;
                Ok(dst)
            }
            "memmove" => {
                let dst = self.arg(0)?;
                let src = self.arg(1)?;
                let len = self.arg(2)?;
                // Page-wise, and the same-page overlap uses `copy_within`: no `Vec` per
                // `memmove`, which is the shape a pack's per-frame string/table shuffle takes.
                self.cpu.mem.copy_bytes(dst, src, len as usize)?;
                Ok(dst)
            }
            "memcmp" => {
                let (a, b, len) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                // Compare page runs and stop at the first difference; no two staging buffers.
                Ok(match self.cpu.mem.compare_bytes(a, b, len as usize)? {
                    std::cmp::Ordering::Less => -1i32,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                } as u32)
            }
            "strlen" => {
                let s = self.arg(0)?;
                // `aee_strlen` has no ceiling of its own, and a fixed 4 KiB one was not a property of
                // the body: a 22 KiB HTML page with its NUL after the last byte read as "not a
                // string" and ended the run. The bound that belongs here is the guest's mapping — the
                // read faults at the first page the guest does not own, which is where a handset
                // would have faulted too. Same shape as `OEMStrLen` below, for the same reason.
                if s == 0 {
                    return Ok(0);
                }
                let mut n = 0u32;
                while self.cpu.mem.read_u8(s.wrapping_add(n))? != 0 {
                    n += 1;
                }
                Ok(n)
            }
            "OEMStrLen" => {
                let s = self.arg(0)?;
                let mut at = 0u32;
                let mut chars = 0u32;
                loop {
                    let b = self.cpu.mem.read_u8(s + at)?;
                    if b == 0 {
                        break;
                    }
                    chars += 1;
                    at += 1;
                    if (b as i8) < 0 {
                        let trail = self.cpu.mem.read_u8(s + at)?;
                        at += 1;
                        if trail == 0 {
                            break;
                        }
                    }
                }
                Ok(chars)
            }
            "OEMStrSize" => {
                let s = self.arg(0)?;
                let bytes = self.cpu.mem.read_cstr(s, 4096)?.len();
                Ok(if bytes == 0 { 0 } else { (bytes + 1) as u32 })
            }
            "strcpy" => {
                let dst = self.arg(0)?;
                let src = self.arg(1)?;
                let bytes = self.cpu.mem.read_cstr(src, 4096)?;
                for (i, b) in bytes.iter().enumerate() {
                    self.cpu.mem.write_u8(dst + i as u32, *b)?;
                }
                self.cpu.mem.write_u8(dst + bytes.len() as u32, 0)?;
                Ok(dst)
            }
            // The one helper that is both a string function and an allocation, so it goes through
            // the guest heap rather than a caller-supplied buffer. A pack may ask for it as its first
            // libc call, and its contract is `malloc`'s: running out of guest heap answers NULL, which
            // the module reacts to as an allocation failure
            // rather than as a missing host function.
            "strdup" => {
                let src = self.arg(0)?;
                let bytes = self.cpu.mem.read_cstr(src, 4096)?;
                let dst = match self.hle.alloc(&mut self.cpu.mem, bytes.len() as u32 + 1) {
                    Ok(addr) => addr,
                    Err(Fault::HeapExhausted { .. }) => return Ok(0),
                    Err(other) => return Err(Error::Hle(HleError::Mem(other))),
                };
                self.cpu.mem.write_bytes(dst, &bytes)?;
                self.cpu.mem.write_u8(dst + bytes.len() as u32, 0)?;
                Ok(dst)
            }
            // The module's libc converts between `AECHAR` and `char` through this pair. Both take
            // `(src, dest, nSize)` where `nSize` is the destination's size in *bytes* — which is why
            // the wideness of the destination is what limits how much arrives — and both answer with
            // the destination pointer, which is what the caller's macro expands to.
            "wstrtostr" => {
                let (src, dst, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?.min(4096));
                let units = self.cpu.mem.read_u16str(src, 4096)?;
                let keep = units
                    .iter()
                    .position(|u| *u == 0)
                    .unwrap_or(units.len())
                    .min(size.saturating_sub(1) as usize);
                for (i, u) in units.iter().take(keep).enumerate() {
                    // The high half of an `AECHAR` has nowhere to go in a single-byte string, which
                    // is the loss the handset's own narrowing conversion accepts.
                    self.cpu.mem.write_u8(dst + i as u32, *u as u8)?;
                }
                self.cpu.mem.write_u8(dst + keep as u32, 0)?;
                Ok(dst)
            }
            "strtowstr" => {
                let (src, dst, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?.min(4096));
                let bytes = self.cpu.mem.read_cstr(src, 4096)?;
                let keep = bytes.len().min(size.saturating_sub(2) as usize / 2);
                for (i, b) in bytes[..keep].iter().enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*b))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(dst)
            }
            // `size_t WSTRLCPY(AECHAR *dst, const AECHAR *src, size_t nSize)`. `nSize` counts
            // `AECHAR`, not bytes — the header says so outright — and the return value is the length
            // of the *source*, which is how the caller learns truncation happened: a return of `n`
            // into a buffer of `m < n` means not all of it fit. The destination is always terminated,
            // which is the whole reason a pack reaches for this instead of `wstrcpy`.
            "wstrlcpy" => {
                let (dst, src, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                if dst == 0 || src == 0 || size == 0 {
                    return Ok(u32::MAX);
                }
                // `read_u16str` stops at the terminator and does not include it, so what comes back
                // is exactly the string the caller named.
                let units = self.cpu.mem.read_u16str(src, 4096)?;
                let keep = units.len().min(size as usize - 1);
                for (i, u) in units[..keep].iter().enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(units.len() as u32)
            }
            "wstrlcat" => {
                let (dst, src, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let held = self.cpu.mem.read_u16str(dst, 4096)?.len() as u32;
                let src_len = self.cpu.mem.read_u16str(src, 4096)?.len() as u32;
                if held >= size {
                    return Ok(held + src_len);
                }
                // `room >= 1` here: the branch above took everything at or below the terminator's own
                // slot, and the terminator is what the body writes last.
                let room = size - held;
                let keep = src_len.min(room - 1);
                for i in 0..keep {
                    let unit = self.cpu.mem.read_u16(src + i * 2)?;
                    let at = dst + (held + i) * 2;
                    self.cpu.mem.write_u16(at, u32::from(unit))?;
                }
                self.cpu.mem.write_u16(dst + (held + keep) * 2, 0)?;
                Ok(held + src_len)
            }
            // The wide counterpart of `strlen`, which a module reaches for as soon as it measures
            // something it loaded as `AECHAR` — `LoadResString` hands back UTF-16 units, and
            // nothing else in this table can measure those.
            "wstrlen" => {
                let s = self.arg(0)?;
                let units = self.cpu.mem.read_u16str(s, 4096)?;
                Ok(units.iter().position(|u| *u == 0).unwrap_or(units.len()) as u32)
            }
            "wstrchr" => {
                let (s, ch) = (self.arg(0)?, self.arg(1)?);
                let mut at = 0u32;
                loop {
                    let unit = self.cpu.mem.read_u16(s + at * 2)?;
                    if u32::from(unit) == ch {
                        return Ok(s + at * 2);
                    }
                    if unit == 0 {
                        return Ok(0);
                    }
                    at += 1;
                }
            }
            "wstrrchr" => {
                let (s, ch) = (self.arg(0)?, self.arg(1)?);
                let units = self.cpu.mem.read_u16str(s, 4096)?;
                let mut at = units.len();
                while at > 0 {
                    at -= 1;
                    if u32::from(units[at]) == ch {
                        return Ok(s + at as u32 * 2);
                    }
                }
                // The body's last question is `*a1 == ch`, reached either from an empty string or after
                // the walk has already compared that unit — so it can only say yes to `("", 0)`.
                Ok(if units.is_empty() && ch == 0 { s } else { 0 })
            }
            // `WSTRLOWER`/`WSTRUPPER` change a string where it stands and answer nothing, so the
            // whole job is knowing which characters move. This is the ASCII range on purpose: what
            // reaches it in this sample set is option names and MIME-ish keys, all of which are ASCII,
            // and a wide character outside it is left exactly as it was rather than run through a
            // case table this host has no measurement for. CJK, which several packs do pass
            // through here, has no case to change, so the same rule answers it correctly.
            "wstrlower" | "wstrupper" => {
                let s = self.arg(0)?;
                let lower = name == "wstrlower";
                // Read first, then write: `read_u16str` stops at the terminator and excludes it, so
                // the indices line up with the buffer, and rewriting a unit in place while still
                // reading would let an early change affect a later measurement.
                let units = self.cpu.mem.read_u16str(s, 4096)?;
                for (i, u) in units.iter().enumerate() {
                    let moved = if lower {
                        (0x0041..=0x005a).contains(u).then(|| u + 0x20)
                    } else {
                        (0x0061..=0x007a).contains(u).then(|| u - 0x20)
                    };
                    if let Some(c) = moved {
                        self.cpu.mem.write_u16(s + i as u32 * 2, u32::from(c))?;
                    }
                }
                Ok(0)
            }
            // `aee_strcmp` is a one-liner onto libc behind two `AEE_CheckPtr` calls that always pass
            // here, so its whole rule is C's: walk to the first difference or terminator and answer its
            // difference. `u32::MAX` is the host's stand-in for "no limit" — the walk stops at the
            // terminator, and a string with none faults the caller on a handset too.
            "strcmp" => {
                let (a, b) = (self.arg(0)?, self.arg(1)?);
                self.compare_cstr(a, b, u32::MAX, false)
                    .map_err(Error::from)
            }
            // ---- the string library ------------------------------------------------------------
            // These reach a module as the `STR*`/`WSTR*` macros, so they are its libc and it is
            // entitled to them. Each keeps the C contract exactly — including the parts a caller's
            // compiler is allowed to assume — because a plausible-but-wrong answer here corrupts the
            // module's own state further on, where it reads as anything but a libc bug.
            "strcat" => {
                let (dst, src) = (self.arg(0)?, self.arg(1)?);
                let head = self.cpu.mem.read_cstr(dst, 4096)?;
                let tail = self.cpu.mem.read_cstr(src, 4096)?;
                self.cpu.mem.write_bytes(dst + head.len() as u32, &tail)?;
                self.cpu
                    .mem
                    .write_u8(dst + (head.len() + tail.len()) as u32, 0)?;
                Ok(dst)
            }
            // `strchr(s, 0)` finds the terminator, which is why the search covers one byte past the
            // string's own bytes; a miss answers NULL, and there is no third answer.
            "strchr" | "strrchr" => {
                let (s, c) = (self.arg(0)?, self.arg(1)?);
                let needle = (c & 0xff) as u8;
                let bytes = self.cpu.mem.read_cstr(s, 4096)?;
                // The terminator is searched too, because `strchr(s, 0)` finds it; appending it to
                // the bytes is what makes both directions of the search cover it.
                let mut hay = bytes;
                hay.push(0);
                let at = if name == "strchr" {
                    hay.iter().position(|b| *b == needle)
                } else {
                    hay.iter().rposition(|b| *b == needle)
                };
                Ok(match at {
                    Some(i) => s + i as u32,
                    None => 0,
                })
            }
            "strchrend" => {
                let (s, c) = (self.arg(0)?, self.arg(1)?);
                let mut at = 0u32;
                loop {
                    let b = self.cpu.mem.read_u8(s + at)?;
                    if b == 0 || u32::from(b) == c {
                        break;
                    }
                    at += 1;
                }
                Ok(s + at)
            }
            "strchrsend" => {
                let (s, set) = (self.arg(0)?, self.arg(1)?);
                let chars = self.cpu.mem.read_cstr(set, 4096)?;
                let mut at = 0u32;
                loop {
                    let b = self.cpu.mem.read_u8(s + at)?;
                    if b == 0 || chars.contains(&b) {
                        break;
                    }
                    at += 1;
                }
                Ok(s + at)
            }
            "memchrend" => {
                let (p, c, len) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let mut at = 0u32;
                while at < len {
                    let b = self.cpu.mem.read_u8(p + at)?;
                    if u32::from(b) == c {
                        break;
                    }
                    at += 1;
                }
                Ok(p + at)
            }
            "memrchrbegin" => {
                let (p, c, len) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let mut at = len;
                loop {
                    if at == 0 {
                        break;
                    }
                    at -= 1;
                    let b = self.cpu.mem.read_u8(p + at)?;
                    if u32::from(b) == c {
                        break;
                    }
                }
                Ok(p + at)
            }
            // C pads out to `n` with NUL, which is what a caller filling a fixed-size field relies
            // on. The source is read once, so an overlapping pair cannot read its own output.
            "strncpy" => {
                let (dst, src, n) = (self.arg(0)?, self.arg(1)?, self.arg(2)?.min(4096));
                let bytes = self.cpu.mem.read_cstr(src, 4096)?;
                for i in 0..n {
                    let b = bytes.get(i as usize).copied().unwrap_or(0);
                    self.cpu.mem.write_u8(dst + i, b)?;
                }
                Ok(dst)
            }
            "strncmp" => {
                let (a, b, n) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                self.compare_cstr(a, b, n, false).map_err(Error::from)
            }
            "stricmp" => {
                let (a, b) = (self.arg(0)?, self.arg(1)?);
                self.compare_cstr(a, b, 0x7fff_ffff, true)
                    .map_err(Error::from)
            }
            "strnicmp" => {
                let (a, b, n) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                self.compare_cstr(a, b, n, true).map_err(Error::from)
            }
            // `strstr(h, "")` answers `h`, which falls out of searching at every position up to and
            // including the end rather than being special-cased.
            "strstr" | "stristr" => {
                let (hay, needle) = (self.arg(0)?, self.arg(1)?);
                let h = self.cpu.mem.read_cstr(hay, 4096)?;
                let n = self.cpu.mem.read_cstr(needle, 4096)?;
                let fold = name == "stristr";
                let same = |a: &[u8], b: &[u8]| {
                    a.iter().zip(b).all(|(x, y)| {
                        if fold {
                            aee_tolower(*x) == aee_tolower(*y)
                        } else {
                            x == y
                        }
                    })
                };
                let at = (0..=h.len())
                    .find(|i| n.len() <= h.len() - i && same(&h[*i..*i + n.len()], &n));
                Ok(match at {
                    Some(i) => hay + i as u32,
                    None => 0,
                })
            }
            "strlower" | "strupper" => {
                let s = self.arg(0)?;
                let bytes = self.cpu.mem.read_cstr(s, 4096)?;
                let lower = name == "strlower";
                for (i, b) in bytes.iter().enumerate() {
                    let want = if lower {
                        aee_tolower(*b)
                    } else {
                        aee_toupper(*b)
                    };
                    self.cpu.mem.write_u8(s + i as u32, want)?;
                }
                Ok(s)
            }
            "memchr" | "memrchr" => {
                let (s, c, n) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let byte = (c & 0xff) as u8;
                let bytes = self.cpu.mem.read_bytes(s, n as usize)?;
                let at = if name == "memchr" {
                    bytes.iter().position(|b| *b == byte)
                } else {
                    bytes.iter().rposition(|b| *b == byte)
                };
                Ok(match at {
                    Some(i) => s + i as u32,
                    None => 0,
                })
            }
            // `atoi` is `strtol(s, NULL, 10)` narrowed to `int`, so text that is not a number is 0
            // rather than an error the caller never asked to have to check.
            "atoi" => {
                let s = self.arg(0)?;
                let text = self.cpu.mem.read_cstr(s, 4096)?;
                let (_, negative, value) = scan_number(&text, 10);
                Ok(if negative {
                    (-i64::from(value)) as i32 as u32
                } else {
                    value
                })
            }
            // The unsigned half of the same scan. A leading `-` wraps rather than being refused,
            // which is what a library that has already decided the result is unsigned does with it.
            "strtoul" => {
                // `uint32 strtoul(const char *nptr, char **endptr, int base)`: the base is the
                // caller's, and `endptr` — when there is one — is told where the number stopped, which
                // is the only way a caller can parse a list of them in a single pass.
                let (s, endptr, base) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let text = self.cpu.mem.read_cstr(s, 4096)?;
                let (used, negative, value) = scan_number(&text, base);
                if endptr != 0 {
                    self.cpu.mem.write_u32(endptr, s + used as u32)?;
                }
                Ok(if negative {
                    value.wrapping_neg()
                } else {
                    value
                })
            }
            "trunc" | "utrunc" => {
                let (lo, hi) = (self.arg(0)?, self.arg(1)?);
                let x = f64::from_bits(((u64::from(hi)) << 32) | u64::from(lo));
                let cut = x.trunc();
                Ok(if name == "trunc" {
                    cut as i32 as u32
                } else {
                    cut as u32
                })
            }
            "wgs84_to_degrees" => {
                let (lo, hi) = (self.arg(0)?, self.arg(1)?);
                let raw = ((i64::from(hi)) << 32) | i64::from(lo) & 0xffff_ffff;
                let bits = ((raw as f64) / POSDET_DEGREES_PER_UNIT).to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "strtod" => {
                let (s, endp) = (self.arg(0)?, self.arg(1)?);
                if s == 0 {
                    // The body's own odd guard, kept exactly: it reads the byte the caller's
                    // end-pointer names and zeroes that pointer **only if the byte is not already NUL**.
                    // Both reads are unconditional in the platform code, so they are unconditional here —
                    // a caller that handed over a null `ppszEnd` faults on a handset too.
                    let at = self.cpu.mem.read_u32(endp)?;
                    if self.cpu.mem.read_u8(at)? != 0 {
                        self.cpu.mem.write_u32(endp, 0)?;
                    }
                    return Ok(0);
                }
                let bytes = self.cpu.mem.read_cstr(s, 4096)?;
                let chars: Vec<u32> = bytes.iter().map(|&b| u32::from(b)).collect();
                let (value, consumed) = decimal_parse(&chars).unwrap_or((0.0, 0));
                if endp != 0 {
                    // `_softfp_strtod` is handed both registers, so the pair the body answers with
                    // includes this write; the pointer is the *caller's* start plus the count.
                    self.cpu
                        .mem
                        .write_u32(endp, s.wrapping_add(consumed as u32))?;
                }
                let bits = value.to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            // `double wstrtofloat(const AECHAR *pszNum)`. The delivered slot is a
            // four-byte jump to `OEM_WStrToFloat`, which none of the 235 members defines — and unlike
            // `floattowstr` this slot **has no error channel at all**: its whole ABI is a `double`, so
            // "refuse" is not a thing it can say. Answering the number is the honest direction, and the
            // one a pack can read; what is missing is the rounding, and that is stated in
            // [`decimal_parse`]. Before this arm the slot had no branch, so a pack that reached
            // `WSTRTOFLOAT` ended its own run on an unimplemented import.
            "wstrtofloat" => {
                let s = self.arg(0)?;
                let chars: Vec<u32> = self
                    .cpu
                    .mem
                    .read_u16str(s, 4096)?
                    .iter()
                    .map(|&u| u32::from(u))
                    .collect();
                let (value, _) = decimal_parse(&chars).unwrap_or((0.0, 0));
                let bits = value.to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "f_assignstr" => {
                let s = self.arg(0)?;
                let bytes = self.cpu.mem.read_cstr(s, 4096)?;
                let chars: Vec<u32> = bytes.iter().take(19).map(|&b| u32::from(b)).collect();
                let (value, _) = decimal_parse(&chars).unwrap_or((0.0, 0));
                let bits = value.to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "floattowstr" => Ok(self.no_implementation_false("AEEHelperFuncs", "floattowstr")),
            // `char *memstr(const char *haystack, const char *needle, size_t nLen)` — the bounded
            // `strstr`, for a buffer the caller knows the length of but has not terminated.
            "memstr" => {
                let (hay, needle, len) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let h = self.cpu.mem.read_bytes(hay, len as usize)?;
                let n = self.cpu.mem.read_cstr(needle, 4096)?;
                let at =
                    (0..=h.len()).find(|i| n.len() <= h.len() - i && h[*i..*i + n.len()] == n[..]);
                Ok(match at {
                    Some(i) => hay + i as u32,
                    None => 0,
                })
            }
            // `strbegins`/`strends` answer a `boolean` about the whole of the second argument, which
            // is the `psz` half of the pair in each prototype.
            "strbegins" => {
                let (prefix, s) = (self.arg(0)?, self.arg(1)?);
                let p = self.cpu.mem.read_cstr(prefix, 4096)?;
                let text = self.cpu.mem.read_cstr(s, 4096)?;
                Ok(if text.starts_with(&p[..]) {
                    g::TRUE
                } else {
                    g::FALSE
                })
            }
            "strends" => {
                let (suffix, s) = (self.arg(0)?, self.arg(1)?);
                let x = self.cpu.mem.read_cstr(suffix, 4096)?;
                let text = self.cpu.mem.read_cstr(s, 4096)?;
                Ok(if text.ends_with(&x[..]) {
                    g::TRUE
                } else {
                    g::FALSE
                })
            }
            "aee_splitpath" => {
                let (path_at, dir_at) = (self.arg(0)?, self.arg(1)?);
                let path = self.cpu.mem.read_cstr(path_at, 4096)?;
                let mut dir = self.cpu.mem.read_cstr(dir_at, 4096)?;
                if dir.last() == Some(&b'/') {
                    dir.pop();
                }
                let n = dir.len();
                if path.len() == n && path[..n] == dir[..] {
                    return Ok(path_at + n as u32);
                }
                if path.len() <= n || path[n] != b'/' || path[..n] != dir[..] {
                    return Ok(0);
                }
                Ok(path_at + (n + 1) as u32)
            }
            "aee_basename" => {
                let path_at = self.arg(0)?;
                let path = self.cpu.mem.read_cstr(path_at, 4096)?;
                Ok(match path.iter().rposition(|b| *b == b'/') {
                    Some(i) => path_at + i as u32 + 1,
                    None => path_at,
                })
            }
            "aee_stribegins" => {
                let (prefix_at, text_at) = (self.arg(0)?, self.arg(1)?);
                let prefix = self.cpu.mem.read_cstr(prefix_at, 4096)?;
                let text = self.cpu.mem.read_cstr(text_at, 4096)?;
                let mut i = 0usize;
                let answer = loop {
                    let left = prefix.get(i).copied().unwrap_or(0);
                    if left == 0 {
                        break g::TRUE;
                    }
                    let right = text.get(i).copied().unwrap_or(0);
                    if aee_tolower(left) != aee_tolower(right) {
                        break g::FALSE;
                    }
                    i += 1;
                };
                Ok(answer)
            }
            "aee_makepath" => {
                let (dir_at, file_at, out, plen) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let dir = self.cpu.mem.read_cstr(dir_at, 4096)?;
                let mut file = self.cpu.mem.read_cstr(file_at, 4096)?;
                if file.first() == Some(&b'/') {
                    file.remove(0);
                }
                let mut want: Vec<u8> = dir.clone();
                // ⚠ The body tests `dir[strlen(dir) - 1]` before it tests `dir[0] != 0`, so an empty
                // directory text reads the byte in front of the caller's buffer. That is the reference
                // code's own out-of-bounds read rather than a rule with a record behind it, so this
                // host asks the two questions in the other order and adds no separator for an empty one.
                if !dir.is_empty() && dir[dir.len() - 1] != b'/' {
                    want.push(b'/');
                }
                want.extend_from_slice(&file);
                want.push(0);
                let room = if out == 0 {
                    0
                } else {
                    self.cpu.mem.read_u32(plen)? as usize
                };
                let written = if out == 0 {
                    want.len()
                } else {
                    want.len().min(room)
                };
                if out != 0 && written != 0 {
                    self.cpu.mem.write_bytes(out, &want[..written])?;
                }
                self.cpu.mem.write_u32(plen, written as u32)?;
                if out == 0 || written == want.len() {
                    return Ok(g::AEE_SUCCESS);
                }
                if room != 0 {
                    self.cpu.mem.write_u8(out + room as u32 - 1, 0)?;
                }
                Ok(g::AEE_EBUFFERTOOSMALL)
            }
            // The BSD `strlcpy`/`strlcat`: copy what fits, always terminate, and answer the length the
            // copy *would* have had — which is how a caller sizes a retry.
            "strlcpy" | "strlcat" => {
                let (dst, src, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let s = self.cpu.mem.read_cstr(src, 4096)?;
                // `strlcat` starts at the destination's own end and counts that prefix in its answer
                // even when none of the source fits.
                let held = if name == "strlcpy" {
                    0
                } else {
                    self.cpu.mem.read_cstr(dst, size.max(1) as usize)?.len()
                };
                let room = (size as usize).saturating_sub(held + 1);
                let keep = s.len().min(room);
                for (i, b) in s[..keep].iter().enumerate() {
                    self.cpu.mem.write_u8(dst + (held + i) as u32, *b)?;
                }
                self.cpu.mem.write_u8(dst + (held + keep) as u32, 0)?;
                Ok((held + s.len()) as u32)
            }
            // `int wstrsize(const AECHAR *p)` is the wide `strsize`: both answer the bytes a string
            // occupies, which is one terminator more than the unit count.
            "wstrsize" => {
                let s = self.arg(0)?;
                let units = self.cpu.mem.read_u16str(s, 4096)?;
                let len = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                Ok((len as u32 + 1) * 2)
            }
            "wstrncopyn" => {
                let (dst, units, src, n) = (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let room = units.saturating_sub(1);
                // `-1` arrives as `0xFFFFFFFF`; the body turns that one value into `wstrlen` and
                // every other into the limit itself, and the loop stops at the terminator either way.
                let stop = if n == u32::MAX { room } else { n.min(room) };
                let mut written = 0u32;
                while written < stop {
                    let unit = self.cpu.mem.read_u16(src + written * 2)?;
                    if unit == 0 {
                        break;
                    }
                    self.cpu.mem.write_u16(dst + written * 2, u32::from(unit))?;
                    written += 1;
                }
                self.cpu.mem.write_u16(dst + written * 2, 0)?;
                Ok(written)
            }
            // `strexpand` is `strtowstr` with the source's length given rather than searched for,
            // which is what a caller has when the bytes came out of a container.
            "strexpand" => {
                let (src, count, dst, size) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let bytes = self.cpu.mem.read_bytes(src, count as usize)?;
                // `STREXPAND` is the narrow-to-wide half of the pair, and it is encoding-aware:
                // on a Shift-JIS device two bytes name one character, and widening them one per
                // byte is the mojibake a menu shows when its text is read as Latin.
                let units = self.text_encoding().bytes_to_units(&bytes);
                let room = (size as usize).saturating_sub(2) / 2;
                let keep = units.len().min(room);
                for (i, u) in units[..keep].iter().enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(dst)
            }
            // `WSTRCOMPRESS(const AECHAR *pSrc, int nLen, byte *pDest, int nSize)` — the inverse of
            // the expander just above, and its entire documented rule is "if a character is less or
            // equal than 127, then reduce the two bytes to one byte". `nLen` counts AECHARs and
            // `nSize` counts bytes, so the walk stops at whichever end arrives first: the terminator
            // the signature promises, the caller's own count, or the room it handed over.
            //
            // Above 127 the header is silent, and a byte cannot hold the character in any case. The
            // low half is what a C cast writes, and it is what `STREXPAND` returns unchanged for any
            // byte sent to it, so the pair round-trips across the whole 0..=255 range rather than
            // only across the ASCII one the documentation bothers to name.
            //
            // The result is terminated because the call answers nothing at all: a compressed name
            // whose end no caller can find is not a name.
            "wstrcompress" => {
                let (src, nchars, dst, size) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let units = self.cpu.mem.read_u16str(src, nchars as usize)?;
                let bytes = self.text_encoding().units_to_bytes(&units);
                let room = (size as usize).saturating_sub(1);
                let keep = bytes.len().min(room);
                for (i, b) in bytes[..keep].iter().enumerate() {
                    self.cpu.mem.write_u8(dst + i as u32, *b)?;
                }
                if size > 0 {
                    self.cpu.mem.write_u8(dst + keep as u32, 0)?;
                }
                Ok(0)
            }
            // The UTF-8 pair: `utf8towstr` is what a module reads a text file through, and
            // `wstrtoutf8` is what it writes one with. Both are told the source's length in the
            // source's own units and the destination's size in bytes, and both answer whether it fit
            // rather than how much of it did — a caller that gets FALSE retries with a smaller piece.
            "wstrtoutf8" => {
                let (src, nlen, dst, size) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let units = self.cpu.mem.read_u16str(src, nlen.max(1) as usize)?;
                let bytes = self.text_encoding().units_to_string(&units).into_bytes();
                // `nSize` is the destination's capacity in bytes with nothing reserved: the
                // handset's own `OEM_WStrToUTF8` writes while a byte still fits and stops the moment
                // one does not, and never writes a terminator - the caller is told the length
                // separately. Reserving one more byte for a NUL, as this once did, refuses calls a
                // handset accepts.
                if bytes.len() > size as usize {
                    return Ok(g::FALSE);
                }
                for (i, b) in bytes.iter().enumerate() {
                    self.cpu.mem.write_u8(dst + i as u32, *b)?;
                }
                Ok(g::TRUE)
            }
            "utf8towstr" => {
                let (src, nlen, dst, size) =
                    (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                let bytes = self.cpu.mem.read_bytes(src, nlen.max(1) as usize)?;
                // Decoded strictly rather than lossily. A `nLen` that ends in the middle of a
                // sequence is a question about a character that is not there, and
                // `from_utf8_lossy` answers it with U+FFFD - which the caller then converts to a
                // wide unit and draws, so a pack that cut its own text at a byte boundary shows a
                // notdef box the handset never showed.
                let text = match std::str::from_utf8(&bytes) {
                    Ok(s) => s,
                    Err(e) => std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap_or_default(),
                };
                let units = self.text_encoding().string_to_units(text);
                // A character at a time, the way `OEM_UTF8ToWStr` does it: `nSize` counts bytes,
                // each `AECHAR` takes two of them, and no terminator is written or reserved for -
                // the caller was told the length separately. A module that passes exactly the room
                // for its one wide character is asking a legal question: `nSize = 2` for a single
                // three-byte UTF-8 hanzi. Answering FALSE while writing nothing leaves whatever the
                // caller's buffer held to be drawn as text, and a game's menus come out as box after
                // box.
                for (i, u) in units.iter().enumerate() {
                    if (i + 1) * 2 > size as usize {
                        return Ok(g::FALSE);
                    }
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                Ok(g::TRUE)
            }
            // The dotted-quad pair. Four decimal octets, most significant first, which is the whole
            // form `inet_aton` accepts here; the IPv6 pair further down the table takes an address
            // family this one has no use for.
            "inet_aton" => {
                let (text, out) = (self.arg(0)?, self.arg(1)?);
                let s = self.cpu.mem.read_cstr(text, 4096)?;
                let Some(addr) = inet_aton_value(&s) else {
                    return Ok(g::FALSE);
                };
                if out != 0 {
                    self.cpu.mem.write_u32(out, addr)?;
                }
                Ok(g::TRUE)
            }
            "inet_ntoa" => {
                let (addr, buf, len) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let text = format!(
                    "{}.{}.{}.{}",
                    addr & 0xff,
                    (addr >> 8) & 0xff,
                    (addr >> 16) & 0xff,
                    (addr >> 24) & 0xff
                );
                let keep = text.len().min((len as usize).saturating_sub(1));
                for (i, b) in text.as_bytes()[..keep].iter().enumerate() {
                    self.cpu.mem.write_u8(buf + i as u32, *b)?;
                }
                self.cpu.mem.write_u8(buf + keep as u32, 0)?;
                // `aee_strlcpy`'s answer, which the body returns unchanged: the length the *whole*
                // text wanted, so a caller can tell truncation from a fit by comparing it with `len`.
                Ok(text.len() as u32)
            }
            // `int inet_pton(int af, const char *src, void *dst)` and
            // `const char *inet_ntop(int af, const void *src, char *dst, size_t size)`: both are in
            // the 4.0.2 table  and **neither has a body in this set** (0
            // definitions, 0 references), so the contract is the published one: a dotted quad in
            // network order, `1`/`0` from pton, the destination pointer or NULL from ntop, and an
            // address family this host does not serve answers the same "no" the standard's
            // `EAFNOSUPPORT` means. `AF_INET` is the family the delivered socket layer knows
            // (`socket::FAMILY_INET`); `AF_INET6` would need a parser no phone here has a table for.
            "inet_pton" => {
                let (af, src, dst) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                if af != u32::from(socket::FAMILY_INET) || src == 0 || dst == 0 {
                    return Ok(0);
                }
                let text = self.cpu.mem.read_cstr(src, 64)?;
                let Some(octets) = dotted_quad(&text) else {
                    return Ok(0);
                };
                self.cpu.mem.write_bytes(dst, &octets)?;
                Ok(1)
            }
            "inet_ntop" => {
                let (af, src, dst, size) = (self.arg(0)?, self.arg(1)?, self.arg(2)?, self.arg(3)?);
                if af != u32::from(socket::FAMILY_INET) || src == 0 || dst == 0 {
                    return Ok(0);
                }
                let mut octets = [0u8; 4];
                octets.copy_from_slice(&self.cpu.mem.read_bytes(src, 4)?);
                let text = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
                if (size as usize) <= text.len() {
                    return Ok(0);
                }
                self.cpu.mem.write_bytes(dst, text.as_bytes())?;
                self.cpu.mem.write_u8(dst + text.len() as u32, 0)?;
                Ok(dst)
            }
            // `int f_toint(double val)`: `aee_ftoint` in the delivered body is 8 bytes of
            // `push {r3, lr}; bl dfix; pop {r3, pc}` — the same one-branch conversion `trunc`
            // forwards to, so the same host choice about NaN and overflow applies (the C runtime
            // behind `dfix` is in none of the 235).
            "f_toint" => {
                let (lo, hi) = (self.arg(0)?, self.arg(1)?);
                let x = f64::from_bits(((u64::from(hi)) << 32) | u64::from(lo));
                Ok(x.trunc() as i32 as u32)
            }
            // `TChType chartype(AECHAR ch)`: the 3.1.2 header names the whole answer language —
            // `SC_UNKNOWN`, `SC_ALPHA`, `SC_DIGIT`, `SC_WHITESPACE` — and the body that produces it
            // (`OEM_GetCHType`) is referenced once and defined nowhere in the 235, so the table it
            // consults is the platform layer's. What is left for this host is the range the enum is
            // about: ASCII classes, and `SC_UNKNOWN` for everything else — which is the enum's own
            // name for "none of the three", not an invented fourth answer.
            "chartype" => {
                let ch = self.arg(0)? & 0xffff;
                Ok(match char::from_u32(ch) {
                    Some(c) if c.is_ascii_alphabetic() => g::SC_ALPHA,
                    Some(c) if c.is_ascii_digit() => g::SC_DIGIT,
                    Some(c) if c.is_ascii_whitespace() => g::SC_WHITESPACE,
                    _ => g::SC_UNKNOWN,
                })
            }
            // `uint32 JulianToSeconds(JulianType *pDate)`: the header defines the answer as
            // "seconds elapsed since CDMA reference time", and the delivered body
            // (`AEEOS_JulianToSeconds` in the delivered body, 30 bytes) is one `bridle_SystemCall` — the
            // conversion itself lives in the platform layer. So the formula is the host's, read off the
            // contract: a civil date-time minus 1980-01-06 00 UTC, which is what "CDMA
            // reference time" names. A null or unreadable pointer answers 0, the epoch itself.
            "aee_JulianToSeconds" => {
                let at = self.arg(0)?;
                let Ok(date) = self.cpu.mem.read_bytes(at, 14) else {
                    return Ok(0);
                };
                let word = |off: usize| u32::from(u16::from_le_bytes([date[off], date[off + 1]]));
                let (year, month, day) = (word(0) as i64, word(2) as u16, word(4) as u16);
                let (hour, minute, second) = (word(6) as i32, word(8) as i32, word(10) as i32);
                let days = days_from_civil(year, month, day) - GPS_EPOCH_DAYS;
                Ok((days as i32 * 86400 + hour * 3600 + minute * 60 + second) as u32)
            }
            // `int32 LocalTimeOffset(boolean *pbDaylightSavings)`: also one bridle call in the
            // delivered body (`AEEOS_LocalTimeOffset` in the delivered body), i.e. the offset is the
            // network's or the user's. This host's clock is UTC (`clock_ms` counts from a fixed
            // epoch), so the honest offset is zero and the daylight flag says "no" — a defined
            // answer, not a measured one.
            "aee_LocalTimeOffset" => {
                let out = self.arg(0)?;
                if out != 0 {
                    self.cpu.mem.write_u8(out, 0)?;
                }
                Ok(0)
            }
            // `int lockmem(void **ppHandle)` / `int unlockmem(void **ppHandle)`: the point of a lock
            // is that the OEM may keep the block where it is. Guest memory in this host never moves
            // — the heap is host-managed and no DMA exists to hand a physical address to — so the
            // property the caller is asking for already holds, and the answer says so. ⚠ These two
            // are the one family here whose *return* is the host's choice rather than a body's (no
            // member defines either name).
            "lockmem" | "unlockmem" => {
                self.display.note(
                    "lockmem/unlockmem: guest memory never moves here, so the lock holds without one",
                );
                Ok(g::TRUE)
            }
            // `IQueryInterface *GetALSContext(void)`: the ambient-light-sensor context. Nothing in
            // the 235 names this symbol, no class in the sample set asks for a light sensor, and this
            // host has none to expose, so the null is the same shape every other absent device gets.
            "GetALSContext" => {
                self.display
                    .note("GetALSContext: this host has no ambient-light sensor");
                Ok(0)
            }
            // A virtual clock does not pass, and a handset's `sleep` is about the wall. This host
            // has no wall-clock wait to offer, so the slot is refused instead of pretending it slept.
            "sleep" => {
                // `void sleep(uint32)` in the header: a status in r0 would be invisible to the
                // caller, so this refusal has to be a host error rather than a returned code.
                return Err(HleError::Unimplemented {
                    id,
                    label: label.to_string(),
                    this,
                }
                .into());
            }
            // The per-region error a module polls is its own bookkeeping. This host has no error
            // state for the helper to report, so refuse it rather than inventing a zero.
            // `aee_getlasterror(errRegion)` (the delivered body, 18 bytes; word 99 of the reference
            // `AEEHelperFuncs`, at table offset 392 — the header publishes the same field
            // as `int (*getlasterror)(int errRegion)`). Region 0 is `ERR_REGION_FLOAT`, and it is the
            // only region the header's own `GETLASTFPERROR`  ever passes: for it the
            // body reads the C library's error word through `__rt_errno_addr` and hands the word back
            // unchanged — no sign handling, no bias, and nothing cleared on either path. Any other
            // region answers zero directly. The answer is an `errno` value, not an
            // `AEEResult`, and no symbol in this delivery joins the two stores, so the number is the
            // whole contract: a pack compares it against zero, and 0 is what a runtime whose own arms
            // return values rather than raising the C word has to report.
            "getlasterror" => Ok(0),
            "aee_GetUTCSeconds" => Ok(self.clock_ms / 1000),
            // The storage pair, answered from the device profile so a module sizing a download sees
            // the same figures the file manager does.
            "GetFSFree" | "GetRAMFree" => {
                let (p1, p2) = (self.arg(0)?, self.arg(1)?);
                if p1 != 0 {
                    self.cpu
                        .mem
                        .write_u32(p1, file::room(self.device.storage_total))?;
                }
                if name == "GetRAMFree" && p2 != 0 {
                    self.cpu
                        .mem
                        .write_u32(p2, file::room(self.device.storage_free))?;
                }
                Ok(file::room(self.device.storage_free))
            }
            // Byte-order helpers: a module writing a network header swaps words by hand.
            "swapl" => Ok(self.arg(0)?.swap_bytes()),
            // `swaps` takes and answers a 16-bit value, so only the low half travels.
            "swaps" => Ok(u32::from((self.arg(0)? as u16).swap_bytes())),
            // ---- and its wide half --------------------------------------------------------------
            // `AECHAR` is UTF-16 on every handset this host models, and the sample set writes those units
            // with `LoadResString`, so these are reached as soon as an applet draws a label.
            "wstrcpy" => {
                let (dst, src) = (self.arg(0)?, self.arg(1)?);
                let units = self.cpu.mem.read_u16str(src, 4096)?;
                let keep = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                for (i, u) in units.iter().take(keep).enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(dst)
            }
            "wstrcat" => {
                let (dst, src) = (self.arg(0)?, self.arg(1)?);
                let head = self.cpu.mem.read_u16str(dst, 4096)?;
                let tail = self.cpu.mem.read_u16str(src, 4096)?;
                let at = head.iter().position(|u| *u == 0).unwrap_or(head.len());
                let keep = tail.iter().position(|u| *u == 0).unwrap_or(tail.len());
                for (i, u) in tail.iter().take(keep).enumerate() {
                    self.cpu
                        .mem
                        .write_u16(dst + (at + i) as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + (at + keep) as u32 * 2, 0)?;
                Ok(dst)
            }
            "wstrcmp" | "wstrncmp" | "wstricmp" | "wstrnicmp" => {
                let (a, b) = (self.arg(0)?, self.arg(1)?);
                let limit = if name == "wstrncmp" || name == "wstrnicmp" {
                    self.arg(2)?.min(4096)
                } else {
                    4096
                };
                let fold = name == "wstricmp" || name == "wstrnicmp";
                self.compare_u16str(a, b, limit, fold).map_err(Error::from)
            }
            // The wide `strdup`: a fresh block, because the caller frees it with the same `FREE`
            // that would free a `MALLOC`. Out of guest heap is `malloc`'s answer here, not this
            // host's error, which is the contract every allocator in this table keeps.
            "wstrdup" => {
                let src = self.arg(0)?;
                let units = self.cpu.mem.read_u16str(src, 4096)?;
                let keep = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                let dst = match self.hle.alloc(&mut self.cpu.mem, (keep as u32 + 1) * 2) {
                    Ok(addr) => addr,
                    Err(Fault::HeapExhausted { .. }) => return Ok(0),
                    Err(other) => return Err(Error::Hle(HleError::Mem(other))),
                };
                for (i, u) in units.iter().take(keep).enumerate() {
                    self.cpu.mem.write_u16(dst + i as u32 * 2, u32::from(*u))?;
                }
                self.cpu.mem.write_u16(dst + keep as u32 * 2, 0)?;
                Ok(dst)
            }
            "GetAppInstance" => self.get_app_instance().map_err(Error::from),
            // `uint32 GetAEEVersion(byte *pszFormatted, int nSize, uint16 wFlags)` — what release this
            // host is, one byte per component with the major in the top byte, and optionally the same
            // thing spelled out in the caller's buffer. Two details there are not
            // incidental: a buffer too small for the whole string and its terminator is left untouched,
            // and the text is wide characters unless `GAV_LATIN1` asked for single bytes. Passing
            // `GAV_INTERNAL` is the shape `AEE_IS_PATCH_PRESENT` uses — the caller names one patch and
            // reads back SUCCESS or not — and this host has no patches applied, so that call answers
            // `AEE_EFAILED` and writes nothing.
            "GetAEEVersion" => {
                let (buf, size, flags) = (self.arg(0)?, self.arg(1)? as i32, self.arg(2)?);
                if flags & g::GAV_INTERNAL != 0 {
                    return Ok(g::AEE_EFAILED);
                }
                // The release this host mirrors: 4.0.2, build 40, which is 0x04000228 packed.
                let text = if flags & g::GAV_UPDATE != 0 {
                    "4.0.2.40 Build 40"
                } else {
                    "4.0.2.40"
                };
                let bytes: Vec<u8> = if flags & g::GAV_LATIN1 == 0 {
                    text.encode_utf16()
                        .chain(std::iter::once(0))
                        .flat_map(u16::to_le_bytes)
                        .collect()
                } else {
                    text.bytes().chain(std::iter::once(0)).collect()
                };
                // A negative or short `nSize` is the header's "leave the buffer alone", and it is
                // compared as a signed number because a caller that passes -1 must not be read as 4 GiB.
                if buf != 0 && size as usize >= bytes.len() && size > 0 {
                    self.cpu.mem.write_bytes(buf, &bytes)?;
                }
                Ok(0x0400_0228)
            }
            // A debug build's `MALLOC` is `dbgheapmark(malloc(size), __FILE__, __LINE__)`, and the helper's
            // whole job is to hand the pointer back after noting where it came from. A pack may be
            // reference built that way, which is how it ends up asking a handset for a debugger's
            // bookkeeping — and a handset with no debugger attached answers with the pointer all the same.
            "dbgheapmark" => Ok(self.arg(0)?),
            "aee_GetTimeMS" | "aee_GetUpTimeMS" => Ok(self.clock_ms),
            "aee_GetSeconds" => Ok(self.clock_ms / 1000),
            // `void aee_GetRand(byte *pDest, int nSize)` — fill the caller's buffer with random bytes.
            //
            // The header's "seeded by the current time" is about the *seed*, not about every call:
            // a handset's generator is a stream that the clock starts and that then advances on its
            // own. Re-seeding from `clock_ms` here read that sentence the other way, and the cost was
            // not academic — two asks inside one virtual millisecond came back with identical bytes,
            // and a pack that draws a number as its loop condition never terminated, because the
            // virtual clock only moves when the guest calls the host and such a loop never does.
            // Three guest packs (sample packs) each burned a
            // billion instructions on `aee_GetRand` before the per-function ceiling stopped them.
            //
            // The seed is [`Runtime::rand_state`], taken at boot, so a scripted run still replays
            // byte for byte: the seed is a constant and the call order is the script's.
            "aee_GetRand" => {
                let (dest, size) = (self.arg(0)?, self.arg(1)? as i32);
                if dest != 0 && size > 0 {
                    let mut written = 0u32;
                    while written < size as u32 {
                        self.rand_state = splitmix_next(self.rand_state);
                        let state = self.rand_state;
                        for i in 0..8 {
                            if written == size as u32 {
                                break;
                            }
                            self.cpu
                                .mem
                                .write_u8(dest + written, (state >> (i * 8)) as u8)?;
                            written += 1;
                        }
                    }
                }
                Ok(0)
            }
            // `void aee_GetJulianDate(uint32 dwSecs, JulianType *pDate)`. Zero means "the current
            // system time", and this host's current time is the same virtual clock every other entry
            // in the table answers from — reading the machine's wall clock here would make a scripted
            // run depend on the day it was run. The layout is read through `AEEDateTime` because that
            // is the struct the interface declarations actually declare; `JulianType` is the same seven
            // `uint16`s under its older name, documented but never defined.
            "aee_GetJulianDate" => {
                let (secs, at) = (self.arg(0)?, self.arg(1)?);
                let secs = if secs == 0 {
                    self.clock_ms / 1000
                } else {
                    secs
                };
                let date = civil_from_gps(secs);
                let f = |name: &str| {
                    abi::offset_of("AEEDateTime", name).expect("AEEDateTime is in the ABI tables")
                };
                for (i, name) in [
                    "wYear", "wMonth", "wDay", "wHour", "wMinute", "wSecond", "wWeekDay",
                ]
                .into_iter()
                .enumerate()
                {
                    self.cpu.mem.write_u16(at + f(name), u32::from(date[i]))?;
                }
                Ok(0)
            }
            "sprintf" => {
                // `int sprintf(char *pszDest, const char *pszFormat,...)`. The varargs are the guest's
                // own registers and stack, which is what `Runtime::arg` reads, so the format string is
                // what decides how many of them are consumed — the contract a handset's printf has.
                let buf = self.arg(0)?;
                let fmt_at = self.arg(1)?;
                let fmt = Runtime::latin1_text(&self.cpu.mem.read_cstr(fmt_at, 1024)?);
                let mut next = 2;
                let text = self.format_raw(&fmt, &mut next, 0)?;
                let bytes = Runtime::narrow_bytes(&text);
                self.cpu.mem.write_bytes(buf, &bytes)?;
                self.cpu.mem.write_u8(buf + bytes.len() as u32, 0)?;
                Ok(bytes.len() as u32)
            }
            // The wide half of the same helper: `void wsprintf(AECHAR *pDest, int nSize,
            // const AECHAR *pFormat,...)`. Both the format and the result are `AECHAR`, which is the
            // only difference from `sprintf` — the conversions a module reaches for are the same ones,
            // so the engine is shared rather than copied. `nSize` is a capacity in characters, and the
            // header documents no return value at all, so this one answers zero.
            "wsprintf" => {
                let buf = self.arg(0)?;
                let room = self.arg(1)? as usize;
                let fmt_at = self.arg(2)?;
                let units = self.cpu.mem.read_u16str(fmt_at, 1024)?;
                let keep = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                let fmt = self.text_encoding().units_to_string(&units[..keep]);
                let mut next = 3;
                let text = self.format(&fmt, &mut next, 0)?;
                let written = self
                    .text_encoding()
                    .string_to_units(&text)
                    .into_iter()
                    .take(room)
                    .collect::<Vec<u16>>();
                for (i, u) in written.iter().enumerate() {
                    self.cpu.mem.write_u16(buf + i as u32 * 2, u32::from(*u))?;
                }
                if room > written.len() {
                    self.cpu.mem.write_u16(buf + written.len() as u32 * 2, 0)?;
                }
                Ok(0)
            }
            // The rest of the `printf` family in the helper table. Each differs from `sprintf` only in
            // where its varargs come from and how much room it may write, so all four are the one
            // engine above plus a bound — and the header is what says where a `va_list`'s words
            // are: `int **` on an ARM build, the address of the `__ap` field, so the arguments start
            // where that word points rather than at the word itself.
            "snprintf" => {
                // `int32 snprintf(char *buf, uint32 f, const char *format,...)`.
                let (buf, room, fmt_at) = (self.arg(0)?, self.arg(1)? as usize, self.arg(2)?);
                let fmt = Runtime::latin1_text(&self.cpu.mem.read_cstr(fmt_at, 1024)?);
                let mut next = 3;
                let text = self.format_raw(&fmt, &mut next, 0)?;
                self.put_bounded(buf, room, &text).map_err(Error::from)
            }
            "vsprintf" => {
                // `int vsprintf(char *pszDest, const char *pszFormat, AEEOldVaList arg)` — unbounded,
                // like C's, so the buffer the caller named is trusted exactly as on `sprintf`.
                let (buf, fmt_at, list) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let va = self.va_words(list)?;
                let fmt = Runtime::latin1_text(&self.cpu.mem.read_cstr(fmt_at, 1024)?);
                let mut next = 0;
                let text = self.format_raw(&fmt, &mut next, va)?;
                self.put_bounded(buf, usize::MAX, &text)
                    .map_err(Error::from)
            }
            "vsnprintf" => {
                // `int32 vsnprintf(char *buf, uint32 f, const char *format, AEEOldVaList list)` — the
                // one a guest pack reaches for, out of its allocator's trace hook.
                let (buf, room, fmt_at, list) = (
                    self.arg(0)?,
                    self.arg(1)? as usize,
                    self.arg(2)?,
                    self.arg(3)?,
                );
                let va = self.va_words(list)?;
                let fmt = Runtime::latin1_text(&self.cpu.mem.read_cstr(fmt_at, 1024)?);
                let mut next = 0;
                let text = self.format_raw(&fmt, &mut next, va)?;
                self.put_bounded(buf, room, &text).map_err(Error::from)
            }
            // `AECHAR *wwritelong(AECHAR *pszBuf, long n)` — the decimal of `n`, terminated, and the
            // answer is the pointer to that terminator, which is what lets a caller carry on writing
            // where this left off. The header says the buffer has room for twelve characters
            // (`'-'` + ten digits + `'\0'`), so no bound is second-guessed here.
            "wwritelong" => {
                let (buf, n) = (self.arg(0)?, self.arg(1)? as i32);
                let units: Vec<u16> = n.to_string().encode_utf16().collect();
                for (i, u) in units.iter().enumerate() {
                    self.cpu.mem.write_u16(buf + i as u32 * 2, u32::from(*u))?;
                }
                let end = buf + units.len() as u32 * 2;
                self.cpu.mem.write_u16(end, 0)?;
                Ok(end)
            }
            "wwritelongex" => {
                let (buf, value, pad, room_at) = (
                    self.arg(0)?,
                    self.arg(1)? as i32,
                    self.arg(2)? as i32,
                    self.arg(3)?,
                );
                let mut room = self.cpu.mem.read_u32(room_at)? as i32;
                if room == 0 {
                    return Ok(buf);
                }
                let mut at = buf;
                let mut digits: Vec<u16> = Vec::new();
                let mut magnitude = value as u32;
                if value < 0 {
                    self.cpu.mem.write_u16(at, u32::from(b'-'))?;
                    at += 2;
                    room -= 1;
                    magnitude = value.wrapping_neg() as u32;
                }
                while magnitude != 0 {
                    digits.push((magnitude % 10) as u16);
                    magnitude /= 10;
                }
                let count = digits.len() as i32;
                let mut width = pad;
                let mut pad_room = true;
                if width != 0 || count != 0 {
                    pad_room = width < 12;
                } else {
                    width = 1;
                }
                if pad_room && width > count && room > 0 {
                    for _ in 0..(width - count) {
                        self.cpu.mem.write_u16(at, u32::from(b'0'))?;
                        at += 2;
                        room -= 1;
                    }
                }
                while let Some(digit) = digits.pop() {
                    if room <= 0 {
                        break;
                    }
                    let cell = u32::from(digit) + u32::from(b'0');
                    self.cpu.mem.write_u16(at, cell)?;
                    at += 2;
                    room -= 1;
                }
                if room <= 0 {
                    at -= 2;
                }
                self.cpu.mem.write_u16(at, 0)?;
                self.cpu.mem.write_u32(room_at, room as u32)?;
                Ok(at)
            }
            // `void *setstaticptr(int nSPId, void *pNew)`, which is what `SETSTACKBOTTOM(p)` and
            // `SETTLS(p)` expand to. Both name a pointer the handset's C runtime keeps for itself for
            // stack checking, and the call's whole contract is the swap: keep the new one, hand back
            // the old one. Which static a module names is an argument, not a fixed pair, so the record
            // is keyed rather than indexed: a `Vec` grown to whatever number a register happened to
            // hold turns one wild word into a sixteen-gigabyte allocation, which kills the host
            // outright rather than answering the pack.
            "setstaticptr" => {
                let (which, pnew) = (self.arg(0)?, self.arg(1)?);
                Ok(self.static_ptr.insert(which, pnew).unwrap_or(0))
            }
            "f_calc" => {
                let (lo, hi, op) = (self.arg(0)?, self.arg(1)?, self.arg(2)?);
                let x = f64::from_bits((u64::from(hi) << 32) | u64::from(lo));
                let value = if op == g::FCALC_FLOOR {
                    x.floor()
                } else if op == g::FCALC_CEIL {
                    x.ceil()
                } else if op == g::FCALC_SQRT {
                    x.sqrt()
                } else if op == g::FCALC_SIN {
                    x.sin()
                } else if op == g::FCALC_COS {
                    x.cos()
                } else if op == g::FCALC_TAN {
                    x.tan()
                } else if op == g::FCALC_ABS {
                    x.abs()
                } else {
                    0.0
                };
                let bits = value.to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "f_op" => {
                let (alo, ahi, blo, bhi, op) = (
                    self.arg(0)?,
                    self.arg(1)?,
                    self.arg(2)?,
                    self.arg(3)?,
                    self.arg(4)?,
                );
                let x = f64::from_bits((u64::from(ahi) << 32) | u64::from(alo));
                let y = f64::from_bits((u64::from(bhi) << 32) | u64::from(blo));
                let value = if op == g::FO_ADD {
                    x + y
                } else if op == g::FO_SUB {
                    x - y
                } else if op == g::FO_MUL {
                    x * y
                } else if op == g::FO_DIV {
                    x / y
                } else if op == g::FO_POW {
                    x.powf(y)
                } else {
                    0.0
                };
                let bits = value.to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "f_cmp" => {
                let (alo, ahi, blo, bhi, op) = (
                    self.arg(0)?,
                    self.arg(1)?,
                    self.arg(2)?,
                    self.arg(3)?,
                    self.arg(4)?,
                );
                let x = f64::from_bits((u64::from(ahi) << 32) | u64::from(alo));
                let y = f64::from_bits((u64::from(bhi) << 32) | u64::from(blo));
                let answer = if op == g::FO_CMP_L {
                    x < y
                } else if op == g::FO_CMP_LE {
                    x <= y
                } else if op == g::FO_CMP_E {
                    x == y
                } else if op == g::FO_CMP_G {
                    x > y
                } else if op == g::FO_CMP_GE {
                    x >= y
                } else {
                    false
                };
                Ok(u32::from(answer))
            }
            "f_get" => {
                let which = self.arg(0)?;
                let bits: u64 = if which == g::FGET_HUGEVAL {
                    0x7ff0_0000_0000_0000
                } else if which == g::FGET_FLTMAX {
                    0x47ef_ffff_e000_0000
                } else if which == g::FGET_FLTMIN {
                    0x3810_0000_0000_0000
                } else {
                    0
                };
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "f_assignint" => {
                let bits = f64::from(self.arg(0)? as i32).to_bits();
                self.cpu.r[1] = (bits >> 32) as u32;
                Ok(bits as u32)
            }
            "dbgprintf" => {
                // `void dbgprintf(const char *fmt,...)`. The arguments *are* the message: a module
                // that says `SETTING ERROR(%s, %i)` is naming the setting it failed on, and printing
                // the format alone throws away the only part a reader can act on. So run it through the
                // engine `sprintf` uses. The register words stay behind it, and a format the engine
                // refuses falls back to the literal — a line with no substituted argument is still
                // worth having, and silently dropping the line is the failure mode to avoid.
                let addr = self.arg(0)?;
                let raw = self.cpu.mem.read_cstr(addr, 256)?;
                // The template is a narrow string in the device's own encoding. Its `%s`
                // arguments are decoded by the same engine below; a literal Japanese word in the
                // template has to arrive the same way.
                let fmt = crate::encoding::stream_text(u32::from(self.device.encoding), &raw);
                let mut next = 1;
                // Keep what could be formatted. `unwrap_or_else(|_| the delivered bodylone)` used to reprint the
                // whole template the moment one argument could not be read, so `fileName %s` arrived
                // as `fileName %s` and looked like a pack that logs format strings instead of a host
                // that ran out of argument registers. Naming the argument is the part that makes the
                // line actionable.
                let mut text = String::new();
                if let Err(e) = self.format_into(&fmt, &mut next, 0, &mut text, false) {
                    text.push_str(&format!(" [argument {next} could not be read: {e}]"));
                }
                let line = format!(
                    "[mod] {text} [r1={:08x} r2={:08x} r3={:08x}]",
                    self.cpu.r[1], self.cpu.r[2], self.cpu.r[3]
                );
                // Collected, not printed. This is the hot path of a talkative module — thousands of
                // calls a frame — and an `eprintln!` here is a synchronous write to a console, so a
                // pack can spend more time writing these lines than running its own code.
                self.keep_debug(line);
                Ok(0)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }
}
