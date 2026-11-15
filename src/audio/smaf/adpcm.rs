// SPDX-License-Identifier: Apache-2.0
//
// Ported from the yamaha-smaf-player C++ engine (Copyright (c) Andreas Wendorf,
// Apache-2.0), `yamaha_adpcm.h`: the 4-bit YM2608 OPNA-B codec used by
// SMAF ATR/stream PCM. The reference engine decodes SMAF wave payloads low-nibble
// first (`buildWaveBank_` passes `highNibbleFirst = false`); the flag is kept because
// the codec itself is nibble-order agnostic.

/// One adaptive decoder state. `decode_nibble` is the single-sample entry point; the state
/// persists across nibbles, so a packed buffer must be fed in order.
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    last: i32,
    step: i32,
}

impl Default for Decoder {
    fn default() -> Self {
        Self { last: 0, step: 127 }
    }
}

impl Decoder {
    pub fn reset(&mut self) {
        self.last = 0;
        self.step = 127;
    }

    pub fn decode_nibble(&mut self, code: u8) -> i16 {
        let mut delta = self.step >> 3;
        if code & 1 != 0 {
            delta += self.step >> 2;
        }
        if code & 2 != 0 {
            delta += self.step >> 1;
        }
        if code & 4 != 0 {
            delta += self.step;
        }
        if code & 8 != 0 {
            delta = -delta;
        }
        self.last = (self.last + delta).clamp(-32768, 32767);
        self.step = match code & 7 {
            0..=3 => self.step * 115 / 128,
            4 => self.step * 307 / 256,
            5 => self.step * 409 / 256,
            6 => self.step * 2,
            _ => self.step * 307 / 128,
        }
        .clamp(127, 24576);
        self.last as i16
    }
}

/// Decode a packed buffer. SMAF wave payloads use low-nibble-first; the reference engine's
/// `highNibbleFirst` switch remains available for other callers.
pub fn decode_all(data: &[u8], high_nibble_first: bool) -> Vec<i16> {
    let mut decoder = Decoder::default();
    let mut out = Vec::with_capacity(data.len() * 2);
    for &byte in data {
        let hi = byte >> 4;
        let lo = byte & 0x0f;
        if high_nibble_first {
            out.push(decoder.decode_nibble(hi));
            out.push(decoder.decode_nibble(lo));
        } else {
            out.push(decoder.decode_nibble(lo));
            out.push(decoder.decode_nibble(hi));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_decoder_on_a_fixed_vector() {
        // Witness: the vendored C++ `YamahaAdpcm::decodeAll(..., false)` on this exact
        // buffer prints these sixteen samples.
        let data = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        assert_eq!(
            decode_all(&data, false),
            vec![
                78, 124, 266, 399, 620, 994, 940, 1668, 1086, 773, -71, -858, -2171, -4393, -4071,
                -8416
            ]
        );
    }

    #[test]
    fn a_fresh_decoder_stays_in_range_and_adapts() {
        let mut decoder = Decoder::default();
        assert_eq!(decoder.decode_nibble(0), 15);
        for code in 0..16 {
            let sample = decoder.decode_nibble(code);
            assert!(sample > -32768 && sample < 32767);
        }
        assert!(decoder.step >= 127 && decoder.step <= 24576);
    }
}
