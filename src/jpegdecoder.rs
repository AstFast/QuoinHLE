/// `AEECLSID_JPEGDECODER` — `gAEEJPEGClasses`' second row (`.constdata + 0x45c`, 16-byte records
/// `{clsid, 1, 0, ctor}`), and `AEEClassIDs`-style `0x01026e22`.
pub const CLS_JPEGDECODER: u32 = 0x0102_6e22;
/// `AEECLSID_JPEG` — the same table's first row, whose constructor is `JPEGViewer_New`. That body creates
/// this decoder, creates `AEECLSID_CGenericViewer` (`0x0101e7c3`) and calls `IImage::SetImage(me, 11,
/// pDecoder, 0)` on it, so the viewer class adds no state of its own.
pub const CLS_JPEG: u32 = 0x0100_4005;
/// `AEEIID_IQI` — the first of exactly three numbers `JPEGDecoder_QueryInterface` accepts.
pub const IID_IQI: u32 = 0x0100_0001;
/// `AEEIID_IImageDecoder`, answered with the same pointer the caller already holds.
pub const IID_IMAGE_DECODER: u32 = 0x0102_6e20;
/// `AEEIID_IForceFeed`, answered with `me + 8`.
pub const IID_FORCEFEED: u32 = 0x0101_eb0b;
/// `AEE_ECLASSNOTSUPPORT`(3) — `QueryInterface`'s answer for anything in that list of three, and the body
/// clears the caller's slot on the way.
pub const AEE_ECLASSNOTSUPPORT: u32 = 3;
/// `AEE_EFAILED`(1) — `GetBitmap` with nothing decoded, and `Write(pBuf = 0)` with nothing fed. Both are
/// the reference one-byte answers, not this host's choice.
pub const AEE_EFAILED: u32 = 1;
/// `AEE_RO_COPY`(2). `JPEGDecoder_GetRop` is a four-byte body — `return 2` — read by nothing at all, and
/// it is right: a baseline JPEG names no transparent pixel, so the default raster op is a copy.
pub const AEE_RO_COPY: u32 = 2;
/// `AEE_ENOMEMORY`(2) — `aee_malloc(40)` in the constructor, and `aee_realloc` of `[+36]` at flush time.
pub const AEE_ENOMEMORY: u32 = 2;

/// One decoder's shared state, whichever face reached it.
#[derive(Debug, Default)]
pub struct Feed {
    /// `[+32]`/`[+36]`: the stream accumulated so far, in the order it arrived.
    pub bytes: Vec<u8>,
    /// `[+24]`: the DIB the last successful flush built. `GetBitmap` hands *this* out and takes another
    /// reference on it, so two calls before the next flush name the same picture.
    pub dib: Option<u32>,
}

/// What the three `QueryInterface` numbers mean for one object.
#[derive(Debug, PartialEq, Eq)]
pub enum Face {
    /// The caller already holds it: hand the same address back.
    Same,
    /// The other face of the same record — `me + 8` in the guest's words.
    Other,
    /// Not in the list. The body writes 0 into the caller's slot and answers 3, and the class id
    /// `AEECLSID_JPEGDECODER` is **not** in that list: asking a JPEG decoder whether it is
    /// `AEECLSID_JPEGDECODER` is a `3`, which is not what the BMP decoder does.
    None,
}

/// What `IForceFeed::Write(pBuf, cbBuf)` is being asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Write {
    /// `cbBuf == 0` with a real pointer: the body returns 0 having read and written nothing.
    Ignored,
    /// Bytes to take. The capacity rounding in the body (`(need >> 12 << 12) + 4096`, then `aee_realloc`)
    /// is an allocator policy with no ABI-visible answer short of `AEE_ENOMEMORY`, so this host appends.
    Append,
    /// A negative `cbBuf`. The body has no sign test: it adds `cbBuf` to `[+32]` as an `unsigned` and only
    /// reallocs when that sum passes `[+28]`. With nothing buffered the sum is `0xFFFF_FFFF` and
    /// `aee_realloc` refuses, answering **2**; with bytes already buffered the sum usually
    /// stays under the capacity, no realloc happens, and the body memmoves `(size_t)cbBuf` through a
    /// buffer it never grew — undefined. This host answers that `2` without touching memory, which is the
    /// only reading of the branch that cannot corrupt the guest.
    Impossible,
    /// `pBuf == 0`: this is the flush, not an empty write. `[+36] == 0` is the first question and
    /// its answer is the initialized `1`, with no decode. A stream that fails to decode answers `1` too,
    /// but the two failure shapes differ in `[+24]`: a header `c_jpeg_decode` rejects leaves the previous
    /// picture cached ( jumps past the release), while a scan line that fails mid-decode releases it.
    ///This host keeps the picture for both — the crate's error does not say which shape it
    /// was — so the mid-decode shape is where a caller can tell this host from the delivered object.
    Flush,
}

impl Feed {
    pub fn face_of(iid: u32) -> Face {
        match iid {
            IID_IQI | IID_IMAGE_DECODER => Face::Same,
            IID_FORCEFEED => Face::Other,
            _ => Face::None,
        }
    }

    /// The split the body makes between its two zero-valued arguments, which is easy to read backwards.
    pub fn write_kind(pbuf_is_null: bool, cb: i32) -> Write {
        if pbuf_is_null {
            Write::Flush
        } else if cb == 0 {
            Write::Ignored
        } else if cb < 0 {
            Write::Impossible
        } else {
            Write::Append
        }
    }

    /// `[+36] == 0` is the first question the flush asks — before the resource pool, before the decode.
    pub fn has_bytes(&self) -> bool {
        !self.bytes.is_empty()
    }

    pub fn bitmap(&self) -> Option<u32> {
        self.dib
    }

    pub fn reset(&mut self) -> Option<u32> {
        self.bytes.clear();
        self.dib.take()
    }
}
