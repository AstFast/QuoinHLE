//! EVRC (TIA/IS-127) frame decoding and synthesis.
//!
//! The tables are generated from FFmpeg's `evrcdata.h` (LGPL-2.1-or-later, Copyright (c) 2013
//! Paul B Mahol). The project is GPL-2.0-or-later, so the reference is compatible; the generated
//! table file carries the attribution.
//!
//! This module decodes the EVRC packets that QCP carries. The adaptive postfilter is not part of
//! this first synthesis pass; the decoded speech is produced by the LPC synthesis filter. That is a
//! deliberate, visible limitation, not a silent substitution.

use crate::audio::evrc_tables::{
    ENERGY_QUANT, LSPQ_FULL1, LSPQ_FULL2, LSPQ_FULL3, LSPQ_FULL4, LSPQ_HALF1, LSPQ_HALF2,
    LSPQ_HALF3, LSPQ_QUANT1, LSPQ_QUANT2, PITCH_GAIN_VQ, SUBFRAME_SIZES,
};
use crate::audio::qcelp::{Codec, Error, Qcp};

const FILTER_ORDER: usize = 10;
const ACB_SIZE: usize = 128;
const NB_SUBFRAMES: usize = 3;
const SUBFRAME_SIZE: usize = 54;
const MIN_DELAY: f32 = 20.0;
const MAX_DELAY: f32 = 120.0;
const MIN_LSP_SEP: f32 = 0.05 / (2.0 * std::f32::consts::PI);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rate {
    Silence,
    Quant,
    Quarter,
    Half,
    Full,
}

impl Rate {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Silence),
            1 => Some(Self::Quant),
            2 => Some(Self::Quarter),
            3 => Some(Self::Half),
            4 => Some(Self::Full),
            _ => None,
        }
    }

    fn packet_len(self) -> usize {
        match self {
            Self::Silence => 1,
            Self::Quant => 3,
            Self::Quarter => 6,
            Self::Half => 11,
            Self::Full => 23,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EvrcFrame {
    lpc_flag: u8,
    lsp: [u16; 4],
    pitch_delay: u8,
    delay_diff: u8,
    acb_gain: [u8; 3],
    fcb_shape: [[u16; 4]; 3],
    fcb_gain: [u8; 3],
    energy_gain: u8,
    tty: u8,
}

struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    fn read(&mut self, count: u8) -> Option<u32> {
        if self.at + usize::from(count) > self.data.len() * 8 {
            return None;
        }
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self.data[self.at / 8];
            value = (value << 1) | u32::from((byte >> (7 - self.at % 8)) & 1);
            self.at += 1;
        }
        Some(value)
    }
}

fn unpack(rate: Rate, payload: &[u8]) -> EvrcFrame {
    let mut bits = Bits::new(payload);
    let mut frame = EvrcFrame::default();
    match rate {
        Rate::Full => {
            frame.lpc_flag = bits.read(1).unwrap_or(0) as u8;
            frame.lsp[0] = bits.read(6).unwrap_or(0) as u16;
            frame.lsp[1] = bits.read(6).unwrap_or(0) as u16;
            frame.lsp[2] = bits.read(9).unwrap_or(0) as u16;
            frame.lsp[3] = bits.read(7).unwrap_or(0) as u16;
            frame.pitch_delay = bits.read(7).unwrap_or(0) as u8;
            frame.delay_diff = bits.read(5).unwrap_or(0) as u8;
            for i in 0..3 {
                frame.acb_gain[i] = bits.read(3).unwrap_or(0) as u8;
                for j in 0..4 {
                    frame.fcb_shape[i][j] =
                        bits.read(if j == 3 { 11 } else { 8 }).unwrap_or(0) as u16;
                }
                frame.fcb_gain[i] = bits.read(5).unwrap_or(0) as u8;
            }
            frame.tty = bits.read(1).unwrap_or(0) as u8;
        }
        Rate::Half => {
            frame.lsp[0] = bits.read(7).unwrap_or(0) as u16;
            frame.lsp[1] = bits.read(7).unwrap_or(0) as u16;
            frame.lsp[2] = bits.read(8).unwrap_or(0) as u16;
            frame.pitch_delay = bits.read(7).unwrap_or(0) as u8;
            for i in 0..3 {
                frame.acb_gain[i] = bits.read(3).unwrap_or(0) as u8;
                frame.fcb_shape[i][0] = bits.read(10).unwrap_or(0) as u16;
                frame.fcb_gain[i] = bits.read(4).unwrap_or(0) as u8;
            }
        }
        Rate::Quant => {
            frame.lsp[0] = bits.read(4).unwrap_or(0) as u16;
            frame.lsp[1] = bits.read(4).unwrap_or(0) as u16;
            frame.energy_gain = bits.read(8).unwrap_or(0) as u8;
        }
        Rate::Quarter | Rate::Silence => {}
    }
    frame
}

fn lsp_values(rate: Rate, frame: &EvrcFrame) -> Option<[f32; 10]> {
    let mut out = [0.0f32; 10];
    let mut at = 0;
    let rows: &[(&[f32], usize)] = match rate {
        Rate::Full => &[
            (&LSPQ_FULL1[frame.lsp[0] as usize & 63], 2),
            (&LSPQ_FULL2[frame.lsp[1] as usize & 63], 2),
            (&LSPQ_FULL3[frame.lsp[2] as usize & 511], 3),
            (&LSPQ_FULL4[frame.lsp[3] as usize & 127], 3),
        ],
        Rate::Half => &[
            (&LSPQ_HALF1[frame.lsp[0] as usize & 127], 3),
            (&LSPQ_HALF2[frame.lsp[1] as usize & 127], 3),
            (&LSPQ_HALF3[frame.lsp[2] as usize & 255], 4),
        ],
        Rate::Quant => &[
            (&LSPQ_QUANT1[frame.lsp[0] as usize & 15], 5),
            (&LSPQ_QUANT2[frame.lsp[1] as usize & 15], 5),
        ],
        _ => return None,
    };
    for &(row, width) in rows {
        if at + width > out.len() {
            return None;
        }
        out[at..at + width].copy_from_slice(&row[..width]);
        at += width;
    }
    for i in 1..FILTER_ORDER {
        if out[i] <= out[i - 1] {
            return None;
        }
    }
    let mut split = 0;
    for &(_, width) in &rows[..rows.len().saturating_sub(1)] {
        split += width;
        if out[split] - out[split - 1] <= MIN_LSP_SEP {
            return None;
        }
    }
    Some(out)
}

fn interpolate_lsp(current: &[f32; 10], previous: &[f32; 10], index: usize) -> [f32; 10] {
    const FACTORS: [f32; 3] = [0.1667, 0.5, 0.8333];
    let factor = FACTORS[index];
    let mut out = [0.0f32; 10];
    for i in 0..10 {
        out[i] = previous[i] * (1.0 - factor) + current[i] * factor;
    }
    out
}

fn interpolate_delay(current: f32, previous: f32, index: usize) -> [f32; 3] {
    const FACTORS: [f32; 5] = [0.0, 0.3313, 0.6625, 1.0, 1.0];
    [
        previous * (1.0 - FACTORS[index]) + current * FACTORS[index],
        previous * (1.0 - FACTORS[index + 1]) + current * FACTORS[index + 1],
        previous * (1.0 - FACTORS[index + 2]) + current * FACTORS[index + 2],
    ]
}

fn predictor_coeffs(lsp: &[f32; 10]) -> [f32; 10] {
    let lsp: [f64; 10] =
        std::array::from_fn(|i| (2.0 * std::f64::consts::PI * f64::from(lsp[i])).cos());
    let mut a = [0.0f64; 6];
    let mut b = [0.0f64; 6];
    let mut a1 = [0.0f64; 5];
    let mut a2 = [0.0f64; 5];
    let mut b1 = [0.0f64; 5];
    let mut b2 = [0.0f64; 5];
    let mut out = [0.0f32; 10];
    for k in 0..=FILTER_ORDER {
        a[0] = if k < 2 { 0.25 } else { 0.0 };
        b[0] = if k < 2 {
            if k < 1 {
                0.25
            } else {
                -0.25
            }
        } else {
            0.0
        };
        for i in 0..5 {
            a[i + 1] = a[i] - 2.0 * lsp[i * 2] * a1[i] + a2[i];
            b[i + 1] = b[i] - 2.0 * lsp[i * 2 + 1] * b1[i] + b2[i];
            a2[i] = a1[i];
            a1[i] = a[i];
            b2[i] = b1[i];
            b1[i] = b[i];
        }
        if k > 0 {
            out[k - 1] = (2.0 * (a[5] + b[5])) as f32;
        }
    }
    out
}

fn bl_interp(coeffs: &[f32; 136], pitch: &[f32], ex: usize, delay: f32) -> f32 {
    let mut offset = delay.round() as i32;
    let mut t = ((offset as f32 - delay + 0.5) * 8.0 + 0.5) as i32;
    if t == 8 {
        t = 0;
        offset -= 1;
    }
    let base = ex as i32 - offset - 8;
    let mut value = 0.0f32;
    for i in 0..17 {
        let index = base + i as i32;
        if index >= 0 && (index as usize) < pitch.len() {
            value += coeffs[(t * 17 + i) as usize] * pitch[index as usize];
        }
    }
    value
}

fn acb_excitation(state: &mut EvrcState, delay: &[f32; 3], gain: f32, length: usize) {
    let inv = 1.0 / length as f32;
    let d0 = (delay[1] - delay[0]) * inv;
    let mut local = [0.0f32; SUBFRAME_SIZE + 10];
    for i in 0..length {
        let loc = delay[0] + i as f32 * d0;
        local[i] = bl_interp(&state.interpolation, &state.pitch, ACB_SIZE + i, loc);
    }
    let d1 = (delay[2] - delay[1]) * inv;
    for i in length..length + 10 {
        let loc = delay[1] + (i - length) as f32 * d1;
        local[i] = bl_interp(&state.interpolation, &state.pitch, ACB_SIZE + i, loc);
    }
    for i in 0..length {
        state.pitch[ACB_SIZE + i] = local[i] * gain;
    }
    for i in length..length + 10 {
        state.pitch[ACB_SIZE + i] = local[i];
    }
}

fn fcb_excitation(
    frame: &EvrcFrame,
    rate: Rate,
    subframe: usize,
    pitch_gain: f32,
    pitch_lag: usize,
    out: &mut [f32],
) {
    if rate == Rate::Full {
        let shape = &frame.fcb_shape[subframe];
        let offset = ((shape[3] >> 9) & 3) as usize;
        for i in 0..3 {
            let pos1 = (((shape[i] & 0x7f) / 11) * 5) as usize + ((i + offset) % 5);
            let pos2 = (((shape[i] & 0x7f) % 11) * 5) as usize + ((i + offset) % 5);
            out[pos1] = if shape[i] & 0x80 != 0 { -1.0 } else { 1.0 };
            if pos2 < pos1 {
                out[pos2] = -out[pos1];
            } else {
                out[pos2] += out[pos1];
            }
        }
        let pos1 = (((shape[3] & 0x7f) / 11) * 5) as usize + ((3 + offset) % 5);
        let pos2 = (((shape[3] & 0x7f) % 11) * 5) as usize + ((4 + offset) % 5);
        out[pos1] = if shape[3] & 0x100 != 0 { -1.0 } else { 1.0 };
        out[pos2] = if shape[3] & 0x80 != 0 { -1.0 } else { 1.0 };
    } else {
        let shape = &frame.fcb_shape[subframe];
        let index = shape[0];
        let sign = if index & 0x200 != 0 { -1.0 } else { 1.0 };
        let mut pos = ((index & 7) * 7) as usize + 4;
        out[pos] += sign;
        pos = (((index >> 3) & 7) * 7) as usize + 2;
        out[pos] -= sign;
        pos = (((index >> 6) & 7) * 7) as usize;
        out[pos] += sign;
    }
    let gain = pitch_gain.clamp(0.2, 0.9);
    if pitch_lag > 0 {
        for i in pitch_lag..out.len() {
            out[i] += gain * out[i - pitch_lag];
        }
    }
}

fn synthesis_filter(input: &[f32], lpc: &[f32; 10], memory: &mut [f32; 10], out: &mut [f32]) {
    for i in 0..input.len() {
        let mut value = input[i];
        for j in (1..FILTER_ORDER).rev() {
            value -= lpc[j] * memory[j];
            memory[j] = memory[j - 1];
        }
        value -= lpc[0] * memory[0];
        memory[0] = value;
        out[i] = value;
    }
}

struct EvrcState {
    prev_lspf: [f32; 10],
    synthesis: [f32; 10],
    pitch: [f32; ACB_SIZE + FILTER_ORDER + SUBFRAME_SIZE],
    pitch_back: [f32; ACB_SIZE],
    interpolation: [f32; 136],
    pitch_delay: f32,
    prev_pitch_delay: f32,
    avg_acb_gain: f32,
    avg_fcb_gain: f32,
    fade_scale: f32,
    energy: [f32; 3],
}

impl Default for EvrcState {
    fn default() -> Self {
        let mut prev_lspf = [0.0f32; 10];
        for (i, value) in prev_lspf.iter_mut().enumerate() {
            *value = (i as f32 + 1.0) * 0.048;
        }
        let mut interpolation = [0.0f32; 136];
        let denom = 2.0 / 17.0;
        let mut at = 0;
        for i in 0..8 {
            let tt = (i as f32 - 4.0) / 8.0;
            for n in -8..=8 {
                let arg1 = std::f32::consts::PI * 0.9 * (tt - n as f32);
                let arg2 = std::f32::consts::PI * (tt - n as f32);
                let mut value = 0.9f32;
                if arg1 != 0.0 {
                    value *= (0.54 + 0.46 * (arg2 * denom).cos()) * arg1.sin() / arg1;
                }
                interpolation[at] = value;
                at += 1;
            }
        }
        Self {
            prev_lspf,
            synthesis: [0.0; 10],
            pitch: [0.0; ACB_SIZE + FILTER_ORDER + SUBFRAME_SIZE],
            pitch_back: [0.0; ACB_SIZE],
            interpolation,
            pitch_delay: 40.0,
            prev_pitch_delay: 40.0,
            avg_acb_gain: 0.0,
            avg_fcb_gain: 0.0,
            fade_scale: 1.0,
            energy: [0.0; 3],
        }
    }
}

/// Stateful EVRC decoder.
#[derive(Default)]
pub struct EvrcDecoder {
    state: EvrcState,
}

impl EvrcDecoder {
    /// Decode one packet to 160 samples at 8 kHz.
    pub fn decode(&mut self, rate: Rate, payload: &[u8]) -> [f32; 160] {
        let frame = unpack(rate, payload);
        let Some(lsp) = lsp_values(rate, &frame) else {
            return self.erasure(rate);
        };
        let mut out = [0.0f32; 160];
        if rate == Rate::Full || rate == Rate::Half {
            if frame.pitch_delay as f32 > MAX_DELAY - MIN_DELAY {
                return self.erasure(rate);
            }
            self.state.pitch_delay = frame.pitch_delay as f32 + MIN_DELAY;
            self.state.avg_acb_gain = 0.0;
            self.state.avg_fcb_gain = 0.0;
        } else if rate == Rate::Quant {
            for i in 0..3 {
                self.state.energy[i] = 10.0f32.powf(ENERGY_QUANT[frame.energy_gain as usize][i]);
            }
        }

        let mut idelay = [MIN_DELAY; 3];
        for i in 0..NB_SUBFRAMES {
            let size = SUBFRAME_SIZES[i];
            let ilsp = interpolate_lsp(&lsp, &self.state.prev_lspf, i);
            if rate != Rate::Quant {
                idelay = interpolate_delay(self.state.pitch_delay, self.state.prev_pitch_delay, i);
            }
            let pitch_lag = ((idelay[1] + idelay[0]) / 2.0).round() as usize;
            let lpc = predictor_coeffs(&ilsp);
            let mut tmp = [0.0f32; SUBFRAME_SIZE + 6];
            if rate != Rate::Quant {
                let gain = PITCH_GAIN_VQ[frame.acb_gain[i] as usize];
                let fcb_gain = ((if rate == Rate::Half { 0.5 } else { 0.25 })
                    * (frame.fcb_gain[i] as f32 + 1.0))
                    .exp();
                self.state.avg_acb_gain += gain / 3.0;
                self.state.avg_fcb_gain += fcb_gain / 3.0;
                acb_excitation(&mut self.state, &idelay, gain, size);
                fcb_excitation(&frame, rate, i, gain, pitch_lag, &mut tmp);
                for j in 0..size {
                    self.state.pitch[ACB_SIZE + j] += fcb_gain * tmp[j];
                }
                self.state.fade_scale = (self.state.fade_scale + 0.2).min(1.0);
            } else {
                for j in 0..size {
                    self.state.pitch[ACB_SIZE + j] = self.state.energy[i];
                }
            }
            self.state.pitch.copy_within(size..size + ACB_SIZE, 0);
            let mut sub = [0.0f32; SUBFRAME_SIZE];
            synthesis_filter(
                &self.state.pitch[ACB_SIZE..ACB_SIZE + size],
                &lpc,
                &mut self.state.synthesis,
                &mut sub,
            );
            out[i * 53..i * 53 + size].copy_from_slice(&sub[..size]);
        }

        self.state.prev_lspf = lsp;
        if rate != Rate::Quant {
            self.state.prev_pitch_delay = self.state.pitch_delay;
        }
        for sample in &mut out {
            *sample /= 32768.0;
        }
        out
    }

    fn erasure(&mut self, _rate: Rate) -> [f32; 160] {
        self.state
            .pitch_back
            .copy_from_slice(&self.state.pitch[..ACB_SIZE]);
        self.state
            .prev_lspf
            .iter_mut()
            .enumerate()
            .for_each(|(i, value)| *value = *value * 0.875 + 0.125 * (i + 1) as f32 * 0.048);
        [0.0; 160]
    }
}

/// Decode an EVRC QCP container to 8 kHz mono PCM.
pub fn decode_qcp(bytes: &[u8]) -> Result<(u32, Vec<f32>), Error> {
    let qcp = Qcp::parse(bytes)?;
    if qcp.codec != Codec::Evrc {
        return Err(Error::UnsupportedCodec);
    }
    let mut decoder = EvrcDecoder::default();
    let mut out = Vec::new();
    let mut at = 0;
    while at < qcp.data.len() {
        let rate = Rate::from_byte(qcp.data[at]).ok_or(Error::BadFrameRate(qcp.data[at]))?;
        let length = rate.packet_len();
        if at + length > qcp.data.len() {
            break;
        }
        let payload = &qcp.data[at + 1..at + length];
        out.extend_from_slice(&decoder.decode(rate, payload));
        at += length;
    }
    Ok((8_000, out))
}

#[cfg(test)]
#[path = "../test/evrc.rs"]
mod tests;
