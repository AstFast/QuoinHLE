//! A device firmware image, reduced to the loadable segments a backend can map.
//!
//! The graphics stack a BREW handset ships is not something that can be linked against or dlopen'd:
//! it is a slice of the phone's own address space, built to run at fixed addresses and to reach the
//! rest of the firmware by absolute address. A backend that would rather execute that code than
//! imitate it therefore needs the map before it needs anything else — which `PT_LOAD` covers which
//! page of the guest's memory, and which bytes of the file become those pages.
//!
//! What this module refuses matters as much as what it returns. A carved image routinely carries a
//! segment whose file range runs past the end of the buffer, or a program-header table that points
//! outside it; taking either at face value would have the loader read whatever followed. Every range
//! is checked here, before any of it is mapped, because the image is untrusted input like any other
//! pack byte.

/// One `PT_LOAD` segment, as the header describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Its index in the program-header table, for messages that have to name it.
    pub index: usize,
    pub vaddr: u32,
    pub file_offset: u32,
    pub file_size: u32,
    /// `memsz`, which is what a mapping has to reserve; the tail past `file_size` starts zeroed.
    pub mem_size: u32,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl Segment {
    /// The address one past the last byte this segment occupies.
    pub fn end(&self) -> u64 {
        u64::from(self.vaddr) + u64::from(self.mem_size)
    }

    /// Whether `addr` falls inside the memory this segment reserves, not just its file bytes.
    pub fn holds(&self, addr: u32) -> bool {
        u64::from(addr) >= u64::from(self.vaddr) && u64::from(addr) < self.end()
    }
}

/// Why an image could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Shorter than an ELF header, or shorter than the table it points at.
    TooShort,
    NotElf,
    Not32Bit,
    NotLittleEndian,
    /// Not `EM_ARM`: the guest is ARM machine code, and another machine's segments mean nothing here.
    NotArm,
    /// Not `ET_EXEC`. A relocatable or shared image has no fixed addresses to run at.
    NotExecutable,
    /// `e_phentsize` smaller than the header it must hold, or a table that does not fit the file.
    BadSegmentTable,
    /// A segment asks for bytes the file does not have.
    SegmentPastFile { index: usize, offset: u32, size: u32 },
    /// `vaddr + memsz` leaves the 32-bit address space.
    SegmentOutsideAddressSpace { index: usize },
    /// The address named is not the start of an interworking slot in this segment.
    NotAVeneer { slot: u32 },
    /// The slot is not inside the segment's file bytes.
    SlotOutsideSegment { slot: u32 },
}

/// A parsed image: its entry point and its loadable segments, in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    entry: u32,
    segments: Vec<Segment>,
}

const ELF_HEADER: usize = 52;
const PROGRAM_HEADER: usize = 32;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS32: u8 = 1;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_ARM: u16 = 40;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn half(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

impl Image {
    /// Read the program headers of an ARM `ET_EXEC` image.
    ///
    /// Sections are not consulted: a carved image usually has none, and nothing here needs them. A
    /// segment with `memsz == 0` is dropped — it reserves no memory and cannot be the reason a
    /// caller asked for the image.
    pub fn parse(bytes: &[u8]) -> Result<Image, Error> {
        if bytes.len() < ELF_HEADER {
            return Err(Error::TooShort);
        }
        if bytes[0..4] != ELF_MAGIC[..] {
            return Err(Error::NotElf);
        }
        if bytes[4] != ELFCLASS32 {
            return Err(Error::Not32Bit);
        }
        if bytes[5] != ELFDATA2LSB {
            return Err(Error::NotLittleEndian);
        }
        if half(bytes, 18) != EM_ARM {
            return Err(Error::NotArm);
        }
        if half(bytes, 16) != ET_EXEC {
            return Err(Error::NotExecutable);
        }
        let entry = word(bytes, 24);
        let phoff = word(bytes, 28) as usize;
        let phentsize = half(bytes, 42) as usize;
        let phnum = half(bytes, 44) as usize;
        if phnum == 0 {
            // An image with no segments has nothing to map; reporting success would invite a caller
            // to map nothing and wonder why the code it wanted is not there.
            return Err(Error::BadSegmentTable);
        }
        if phentsize < PROGRAM_HEADER {
            return Err(Error::BadSegmentTable);
        }
        // The multiplication is done in `usize` and the sum checked against the buffer, so a
        // crafted `phnum` cannot wrap either one.
        let table = phnum
            .checked_mul(phentsize)
            .and_then(|n| n.checked_add(phoff))
            .ok_or(Error::BadSegmentTable)?;
        if table > bytes.len() {
            return Err(Error::BadSegmentTable);
        }
        let mut segments = Vec::new();
        for index in 0..phnum {
            let at = phoff + index * phentsize;
            let kind = word(bytes, at);
            if kind != PT_LOAD {
                continue;
            }
            let file_offset = word(bytes, at + 4);
            let vaddr = word(bytes, at + 8);
            let file_size = word(bytes, at + 16);
            let mem_size = word(bytes, at + 20);
            let flags = word(bytes, at + 24);
            if mem_size == 0 {
                continue;
            }
            let end = u64::from(file_offset) + u64::from(file_size);
            if end > bytes.len() as u64 {
                return Err(Error::SegmentPastFile {
                    index,
                    offset: file_offset,
                    size: file_size,
                });
            }
            if mem_size < file_size {
                // `memsz` below `filesz` is malformed, and treating it as `filesz` would hide a
                // table we do not understand.
                return Err(Error::SegmentPastFile {
                    index,
                    offset: file_offset,
                    size: file_size,
                });
            }
            if u64::from(vaddr) + u64::from(mem_size) > u64::from(u32::MAX) + 1 {
                return Err(Error::SegmentOutsideAddressSpace { index });
            }
            segments.push(Segment {
                index,
                vaddr,
                file_offset,
                file_size,
                mem_size,
                readable: flags & PF_R != 0,
                writable: flags & PF_W != 0,
                executable: flags & PF_X != 0,
            });
        }
        if segments.is_empty() {
            return Err(Error::BadSegmentTable);
        }
        Ok(Image { entry, segments })
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The first segment whose reserved memory covers `addr`.
    pub fn segment_at(&self, addr: u32) -> Option<&Segment> {
        self.segments.iter().find(|s| s.holds(addr))
    }

    /// The bytes a segment loads from the file. Its tail past `file_size` is not part of the file
    /// and is expected to be zeroed by whoever maps it.
    pub fn file_bytes<'a>(&self, image: &'a [u8], segment: &Segment) -> &'a [u8] {
        let start = segment.file_offset as usize;
        let end = start + segment.file_size as usize;
        // `parse` proved the range fits, so this cannot panic on an image it accepted; the guard is
        // for a `Segment` that came from somewhere else.
        if end > image.len() {
            return &[];
        }
        &image[start..end]
    }

    /// The smallest span of addresses that covers every segment, as `(start, end)`.
    ///
    /// A caller that would rather map one window than a dozen segments can use this, at the cost of
    /// touching addresses in between that no segment owns.
    pub fn span(&self) -> (u32, u32) {
        let start = self.segments.iter().map(|s| s.vaddr).min().unwrap_or(0);
        let end = self
            .segments
            .iter()
            .map(|s| s.end())
            .max()
            .unwrap_or(0)
            .min(u64::from(u32::MAX) + 1) as u32;
        (start, end)
    }
}

/// One slot of an ARM interworking table: `ldr pc,[pc,#-4]` followed by the address it jumps to.
///
/// The vendor code that runs in a handset is Thumb, and the firmware libraries under it are ARM, so
/// every call that crosses the boundary goes through a slot like this — the compiler's answer to a
/// branch that also has to change state. That makes the table the one place a host can stand between
/// the vendor's code and everything it calls, without knowing where in its 5 000 functions a slot is
/// used: rewrite the target and every caller follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Veneer {
    /// Address of the `ldr`; the target word sits at `slot + 4`.
    pub slot: u32,
    /// The address the slot jumps to, Thumb bit included.
    pub target: u32,
}

/// `ldr pc, [pc, #-4]`, little-endian: the only instruction a slot may start with.
const VENEER_LDR: [u8; 4] = [0x04, 0xf0, 0x1f, 0xe5];

/// Every slot in the segment whose target `is_code` accepts.
///
/// The predicate is the caller's, because only the caller knows what the image's own address space
/// looks like: a word that happens to follow the instruction pattern is worth nothing unless it
/// points at something the caller would be willing to jump to. Slots are reported in address order.
pub fn scan(image: &[u8], segment: &Segment, mut is_code: impl FnMut(u32) -> bool) -> Vec<Veneer> {
    let start = segment.file_offset as usize;
    let end = start + segment.file_size as usize;
    if end > image.len() || segment.file_size < 8 {
        return Vec::new();
    }
    let bytes = &image[start..end];
    let mut found = Vec::new();
    // ARM instructions are four-byte aligned, and so is every slot the compiler emits.
    let mut at = 0usize;
    while at + 8 <= bytes.len() {
        if bytes[at..at + 4] == VENEER_LDR[..] {
            let target = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]);
            if is_code(target) {
                found.push(Veneer {
                    slot: segment.vaddr + at as u32,
                    target,
                });
            }
        }
        at += 4;
    }
    found
}

/// Keep the slots that sit in a run of at least two consecutive entries.
///
/// A single match is a plausible coincidence — the four bytes appear in ordinary code and in data —
/// while a compiler-emitted table is contiguous by construction. Dropping the lone ones keeps a
/// caller from retargeting a word that was never a branch.
pub fn within_runs(veneers: &[Veneer]) -> Vec<Veneer> {
    veneers
        .iter()
        .copied()
        .filter(|v| {
            veneers.iter().any(|o| {
                o.slot == v.slot.wrapping_add(8) || v.slot == o.slot.wrapping_add(8)
            })
        })
        .collect()
}

/// Point one slot at `target`, leaving its instruction alone.
///
/// Refuses a slot that is not in the segment's file bytes, or that does not currently hold the
/// instruction: rewriting a word that only looks like a slot would corrupt code that is about to run.
pub fn set_target(image: &mut [u8], segment: &Segment, slot: u32, target: u32) -> Result<(), Error> {
    let start = segment.file_offset as usize;
    let end = start + segment.file_size as usize;
    if !segment.holds(slot) || slot < segment.vaddr || end > image.len() {
        return Err(Error::SlotOutsideSegment { slot });
    }
    let at = start + (slot - segment.vaddr) as usize;
    if at + 8 > end || image[at..at + 4] != VENEER_LDR[..] {
        return Err(Error::NotAVeneer { slot });
    }
    image[at + 4..at + 8].copy_from_slice(&target.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal image: `segments` are `(vaddr, offset, filesz, memsz, flags)`.
    fn image(machine: u16, segments: &[(u32, u32, u32, u32, u32)]) -> Vec<u8> {
        let phoff = ELF_HEADER as u32;
        let mut bytes = vec![0u8; ELF_HEADER + PROGRAM_HEADER * segments.len() + 64];
        bytes[0..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = ELFCLASS32;
        bytes[5] = ELFDATA2LSB;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&phoff.to_le_bytes());
        bytes[32..34].copy_from_slice(&(ELF_HEADER as u16).to_le_bytes());
        bytes[34..36].copy_from_slice(&(PROGRAM_HEADER as u16).to_le_bytes());
        bytes[42..44].copy_from_slice(&(PROGRAM_HEADER as u16).to_le_bytes());
        bytes[44..46].copy_from_slice(&(segments.len() as u16).to_le_bytes());
        for (i, &(vaddr, off, filesz, memsz, flags)) in segments.iter().enumerate() {
            let at = ELF_HEADER + i * PROGRAM_HEADER;
            bytes[at..at + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
            bytes[at + 4..at + 8].copy_from_slice(&off.to_le_bytes());
            bytes[at + 8..at + 12].copy_from_slice(&vaddr.to_le_bytes());
            bytes[at + 16..at + 20].copy_from_slice(&filesz.to_le_bytes());
            bytes[at + 20..at + 24].copy_from_slice(&memsz.to_le_bytes());
            bytes[at + 24..at + 28].copy_from_slice(&flags.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn every_loadable_segment_is_kept_and_nothing_else_is() {
        let bytes = image(
            EM_ARM,
            &[(0x1015_0000, 0, 8, 0x1000, PF_R | PF_X), (0, 0, 0, 0, PF_R | PF_W)],
        );
        let parsed = Image::parse(&bytes).unwrap();
        assert_eq!(parsed.segments().len(), 1);
        assert_eq!(parsed.entry(), 0x1234_5678);
        let only = parsed.segments()[0];
        assert_eq!(only.vaddr, 0x1015_0000);
        assert!(only.executable && only.readable && !only.writable);
    }

    #[test]
    fn a_segment_is_located_by_any_address_it_reserves() {
        let bytes = image(EM_ARM, &[(0x1015_0000, 0, 8, 0x1000, PF_R | PF_X)]);
        let parsed = Image::parse(&bytes).unwrap();
        assert!(parsed.segment_at(0x1015_0000).is_some());
        assert!(parsed.segment_at(0x1015_0fff).is_some());
        // The tail past the file bytes is still the segment's memory.
        assert!(parsed.segment_at(0x1015_0800).is_some());
        assert!(parsed.segment_at(0x1014_ffff).is_none());
        assert!(parsed.segment_at(0x1015_1000).is_none());
    }

    #[test]
    fn a_program_header_table_past_the_end_of_the_file_is_rejected() {
        let mut bytes = image(EM_ARM, &[(0x1015_0000, 0, 8, 0x100, PF_R | PF_X)]);
        bytes.truncate(ELF_HEADER + PROGRAM_HEADER - 1);
        assert_eq!(Image::parse(&bytes), Err(Error::BadSegmentTable));
    }

    #[test]
    fn a_segment_whose_bytes_run_past_the_file_is_rejected() {
        let bytes = image(EM_ARM, &[(0x1015_0000, 0, 0xffff, 0x1_0000, PF_R | PF_X)]);
        assert_eq!(
            Image::parse(&bytes),
            Err(Error::SegmentPastFile {
                index: 0,
                offset: 0,
                size: 0xffff
            })
        );
    }

    #[test]
    fn an_image_for_another_machine_is_not_mistaken_for_this_one() {
        let bytes = image(3, &[(0x1015_0000, 0, 8, 0x100, PF_R | PF_X)]);
        assert_eq!(Image::parse(&bytes), Err(Error::NotArm));
    }

    #[test]
    fn a_file_that_is_not_an_elf_is_refused_before_anything_is_read() {
        assert_eq!(Image::parse(&[0u8; 64]), Err(Error::NotElf));
        assert_eq!(Image::parse(&[0x7f, b'E']), Err(Error::TooShort));
    }

    #[test]
    fn the_file_bytes_of_a_segment_are_the_range_its_header_names() {
        let mut bytes = image(EM_ARM, &[(0x1015_0000, 0x60, 4, 0x10, PF_R | PF_X)]);
        bytes[0x60..0x64].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        let parsed = Image::parse(&bytes).unwrap();
        let only = parsed.segments()[0];
        assert_eq!(parsed.file_bytes(&bytes, &only), &[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(only.end(), 0x1015_0010);
    }

    /// A segment whose bytes are the given slots, each `ldr pc,[pc,#-4]` plus its target word.
    fn slots(vaddr: u32, targets: &[u32]) -> (Vec<u8>, Segment) {
        let mut bytes = Vec::with_capacity(targets.len() * 8);
        for t in targets {
            bytes.extend_from_slice(&VENEER_LDR);
            bytes.extend_from_slice(&t.to_le_bytes());
        }
        let size = bytes.len() as u32;
        let segment = Segment {
            index: 0,
            vaddr,
            file_offset: 0,
            file_size: size,
            mem_size: size,
            readable: true,
            writable: false,
            executable: true,
        };
        (bytes, segment)
    }

    fn is_vendor_code(target: u32) -> bool {
        (0x1025_0000..0x1026_0000).contains(&(target & !1))
    }

    #[test]
    fn a_run_of_slots_is_found_and_each_target_is_the_word_after_its_instruction() {
        let (bytes, segment) = slots(0x105a_1700, &[0x1025_50ed, 0x1025_8a79, 0x1025_a0f5]);
        let found = scan(&bytes, &segment, is_vendor_code);
        assert_eq!(
            found,
            vec![
                Veneer { slot: 0x105a_1700, target: 0x1025_50ed },
                Veneer { slot: 0x105a_1708, target: 0x1025_8a79 },
                Veneer { slot: 0x105a_1710, target: 0x1025_a0f5 },
            ]
        );
        assert_eq!(within_runs(&found), found);
    }

    #[test]
    fn a_target_the_caller_would_not_jump_to_is_not_a_slot() {
        let (bytes, segment) = slots(0x105a_1700, &[0x1025_50ed, 0xdead_beef]);
        let found = scan(&bytes, &segment, is_vendor_code);
        assert_eq!(found.len(), 1);
        // And a lone match is dropped once runs are required: it is not a run of one.
        assert!(within_runs(&found).is_empty());
    }

    #[test]
    fn a_slot_can_be_retargeted_without_losing_its_instruction() {
        let (mut bytes, segment) = slots(0x105a_1700, &[0x1025_50ed, 0x1025_8a79]);
        set_target(&mut bytes, &segment, 0x105a_1700, 0x0abc_1234).unwrap();
        assert_eq!(&bytes[0..4], &VENEER_LDR);
        assert_eq!(&bytes[4..8], &0x0abc_1234u32.to_le_bytes());
        // The untouched neighbour still reads as its own target.
        assert_eq!(&bytes[12..16], &0x1025_8a79u32.to_le_bytes());
    }

    #[test]
    fn retargeting_refuses_a_word_that_is_not_a_slot() {
        let (mut bytes, segment) = slots(0x105a_1700, &[0x1025_50ed, 0x1025_8a79]);
        assert_eq!(
            set_target(&mut bytes, &segment, 0x105a_1704, 0x1),
            Err(Error::NotAVeneer { slot: 0x105a_1704 })
        );
        assert_eq!(
            set_target(&mut bytes, &segment, 0x105b_0000, 0x1),
            Err(Error::SlotOutsideSegment { slot: 0x105b_0000 })
        );
    }
}
