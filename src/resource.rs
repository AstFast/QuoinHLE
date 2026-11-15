//! The handset's app resource containers, and the pictures inside them.
//!
//! A shipped app's art and text live in a `.bar` file — and, as it turns out, in a `.mif` font
//! too: both are the same container, found here by matching its signature across a third-party
//! corpus. The layout below was read off every one of them and holds on all of them, with the two
//! exceptions [`Container::parse`] says it tolerates:
//!
//! ```text
//! 0x00  u8 signature 0x11, u8 pad, u16 version, u16 oldest readable, u16 map entries
//! 0x08  u32 map_off, u32 map_bytes(=entries*8), u32 offsets_off, u32 resource count,
//!       u32 data_off, u32 data_bytes
//!       map: entries of { u16 kind, u16 first_id, u16 extra_ids, u16 offset_index }
//!            covering first_id..= first_id+extra_ids at consecutive table slots
//!       table: resource count + 1 *absolute* u32 file offsets; resource j is
//!            [table[j], table[j+1]) and the table's last entry is the end of the data, which
//!            a handful of shipped files leave a few bytes short of the real file size
//! ```
//!
//! Each resource starts with a property block — a u16 for its total length including those two
//! bytes, then NUL-terminated name/value pairs such as `image/bmp` — and the payload follows at
//! that length. For an image the payload is a whole Windows BMP, so no compression is involved.

use std::fmt;

/// `RESTYPE_*` as the containers spell them.
pub const RESTYPE_STRING: u16 = 1;
pub const RESTYPE_IMAGE: u16 = 6;
pub const RESTYPE_BINARY: u16 = 0x5000;

const SIGNATURE: u8 = 0x11;
const HEADER_BYTES: usize = 0x20;
const MAP_ENTRY_BYTES: usize = 8;

#[derive(Debug)]
pub enum Error {
    /// The bytes are not a container: wrong signature, a table that does not add up, or an offset
    /// outside the file. Named rather than partially accepted, because a container half-read would
    /// hand an app a picture made of the wrong bytes.
    Malformed(String),
    /// A shape this reader knows it cannot decode: OS/2 headers, RLE, and anything but 8, 24
    /// or 32 bits per pixel.
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Malformed(why) => write!(f, "not a resource container: {why}"),
            Error::Unsupported(what) => write!(f, "unsupported resource: {what}"),
        }
    }
}

impl std::error::Error for Error {}

type R<T> = Result<T, Error>;

fn u16_at(b: &[u8], at: usize) -> R<usize> {
    let Some(w) = b.get(at..at + 2) else {
        return Err(Error::Malformed(format!("no u16 at {at:#x}")));
    };
    Ok(usize::from(u16::from_le_bytes([w[0], w[1]])))
}

fn u32_at(b: &[u8], at: usize) -> R<usize> {
    let Some(w) = b.get(at..at + 4) else {
        return Err(Error::Malformed(format!("no u32 at {at:#x}")));
    };
    Ok(u32::from_le_bytes([w[0], w[1], w[2], w[3]]) as usize)
}

/// One resource, found by kind and id.
#[derive(Debug, Clone, Copy)]
pub struct Resource<'a> {
    pub kind: u16,
    pub id: u16,
    /// Everything after the property block: the BMP for an image, the text for a string.
    pub body: &'a [u8],
    /// The property block as written, NUL-separated: `image/bmp` for a picture.
    pub props: &'a [u8],
    /// The resource exactly as the container holds it - block and payload together. A shell that
    /// parses the block itself hands this to the app, and one game was measured doing its own
    /// skipping from its first byte, so the distinction belongs here rather than in each caller.
    pub stored: &'a [u8],
}

#[cfg(test)]
#[rustfmt::skip]  // a byte table, not code: twelve per line is how the fixture is read
pub const JPEG_GRAY: &[u8] = &[
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01,
    0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43,
    0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x00, 0x02,
    0x00, 0x03, 0x01, 0x01, 0x11, 0x00, 0xff, 0xc4, 0x00, 0x1f, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x10, 0x00, 0x02, 0x01, 0x03,
    0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7d,
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
    0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08,
    0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72,
    0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45,
    0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
    0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75,
    0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3,
    0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
    0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9,
    0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4,
    0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01,
    0x00, 0x00, 0x3f, 0x00, 0xfe, 0xa4, 0x3f, 0x66, 0x1f, 0xd9, 0x63, 0xf6,
    0x62, 0xff, 0x00, 0x85, 0x03, 0xf0, 0xb3, 0xfe, 0x31, 0xcb, 0xe0, 0x3f,
    0xfc, 0x8a, 0xf6, 0xff, 0x00, 0xf3, 0x48, 0x7e, 0x1f, 0x7f, 0xcf, 0xcd,
    0xcf, 0xfd, 0x4b, 0xd5, 0xff, 0xd9,
];
#[cfg(test)]
#[rustfmt::skip]  // a byte table, not code: twelve per line is how the fixture is read
/// The three-component partner of [`JPEG_GRAY`]: the arm the shipped `JPEGDecoder_Write` takes when
/// `c_jpeg_get_bytes_per_pixel` answers 3, which is the 24-bpp DIB.
pub const JPEG_RGB: &[u8] = &[
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01,
    0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43,
    0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0xff, 0xdb, 0x00, 0x43, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x03, 0x03,
    0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xff, 0xc4, 0x00,
    0x1f, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
    0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x10, 0x00,
    0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00,
    0x00, 0x01, 0x7d, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21,
    0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81,
    0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24,
    0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25,
    0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56,
    0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a,
    0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86,
    0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
    0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3,
    0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6,
    0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9,
    0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1,
    0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff, 0xc4, 0x00,
    0x1f, 0x01, 0x00, 0x03, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
    0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x11, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00,
    0x01, 0x02, 0x77, 0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31,
    0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08,
    0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15,
    0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18,
    0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39,
    0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55,
    0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84,
    0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
    0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa,
    0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4,
    0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7,
    0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea,
    0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff, 0xda, 0x00,
    0x0c, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00, 0xf0,
    0x3f, 0x12, 0xdd, 0xdd, 0x68, 0x9a, 0xc4, 0xfa, 0x46, 0x8d, 0x73, 0x71,
    0xa4, 0x69, 0x3a, 0x6d, 0xb6, 0x99, 0x67, 0xa7, 0x69, 0x7a, 0x64, 0xd2,
    0x58, 0x69, 0xd6, 0x16, 0x90, 0x69, 0x96, 0x71, 0xc1, 0x6b, 0x65, 0x65,
    0x6a, 0xd1, 0x5b, 0x5a, 0xdb, 0x43, 0x18, 0x09, 0x14, 0x10, 0x45, 0x1c,
    0x51, 0xa0, 0x0a, 0x8a, 0xa0, 0x01, 0x5f, 0xc9, 0x1e, 0x31, 0xf1, 0xbf,
    0x19, 0xd6, 0xf1, 0x2f, 0x8a, 0xf1, 0x35, 0x78, 0xbb, 0x89, 0xea, 0xe2,
    0x71, 0xb8, 0xcc, 0x2e, 0x61, 0x8c, 0xc4, 0x54, 0xcf, 0xf3, 0x59, 0xd7,
    0xc5, 0xe3, 0xf3, 0x0c, 0xb3, 0x05, 0x8e, 0xc7, 0xe3, 0xb1, 0x35, 0x65,
    0x8b, 0x75, 0x2b, 0xe3, 0x31, 0xd8, 0xdc, 0x45, 0x7c, 0x66, 0x33, 0x13,
    0x56, 0x53, 0xad, 0x89, 0xc5, 0x57, 0xad, 0x88, 0xad, 0x39, 0xd5, 0xa9,
    0x39, 0xcb, 0xfd, 0xac, 0xfa, 0x22, 0x78, 0x4d, 0xe1, 0x5f, 0x10, 0x7d,
    0x1d, 0xfc, 0x3b, 0xcd, 0xb3, 0xef, 0x0d, 0x3c, 0x3f, 0xce, 0xf3, 0x5c,
    0x4c, 0x38, 0xa6, 0x9e, 0x23, 0x32, 0xcd, 0xf8, 0x37, 0x87, 0x33, 0x2c,
    0xc2, 0xbc, 0x30, 0x9c, 0x6d, 0xc4, 0x98, 0x2c, 0x24, 0x2b, 0x63, 0x31,
    0x99, 0x6d, 0x6c, 0x4d, 0x58, 0x61, 0x70, 0x78, 0x7c, 0x3e, 0x13, 0x0f,
    0x19, 0xd4, 0x92, 0xa3, 0x86, 0xa1, 0x46, 0x85, 0x35, 0x1a, 0x54, 0xa1,
    0x08, 0xff, 0x00, 0xff, 0xd9,
];

/// A decoded container. Borrowed, not copied: the payloads stay where they are.
#[derive(Debug)]
pub struct Container<'a> {
    bytes: &'a [u8],
    map: usize,
    entries: usize,
    offsets: usize,
    count: usize,
}

impl<'a> Container<'a> {
    /// Read a container's header and check it adds up, without copying anything.
    pub fn parse(bytes: &'a [u8]) -> R<Self> {
        if bytes.len() < HEADER_BYTES {
            return Err(Error::Malformed(format!(
                "only {} bytes, a header alone is {HEADER_BYTES:#x}",
                bytes.len()
            )));
        }
        if bytes[0] != SIGNATURE {
            return Err(Error::Malformed(format!(
                "signature is {:02x}, not {SIGNATURE:02x}",
                bytes[0]
            )));
        }
        let c = Container {
            bytes,
            map: u32_at(bytes, 0x08)?,
            entries: u16_at(bytes, 0x06)?,
            offsets: u32_at(bytes, 0x10)?,
            count: u32_at(bytes, 0x14)?,
        };
        let first = c.at(c.offsets)?;
        // Reading the terminal slot *is* the check that matters about it: it has to be inside the
        // file. What it points at is a sentinel, and see below for why its value is not compared.
        let _sentinel = c.at(c.offsets + 4 * c.count)?;
        let data = u32_at(bytes, 0x18)?;
        // The layout facts that held on all 62 containers in this corpus: the map and the offset
        // table fit, the table stops before the data section, and the first resource starts exactly
        // where the header says the data does. Checked here so a wrong guess about the format is
        // caught at parse time rather than as a garbled picture later.
        if c.map + c.entries * MAP_ENTRY_BYTES > c.offsets {
            return Err(Error::Malformed(
                "the map runs past the offset table".into(),
            ));
        }
        if u32_at(bytes, 0x0c)? != c.entries * MAP_ENTRY_BYTES {
            return Err(Error::Malformed("map size does not match its count".into()));
        }
        if c.offsets + (c.count + 1) * 4 > data || first != data {
            return Err(Error::Malformed(format!(
                "the table ends at {:#x} and its first resource is at {first:#x}, but the data is \
                 at {data:#x}",
                c.offsets + (c.count + 1) * 4
            )));
        }
        // What is *not* required is that the file ends where the table's last entry or the header's
        // `data_bytes` says. Those two fields disagree with the file length on five of those 62
        // shipped containers — one art container claims 3 309 bytes more data than there are bytes —
        // and every span the reader walks is bounds-checked against the file anyway. So a stale
        // length costs nothing to tolerate, while refusing on it hides *every* resource in the
        // container behind one bad word: a font table asked for by name comes back NULL, and the app
        // faults tens of thousands of instructions later.
        //
        // The terminal table entry is read as part of that same lesson, not as a separate one. It is a
        // sentinel — no byte is ever served *from* it — and on the wider corpus the writers close a
        // table with it past the end of the file as a matter of course: of the 2 343 `.mif`/`.bar`
        // files across both corpora, **943 were rejected by this check alone** (the rest of that
        // survey: 1 278 clean, 112 without the `0x11` first byte, 4 too short for a header, 6 whose
        // table does not fit). The overrun is exactly one byte 705 times and within eight bytes 923
        // times, and those same files all satisfy `first == data`, which is the check that says this
        // really is this format rather than a lucky byte run.
        //
        // **None of the game corpus's 62 containers is among the 943**, and none of its 35 modules
        // changes which manifest it calls its own (measured: 35/35 still resolve through the ancestor
        // rule) — so tolerating this cannot change how any pack already read answers.
        //
        // Refusing costs the whole container — every picture, every string — and does so at fifteen
        // times the scale of the fault above. Tolerating it costs the records the overrunning tail bounds:
        // 929 of those 943 files lose exactly their last record and 14 lose two to four, 963 records
        // in all out of tens of thousands. A costed record stays unserved rather than served short —
        // [`Container::span`] declines it, `all` skips it, and `Store::find` answers "not here", all
        // existing policy. Nothing here widens what is served: a record whose bytes are not in the
        // file is not invented.
        // (Do not judge a container by whether
        // some field equals the file length.)
        Ok(c)
    }

    fn at(&self, abs: usize) -> R<usize> {
        u32_at(self.bytes, abs)
    }

    /// The offset of resource `index`, and where it ends.
    fn span(&self, index: usize) -> R<(usize, usize)> {
        if index >= self.count {
            return Err(Error::Malformed(format!("no resource #{index}")));
        }
        let start = self.at(self.offsets + 4 * index)?;
        let stop = self.at(self.offsets + 4 * (index + 1))?;
        if stop < start || stop > self.bytes.len() {
            return Err(Error::Malformed(format!(
                "resource #{index} spans {start:#x}..{stop:#x} in a {}-byte file",
                self.bytes.len()
            )));
        }
        Ok((start, stop))
    }

    /// Every resource in the container, in id order, with its body already split off.
    pub fn all(&self) -> R<Vec<Resource<'_>>> {
        let mut out = Vec::with_capacity(self.count);
        for e in 0..self.entries {
            let at = self.map + e * MAP_ENTRY_BYTES;
            let kind = u16_at(self.bytes, at)? as u16;
            let first = u16_at(self.bytes, at + 2)? as u16;
            let extra = u16_at(self.bytes, at + 4)?;
            let index = u16_at(self.bytes, at + 6)?;
            for k in 0..=u16::try_from(extra).unwrap_or(u16::MAX) {
                // One resource this reader cannot frame is left out; the rest of the container still
                // answers. Failing the whole file because a single payload's property block is not
                // the expected shape hides every other resource in it behind that one, and turns a
                // question the app asked into a container it never sees at all.
                //
                // The id is `first + k` in the format, and both are 16-bit: an entry that says
                // `first = 0xFFF8, extra = 0x20` runs past `u16::MAX`. Wrapping there would file the
                // payloads under ids near zero — answers to questions nobody asked — so an id the
                // sum cannot hold is dropped rather than wrapped.
                let Some(id) = first.checked_add(k) else {
                    continue;
                };
                let Ok((start, stop)) = self.span(index + usize::from(k)) else {
                    continue;
                };
                if let Ok(r) = self.resource(kind, id, start, stop) {
                    out.push(r);
                }
            }
        }
        Ok(out)
    }

    /// One resource by kind and id, as the handset looks it up.
    pub fn find(&self, kind: u16, id: u16) -> R<Option<Resource<'_>>> {
        for e in 0..self.entries {
            let at = self.map + e * MAP_ENTRY_BYTES;
            let k = u16_at(self.bytes, at)? as u16;
            let first = u16_at(self.bytes, at + 2)? as u16;
            let extra = u16_at(self.bytes, at + 4)?;
            let index = u16_at(self.bytes, at + 6)?;
            if k == kind && id >= first && usize::from(id - first) <= extra {
                let slot = index + usize::from(id - first);
                let (start, stop) = self.span(slot)?;
                return self.resource(kind, id, start, stop).map(Some);
            }
        }
        Ok(None)
    }

    /// Split a resource into its property block and its payload.
    ///
    /// The u16 counts itself, which is how the real files were written: a picture's block reads
    /// `0c 00 "image/bmp" 00` and the `BM` follows at offset `0x0c`.
    fn resource(&self, kind: u16, id: u16, start: usize, stop: usize) -> R<Resource<'a>> {
        let raw = self.bytes.get(start..stop).ok_or_else(|| {
            Error::Malformed(format!("resource {kind}/{id} lies outside the file"))
        })?;
        if kind != RESTYPE_STRING && kind != RESTYPE_IMAGE {
            // A binary resource is its payload from the first byte — the corpus shows the property
            // block belongs to the two kinds the shell hands back as objects and as text.
            return Ok(Resource {
                kind,
                id,
                props: &[],
                stored: raw,
                body: raw,
            });
        }
        let head = u16_at(raw, 0)?;
        if head < 2 || head > raw.len() {
            // A property block cannot be longer than the resource holding it, so a length that
            // fails that test is not a corrupt block but the absence of one: the payload starts at
            // the first byte. Two shapes in the corpus arrive here - a UTF-16 BOM, whose `fe ff`
            // reads as 0xfeff, and text whose first character is a printable byte. The readers
            // downstream still refuse what they cannot parse, so this widens what is *offered*
            // without widening what is claimed.
            return Ok(Resource {
                kind,
                id,
                props: &[],
                stored: raw,
                body: raw,
            });
        }
        Ok(Resource {
            kind,
            id,
            props: &raw[2..head],
            stored: raw,
            body: &raw[head..],
        })
    }
}

/// A picture, unpacked to one 8-bit grey or 24-bit RGB value per pixel, top row first.
///
/// 8-bit images keep their indices and carry the palette beside them: an app that tints a sprite
/// asks for the palette by index, and flattening to RGB would make that unanswerable.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    /// 8-bit: `width * height` indices. 24- and 32-bit: `width * height` RGB triples.
    pub pixels: Vec<u8>,
    /// 8-bit only: 256 BGR triples.
    pub palette: Option<Vec<u8>>,
    /// Per-pixel transparency the *file itself* named. Kept beside the pixels instead of blended into
    /// them, because the pack is entitled to read the colours it shipped: rewriting a see-through
    /// pixel into some mask colour would fabricate a byte the app never stored, and a picture is
    /// blitted far more often than it is read.
    pub alpha: Alpha,
}

/// How a decoded picture says which of its pixels are see-through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alpha {
    /// Nothing in the file named any pixel transparent, so every pixel is painted as stored. This is
    /// also the answer for a Windows BMP: a 32-bpp BMP's fourth byte is *padding*, and reading it as
    /// alpha would blank a picture whose author wrote zeros there — which is what most writers do.
    Opaque,
    /// An 8-bit indexed picture whose PNG `tRNS` chunk gave an alpha for the first few palette
    /// entries. `bytes[i]` is the alpha of index `i`; an index at or past the end of the chunk is
    /// fully opaque, which is what the chunk's own length is documented to mean.
    Index(Vec<u8>),
    /// A truecolour picture carrying its own alpha: one byte per pixel, row-major, same order as
    /// `Bitmap::pixels` walks.
    Pixel(Vec<u8>),
}

impl Alpha {
    /// True when the picture names at least one pixel that is not fully painted. Partial coverage is
    /// decided per pixel downstream (`AlphaMap::new` looks at the flattened picture, not at this
    /// table), because an indexed file can carry an alpha value on an entry no pixel ever uses.
    pub fn any_transparent(&self) -> bool {
        match self {
            Alpha::Opaque => false,
            Alpha::Index(a) | Alpha::Pixel(a) => a.iter().any(|&v| v != u8::MAX),
        }
    }
}

impl Bitmap {
    /// The alpha of one pixel, resolved through the palette when the picture is indexed.
    ///
    /// Out of range is opaque, not zero: [`Bitmap::rgb`] already answers a missing pixel with black
    /// rather than failing, and a black *and* invisible pixel would be a second invention on top of
    /// the first one.
    pub fn alpha_of(&self, x: usize, y: usize) -> u8 {
        let i = y * self.width + x;
        match &self.alpha {
            Alpha::Opaque => u8::MAX,
            Alpha::Pixel(a) => a.get(i).copied().unwrap_or(u8::MAX),
            Alpha::Index(a) => {
                let Some(&pixel) = self.pixels.get(i) else {
                    return u8::MAX;
                };
                a.get(usize::from(pixel)).copied().unwrap_or(u8::MAX)
            }
        }
    }

    /// The colour of one pixel, resolved through the palette when there is one.
    ///
    /// Out of range is answered, not panicked: a shipped container can declare a frame bigger than
    /// the bytes it supplies, and a caller that walks `0..height × 0..width` to build an icon would
    /// otherwise take the host down over a picture that is merely short.
    pub fn rgb(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let i = y * self.width + x;
        match &self.palette {
            Some(p) => {
                if i >= self.pixels.len() {
                    return (0, 0, 0);
                }
                let index = usize::from(self.pixels[i]);
                // A file may declare fewer table entries than its own depth can address — `biClrUsed`
                // of 9 on an 8-bpp picture is a shape that occurs in the shipped corpus — so a pixel
                // can name a colour the pack never supplied. Black is the answer that does not take
                // the host down with it, which an indexed read past the end of the table would.
                if index * 3 + 2 >= p.len() {
                    return (0, 0, 0);
                }
                (p[index * 3 + 2], p[index * 3 + 1], p[index * 3])
            }
            None => {
                if i * 3 + 2 >= self.pixels.len() {
                    return (0, 0, 0);
                }
                (
                    self.pixels[i * 3],
                    self.pixels[i * 3 + 1],
                    self.pixels[i * 3 + 2],
                )
            }
        }
    }
}

/// The largest image this reader will agree a bitmap occupies.
///
/// `CONVERTBMP` is handed a pointer and no length, so the length has to come out of the header the
/// app's own bytes carry. Without a cap, a buffer that is not a bitmap at all can ask the host to
/// copy anything up to four gigabytes.
pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// How many bytes the Windows bitmap at the start of `head` occupies in total, or `None` when `head`
/// is not a header this reader can size. `head` only has to be long enough to hold the header
/// itself; the pixels are not looked at here.
pub fn file_size(head: &[u8]) -> Option<usize> {
    // The file header is 14 bytes and the only header shape `bmp` accepts is the 40-byte
    // BITMAPINFOHEADER, so 54 bytes is everything this function needs.
    if head.len() < 54 || &head[0..2] != b"BM" {
        return None;
    }
    // Every offset below is inside the 54 bytes already checked, and the module's own readers answer
    // in `usize`, which is the width this sizing arithmetic wants.
    if u32_at(head, 14).ok()? != 40 {
        return None;
    }
    let (width, height) = (u32_at(head, 18).ok()? as i32, u32_at(head, 22).ok()? as i32);
    if width <= 0 || height == 0 || u16_at(head, 26).ok()? != 1 {
        return None;
    }
    let bits = u16_at(head, 28).ok()?;
    if !(bits <= 8 || matches!(bits, 24 | 32)) {
        return None;
    }
    let compressed = u32_at(head, 30).ok()?;
    if compressed != 0 && compressed != 3 {
        return None;
    }
    let palette = if bits <= 8 {
        let entries = match u32_at(head, 46).ok()? {
            0 => 1u64 << bits,
            n => n as u64,
        };
        // A table the file claims but `bmp` would refuse to read would size a buffer this function
        // cannot hand over, so it is treated as not-a-bitmap here too.
        if entries > 256 {
            return None;
        }
        entries * 4
    } else {
        0
    };
    let rows = height.unsigned_abs() as u64;
    let row_bytes = (width as u64 * bits as u64).div_ceil(32) * 4;
    let pixels = u32_at(head, 10).ok()? as u64;
    let smallest = 14 + 40 + palette;
    let pixels = pixels.max(smallest);
    let total = pixels.checked_add(row_bytes.checked_mul(rows)?)?;
    usize::try_from(total)
        .ok()
        .filter(|n| *n <= MAX_IMAGE_BYTES)
}

/// Read a Windows BMP, the format every image resource in the corpus turned out to be.
///
/// Bottom-up rows are flipped here so a caller never has to know; the row padding is dropped.
pub fn bmp(bytes: &[u8]) -> R<Bitmap> {
    if bytes.len() < 26 || &bytes[0..2] != b"BM" {
        return Err(Error::Malformed("an image resource is not a BMP".into()));
    }
    let header = u32_at(bytes, 14)?;
    // The planes and bitcount fields, and the palette that follows the header, are all placed
    // relative to the header's own declared size, so a size this function has not been checked
    // against is refused rather than read at guessed offsets.
    if header != 40 {
        return Err(Error::Unsupported(format!("BMP header size {header}")));
    }
    let width = u32_at(bytes, 18)? as i32;
    let height = u32_at(bytes, 22)? as i32;
    if width <= 0 || height == 0 {
        return Err(Error::Malformed(format!("BMP {width}x{height}")));
    }
    let top_down = height < 0;
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    let plane = u16_at(bytes, 26)?;
    if plane != 1 {
        return Err(Error::Malformed(format!("BMP with {plane} planes")));
    }
    let bits = u16_at(bytes, 28)?;
    let compressed = u32_at(bytes, 30)?;
    // 3 is BI_RGB as well, for the forms that carry bit masks.
    if compressed != 0 && compressed != 3 {
        return Err(Error::Unsupported(format!(
            "compressed BMP (biCompression {compressed})"
        )));
    }
    let start = u32_at(bytes, 10)?;
    // An indexed depth packs several pixels into a byte, most significant first, and the row is
    // still padded to four bytes — which is the part a reader gets wrong by counting pixels.
    let indexed = matches!(bits, 1 | 2 | 4 | 8);
    let row_bytes = if indexed {
        (w * bits + 7) / 8
    } else {
        match bits {
            24 => w * 3,
            32 => w * 4,
            other => return Err(Error::Unsupported(format!("{other}-bit BMP"))),
        }
    };
    let stride = (row_bytes + 3) & !3;
    let need = start + stride * h;
    if need > bytes.len() {
        return Err(Error::Malformed(format!(
            "BMP pixels start at {start:#x} and need {need:#x}, the resource is {}",
            bytes.len()
        )));
    }
    let palette = if indexed {
        // biClrUsed sits 32 bytes into the info header; zero means the whole table for the depth.
        let used = u32_at(bytes, 46)?;
        let entries = if used == 0 {
            1usize << bits
        } else {
            used.min(256)
        };
        let mut p = vec![0u8; entries * 3];
        for i in 0..entries {
            let at = 14 + header as usize + i * 4;
            let quad = bytes
                .get(at..at + 4)
                .ok_or_else(|| Error::Malformed(format!("palette entry {i} is missing")))?;
            // The quad is B,G,R,reserved and [`Bitmap::palette`] is documented as BGR triples, which is
            // also what [`Bitmap::rgb`] indexes and what the PNG reader converts its PLTE into. Copy the
            // three bytes as they lie: storing them as RGB here hands every indexed picture back with
            // its red and blue exchanged.
            p[i * 3..i * 3 + 3].copy_from_slice(&quad[..3]);
        }
        // Trimmed to what the file declares, because `AEEImageInfo.nColors` is a `uint8` and a
        // table padded to its depth would report 256 - which truncates to 0, the same answer as
        // "no palette", for a picture that has one.
        Some(p)
    } else {
        None
    };
    let mut out = vec![0u8; if indexed { w * h } else { w * h * 3 }];
    for row in 0..h {
        // A BMP's first row is the bottom unless the height was negative.
        let src = if top_down { row } else { h - 1 - row };
        let from = start + stride * src;
        for col in 0..w {
            let dst = row * w + col;
            if indexed {
                let bit = col * bits;
                let shift = 8 - bits - bit % 8;
                out[dst] = (bytes[from + bit / 8] >> shift) as u8 & (0xff >> (8 - bits));
            } else {
                let at = from + col * if bits == 32 { 4 } else { 3 };
                let px = &bytes[at..at + 3];
                let dst = dst * 3;
                out[dst] = px[2];
                out[dst + 1] = px[1];
                out[dst + 2] = px[0];
            }
        }
    }
    Ok(Bitmap {
        width: w,
        height: h,
        pixels: out,
        palette,
        // A BMP names no transparency anywhere, and a 32-bpp one's fourth byte is padding.
        alpha: Alpha::Opaque,
    })
}

/// One byte per sample, for a stream that packed `depth` of them into each byte.
///
/// PNG fills a byte from its most significant end and pads the end of the row to a byte boundary, so
/// the row length is `ceil(width * depth / 8)` and the tail bits of a narrow row belong to no pixel.
/// `grey` scales the sample into an eight-bit brightness - a `d`-bit grey stands for
/// `v * 255 / (2^d - 1)`, which is what the format's own expansion says - while an index is left as
/// the number it is, because it addresses a palette table rather than a brightness.
/// The eight-bit brightness a `d`-bit grey sample stands for: `v * 255 / (2^d - 1)`, which is what
/// the format's own expansion says. Spelled once because a picture's samples and a `tRNS` colour key
/// naming one of them have to go through the *same* scale, or the key of a narrow picture compares
/// against values it can never reach.
fn grey_brightness(v: u16, depth: u8) -> u8 {
    let full = (1u16 << depth) - 1;
    (v * 255 / full) as u8
}

fn expand_bits(packed: &[u8], depth: u8, w: usize, h: usize, grey: bool) -> Vec<u8> {
    let per_byte = (8 / depth) as usize;
    let row = w.div_ceil(per_byte);
    let full = (1u16 << depth) - 1;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let bit = x * depth as usize;
            let Some(&byte) = packed.get(y * row + bit / 8) else {
                out.push(0);
                continue;
            };
            let shift = (8 - depth as usize - bit % 8) as u32;
            let v = (u16::from(byte) >> shift) & full;
            let sample = if grey {
                grey_brightness(v, depth)
            } else {
                // An index below eight bits is at most 255 by construction, since it is masked by
                // `full` above; it is left as the number itself because it addresses a palette entry.
                v as u8
            };
            out.push(sample);
        }
    }
    out
}

/// The picture in a PNG resource, in the same shape [`bmp`] answers with.
///
/// A pack's art is not always a BMP: some packs keep every image they have as a PNG, with the
/// property block saying so, and asking a viewer for one has to be answered from the PNG decoder
/// rather than with a null — a null gets stored in a field and dereferenced later. The result
/// is normalized exactly as the BMP path normalizes its own: eight-bit indices with a palette when
/// the file has one, RGB triples otherwise, so nothing above this function can tell which container
/// a picture arrived in.
pub fn png(bytes: &[u8]) -> R<Bitmap> {
    // A `Cursor` rather than the slice itself: the decoder wants to seek, and a resource is already
    // a whole in-memory buffer.
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // 16-bit samples have to be narrowed before anything below can read them, because every path
    // here - `Bitmap::rgb`, `alpha_of`, the batched row blit, `expand_bits` above eight bits -
    // walks one byte per channel. The default is the identity transform, so a 16-bit file used to
    // arrive with two bytes per channel and every pixel past the first was read from its colour's
    // low byte: `chunks_exact(3)` over that stream yields 8/3 times as many "pixels" as the file
    // has, and `Rgba`/`GrayscaleAlpha` sliced alpha out of the wrong bytes.
    //
    // Only `STRIP_16`, deliberately: `EXPAND` would flatten an indexed picture to RGB and turn its
    // `tRNS` into per-pixel alpha, and keeping the indices plus the palette beside them is what the
    // ABI needs an 8-bit `IBitmap` to be able to answer (`Bitmap::palette`).
    decoder.set_transformations(png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::Malformed(format!("PNG header: {e}")))?;
    // An interlaced or otherwise unbounded frame has no single buffer size to allocate, and this
    // host has no use for one it cannot lay out row by row.
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| Error::Unsupported("a PNG with no fixed frame size".into()))?;
    // The header decides this number, so it is a budget the file asks the host to spend, not one
    // the host chose: a few dozen bytes of `IHDR` can name a frame of gigabytes. The limit is the
    // same one `IBitmap` and the BMP path already work under — anything larger is a picture this
    // host has no way to hand to an app.
    if size > MAX_IMAGE_BYTES {
        return Err(Error::Unsupported(format!(
            "a PNG frame of {size} bytes, over the {MAX_IMAGE_BYTES}-byte limit"
        )));
    }
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| Error::Malformed(format!("PNG data: {e}")))?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width as usize, info.height as usize);
    // A bit depth under eight arrives *packed*: `output_buffer_size` is `ceil(width * depth / 8)`
    // bytes per row and the decoder's transformations default to `IDENTITY`, so the buffer holds
    // several indices in one byte while every reader above this one - `Bitmap::rgb` and
    // `Bitmap::alpha_of` - walks one byte per pixel. Over the corpus, 15 of 921 PNG records are
    // indexed at depth 1, 2 or 4, and 13 of those carry `tRNS`, so the stride is
    // not cosmetic here; left packed it would name the wrong pixels transparent.
    let depth = info.bit_depth as u8;
    let grey = info.color_type == png::ColorType::Grayscale;
    let indexed = info.color_type == png::ColorType::Indexed;
    if depth < 8 && (indexed || grey) {
        buf = expand_bits(&buf, depth, w, h, grey);
    }

    // The palette goes back as BGR, which is what [`Bitmap::rgb`] indexes with - the containers and
    // the BMP reader both settled on that order, and the PNG one is RGB.
    let palette = |rgb: Vec<u8>| -> Option<Vec<u8>> {
        let mut bgr = vec![0u8; (rgb.len() / 3) * 3];
        for (i, px) in rgb.chunks_exact(3).enumerate() {
            bgr[i * 3] = px[2];
            bgr[i * 3 + 1] = px[1];
            bgr[i * 3 + 2] = px[0];
        }
        Some(bgr)
    };

    match info.color_type {
        png::ColorType::Rgb => {
            // A truecolour `tRNS` names *one colour*, not a per-pixel alpha. The pack can say the same
            // thing through `IBitmap::SetTransparencyColor` / `IDIB::ncTransparent`, and the header
            // makes the *platform* pick the raster op, so the file's answer is
            // honoured here rather than dropped - as coverage, with the colours left exactly as stored.
            // That is also what a libpng-based decoder is told to do (`libpng-manual.txt`: "if
            // (png_get_valid(..., PNG_INFO_tRNS)) png_set_tRNS_to_alpha(...)"), and both real images
            // carry libpng's own error strings.
            //
            // The key arrives as three bytes because the decoder normalises a chunk of 16-bit samples
            // for a picture shallower than 16 bits (`stream.rs`, `parse_trns`, `Rgb` arm). A genuine
            // 16-bit file keeps the six-byte form and this reader does not narrow *keys* - only samples
            // (`STRIP_16`) - so such a picture stays opaque. libpng compares 16-bit keys at 16-bit
            // precision; guessing which half to keep instead of doing that is a rule no witness here
            // states, and the wrong guess paints a pixel transparent a file never named.
            let alpha = match reader.info().trns.as_deref() {
                Some(k) if k.len() == 3 => Alpha::Pixel(
                    buf.chunks_exact(3)
                        .map(|px| if px == k { 0 } else { u8::MAX })
                        .collect(),
                ),
                _ => Alpha::Opaque,
            };
            Ok(Bitmap {
                width: w,
                height: h,
                pixels: buf,
                palette: None,
                alpha,
            })
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(w * h * 3);
            for g in &buf {
                out.extend_from_slice(&[*g, *g, *g]);
            }
            // One sample, so one key byte after the same normalisation. A greyscale pixel equal to it
            // is the transparent one; the value itself is left in the picture, because a pack that
            // reads the pixels back is entitled to the bytes the file stored.
            //
            // Below eight bits the buffer holds `expand_bits`' scaled brightness while the key is still
            // a raw sample in `0..2^d-1`, so the key goes through `grey_brightness` too - otherwise a
            // key of 1 in a one-bit picture, the only meaningful transparent value that file can name,
            // compares against 255 and matches nothing. libpng states the same rule: the transparency
            // colour "should be supplied in the same format/depth as the current image data"
            // (`libpng-manual.txt`). A key outside the picture's range scales to a value no sample can
            // hold, and names nothing.
            let key = match reader.info().trns.as_deref() {
                Some(k) if k.len() == 1 => Some(if depth < 8 {
                    grey_brightness(u16::from(k[0]), depth)
                } else {
                    k[0]
                }),
                _ => None,
            };
            let alpha = match key {
                Some(key) => Alpha::Pixel(
                    buf.iter()
                        .map(|&g| if g == key { 0 } else { u8::MAX })
                        .collect(),
                ),
                None => Alpha::Opaque,
            };
            Ok(Bitmap {
                width: w,
                height: h,
                pixels: out,
                palette: None,
                alpha,
            })
        }
        png::ColorType::Rgba => {
            // The alpha is kept and taken out of the triples: the picture's own colours stay exactly
            // as the file stored them, because the pack may read them back, and what the alpha says is
            // a question for the blitter, not a byte to overwrite.
            //
            // A stray `tRNS` on such a file - PNG's own format error - never reaches this arm: the
            // decoder drops it as a benign ancillary error (the `!is_critical` arm of `parse_chunk`),
            // so `info.trns` is `None` and this channel stays the only authority. PIL answers the same
            // way for the same bytes, which is why there is no test for it here:
            // nothing inside this function can change that answer.
            let alpha: Vec<u8> = buf.chunks_exact(4).map(|px| px[3]).collect();
            let mut out = Vec::with_capacity(w * h * 3);
            for px in buf.chunks_exact(4) {
                out.extend_from_slice(&px[..3]);
            }
            Ok(Bitmap {
                width: w,
                height: h,
                pixels: out,
                palette: None,
                alpha: Alpha::Pixel(alpha),
            })
        }
        png::ColorType::GrayscaleAlpha => {
            let alpha: Vec<u8> = buf.chunks_exact(2).map(|px| px[1]).collect();
            let mut out = Vec::with_capacity(w * h * 3);
            for px in buf.chunks_exact(2) {
                out.extend_from_slice(&[px[0], px[0], px[0]]);
            }
            Ok(Bitmap {
                width: w,
                height: h,
                pixels: out,
                palette: None,
                alpha: Alpha::Pixel(alpha),
            })
        }
        png::ColorType::Indexed => {
            let table = reader.info().palette.clone().unwrap_or_default();
            // `tRNS` for an indexed picture is one alpha byte for each of the first few palette
            // entries; entries it does not reach are opaque. This is the shape 746 records in the
            // corpus use - 443 of them one pack's tiles - and without it their see-through background
            // paints as its stored white.
            let alpha = match reader.info().trns.as_ref().map(|t| t.to_vec()) {
                Some(a) if !a.is_empty() => Alpha::Index(a),
                _ => Alpha::Opaque,
            };
            Ok(Bitmap {
                width: w,
                height: h,
                pixels: buf,
                palette: palette(table.to_vec()),
                alpha,
            })
        }
    }
}

pub fn jpeg(bytes: &[u8]) -> R<Bitmap> {
    use jpeg_decoder::{Decoder, PixelFormat};
    let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
    decoder
        .read_info()
        .map_err(|e| Error::Malformed(format!("JPEG header: {e}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Malformed("a JPEG that declared no frame".into()))?;
    let (width, height) = (usize::from(info.width), usize::from(info.height));
    let per = info.pixel_format.pixel_bytes();
    // The header's own frame is the only size to allocate: this host cannot lay out a picture an
    // `IBitmap` cannot name (`Runtime::raster` refuses one), so the decoder is pinned to the declared
    // product and cannot grow past it.
    let total = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(per))
        .ok_or_else(|| Error::Malformed(format!("JPEG {width}x{height} overflows a frame size")))?;
    // `set_max_decoding_buffer_size(total)` pins the decoder to the declared product, which stops it
    // growing past the header — but a header is all a file needs to declare a frame of tens of
    // gigabytes, and a limit that equals the request is not a limit. The ceiling is the same one the
    // BMP and PNG paths work under.
    if total > MAX_IMAGE_BYTES {
        return Err(Error::Unsupported(format!(
            "a JPEG frame of {total} bytes, over the {MAX_IMAGE_BYTES}-byte limit"
        )));
    }
    decoder.set_max_decoding_buffer_size(total);
    let buf = decoder
        .decode()
        .map_err(|e| Error::Malformed(format!("JPEG data: {e}")))?;
    match info.pixel_format {
        PixelFormat::L8 => {
            // The shipped DIB is 8 bpp and holds the luminance itself. The grey ramp beside it is this
            // host's choice, and it is said as one: `Bitmap::rgb` resolves an index through the palette,
            // so an indexed picture with no table would answer black for every pixel. Nothing in the
            // delivered members says what the shipped `CreateDIB` left in that block.
            Ok(Bitmap {
                width,
                height,
                pixels: buf,
                palette: Some((0u8..=255).flat_map(|g| [g, g, g]).collect()),
                alpha: Alpha::Opaque,
            })
        }
        PixelFormat::RGB24 => Ok(Bitmap {
            width,
            height,
            pixels: buf,
            palette: None,
            alpha: Alpha::Opaque,
        }),
        PixelFormat::CMYK32 => {
            // Three bytes out of every four, the way the shipped loop does it — the fourth component is
            // dropped rather than composited, so a CMYK stream comes back as its first three channels.
            let mut rgb = Vec::with_capacity(width * height * 3);
            for pixel in buf.chunks_exact(4) {
                rgb.extend_from_slice(&pixel[0..3]);
            }
            Ok(Bitmap {
                width,
                height,
                pixels: rgb,
                palette: None,
                alpha: Alpha::Opaque,
            })
        }
        other => Err(Error::Unsupported(format!(
            "a JPEG frame in {other:?}, which the shipped JPEG writer has no branch for"
        ))),
    }
}

/// The three formats this module has a decoder for, spelled the way a container's property block
/// spells them — which is also the way `IShell::DetectType` has to spell its answer.
///
/// Constants rather than a literal at each use, because the same set carries two roles that have
/// to agree: an arm of the switch below, and the name a caller is handed before it passes the
/// payload to that switch. A type named in one place and decoded in the other is exactly the drift
/// [`sniff`] exists to rule out, so both go through these words.
pub const MIME_PNG: &str = "image/png";
pub const MIME_BMP: &str = "image/bmp";
pub const MIME_JPEG: &str = "image/jpeg";
pub const MIME_JPG: &str = "image/jpg";
/// GIF. No delivered member decodes it — see [`gif`], which says what that means for the answer this
/// host gives at `IShell::DetectType`.
pub const MIME_GIF: &str = "image/gif";

/// A file-name suffix that is a shorter spelling of a MIME in [`DECODABLE`], rather than a format of its
/// own. It is one pair because [`sniff`] can only ever answer [`MIME_JPEG`]: no payload says whether its
/// author meant to write `jpeg` or `jpg`, and a name a caller cannot load back is the drift [`DECODABLE`]
/// exists to prevent.
const SUFFIX_ALIASES: [(&str, &str); 1] = [("jpg", MIME_JPEG)];

/// Every MIME [`image`] will then agree to decode, and so every MIME [`sniff`] may name.
const DECODABLE: [&str; 4] = [MIME_PNG, MIME_BMP, MIME_JPEG, MIME_GIF];

/// The eight bytes every PNG stream opens with.
///
/// Named because two readers ask the same question of a payload that carries no property block:
/// [`image`] decides which decoder to call, and [`sniff`] decides what to call the answer. Those
/// two cannot be allowed to disagree about what a PNG starts with.
pub const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The three bytes every JPEG stream opens with: `SOI` followed by the first byte of the next marker.
///
/// The same pair of readers asks the same question as [`PNG_MAGIC`] does — [`image`] with no property
/// block, [`sniff`] with no name — and both must land on the same arm.
pub const JPEG_MAGIC: &[u8] = b"\xFF\xD8\xFF";

/// The four bytes both GIF versions open with — `GIF87a` and `GIF89a` share them, so the magic does not
/// have to know which one it is looking at, and a stream that says `GIF8` is a stream [`gif`] can try.
pub const GIF_MAGIC: &[u8] = b"GIF8";

pub fn gif(bytes: &[u8]) -> R<Bitmap> {
    use gif::{ColorOutput, DecodeOptions};
    if !bytes.starts_with(GIF_MAGIC) {
        return Err(Error::Malformed("a GIF stream that does not say so".into()));
    }
    let mut options = DecodeOptions::new();
    // Indices, not the crate's RGBA default: the picture's own colour table is the answer an indexed
    // `IBitmap` has to be able to give back, and expanding it here would throw the table away.
    options.set_color_output(ColorOutput::Indexed);
    let mut decoder = options
        .read_info(std::io::Cursor::new(bytes))
        .map_err(|e| Error::Malformed(format!("GIF header: {e}")))?;
    let frame = decoder
        .read_next_frame()
        .map_err(|e| Error::Malformed(format!("GIF data: {e}")))?
        .ok_or_else(|| Error::Malformed("a GIF with no frame in it".into()))?;
    let (width, height) = (usize::from(frame.width), usize::from(frame.height));
    if width == 0 || height == 0 {
        return Err(Error::Malformed(format!(
            "GIF frame {width}x{height} names no picture"
        )));
    }
    let pixels = frame.buffer.to_vec();
    // Taken here rather than where it is used: the frame borrows the decoder mutably, and the
    // effective colour table is a question for the decoder afterwards.
    let transparent = frame.transparent;
    // The de-interlaced frame, exactly as wide as its own header says: a short buffer is a stream that
    // lied about its size, and padding it here would invent rows the file does not have.
    if pixels.len() != width * height {
        return Err(Error::Malformed(format!(
            "GIF frame {width}x{height} delivered {} byte(s)",
            pixels.len()
        )));
    }
    let rgb = decoder
        .palette()
        .map_err(|e| Error::Malformed(format!("GIF colour table: {e}")))?
        .to_vec();
    if rgb.len() < 3 || rgb.len() % 3 != 0 {
        return Err(Error::Malformed(format!(
            "a GIF colour table of {} byte(s), which is not a list of triples",
            rgb.len()
        )));
    }
    // BGR, because that is the order `Bitmap::rgb` indexes with — the same normalization the PNG path
    // applies to its own RGB table.
    let mut bgr = vec![0u8; rgb.len()];
    for (i, px) in rgb.chunks_exact(3).enumerate() {
        bgr[i * 3] = px[2];
        bgr[i * 3 + 1] = px[1];
        bgr[i * 3 + 2] = px[0];
    }
    // GIF names one transparent index per frame, so the table is the whole story: that entry is
    // see-through and every other one is painted as stored.
    let alpha = match transparent {
        None => Alpha::Opaque,
        Some(t) => {
            let mut table = vec![u8::MAX; bgr.len() / 3];
            let at = usize::from(t);
            if at < table.len() {
                table[at] = 0;
            } else {
                // The file pointed its transparent pen past its own colour table. Saying so out loud is
                // the point: an index nobody can resolve is drawn as whatever its colour is, which is
                // what a decoder that silently ignored it would claim is fine.
                return Err(Error::Malformed(format!(
                    "a GIF transparent index {at} past a table of {} entries",
                    table.len()
                )));
            }
            Alpha::Index(table)
        }
    };
    Ok(Bitmap {
        width,
        height,
        pixels,
        palette: Some(bgr),
        alpha,
    })
}

/// The picture a resource holds, whichever of the formats this host can read it arrived in.
///
/// The property block names the format and is the thing to trust, not the payload's first bytes:
/// the corpus files MIDI and text under the image kind, so a magic-number test alone would call a
/// `.mid` a picture. A resource that names no format falls back to its own contents, because the
/// `.par` reader synthesizes a kind from the payload and has no property block to offer.
///
/// Not every format an app ships is this host's to learn. A pack may keep art in its own formats
/// (`image/zbm`, `znt`, `zyn`, `zmf`) and decode them itself: it asks for them through
/// `IShell::LoadResData` - the data path, which hands over the stored bytes - selects them with
/// `strstr(mime, "/z")`, and runs its own inflate over what it gets. Only PNG reaches a viewer
/// request from that pack. So refusing a `z*` name here is the right answer rather than a gap: a
/// decoder below would be work the app has already done, on bytes it never asks this function for.
pub fn image(props: &[u8], bytes: &[u8]) -> R<Bitmap> {
    match mime(props) {
        Some(name) if name == MIME_PNG => png(bytes),
        Some(name) if name == MIME_BMP => bmp(bytes),
        Some(name) if name == MIME_JPEG || name == MIME_JPG => jpeg(bytes),
        Some(name) if name == MIME_GIF => gif(bytes),
        // No declared type: the payload has to name itself.
        None => {
            if bytes.starts_with(PNG_MAGIC) {
                png(bytes)
            } else if bytes.starts_with(JPEG_MAGIC) {
                jpeg(bytes)
            } else if bytes.starts_with(GIF_MAGIC) {
                gif(bytes)
            } else {
                bmp(bytes)
            }
        }
        Some(other) => Err(Error::Unsupported(format!("image format {other}"))),
    }
}

/// What these bytes are, in the only sense this host can honestly mean: the name [`image`] will
/// then decode them under, or `None` when there is no such name.
///
/// This is [`IShell::DetectType`](crate::runtime)'s answer, and the reason it is here rather than
/// in the dispatch is the reason it decodes rather than matches magics. A caller of that slot takes
/// the MIME string and goes off to *load* the payload with it, so a name produced by a two-byte
/// prefix test is a claim this module then has to honour: `image/png` from a sniffer, `not a
/// picture this host decodes` from the viewer, and the pack is left holding a value it has no way
/// to disbelieve. Running the decoder is the only answer that cannot drift out of sync with it.
///
/// What that costs is precision on a *partial* buffer: the first eight bytes of a real PNG name
/// nothing here, because eight bytes are not a picture this module can decode. That is the
/// deliberate direction of the error — a refusal the caller can ask again about, rather than a type
/// the caller's own load then fails on.
///
/// A format with no decoder below is not named at all. GIF *was* the case in point — no delivered member
/// decodes it — until [`gif`] added one on this side, which is why the sentence still matters: what makes
/// `image/gif` nameable now is that [`image`] will agree to decode it, not that the extracted members
/// would. Read [`gif`] before treating a GIF that draws here as a verdict either way about a real handset:
/// its symbol census is an account of this set, and the same search over a shipped ROM image answers 0 for
/// formats that phone serves.
///
/// What it costs is a decode: a caller that sniffs a picture and then loads it pays for the
/// inflate twice. That is the price of the claim being worth making, and the alternative — a
/// prefix test — is the thing this function exists to avoid.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(PNG_MAGIC) && png(bytes).is_ok() {
        return Some(MIME_PNG);
    }
    if bytes.starts_with(JPEG_MAGIC) && jpeg(bytes).is_ok() {
        return Some(MIME_JPEG);
    }
    if bytes.starts_with(GIF_MAGIC) && gif(bytes).is_ok() {
        return Some(MIME_GIF);
    }
    // `bmp` says no inside its first four checks, so a payload that is neither is not paying for a
    // second decode.
    bmp(bytes).is_ok().then_some(MIME_BMP)
}

/// What a *name* is, for the caller that has the one and not the other.
///
/// No bytes, so nothing can be decoded and the suffix is all there is. The suffix is therefore not
/// free to invent a vocabulary: it is the subtype of a MIME in [`DECODABLE`], so the two answers
/// this can give are the two [`image`] has an arm for. `audio/mid` and the private `image/z*` names
/// answer `None`, which is the difference between "this is not a picture" and "we have no decoder"
/// — the corpus files MIDI under the image resource kind, and a sniffer that guessed from the
/// extension there would have called a song a picture. `audio/mid` is refused *here* and named by
/// [`crate::audio::sniff_name`], which is where the authority behind that name lives;
/// `IShell::DetectType` asks the two in turn.
///
/// The gap [`sniff`] closes is not filled here: a `.png` whose bytes are a truncated PNG still
/// answers `image/png`, because the caller that names only a name has no payload for this host to
/// check, and saying "unknown" about `art.png` would be a worse answer than the one it asks for.
pub fn sniff_name(name: &str) -> Option<&'static str> {
    let file = name.rsplit(['/', '\\']).next()?;
    let (_, ext) = file.rsplit_once('.')?;
    DECODABLE
        .into_iter()
        .find(|mime| {
            // `png` and `bmp` are what these formats answer to in a file name, in a container's property
            // block and in a handler registry all at once, so the suffix is read out of the MIME instead
            // of being listed beside it: one list of decodable formats rather than two that a later
            // commit can update separately.
            mime.rsplit_once('/').is_some_and(|(_, subtype)| {
                !subtype.is_empty() && subtype.eq_ignore_ascii_case(ext)
            })
        })
        .or_else(|| {
            // The one exception, spelled as one: a `.jpg` is `image/jpeg` and nothing else this host can
            // decode, and only [`sniff`] could tell the two spellings apart — which it cannot.
            SUFFIX_ALIASES
                .iter()
                .find(|(suffix, _)| suffix.eq_ignore_ascii_case(ext))
                .map(|(_, mime)| *mime)
        })
}

/// What a payload is, according to its own property block.
///
/// The resource's *kind* only says which table the handset filed it under, and the corpus files
/// plenty of things under the wrong one: music filed as `RESTYPE_IMAGE` holding `audio/mid`, and a
/// "Loading." caption filed as `RESTYPE_IMAGE` holding `text/plain`. So the difference between "this
/// reader is missing a decoder" and "this was never a picture" is here, not in the kind number.
pub fn mime(props: &[u8]) -> Option<&str> {
    // The block is NUL-separated fields, and the MIME is the first.
    let first = props.split(|c| *c == 0).next()?;
    let text = std::str::from_utf8(first).ok()?;
    (!text.is_empty()).then_some(text)
}

/// Shift-JIS bytes as the `AECHAR` code units a face is looked up by.
///
/// The same decoding serves a `FD FE`-marked resource string and a narrow string the guest
/// passes to `STREXPAND`: both are byte streams in the device's `AEE_ENC_S_JIS` encoding, and
/// widening one byte at a time is what turns `83 65` ("テ") into two Latin-1 code units.
pub fn decode_shift_jis(bytes: &[u8]) -> Vec<u16> {
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    text.encode_utf16().collect()
}

/// The inverse: `AECHAR` units to the Shift-JIS bytes a `char *` buffer holds.
pub fn encode_shift_jis(units: &[u16]) -> Vec<u8> {
    let text: String = units
        .iter()
        .filter_map(|u| char::from_u32(u32::from(*u)))
        .collect();
    encoding_rs::SHIFT_JIS.encode(&text).0.into_owned()
}

/// The language strings of a `RESTYPE_STRING` resource, as the containers hold them.
///
/// Two shapes reach here. The documented one is one count byte, then that many strings separated by
/// `^` and closed by `~`; the first is the language the container was built for — English on every
/// file in the corpus, Spanish on the localized one — and the handset picks by index. The other is a
/// `.mif`'s own metadata — vendor, version, app name — which is **one plain string with no count in
/// front of it**: `Popcap`, whose first byte is `0x50`. Reading that as "80 languages" is what this
/// function used to do to a file that had exactly one string to give.
///
/// The terminator tells the two apart, because a counted resource always ends with `~` and none of
/// the plain ones in the corpus contains one at all — which is a better test than whether the first
/// byte looks like a small number, since half the alphabet does.
pub fn strings(body: &[u8]) -> R<Vec<Vec<u16>>> {
    if body.is_empty() {
        return Err(Error::Malformed("an empty string resource".into()));
    }
    // KDDI's containers put `FD FE` in front of every Japanese string: `0xFD` is
    // `AEE_ENC_S_JIS` and the bytes behind the pair are Shift-JIS, not UTF-16. Widening those
    // bytes one per byte turns `83 65` ("テ") into two Latin-1 code units, which is exactly the
    // mojibake a menu shows when its text is read as Latin. The two bytes are a marker, not
    // characters, so they are dropped.
    if let Some(rest) = body.strip_prefix(&[0xfd, 0xfe][..]) {
        return Ok(vec![decode_shift_jis(rest)]);
    }
    // The big-endian spelling of the UTF-16 mark. No corpus file carries it yet; the little-endian
    // one below is what the packing tools write, and this keeps the pair from disagreeing.
    if let Some(rest) = body.strip_prefix(&[0xfe, 0xff][..]) {
        return Ok(vec![rest
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect()]);
    }
    // A `.bar` files every string behind a leading `0x03`, one per record: all sixty-two
    // `RESTYPE_STRING` records of `chess_comb.bar` begin with it, while the three plain strings in
    // the manifest beside it do not. It is the container's "a string starts here" tag and not a
    // character — a menu that printed it drew a box in front of every label, which is exactly what
    // the first chess run did. Nothing in the corpus stores `U+0003` as text, so the tag is dropped
    // rather than turned into a cell.
    let body = body.strip_prefix(&[0x03][..]).unwrap_or(body);
    let wide = |text: &[u8]| text.iter().map(|b| u16::from(*b)).collect::<Vec<u16>>();
    // A manifest written by a packing tool stores its text as UTF-16 behind a byte-order mark, and
    // those bytes are already `AECHAR`s: widening one per byte hands the caller `U+00FF, U+00FE, 'v'`
    // where the handset gives it `v1.0.90222`, and a count of twice as many characters as it filled.
    // A manifest may be this shape or plain bytes, so both readings live here. The mark
    // itself is dropped — it says how the pairs are ordered, and is not a character of the string, so
    // a pack that prints the version it asked for would otherwise print a box where no box belongs.
    if let Some(rest) = body.strip_prefix(&[0xff, 0xfe]) {
        return Ok(vec![rest
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect()]);
    }
    if body.last() != Some(&b'~') {
        return Ok(vec![wide(body)]);
    }
    let Some((&count, rest)) = body.split_first() else {
        return Err(Error::Malformed("an empty string resource".into()));
    };
    let count = usize::from(count);
    if count == 0 || count > 64 {
        return Err(Error::Malformed(format!(
            "a string resource claiming {count} languages"
        )));
    }
    let mut out = Vec::with_capacity(count);
    let mut field = Vec::new();
    let mut at = 0;
    while at < rest.len() {
        let b = rest[at];
        at += 1;
        if b == b'^' || b == b'~' {
            out.push(field);
            field = Vec::new();
            if b == b'~' {
                break;
            }
            continue;
        }
        field.push(b);
    }
    if out.len() < count {
        return Err(Error::Malformed(format!(
            "{} languages found, {count} claimed",
            out.len()
        )));
    }
    Ok(out.into_iter().take(count).map(|s| wide(&s)).collect())
}

/// A container written the way the shipped files are, for tests that need one without a corpus: eight
/// header words, one map entry per record (each covering a single id), an offset table with one more
/// entry than there are records, and every payload behind its own `len u16 + NUL-separated block`.
///
/// `sentinel_slack` is added to the table's final entry. That entry bounds the *end* of the last
/// record and is never itself served, and the collection writes it past the end of the file as a
/// matter of course — 943 of its 2 343 `.mif`/`.bar` files, 705 of them by exactly one byte — so a
/// test that wants the shape the writers actually produce has to be able to ask for it.
#[cfg(test)]
pub(crate) fn synthetic_container(records: &[(u16, u16, &[u8])], sentinel_slack: usize) -> Vec<u8> {
    let block: &[u8] = b"\x0c\x00image/bmp\x00";
    let entries = records.len();
    let map = HEADER_BYTES;
    let offsets = map + entries * MAP_ENTRY_BYTES;
    let data = offsets + (entries + 1) * 4;
    let bodies: Vec<Vec<u8>> = records
        .iter()
        .map(|(_, _, payload)| [block, payload].concat())
        .collect();
    let mut starts = vec![data];
    for body in &bodies {
        starts.push(starts.last().unwrap() + body.len());
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&[SIGNATURE, 0]);
    for word in [1u16, 1, entries as u16] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in [
        map as u32,
        (entries * MAP_ENTRY_BYTES) as u32,
        offsets as u32,
        entries as u32,
        data as u32,
        bodies.iter().map(|b| b.len()).sum::<usize>() as u32,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    assert_eq!(out.len(), map, "the header is one block of eight words");
    for (index, (kind, id, _)) in records.iter().enumerate() {
        for word in [*kind, *id, 0u16, index as u16] {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
    for (i, start) in starts.iter().enumerate() {
        // `i == entries` is the sentinel slot: the end of the last record.
        let value = if i == entries {
            start.saturating_add(sentinel_slack)
        } else {
            *start
        };
        out.extend_from_slice(&(value as u32).to_le_bytes());
    }
    for body in &bodies {
        out.extend_from_slice(body);
    }
    out
}

#[cfg(test)]
#[path = "test/resource.rs"]
mod tests;
