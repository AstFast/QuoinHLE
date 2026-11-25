//! QCP container and QCELP/EVRC frame layer.
//!
//! This is the standalone home for the PureVoice port. The container parser and packet framing are
//! independent of the vocoder itself: QCP is a RIFF `QLCM` file, and each packet starts with a rate
//! byte followed by the payload for that rate.
//!
//! The frame bitmap unpacking for QCELP 13K is also here: `QcelpFrame::unpack` turns a payload
//! into the LSP/codebook/pitch fields using the TIA/EIA/IS-733 tables. [`QcelpDecoder`] carries
//! those fields the rest of the way — LSP interpolation, the pitch and formant filters, and the
//! adaptive postfilter — into the 160 samples a 20 ms frame is worth. [`decode_qcp`] is the
//! whole-file door: it walks the packets and concatenates what the decoder returns, handing the
//! EVRC rate to `crate::audio::evrc`, which is a different vocoder inside the same container.
//!
//! None of it is a stub or an approximation of the shape of a decoder; the one thing that is
//! deliberately absent is named where it lives (`crate::audio::evrc` has no postfilter).

use std::fmt;

use crate::audio::qcelp_tables::{
    FULL_CODEBOOK, G12GA, HALF_CODEBOOK, LSPVQ1, LSPVQ2, LSPVQ3, LSPVQ4, LSPVQ5, RND_FIR_COEFS,
};

/// QCP is an 8 kHz speech format; every QCELP frame is 20 ms (160 samples).
pub const SAMPLE_RATE: u32 = 8_000;
pub const SAMPLES_PER_FRAME: usize = 160;
pub const FRAME_MS: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Qcelp13,
    Evrc,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameRate {
    Silence,
    Eighth,
    Quarter,
    Half,
    Full,
}

impl FrameRate {
    /// The rate byte used by QCP/RFC 3625 packets.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Silence),
            1 => Some(Self::Eighth),
            2 => Some(Self::Quarter),
            3 => Some(Self::Half),
            4 => Some(Self::Full),
            _ => None,
        }
    }

    /// Encoded payload bytes, excluding the one-byte rate prefix.
    pub fn payload_len(self) -> usize {
        match self {
            Self::Silence => 0,
            Self::Eighth => 3,
            Self::Quarter => 7,
            Self::Half => 16,
            Self::Full => 34,
        }
    }

    /// Packet bytes, including the rate prefix.
    pub fn packet_len(self) -> usize {
        1 + self.payload_len()
    }

    pub fn samples(self) -> usize {
        SAMPLES_PER_FRAME
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub rate: FrameRate,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct Qcp<'a> {
    pub codec: Codec,
    pub format: &'a [u8],
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotRiff,
    NotQlcm,
    Truncated,
    MissingFormat,
    MissingData,
    BadFrameRate(u8),
    BadLsp,
    UnsupportedCodec,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRiff => f.write_str("not a RIFF file"),
            Self::NotQlcm => f.write_str("RIFF file is not QLCM"),
            Self::Truncated => f.write_str("truncated RIFF chunk"),
            Self::MissingFormat => f.write_str("QCP has no fmt chunk"),
            Self::MissingData => f.write_str("QCP has no data chunk"),
            Self::BadFrameRate(byte) => write!(f, "unknown QCP frame rate byte {byte}"),
            Self::BadLsp => f.write_str("QCELP LSP parameters are out of range"),
            Self::UnsupportedCodec => f.write_str("this QCP codec is not ported"),
        }
    }
}

impl std::error::Error for Error {}

impl<'a> Qcp<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" {
            return Err(Error::NotRiff);
        }
        if &bytes[8..12] != b"QLCM" {
            return Err(Error::NotQlcm);
        }

        let mut offset = 12usize;
        let mut format = None;
        let mut data = None;
        while offset + 8 <= bytes.len() {
            let id = &bytes[offset..offset + 4];
            let size = u32::from_le_bytes(
                bytes[offset + 4..offset + 8]
                    .try_into()
                    .expect("four-byte chunk size"),
            ) as usize;
            let start = offset + 8;
            let end = start.checked_add(size).ok_or(Error::Truncated)?;
            if end > bytes.len() {
                return Err(Error::Truncated);
            }
            match id {
                b"fmt " => format = Some(&bytes[start..end]),
                b"data" => data = Some(&bytes[start..end]),
                _ => {}
            }
            // RIFF chunks are word-aligned; an odd size carries one pad byte.
            offset = end + (size & 1);
        }

        Ok(Self {
            codec: detect_codec(format.ok_or(Error::MissingFormat)?),
            format: format.ok_or(Error::MissingFormat)?,
            data: data.ok_or(Error::MissingData)?,
        })
    }

    /// Split the `data` chunk into rate-prefixed packets.
    pub fn frames(&self) -> Result<Vec<Frame<'a>>, Error> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        while offset < self.data.len() {
            let byte = self.data[offset];
            let rate = FrameRate::from_byte(byte).ok_or(Error::BadFrameRate(byte))?;
            let end = offset + rate.packet_len();
            if end > self.data.len() {
                // RFC 3625 files in the sample set can end with a trimmed packet. The complete packets
                // before it are still usable; refusing the whole clip over the last fragment would
                // turn a playable ring tone into silence.
                break;
            }
            out.push(Frame {
                rate,
                payload: &self.data[offset + 1..end],
            });
            offset = end;
        }
        Ok(out)
    }

    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    /// Duration from complete packets; silent packets still occupy one frame.
    pub fn duration_ms(&self) -> Result<u64, Error> {
        if self.codec == Codec::Evrc {
            let mut at = 0usize;
            let mut count = 0u64;
            while at < self.data.len() {
                let byte = self.data[at];
                let length = match byte {
                    0 => 1,
                    1 => 3,
                    2 => 6,
                    3 => 11,
                    4 => 23,
                    _ => return Err(Error::BadFrameRate(byte)),
                };
                if at + length > self.data.len() {
                    break;
                }
                count += 1;
                at += length;
            }
            return Ok(count * FRAME_MS);
        }
        Ok(self.frames()?.len() as u64 * FRAME_MS)
    }
}

fn detect_codec(format: &[u8]) -> Codec {
    let folded: Vec<u8> = format.iter().map(u8::to_ascii_lowercase).collect();
    if contains(&folded, b"evrc") {
        Codec::Evrc
    } else if contains(&folded, b"qcelp") || contains(&folded, b"13k") {
        Codec::Qcelp13
    } else {
        Codec::Unknown
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QcelpFrame {
    pub lspv: [u8; 10],
    pub cbsign: [u8; 16],
    pub cbgain: [u8; 16],
    pub cindex: [u8; 16],
    pub plag: [u8; 4],
    pub pfrac: [u8; 4],
    pub pgain: [u8; 4],
    pub reserved: u8,
}

// Bitmap tables ported from FFmpeg's libavcodec/qcelpdata.h (LGPL-2.1-or-later),
// Copyright (c) 2007 Reynaldo H. Verdejo Pinochet. The table layout is part of the
// QCELP specification (TIA/EIA/IS-733 2.4.7), not an original choice here.
const FULL: &[(u8, u8, u8)] = &[
    (2, 0, 3),
    (1, 0, 7),
    (0, 0, 6),
    (4, 0, 6),
    (3, 0, 6),
    (2, 3, 4),
    (10, 0, 1),
    (26, 0, 4),
    (62, 0, 1),
    (58, 0, 7),
    (66, 0, 3),
    (43, 0, 4),
    (11, 0, 1),
    (27, 0, 4),
    (42, 0, 7),
    (29, 0, 1),
    (44, 0, 7),
    (12, 0, 1),
    (28, 0, 4),
    (43, 4, 3),
    (59, 0, 3),
    (67, 0, 3),
    (45, 0, 7),
    (13, 0, 1),
    (29, 1, 2),
    (46, 0, 6),
    (14, 0, 1),
    (30, 0, 4),
    (63, 0, 1),
    (59, 3, 4),
    (32, 0, 3),
    (47, 0, 7),
    (15, 0, 1),
    (31, 0, 4),
    (46, 6, 1),
    (49, 0, 3),
    (17, 0, 1),
    (33, 0, 3),
    (48, 0, 7),
    (16, 0, 1),
    (32, 3, 1),
    (34, 0, 1),
    (64, 0, 1),
    (60, 0, 7),
    (68, 0, 3),
    (49, 3, 4),
    (19, 0, 1),
    (35, 0, 4),
    (50, 0, 7),
    (18, 0, 1),
    (34, 1, 3),
    (52, 0, 4),
    (20, 0, 1),
    (36, 0, 4),
    (51, 0, 7),
    (69, 0, 2),
    (53, 0, 7),
    (21, 0, 1),
    (37, 0, 3),
    (52, 4, 3),
    (54, 0, 2),
    (22, 0, 1),
    (38, 0, 4),
    (65, 0, 1),
    (61, 0, 7),
    (69, 2, 1),
    (55, 0, 6),
    (23, 0, 1),
    (39, 0, 4),
    (54, 2, 5),
    (41, 0, 3),
    (56, 0, 7),
    (24, 0, 1),
    (40, 0, 4),
    (55, 6, 1),
    (70, 0, 2),
    (57, 0, 7),
    (25, 0, 1),
];
const HALF: &[(u8, u8, u8)] = &[
    (2, 0, 3),
    (1, 0, 7),
    (0, 0, 6),
    (4, 0, 6),
    (3, 0, 6),
    (2, 3, 4),
    (10, 0, 1),
    (26, 0, 4),
    (62, 0, 1),
    (58, 0, 7),
    (66, 0, 3),
    (59, 0, 6),
    (67, 0, 3),
    (42, 0, 7),
    (68, 0, 2),
    (43, 0, 7),
    (11, 0, 1),
    (27, 0, 4),
    (63, 0, 1),
    (59, 6, 1),
    (44, 0, 2),
    (12, 0, 1),
    (28, 0, 4),
    (64, 0, 1),
    (60, 0, 7),
    (68, 2, 1),
    (65, 0, 1),
    (61, 0, 7),
    (69, 0, 3),
    (44, 2, 5),
    (45, 0, 7),
    (13, 0, 1),
    (29, 0, 4),
];
const QUARTER: &[(u8, u8, u8)] = &[
    (2, 0, 3),
    (1, 0, 7),
    (0, 0, 6),
    (4, 0, 6),
    (3, 0, 6),
    (2, 3, 4),
    (29, 0, 4),
    (28, 0, 4),
    (27, 0, 4),
    (26, 0, 4),
    (70, 0, 2),
    (30, 0, 4),
];
const OCTAVE: &[(u8, u8, u8)] = &[
    (25, 3, 1),
    (0, 0, 1),
    (1, 0, 1),
    (2, 0, 1),
    (25, 2, 1),
    (3, 0, 1),
    (4, 0, 1),
    (5, 0, 1),
    (25, 1, 1),
    (6, 0, 1),
    (7, 0, 1),
    (8, 0, 1),
    (25, 0, 1),
    (9, 0, 1),
    (26, 0, 2),
    (70, 0, 4),
];

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit: 0 }
    }

    fn read(&mut self, len: u8) -> Option<u8> {
        if self.bit + usize::from(len) > self.data.len() * 8 {
            return None;
        }
        let mut value = 0u8;
        for _ in 0..len {
            let byte = self.data[self.bit / 8];
            let bit = (byte >> (7 - self.bit % 8)) & 1;
            value = (value << 1) | bit;
            self.bit += 1;
        }
        Some(value)
    }
}

impl QcelpFrame {
    pub fn unpack(rate: FrameRate, payload: &[u8]) -> Result<Self, Error> {
        let table = match rate {
            FrameRate::Silence => return Ok(Self::default()),
            FrameRate::Eighth => OCTAVE,
            FrameRate::Quarter => QUARTER,
            FrameRate::Half => HALF,
            FrameRate::Full => FULL,
        };
        let mut reader = BitReader::new(payload);
        let mut frame = Self::default();
        for &(target, bit, len) in table {
            let value = reader.read(len).ok_or(Error::Truncated)?;
            frame.set(target, bit, value);
        }
        Ok(frame)
    }

    fn set(&mut self, target: u8, bit: u8, value: u8) {
        let value = value << bit;
        match target {
            0..=9 => self.lspv[usize::from(target)] |= value,
            10..=25 => self.cbsign[usize::from(target - 10)] |= value,
            26..=41 => self.cbgain[usize::from(target - 26)] |= value,
            42..=57 => self.cindex[usize::from(target - 42)] |= value,
            58..=61 => self.plag[usize::from(target - 58)] |= value,
            62..=65 => self.pfrac[usize::from(target - 62)] |= value,
            66..=69 => self.pgain[usize::from(target - 66)] |= value,
            70 => self.reserved |= value,
            _ => {}
        }
    }
}

const LSP_SPREAD_FACTOR: f32 = 0.02;
const LSP_OCTAVE_PREDICTOR: f32 = 29.0 / 32.0;

/// Inter-frame state for QCELP LSP decoding.
///
/// The predictor memory is deliberately separate from `prev_lspf`: octave frames predict from the
/// previous octave predictor, while full/half/quarter frames replace it.
#[derive(Debug, Clone)]
pub struct LspState {
    prev_lspf: [f32; 10],
    predictor_lspf: [f32; 10],
    prev_rate: Option<FrameRate>,
    octave_count: u32,
}

impl Default for LspState {
    fn default() -> Self {
        let mut prev_lspf = [0.0; 10];
        for (i, value) in prev_lspf.iter_mut().enumerate() {
            *value = (i as f32 + 1.0) / 11.0;
        }
        Self {
            prev_lspf,
            predictor_lspf: [0.0; 10],
            prev_rate: None,
            octave_count: 0,
        }
    }
}

impl LspState {
    pub fn previous(&self) -> [f32; 10] {
        self.prev_lspf
    }

    /// Decode the ten quantized LSP frequencies for one frame.
    pub fn decode(&mut self, frame: &QcelpFrame, rate: FrameRate) -> Result<[f32; 10], Error> {
        let mut lspf = [0.0f32; 10];

        if rate == FrameRate::Eighth {
            let predictors = if self.prev_rate == Some(FrameRate::Eighth) {
                self.predictor_lspf
            } else {
                self.prev_lspf
            };
            self.octave_count = self.octave_count.saturating_add(1);
            for i in 0..10 {
                let spread = if frame.lspv[i] != 0 {
                    LSP_SPREAD_FACTOR
                } else {
                    -LSP_SPREAD_FACTOR
                };
                let value = spread
                    + predictors[i] * LSP_OCTAVE_PREDICTOR
                    + (i as f32 + 1.0) * ((1.0 - LSP_OCTAVE_PREDICTOR) / 11.0);
                self.predictor_lspf[i] = value;
                lspf[i] = value;
            }
            lspf[0] = lspf[0].max(LSP_SPREAD_FACTOR);
            for i in 1..10 {
                lspf[i] = lspf[i].max(lspf[i - 1] + LSP_SPREAD_FACTOR);
            }
            lspf[9] = lspf[9].min(1.0 - LSP_SPREAD_FACTOR);
            for i in (1..10).rev() {
                lspf[i - 1] = lspf[i - 1].min(lspf[i] - LSP_SPREAD_FACTOR);
            }
            let smooth = if self.octave_count < 10 { 0.875 } else { 0.1 };
            for i in 0..10 {
                lspf[i] = lspf[i] * smooth + self.prev_lspf[i] * (1.0 - smooth);
            }
        } else {
            self.octave_count = 0;
            if rate == FrameRate::Silence {
                lspf = self.prev_lspf;
            } else {
                let tables: [&[[u16; 2]]; 5] = [&LSPVQ1, &LSPVQ2, &LSPVQ3, &LSPVQ4, &LSPVQ5];
                let mut cumulative = 0.0;
                for i in 0..5 {
                    let index = usize::from(frame.lspv[i]);
                    let pair = tables[i].get(index).ok_or(Error::BadLsp)?;
                    cumulative += f32::from(pair[0]) * 0.0001;
                    lspf[2 * i] = cumulative;
                    cumulative += f32::from(pair[1]) * 0.0001;
                    lspf[2 * i + 1] = cumulative;
                }
                if rate == FrameRate::Quarter {
                    if lspf[9] <= 0.70 || lspf[9] >= 0.97 {
                        return Err(Error::BadLsp);
                    }
                    for i in 3..10 {
                        if (lspf[i] - lspf[i - 2]).abs() < 0.08 {
                            return Err(Error::BadLsp);
                        }
                    }
                } else {
                    if lspf[9] <= 0.66 || lspf[9] >= 0.985 {
                        return Err(Error::BadLsp);
                    }
                    for i in 4..10 {
                        if (lspf[i] - lspf[i - 2]).abs() < 0.08 {
                            return Err(Error::BadLsp);
                        }
                    }
                }
            }
        }

        self.prev_lspf = lspf;
        self.prev_rate = Some(rate);
        Ok(lspf)
    }
}

#[derive(Debug, Clone, Default)]
pub struct GainState {
    prev_g1: [i32; 2],
    last_codebook_gain: f32,
}

impl GainState {
    fn g12(index: i32) -> f32 {
        G12GA[index.clamp(0, 60) as usize]
    }

    fn clip(value: i32, low: i32, high: i32) -> usize {
        value.clamp(low, high) as usize
    }

    /// Decode the per-subframe gains and adjust signed codebook indices.
    pub fn decode(&mut self, frame: &mut QcelpFrame, rate: FrameRate) -> [f32; 16] {
        let mut gain = [0.0f32; 16];
        let mut g1 = [0i32; 16];
        match rate {
            FrameRate::Full | FrameRate::Half | FrameRate::Quarter => {
                let subframes = match rate {
                    FrameRate::Full => 16,
                    FrameRate::Half => 4,
                    FrameRate::Quarter => 5,
                    _ => unreachable!(),
                };
                for i in 0..subframes {
                    g1[i] = 4 * i32::from(frame.cbgain[i]);
                    if rate == FrameRate::Full && (i + 1) % 4 == 0 {
                        g1[i] +=
                            Self::clip((g1[i - 1] + g1[i - 2] + g1[i - 3]) / 3 - 6, 0, 32) as i32;
                    }
                    gain[i] = Self::g12(g1[i]);
                    if frame.cbsign[i] != 0 {
                        gain[i] = -gain[i];
                        frame.cindex[i] = (frame.cindex[i].wrapping_sub(89)) & 127;
                    }
                }
                self.prev_g1[0] = g1[subframes - 2];
                self.prev_g1[1] = g1[subframes - 1];
                self.last_codebook_gain = Self::g12(g1[subframes - 1]);
                if rate == FrameRate::Quarter {
                    gain[7] = gain[4];
                    gain[6] = 0.4 * gain[3] + 0.6 * gain[4];
                    gain[5] = gain[3];
                    gain[4] = 0.8 * gain[2] + 0.2 * gain[3];
                    gain[3] = 0.2 * gain[1] + 0.8 * gain[2];
                    gain[2] = gain[1];
                    gain[1] = 0.6 * gain[0] + 0.4 * gain[1];
                }
            }
            FrameRate::Eighth => {
                g1[0] = 2 * i32::from(frame.cbgain[0])
                    + Self::clip((self.prev_g1[0] + self.prev_g1[1]) / 2 - 5, 0, 54) as i32;
                let slope = 0.5 * (Self::g12(g1[0]) - self.last_codebook_gain) / 8.0;
                for i in 1..=8 {
                    gain[i - 1] = self.last_codebook_gain + slope * i as f32;
                }
                self.last_codebook_gain = gain[6];
                self.prev_g1[0] = self.prev_g1[1];
                self.prev_g1[1] = g1[0];
            }
            FrameRate::Silence => {}
        }
        gain
    }
}

/// Decoded parameters for one QCELP 13K frame before synthesis.
#[derive(Debug, Clone)]
pub struct QcelpParams {
    pub lsp: [f32; 10],
    pub gain: [f32; 16],
    pub pitch_gain: [f32; 4],
    pub pitch_lag: [u8; 4],
}

const QCELP_SQRT1887: f32 = 1.373681186;
const HAMMSINC: [f32; 4] = [-0.006822, 0.041249, -0.143459, 0.588863];
const BANDWIDTH_EXPANSION: f32 = 0.9883;

#[derive(Debug, Clone)]
struct SynthState {
    pitch_synthesis_filter_mem: [f32; 303],
    pitch_pre_filter_mem: [f32; 303],
    rnd_fir_filter_mem: [f32; 180],
    formant_mem: [f32; 170],
    postfilter_synth_mem: [f32; 10],
    postfilter_tilt_mem: f32,
    postfilter_agc_mem: f32,
}

impl Default for SynthState {
    fn default() -> Self {
        Self {
            pitch_synthesis_filter_mem: [0.0; 303],
            pitch_pre_filter_mem: [0.0; 303],
            rnd_fir_filter_mem: [0.0; 180],
            formant_mem: [0.0; 170],
            postfilter_synth_mem: [0.0; 10],
            postfilter_tilt_mem: 0.0,
            postfilter_agc_mem: 0.0,
        }
    }
}

/// Stateful frame decoder: LSP prediction and gain smoothing live across packets.
#[derive(Debug, Clone, Default)]
pub struct QcelpDecoder {
    lsp: LspState,
    gain: GainState,
    synth: SynthState,
}

impl QcelpDecoder {
    pub fn decode(
        &mut self,
        frame: &mut QcelpFrame,
        rate: FrameRate,
    ) -> Result<QcelpParams, Error> {
        let lsp = match self.lsp.decode(frame, rate) {
            Ok(lsp) => lsp,
            // IS-733 treats a failed LSP stability check as an erased packet: keep the previous
            // spectrum and continue, rather than turning one bad frame into a dead clip.
            Err(Error::BadLsp) => self.lsp.previous(),
            Err(error) => return Err(error),
        };
        let gain = self.gain.decode(frame, rate);
        let mut pitch_gain = [0.0f32; 4];
        let mut pitch_lag = [0u8; 4];
        for i in 0..4 {
            pitch_gain[i] = if frame.plag[i] != 0 {
                (f32::from(frame.pgain[i]) + 1.0) * 0.25
            } else {
                0.0
            };
            pitch_lag[i] = frame.plag[i].saturating_add(16);
        }
        Ok(QcelpParams {
            lsp,
            gain,
            pitch_gain,
            pitch_lag,
        })
    }

    /// Decode one packet to 160 PCM samples at 8 kHz.
    ///
    /// `first16bits` is the big-endian first two payload bytes; it seeds the 1/8-rate random
    /// excitation, as in IS-733 2.4.8.1.2.
    pub fn decode_pcm(
        &mut self,
        frame: &mut QcelpFrame,
        rate: FrameRate,
        first16bits: u16,
    ) -> Result<[f32; 160], Error> {
        let prev_lspf = self.lsp.previous();
        let params = self.decode(frame, rate)?;
        let mut excitation = [0.0f32; 160];
        self.compute_svector(frame, rate, &params.gain, first16bits, &mut excitation);
        self.apply_pitch_filters(
            frame,
            rate,
            &params.pitch_gain,
            &params.pitch_lag,
            &mut excitation,
        );

        let mut out = [0.0f32; 160];
        let mut lpc = [0.0f32; 10];
        for subframe in 0..4 {
            let weight = match rate {
                FrameRate::Full | FrameRate::Half | FrameRate::Quarter => {
                    0.25 * (subframe as f32 + 1.0)
                }
                FrameRate::Eighth if subframe == 0 => 0.625,
                _ => 1.0,
            };
            let lspf = if weight != 1.0 {
                let mut mixed = [0.0f32; 10];
                for i in 0..10 {
                    mixed[i] = params.lsp[i] * weight + prev_lspf[i] * (1.0 - weight);
                }
                mixed
            } else if rate == FrameRate::Silence {
                prev_lspf
            } else {
                params.lsp
            };
            lpc = lspf2lpc(&lspf);

            let start = subframe * 40;
            let hist_start = 10 + start;
            let history: [f32; 10] = self.synth.formant_mem[hist_start - 10..hist_start]
                .try_into()
                .expect("ten history samples");
            let mut block = [0.0f32; 40];
            all_pole_filter(&mut block, &history, &lpc, &excitation[start..start + 40]);
            self.synth.formant_mem[hist_start..hist_start + 40].copy_from_slice(&block);
            out[start..start + 40].copy_from_slice(&block);
        }

        self.apply_postfilter(&mut out, &lpc);
        let tail: [f32; 10] = self.synth.formant_mem[160..170].try_into().unwrap();
        self.synth.formant_mem[..10].copy_from_slice(&tail);
        Ok(out)
    }

    fn compute_svector(
        &mut self,
        frame: &QcelpFrame,
        rate: FrameRate,
        gain: &[f32; 16],
        first16bits: u16,
        cdn: &mut [f32; 160],
    ) {
        match rate {
            FrameRate::Full => {
                for i in 0..16 {
                    let g = gain[i] * 0.01;
                    for j in 0..10 {
                        let index = ((j as i32 - i32::from(frame.cindex[i])) & 127) as usize;
                        cdn[i * 10 + j] = g * f32::from(FULL_CODEBOOK[index]);
                    }
                }
            }
            FrameRate::Half => {
                for i in 0..4 {
                    let g = gain[i] * 0.5;
                    for j in 0..40 {
                        let index = ((j as i32 - i32::from(frame.cindex[i])) & 127) as usize;
                        cdn[i * 40 + j] = g * f32::from(HALF_CODEBOOK[index]);
                    }
                }
            }
            FrameRate::Quarter => {
                let mut seed = (u16::from(frame.lspv[4] & 3) << 14
                    | u16::from(frame.lspv[3] & 0x3f) << 8
                    | u16::from(frame.lspv[2] & 0x60) << 1
                    | u16::from(frame.lspv[1] & 7) << 3
                    | u16::from(frame.lspv[0] & 0x38) >> 3) as u16;
                let memory = &mut self.synth.rnd_fir_filter_mem;
                let mut at = 20usize;
                for i in 0..8 {
                    let g = gain[i] * (QCELP_SQRT1887 / 32768.0);
                    for _ in 0..20 {
                        seed = seed.wrapping_mul(521).wrapping_add(259);
                        memory[at] = f32::from(seed as i16);
                        let mut fir = 0.0f32;
                        for j in 0..10 {
                            fir += RND_FIR_COEFS[j] as f32 * (memory[at - j] + memory[at - 20 + j]);
                        }
                        fir += RND_FIR_COEFS[10] as f32 * memory[at - 10];
                        cdn[i * 20 + (at - 20 - i * 20)] = g * fir;
                        at += 1;
                    }
                }
                memory.copy_within(160..180, 0);
            }
            FrameRate::Eighth => {
                let mut seed = first16bits;
                for i in 0..8 {
                    let g = gain[i] * (QCELP_SQRT1887 / 32768.0);
                    for j in 0..20 {
                        seed = seed.wrapping_mul(521).wrapping_add(259);
                        cdn[i * 20 + j] = g * f32::from(seed as i16);
                    }
                }
            }
            FrameRate::Silence => {}
        }
    }

    fn apply_pitch_filters(
        &mut self,
        frame: &QcelpFrame,
        rate: FrameRate,
        pitch_gain: &[f32; 4],
        pitch_lag: &[u8; 4],
        cdn: &mut [f32; 160],
    ) {
        if matches!(rate, FrameRate::Full | FrameRate::Half | FrameRate::Silence) {
            let synthesis = do_pitchfilter(
                &mut self.synth.pitch_synthesis_filter_mem,
                cdn,
                pitch_gain,
                pitch_lag,
                &frame.pfrac,
            );
            let mut pre_gain = *pitch_gain;
            for gain in &mut pre_gain {
                *gain = 0.5 * gain.min(1.0);
            }
            let pre = do_pitchfilter(
                &mut self.synth.pitch_pre_filter_mem,
                &synthesis,
                &pre_gain,
                pitch_lag,
                &frame.pfrac,
            );
            for block in (0..160).step_by(40) {
                let energy: f32 = (0..40)
                    .map(|i| synthesis[block + i] * synthesis[block + i])
                    .sum();
                let pre_energy: f32 = (0..40).map(|i| pre[block + i] * pre[block + i]).sum();
                let scale = if pre_energy != 0.0 {
                    (energy / pre_energy).sqrt()
                } else {
                    1.0
                };
                for i in 0..40 {
                    cdn[block + i] = pre[block + i] * scale;
                }
            }
        } else {
            self.synth.pitch_synthesis_filter_mem[..143].copy_from_slice(&cdn[17..160]);
            self.synth.pitch_pre_filter_mem[..143].copy_from_slice(&cdn[17..160]);
        }
    }

    fn apply_postfilter(&mut self, samples: &mut [f32; 160], lpc: &[f32; 10]) {
        let mut pow_625 = 1.0f32;
        let mut pow_775 = 1.0f32;
        let mut lpc_s = [0.0f32; 10];
        let mut lpc_p = [0.0f32; 10];
        for i in 0..10 {
            pow_625 *= 0.625;
            pow_775 *= 0.775;
            lpc_s[i] = lpc[i] * pow_625;
            lpc_p[i] = lpc[i] * pow_775;
        }
        let history: [f32; 10] = self.synth.formant_mem[..10].try_into().unwrap();
        let mut zero = [0.0f32; 160];
        zero_filter(
            &mut zero,
            &history,
            &lpc_s,
            &self.synth.formant_mem[10..170],
        );
        let mut pole = [0.0f32; 160];
        all_pole_filter(&mut pole, &self.synth.postfilter_synth_mem, &lpc_p, &zero);
        self.synth
            .postfilter_synth_mem
            .copy_from_slice(&pole[150..160]);

        let mut tilt_mem = self.synth.postfilter_tilt_mem;
        let last = pole[159];
        for i in (1..160).rev() {
            pole[i] -= 0.3 * pole[i - 1];
        }
        pole[0] -= 0.3 * tilt_mem;
        tilt_mem = last;
        self.synth.postfilter_tilt_mem = tilt_mem;

        let speech_energy: f32 = self.synth.formant_mem[10..170].iter().map(|v| v * v).sum();
        let post_energy: f32 = pole.iter().map(|v| v * v).sum();
        let mut gain_scale = if post_energy != 0.0 {
            (speech_energy / post_energy).sqrt()
        } else {
            1.0
        };
        gain_scale *= 1.0 - 0.9375;
        let mut mem = self.synth.postfilter_agc_mem;
        for i in 0..160 {
            mem = 0.9375 * mem + gain_scale;
            samples[i] = pole[i] * mem;
        }
        self.synth.postfilter_agc_mem = mem;
    }
}

fn lsp2polyf(lsp: &[f64; 10], offset: usize, f: &mut [f64; 6]) {
    f[0] = 1.0;
    f[1] = -2.0 * lsp[offset];
    for i in 2..=5 {
        let val = -2.0 * lsp[offset + 2 * i - 2];
        f[i] = val * f[i - 1] + 2.0 * f[i - 2];
        for j in (2..i).rev() {
            f[j] += f[j - 1] * val + f[j - 2];
        }
        f[1] += val;
    }
}

fn lspf2lpc(lspf: &[f32; 10]) -> [f32; 10] {
    let lsp: [f64; 10] = std::array::from_fn(|i| (std::f64::consts::PI * f64::from(lspf[i])).cos());
    let mut pa = [0.0f64; 6];
    let mut qa = [0.0f64; 6];
    lsp2polyf(&lsp, 0, &mut pa);
    lsp2polyf(&lsp, 1, &mut qa);
    let mut lpc = [0.0f32; 10];
    for i in 0..5 {
        let paf = pa[i + 1] + pa[i];
        let qaf = qa[i + 1] - qa[i];
        lpc[i] = (0.5 * (paf + qaf)) as f32;
        lpc[9 - i] = (0.5 * (paf - qaf)) as f32;
    }
    let mut bandwidth = BANDWIDTH_EXPANSION;
    for value in &mut lpc {
        *value *= bandwidth;
        bandwidth *= BANDWIDTH_EXPANSION;
    }
    lpc
}

fn all_pole_filter(out: &mut [f32], history: &[f32; 10], lpc: &[f32; 10], input: &[f32]) {
    for n in 0..input.len() {
        let mut value = input[n];
        for i in 0..10 {
            let prev = if n > i {
                out[n - i - 1]
            } else {
                history[10 + n - i - 1]
            };
            value -= lpc[i] * prev;
        }
        out[n] = value;
    }
}

fn zero_filter(out: &mut [f32], history: &[f32; 10], lpc: &[f32; 10], input: &[f32]) {
    for n in 0..input.len() {
        let mut value = input[n];
        for i in 0..10 {
            let prev = if n > i {
                input[n - i - 1]
            } else {
                history[10 + n - i - 1]
            };
            value += lpc[i] * prev;
        }
        out[n] = value;
    }
}

fn do_pitchfilter(
    memory: &mut [f32; 303],
    input: &[f32],
    gain: &[f32; 4],
    lag: &[u8; 4],
    pfrac: &[u8; 4],
) -> [f32; 160] {
    let mut out = [0.0f32; 160];
    for i in 0..4 {
        let start = 143 + 40 * i - usize::from(lag[i]);
        for sample in 0..40 {
            let source = if pfrac[i] != 0 {
                let mut value = 0.0f32;
                for j in 0..4 {
                    let a = start + sample + j;
                    let b = start + sample + 3 - j;
                    let left = if a >= 4 { memory[a - 4] } else { 0.0 };
                    let right = memory[b];
                    value += HAMMSINC[j] * (left + right);
                }
                value
            } else {
                memory[start + sample]
            };
            out[i * 40 + sample] = input[i * 40 + sample] + gain[i] * source;
        }
    }
    memory[143..303].copy_from_slice(&out);
    memory.copy_within(160..303, 0);
    out
}

/// Decode a whole QCELP 13K QCP clip to mono f32 PCM at 8 kHz.
///
/// This is the single decoder entry the player should call. EVRC returns
/// [`Error::UnsupportedCodec`] instead of the host claiming a format it cannot render.
pub fn decode_qcp(bytes: &[u8]) -> Result<(u32, Vec<f32>), Error> {
    let qcp = Qcp::parse(bytes)?;
    if qcp.codec == Codec::Evrc {
        return crate::audio::evrc::decode_qcp(bytes);
    }
    if qcp.codec != Codec::Qcelp13 {
        return Err(Error::UnsupportedCodec);
    }
    let mut decoder = QcelpDecoder::default();
    let mut out = Vec::new();
    for frame in qcp.frames()? {
        let mut unpacked = QcelpFrame::unpack(frame.rate, frame.payload)?;
        let seed = if frame.payload.len() >= 2 {
            u16::from_be_bytes([frame.payload[0], frame.payload[1]])
        } else {
            0
        };
        out.extend_from_slice(&decoder.decode_pcm(&mut unpacked, frame.rate, seed)?);
    }
    Ok((8_000, out))
}

#[cfg(test)]
#[path = "../test/qcelp.rs"]
mod tests;
