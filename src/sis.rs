//! A carrier `.sis` (SAF/SIS) pictures.
//!
//! The file splits one raw DEFLATE stream in two: a type-11 chunk carries the dynamic Huffman
//! header, and each type-4 frame names its own payload. The payload opens with a 16-bit zlib
//! header and a 3-bit block header, neither of which belongs to the block the container actually
//! stores, so decoding means stitching the header back in front of the payload and inflating the
//! result as raw DEFLATE. The palette is the handset's master table rather than anything in the
//! file.
//!
//! The split-stream layout and the master palette follow `third_party/ReSIS-X` (MIT).

use crate::resource::{Alpha, Bitmap, Error, MAX_IMAGE_BYTES, SAF_MAGIC, SIS_MAGIC};
use std::io::Read;

const CONTAINER_HEADER: usize = 12;
const MAX_FRAMES: usize = 128;
/// Total indexed pixels one animation may ask this host to hold (64 MiB at one byte each).
const MAX_TOTAL_FRAME_PIXELS: usize = 64 * 1024 * 1024;

/// The order DEFLATE's code-length alphabet is stored in.
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

type R<T> = Result<T, Error>;

fn bad(why: impl Into<String>) -> Error {
    Error::Malformed(why.into())
}

fn be16(bytes: &[u8], at: usize) -> R<usize> {
    let at_bytes = bytes
        .get(at..at + 2)
        .ok_or_else(|| bad(format!("no u16 at {at:#x}")))?;
    Ok(usize::from(u16::from_be_bytes([at_bytes[0], at_bytes[1]])))
}

/// LSB-first, the order DEFLATE writes its bits in.
struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    buf: u32,
    count: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte: 0,
            buf: 0,
            count: 0,
        }
    }

    fn read(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        while self.count < n {
            // Reading a zero past the end mirrors the handset's reader; the callers that can run
            // out of bytes check the position before asking.
            let byte = self.data.get(self.byte).copied().unwrap_or(0);
            self.byte += 1;
            self.buf |= u32::from(byte) << self.count;
            self.count += 8;
        }
        let value = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.count -= n;
        value
    }

    fn bit_pos(&self) -> u32 {
        self.byte as u32 * 8 - self.count
    }
}

struct BitWriter {
    out: Vec<u8>,
    buf: u32,
    count: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            buf: 0,
            count: 0,
        }
    }

    fn bits(&mut self, value: u32, n: u32) {
        self.buf |= (value & ((1u32 << n) - 1)) << self.count;
        self.count += n;
        while self.count >= 8 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.count -= 8;
        }
    }

    fn flush(&mut self) {
        if self.count > 0 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf = 0;
            self.count = 0;
        }
    }
}

struct HuffmanEntry {
    symbol: u16,
    length: u8,
    code: u16,
}

/// Canonical Huffman codes from DEFLATE's code lengths.
///
/// The handset pads the longest code length so the tree is complete before assigning codes; a
/// stream produced against that padding decodes with the same padding and not otherwise.
fn huffman_build(lengths: &[u8]) -> Vec<HuffmanEntry> {
    let max_len = lengths.iter().copied().max().unwrap_or(0) as usize;
    if max_len == 0 {
        return Vec::new();
    }
    let mut counts = [0u32; 16];
    for &len in lengths {
        if len > 0 {
            counts[usize::from(len)] += 1;
        }
    }
    let Some(min_len) = (1..=max_len).find(|&l| counts[l] > 0) else {
        return Vec::new();
    };
    let mut missing = 1i32 << min_len;
    for l in min_len..max_len {
        missing -= counts[l] as i32;
        missing <<= 1;
    }
    missing -= counts[max_len] as i32;
    if missing >= 0 {
        counts[max_len] += missing as u32;
    }

    let mut next_code = [0u32; 16];
    let mut code = 0u32;
    for i in 1..=max_len {
        code = (code + counts[i - 1]) << 1;
        next_code[i] = code;
    }

    let mut entries = Vec::new();
    for (symbol, &len) in lengths.iter().enumerate() {
        if len > 0 {
            let at = usize::from(len);
            entries.push(HuffmanEntry {
                symbol: symbol as u16,
                length: len,
                code: next_code[at] as u16,
            });
            next_code[at] += 1;
        }
    }
    entries
}

fn huffman_decode(entries: &[HuffmanEntry], reader: &mut BitReader) -> Option<u16> {
    let mut code = 0u32;
    for len in 1..=15u32 {
        if reader.count == 0 && reader.byte >= reader.data.len() {
            return None;
        }
        code = (code << 1) | reader.read(1);
        if let Some(entry) = entries
            .iter()
            .find(|entry| u32::from(entry.length) == len && u32::from(entry.code) == code)
        {
            return Some(entry.symbol);
        }
    }
    None
}

/// How many bits of the type-11 chunk make up the dynamic Huffman header.
fn huffman_header_bits(data: &[u8]) -> R<u32> {
    let mut reader = BitReader::new(data);
    let head = reader.read(14);
    let literals = (head & 0x1F) as usize + 257;
    let distances = ((head >> 5) & 0x1F) as usize + 1;
    let code_lengths = ((head >> 10) & 0xF) as usize + 4;
    let total = literals + distances;

    let mut lengths = [0u8; 19];
    for &slot in CODE_LENGTH_ORDER.iter().take(code_lengths) {
        lengths[slot] = reader.read(3) as u8;
    }
    let tree = huffman_build(&lengths);

    let mut out = Vec::with_capacity(total);
    while out.len() < total {
        let symbol = huffman_decode(&tree, &mut reader)
            .ok_or_else(|| bad("SIS Huffman chunk ends inside its header"))?;
        match symbol {
            0..=15 => out.push(symbol as u8),
            16 => {
                let repeat = reader.read(2) as usize + 3;
                let last = out.last().copied().unwrap_or(0);
                if out.len() + repeat > total {
                    return Err(bad("SIS Huffman repeat runs past its alphabet"));
                }
                out.resize(out.len() + repeat, last);
            }
            17 => {
                let repeat = reader.read(3) as usize + 3;
                if out.len() + repeat > total {
                    return Err(bad("SIS Huffman repeat runs past its alphabet"));
                }
                out.resize(out.len() + repeat, 0);
            }
            18 => {
                let repeat = reader.read(7) as usize + 11;
                if out.len() + repeat > total {
                    return Err(bad("SIS Huffman repeat runs past its alphabet"));
                }
                out.resize(out.len() + repeat, 0);
            }
            _ => return Err(bad("SIS Huffman header names a symbol past 18")),
        }
    }
    Ok(reader.bit_pos())
}

/// Rebuild the raw DEFLATE block from the Huffman chunk and one frame payload, then inflate it.
fn inflate_frame(chunk4: &[u8], chunk11: &[u8], header_bits: u32, expected: usize) -> R<Vec<u8>> {
    if expected == 0 || expected > MAX_IMAGE_BYTES {
        return Err(Error::Unsupported(format!(
            "a SIS frame of {expected} byte(s), outside the {MAX_IMAGE_BYTES}-byte limit"
        )));
    }
    if chunk4.len() < 3 {
        return Err(bad("SIS frame payload is shorter than its zlib header"));
    }
    if u64::from(header_bits) > chunk11.len() as u64 * 8 {
        return Err(bad("SIS Huffman header is longer than its chunk"));
    }

    let mut stream = BitWriter::new();
    // BFINAL=1, BTYPE=2: one dynamic-Huffman block, written LSB-first.
    stream.bits(5, 3);

    let mut header = BitReader::new(chunk11);
    for _ in 0..header_bits {
        stream.bits(header.read(1), 1);
    }

    let mut payload = BitReader::new(chunk4);
    // The stored prefix is a zlib CMF/FLG pair and the block header the split removed.
    payload.read(19);
    let payload_bits = chunk4.len() as u32 * 8 - 19;
    for _ in 0..payload_bits {
        stream.bits(payload.read(1), 1);
    }
    stream.flush();

    let mut out = vec![0u8; expected];
    let mut decoder = flate2::read::DeflateDecoder::new(stream.out.as_slice());
    decoder
        .read_exact(&mut out)
        .map_err(|e| bad(format!("SIS frame inflate: {e}")))?;
    Ok(out)
}

struct Frame {
    width: usize,
    height: usize,
    chunk4: std::ops::Range<usize>,
}

struct Parsed<'a> {
    huffman: Option<&'a [u8]>,
    transparent: Option<u8>,
    frames: Vec<Frame>,
}

fn parse(bytes: &[u8]) -> R<Parsed<'_>> {
    if bytes.len() < CONTAINER_HEADER
        || !(bytes.starts_with(SAF_MAGIC) || bytes.starts_with(SIS_MAGIC))
    {
        return Err(bad("not a SAF/SIS picture"));
    }
    let header_bytes = be16(bytes, 10)?;
    let mut off = CONTAINER_HEADER
        .checked_add(header_bytes)
        .ok_or_else(|| bad("SIS header length overflows"))?;
    if off > bytes.len() {
        return Err(bad("SIS header runs past the file"));
    }

    let mut huffman: Option<&[u8]> = None;
    let mut transparent = None;
    let mut frames: Vec<Frame> = Vec::new();

    while off + 3 <= bytes.len() {
        let kind = bytes[off];
        if kind == 0xFF {
            break;
        }
        let len = be16(bytes, off + 1)?;
        let body = off + 3;
        let end = body
            .checked_add(len)
            .ok_or_else(|| bad("SIS chunk length overflows"))?;
        if end > bytes.len() {
            return Err(bad("SIS chunk runs past the file"));
        }
        match kind {
            11 => huffman = Some(&bytes[body..end]),
            6 if len >= 1 => transparent = Some(bytes[body]),
            4 if len >= 5 => {
                let desc = &bytes[body..end];
                let width = usize::from(desc[1]);
                let height = usize::from(desc[2]);
                let comp_at = end;
                if comp_at + 2 > bytes.len() {
                    return Err(bad("SIS frame has no payload length"));
                }
                let comp_len = be16(bytes, comp_at)?;
                let comp_end = comp_at + 2 + comp_len;
                if comp_end > bytes.len() {
                    return Err(bad("SIS frame payload runs past the file"));
                }
                if frames.len() >= MAX_FRAMES {
                    return Err(Error::Unsupported(format!(
                        "a SIS picture with more than {MAX_FRAMES} frames"
                    )));
                }
                frames.push(Frame {
                    width,
                    height,
                    chunk4: comp_at + 2..comp_end,
                });
                off = comp_end;
                continue;
            }
            _ => {}
        }
        off = end;
    }

    Ok(Parsed {
        huffman,
        transparent,
        frames,
    })
}

/// The metadata block's type-10 duration in centiseconds, when it names one.
///
/// The type-2 palette chunk is walked with its palette length, because a header that names an
/// external table puts the table's bytes between that chunk and the next one.
pub(crate) fn frame_delay_cs(bytes: &[u8]) -> Option<u16> {
    if bytes.len() < CONTAINER_HEADER
        || !(bytes.starts_with(SAF_MAGIC) || bytes.starts_with(SIS_MAGIC))
    {
        return None;
    }
    let header_bytes = be16(bytes, 10).ok()?;
    let end = CONTAINER_HEADER.checked_add(header_bytes)?.min(bytes.len());
    let mut off = CONTAINER_HEADER;
    let mut delay = None;
    while off + 2 <= end {
        let kind = bytes[off];
        let len = usize::from(bytes[off + 1]);
        let body = off + 2;
        let next = body.checked_add(len)?;
        if next > end {
            break;
        }
        match kind {
            10 if len >= 2 => {
                let ms = u16::from_be_bytes([bytes[body], bytes[body + 1]]);
                delay = Some((ms / 10).max(1));
            }
            2 if len >= 2 => {
                let palette_type = (bytes[body + 1] & 0x60) >> 5;
                if (palette_type == 1 || palette_type == 3) && len >= 4 {
                    let palette_len =
                        usize::from(u16::from_be_bytes([bytes[body + 2], bytes[body + 3]]));
                    off = next.checked_add(palette_len)?.checked_add(2)?;
                    continue;
                }
            }
            _ => {}
        }
        off = next;
    }
    delay
}

fn master_palette() -> Vec<u8> {
    // The master table is stored BGRA; `Bitmap::rgb` reads its table as BGR triples.
    let mut palette = Vec::with_capacity(768);
    for entry in MASTER_PALETTE.chunks_exact(4) {
        palette.extend_from_slice(&entry[..3]);
    }
    palette
}

fn alpha_for(transparent: Option<u8>) -> Alpha {
    match transparent {
        Some(index) => {
            let mut table = vec![u8::MAX; 256];
            table[usize::from(index)] = 0;
            Alpha::Index(table)
        }
        None => Alpha::Opaque,
    }
}

fn inflate_one(
    bytes: &[u8],
    huffman: &[u8],
    header_bits: u32,
    frame: &Frame,
    palette: &[u8],
    alpha: &Alpha,
) -> R<Bitmap> {
    if frame.width == 0 || frame.height == 0 {
        return Err(bad("SIS frame names no picture"));
    }
    let expected = frame
        .width
        .checked_mul(frame.height)
        .ok_or_else(|| bad("SIS frame dimensions overflow"))?;
    let pixels = inflate_frame(&bytes[frame.chunk4.clone()], huffman, header_bits, expected)?;
    Ok(Bitmap {
        width: frame.width,
        height: frame.height,
        pixels,
        palette: Some(palette.to_vec()),
        alpha: alpha.clone(),
    })
}

/// Decode the first frame of a `.sis` picture. This is the path `resource::sis` and the sniffer
/// use; it stops before touching later frames.
pub(crate) fn decode_first(bytes: &[u8]) -> R<Bitmap> {
    let parsed = parse(bytes)?;
    let frame = parsed
        .frames
        .first()
        .ok_or_else(|| bad("SIS picture has no type-4 frame"))?;
    let huffman = parsed
        .huffman
        .ok_or_else(|| bad("SIS picture has no type-11 Huffman chunk"))?;
    let header_bits = huffman_header_bits(huffman)?;
    let palette = master_palette();
    let alpha = alpha_for(parsed.transparent);
    inflate_one(bytes, huffman, header_bits, frame, &palette, &alpha)
}

/// Decode every type-4 frame of a `.sis` picture.
pub(crate) fn decode_frames(bytes: &[u8]) -> R<Vec<Bitmap>> {
    let parsed = parse(bytes)?;
    if parsed.frames.is_empty() {
        return Err(bad("SIS picture has no type-4 frame"));
    }
    let huffman = parsed
        .huffman
        .ok_or_else(|| bad("SIS picture has no type-11 Huffman chunk"))?;
    let header_bits = huffman_header_bits(huffman)?;
    let palette = master_palette();
    let alpha = alpha_for(parsed.transparent);

    let mut out = Vec::with_capacity(parsed.frames.len());
    let mut total_pixels = 0usize;
    for frame in &parsed.frames {
        let expected = frame
            .width
            .checked_mul(frame.height)
            .ok_or_else(|| bad("SIS frame dimensions overflow"))?;
        total_pixels = total_pixels.saturating_add(expected);
        if total_pixels > MAX_TOTAL_FRAME_PIXELS {
            return Err(Error::Unsupported(format!(
                "a SIS animation over the {MAX_TOTAL_FRAME_PIXELS}-pixel limit"
            )));
        }
        out.push(inflate_one(
            bytes,
            huffman,
            header_bits,
            frame,
            &palette,
            &alpha,
        )?);
    }
    Ok(out)
}

/// Decode the first frame of a `.sis` picture.
pub(crate) fn decode(bytes: &[u8]) -> R<Bitmap> {
    decode_first(bytes)
}

pub(crate) const MASTER_PALETTE: [u8; 1024] = [
    0x00, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00, 0x00, 0xaa, 0x00, 0x00, 0x00, 0xe1, 0x00, 0x00, 0x00,
    0x00, 0x24, 0x00, 0x00, 0x55, 0x24, 0x00, 0x00, 0xaa, 0x24, 0x00, 0x00, 0xff, 0x24, 0x00, 0x00,
    0x00, 0x49, 0x00, 0x00, 0x55, 0x49, 0x00, 0x00, 0xaa, 0x49, 0x00, 0x00, 0xff, 0x49, 0x00, 0x00,
    0x00, 0x6d, 0x00, 0x00, 0x55, 0x6d, 0x00, 0x00, 0xaa, 0x6d, 0x00, 0x00, 0xff, 0x6d, 0x00, 0x00,
    0x00, 0x92, 0x00, 0x00, 0x55, 0x92, 0x00, 0x00, 0xaa, 0x92, 0x00, 0x00, 0xff, 0x92, 0x00, 0x00,
    0x00, 0xb6, 0x00, 0x00, 0x55, 0xb6, 0x00, 0x00, 0xaa, 0xb6, 0x00, 0x00, 0xff, 0xb6, 0x00, 0x00,
    0x00, 0xdb, 0x00, 0x00, 0x55, 0xdb, 0x00, 0x00, 0xaa, 0xdb, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x00,
    0x00, 0xff, 0x00, 0x00, 0x55, 0xff, 0x00, 0x00, 0xaa, 0xff, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00,
    0x00, 0x00, 0x24, 0x00, 0x55, 0x00, 0x24, 0x00, 0xaa, 0x00, 0x24, 0x00, 0xff, 0x00, 0x24, 0x00,
    0x00, 0x24, 0x24, 0x00, 0x55, 0x24, 0x24, 0x00, 0xaa, 0x24, 0x24, 0x00, 0xff, 0x24, 0x24, 0x00,
    0x00, 0x49, 0x24, 0x00, 0x55, 0x49, 0x24, 0x00, 0xaa, 0x49, 0x24, 0x00, 0xff, 0x49, 0x24, 0x00,
    0x00, 0x6d, 0x24, 0x00, 0x55, 0x6d, 0x24, 0x00, 0xaa, 0x6d, 0x24, 0x00, 0xff, 0x6d, 0x24, 0x00,
    0x00, 0x92, 0x24, 0x00, 0x55, 0x92, 0x24, 0x00, 0xaa, 0x92, 0x24, 0x00, 0xff, 0x92, 0x24, 0x00,
    0x00, 0xb6, 0x24, 0x00, 0x55, 0xb6, 0x24, 0x00, 0xaa, 0xb6, 0x24, 0x00, 0xff, 0xb6, 0x24, 0x00,
    0x00, 0xdb, 0x24, 0x00, 0x55, 0xdb, 0x24, 0x00, 0xaa, 0xdb, 0x24, 0x00, 0xff, 0xdb, 0x24, 0x00,
    0x00, 0xff, 0x24, 0x00, 0x55, 0xff, 0x24, 0x00, 0xaa, 0xff, 0x24, 0x00, 0xff, 0xff, 0x24, 0x00,
    0x00, 0x00, 0x49, 0x00, 0x55, 0x00, 0x49, 0x00, 0xaa, 0x00, 0x49, 0x00, 0xff, 0x00, 0x49, 0x00,
    0x00, 0x24, 0x49, 0x00, 0x55, 0x24, 0x49, 0x00, 0xaa, 0x24, 0x49, 0x00, 0xff, 0x24, 0x49, 0x00,
    0x00, 0x49, 0x49, 0x00, 0x55, 0x49, 0x49, 0x00, 0xaa, 0x49, 0x49, 0x00, 0xff, 0x49, 0x49, 0x00,
    0x00, 0x6d, 0x49, 0x00, 0x55, 0x6d, 0x49, 0x00, 0xaa, 0x6d, 0x49, 0x00, 0xff, 0x6d, 0x49, 0x00,
    0x00, 0x92, 0x49, 0x00, 0x55, 0x92, 0x49, 0x00, 0xaa, 0x92, 0x49, 0x00, 0xff, 0x92, 0x49, 0x00,
    0x00, 0xb6, 0x49, 0x00, 0x55, 0xb6, 0x49, 0x00, 0xaa, 0xb6, 0x49, 0x00, 0xff, 0xb6, 0x49, 0x00,
    0x00, 0xdb, 0x49, 0x00, 0x55, 0xdb, 0x49, 0x00, 0xaa, 0xdb, 0x49, 0x00, 0xff, 0xdb, 0x49, 0x00,
    0x00, 0xff, 0x49, 0x00, 0x55, 0xff, 0x49, 0x00, 0xaa, 0xff, 0x49, 0x00, 0xff, 0xff, 0x49, 0x00,
    0x00, 0x00, 0x6d, 0x00, 0x55, 0x00, 0x6d, 0x00, 0xaa, 0x00, 0x6d, 0x00, 0xff, 0x00, 0x6d, 0x00,
    0x00, 0x24, 0x6d, 0x00, 0x55, 0x24, 0x6d, 0x00, 0xaa, 0x24, 0x6d, 0x00, 0xff, 0x24, 0x6d, 0x00,
    0x00, 0x49, 0x6d, 0x00, 0x55, 0x49, 0x6d, 0x00, 0xaa, 0x49, 0x6d, 0x00, 0xff, 0x49, 0x6d, 0x00,
    0x00, 0x6d, 0x6d, 0x00, 0x55, 0x6d, 0x6d, 0x00, 0xaa, 0x6d, 0x6d, 0x00, 0xff, 0x6d, 0x6d, 0x00,
    0x00, 0x92, 0x6d, 0x00, 0x55, 0x92, 0x6d, 0x00, 0xaa, 0x92, 0x6d, 0x00, 0xff, 0x92, 0x6d, 0x00,
    0x00, 0xb6, 0x6d, 0x00, 0x55, 0xb6, 0x6d, 0x00, 0xaa, 0xb6, 0x6d, 0x00, 0xff, 0xb6, 0x6d, 0x00,
    0x00, 0xdb, 0x6d, 0x00, 0x55, 0xdb, 0x6d, 0x00, 0xaa, 0xdb, 0x6d, 0x00, 0xff, 0xdb, 0x6d, 0x00,
    0x00, 0xff, 0x6d, 0x00, 0x55, 0xff, 0x6d, 0x00, 0xaa, 0xff, 0x6d, 0x00, 0xff, 0xff, 0x6d, 0x00,
    0x00, 0x00, 0x92, 0x00, 0x55, 0x00, 0x92, 0x00, 0xaa, 0x00, 0x92, 0x00, 0xff, 0x00, 0x92, 0x00,
    0x00, 0x24, 0x92, 0x00, 0x55, 0x24, 0x92, 0x00, 0xaa, 0x24, 0x92, 0x00, 0xff, 0x24, 0x92, 0x00,
    0x00, 0x49, 0x92, 0x00, 0x55, 0x49, 0x92, 0x00, 0xaa, 0x49, 0x92, 0x00, 0xff, 0x49, 0x92, 0x00,
    0x00, 0x6d, 0x92, 0x00, 0x55, 0x6d, 0x92, 0x00, 0xaa, 0x6d, 0x92, 0x00, 0xff, 0x6d, 0x92, 0x00,
    0x00, 0x92, 0x92, 0x00, 0x55, 0x92, 0x92, 0x00, 0xaa, 0x92, 0x92, 0x00, 0xff, 0x92, 0x92, 0x00,
    0x00, 0xb6, 0x92, 0x00, 0x55, 0xb6, 0x92, 0x00, 0xaa, 0xb6, 0x92, 0x00, 0xff, 0xb6, 0x92, 0x00,
    0x00, 0xdb, 0x92, 0x00, 0x55, 0xdb, 0x92, 0x00, 0xaa, 0xdb, 0x92, 0x00, 0xff, 0xdb, 0x92, 0x00,
    0x00, 0xff, 0x92, 0x00, 0x55, 0xff, 0x92, 0x00, 0xaa, 0xff, 0x92, 0x00, 0xff, 0xff, 0x92, 0x00,
    0x00, 0x00, 0xb6, 0x00, 0x55, 0x00, 0xb6, 0x00, 0xaa, 0x00, 0xb6, 0x00, 0xff, 0x00, 0xb6, 0x00,
    0x00, 0x24, 0xb6, 0x00, 0x55, 0x24, 0xb6, 0x00, 0xaa, 0x24, 0xb6, 0x00, 0xff, 0x24, 0xb6, 0x00,
    0x00, 0x49, 0xb6, 0x00, 0x55, 0x49, 0xb6, 0x00, 0xaa, 0x49, 0xb6, 0x00, 0xff, 0x49, 0xb6, 0x00,
    0x00, 0x6d, 0xb6, 0x00, 0x55, 0x6d, 0xb6, 0x00, 0xaa, 0x6d, 0xb6, 0x00, 0xff, 0x6d, 0xb6, 0x00,
    0x00, 0x92, 0xb6, 0x00, 0x55, 0x92, 0xb6, 0x00, 0xaa, 0x92, 0xb6, 0x00, 0xff, 0x92, 0xb6, 0x00,
    0x00, 0xb6, 0xb6, 0x00, 0x55, 0xb6, 0xb6, 0x00, 0xaa, 0xb6, 0xb6, 0x00, 0xff, 0xb6, 0xb6, 0x00,
    0x00, 0xdb, 0xb6, 0x00, 0x55, 0xdb, 0xb6, 0x00, 0xaa, 0xdb, 0xb6, 0x00, 0xff, 0xdb, 0xb6, 0x00,
    0x00, 0xff, 0xb6, 0x00, 0x55, 0xff, 0xb6, 0x00, 0xaa, 0xff, 0xb6, 0x00, 0xff, 0xff, 0xb6, 0x00,
    0x00, 0x00, 0xdb, 0x00, 0x55, 0x00, 0xdb, 0x00, 0xaa, 0x00, 0xdb, 0x00, 0xff, 0x00, 0xdb, 0x00,
    0x00, 0x24, 0xdb, 0x00, 0x55, 0x24, 0xdb, 0x00, 0xaa, 0x24, 0xdb, 0x00, 0xff, 0x24, 0xdb, 0x00,
    0x00, 0x49, 0xdb, 0x00, 0x55, 0x49, 0xdb, 0x00, 0xaa, 0x49, 0xdb, 0x00, 0xff, 0x49, 0xdb, 0x00,
    0x00, 0x6d, 0xdb, 0x00, 0x55, 0x6d, 0xdb, 0x00, 0xaa, 0x6d, 0xdb, 0x00, 0xff, 0x6d, 0xdb, 0x00,
    0x00, 0x92, 0xdb, 0x00, 0x55, 0x92, 0xdb, 0x00, 0xaa, 0x92, 0xdb, 0x00, 0xff, 0x92, 0xdb, 0x00,
    0x00, 0xb6, 0xdb, 0x00, 0x55, 0xb6, 0xdb, 0x00, 0xaa, 0xb6, 0xdb, 0x00, 0xff, 0xb6, 0xdb, 0x00,
    0x00, 0xdb, 0xdb, 0x00, 0x55, 0xdb, 0xdb, 0x00, 0xaa, 0xdb, 0xdb, 0x00, 0xff, 0xdb, 0xdb, 0x00,
    0x00, 0xff, 0xdb, 0x00, 0x55, 0xff, 0xdb, 0x00, 0xaa, 0xff, 0xdb, 0x00, 0xff, 0xff, 0xdb, 0x00,
    0x00, 0x00, 0xff, 0x00, 0x55, 0x00, 0xff, 0x00, 0xaa, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00,
    0x00, 0x24, 0xff, 0x00, 0x55, 0x24, 0xff, 0x00, 0xaa, 0x24, 0xff, 0x00, 0xff, 0x24, 0xff, 0x00,
    0x00, 0x49, 0xff, 0x00, 0x55, 0x49, 0xff, 0x00, 0xaa, 0x49, 0xff, 0x00, 0xff, 0x49, 0xff, 0x00,
    0x00, 0x6d, 0xff, 0x00, 0x55, 0x6d, 0xff, 0x00, 0xaa, 0x6d, 0xff, 0x00, 0xff, 0x6d, 0xff, 0x00,
    0x00, 0x92, 0xff, 0x00, 0x55, 0x92, 0xff, 0x00, 0xaa, 0x92, 0xff, 0x00, 0xff, 0x92, 0xff, 0x00,
    0x00, 0xb6, 0xff, 0x00, 0x55, 0xb6, 0xff, 0x00, 0xaa, 0xb6, 0xff, 0x00, 0xff, 0xb6, 0xff, 0x00,
    0x00, 0xdb, 0xff, 0x00, 0x55, 0xdb, 0xff, 0x00, 0xaa, 0xdb, 0xff, 0x00, 0xff, 0xdb, 0xff, 0x00,
    0x00, 0xff, 0xff, 0x00, 0x55, 0xff, 0xff, 0x00, 0xaa, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0x00,
];
