// SPDX-License-Identifier: Apache-2.0
//
// Ported from the yamaha-smaf-player C++ engine (Copyright (c) Andreas Wendorf,
// Apache-2.0), `smaf_voice`. Decodes the VM35 (MA-3/MA-5) and
// VMA (MA-1/MA-2) voice exclusives into the FM patch the core consumes.

use super::fm::{OpPatch, VoicePatch};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoiceKey {
    pub bank_msb: i32,
    pub bank_lsb: i32,
    pub pc: i32,
    pub drum_note: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct PcmParams {
    pub fs: i32,
    pub wave_id: i32,
    pub loop_pt: i32,
    pub end_pt: i32,
    pub loop_: bool,
    pub env: OpPatch,
}

impl Default for PcmParams {
    fn default() -> Self {
        Self {
            fs: 8000,
            wave_id: 0,
            loop_pt: 0,
            end_pt: 0,
            loop_: false,
            env: OpPatch::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParsedVoice {
    pub key: VoiceKey,
    pub patch: VoicePatch,
    pub pcm: PcmParams,
    pub is_pcm: bool,
    pub valid: bool,
}

impl Default for ParsedVoice {
    fn default() -> Self {
        Self {
            key: VoiceKey::default(),
            patch: VoicePatch::default_patch(),
            pcm: PcmParams::default(),
            is_pcm: false,
            valid: false,
        }
    }
}

fn bo_to_semitones(bo: i32) -> i32 {
    match bo & 3 {
        0 => 12,
        1 => 0,
        2 => -12,
        _ => -24,
    }
}

fn panpot_to_pan(pp: i32) -> f32 {
    let pp = pp & 31;
    if pp == 15 {
        0.0
    } else if pp < 15 {
        -((15 - pp) as f32) / 15.0
    } else {
        ((pp - 15) as f32) / 16.0
    }
}

fn apply_alg(v: &mut VoicePatch, alg: i32) {
    let alg = alg & 7;
    v.four_op = alg >= 2;
    v.algorithm = alg as u8;
}

fn op_count_from_alg(alg: i32) -> usize {
    if alg & 7 <= 1 {
        2
    } else {
        4
    }
}

fn apply_vm35_body(g: &[u8], operator_count: usize, v: &mut VoicePatch) {
    if g.len() < 3 {
        return;
    }
    let bo = i32::from(g[1] & 0x03);
    let pp = i32::from((g[1] >> 3) & 0x1f);
    let pe = (g[2] >> 5) & 1 != 0;
    let alg = i32::from(g[2] & 0x07);
    apply_alg(v, alg);
    v.note_shift = bo_to_semitones(bo);
    v.pan_default = if pe { panpot_to_pan(pp) } else { 0.0 };
    v.lfo = (g[2] >> 6) & 0x03;

    let mut pos = 3;
    let count = operator_count.clamp(1, 4);
    for i in 0..count {
        if pos + 7 > g.len() {
            break;
        }
        let op = &g[pos..pos + 7];
        let o = &mut v.ops[i];
        o.sr = (op[0] >> 4) & 0x0f;
        o.ksr = op[0] & 0x01;
        o.eg_type = o.sr != 0;
        o.rr = (op[1] >> 4) & 0x0f;
        o.dr = op[1] & 0x0f;
        o.ar = (op[2] >> 4) & 0x0f;
        o.sl = op[2] & 0x0f;
        o.tl = (op[3] >> 2) & 0x3f;
        o.ksl = op[3] & 0x03;
        o.xof = (op[0] >> 3) & 1 != 0;
        o.am = (op[4] >> 4) & 1 != 0;
        o.dam = (op[4] >> 5) & 3;
        o.vib = op[4] & 1 != 0;
        o.dvb = (op[4] >> 1) & 3;
        o.multi = (op[5] >> 4) & 0x0f;
        o.dt = op[5] & 0x07;
        o.wave = (op[6] >> 3) & 0x1f;
        o.fb = op[6] & 0x07;
        if i == 0 {
            v.feedback = o.fb;
        }
        pos += 7;
    }
}

fn apply_ma3_packed(body: &[u8], operator_count: usize, v: &mut VoicePatch) {
    let mut raw = [0u8; 36];
    let copy = body.len().min(36);
    raw[..copy].copy_from_slice(&body[..copy]);

    raw[2] |= (raw[0] << 2) & 0x80;
    raw[3] |= (raw[0] << 3) & 0x80;
    for op in 0..4 {
        raw[4 + op * 8] |= (raw[op * 8] << 4) & 0x80;
        raw[5 + op * 8] |= (raw[op * 8] << 5) & 0x80;
        raw[6 + op * 8] |= (raw[op * 8] << 6) & 0x80;
        raw[7 + op * 8] |= (raw[op * 8] << 7) & 0x80;
        raw[10 + op * 8] |= (raw[8 + op * 8] << 2) & 0x80;
        raw[11 + op * 8] |= (raw[8 + op * 8] << 3) & 0x80;
    }

    let mut fixed = [0u8; 3 + 7 * 4];
    fixed[0] = raw[1];
    fixed[1] = raw[2];
    fixed[2] = raw[3];
    let mut w = 3;
    for op in 0..4 {
        fixed[w] = raw[4 + op * 8];
        fixed[w + 1] = raw[5 + op * 8];
        fixed[w + 2] = raw[6 + op * 8];
        fixed[w + 3] = raw[7 + op * 8];
        fixed[w + 4] = raw[9 + op * 8];
        fixed[w + 5] = raw[10 + op * 8];
        fixed[w + 6] = raw[11 + op * 8];
        w += 7;
    }
    apply_vm35_body(&fixed[..w], operator_count, v);
}

/// Parse one Yamaha exclusive payload (starting at the `0x43` maker id, ending
/// before the trailing `0xF7`).
pub fn parse_voice_exclusive(p: &[u8]) -> ParsedVoice {
    let mut out = ParsedVoice::default();
    if p.len() < 5 || p[0] != 0x43 {
        return out;
    }

    // MA-3 / MA-5 long form: 43 79 06|07 7F 01 [bank...] body
    if p.len() >= 11
        && p[1] == 0x79
        && (p[2] == 0x06 || p[2] == 0x07)
        && p[3] == 0x7f
        && p[4] == 0x01
    {
        out.key.bank_msb = i32::from(p[5]);
        out.key.bank_lsb = i32::from(p[6]);
        out.key.pc = i32::from(p[7]);
        out.key.drum_note = i32::from(p[8]);
        let voice_type = p[9];
        if voice_type != 0 {
            out.is_pcm = true;
            let b = &p[10..];
            if b.len() >= 16 {
                let pc = &mut out.pcm;
                pc.fs = (i32::from(b[0]) << 8) | i32::from(b[1]);
                pc.env.tl = (b[7] >> 2) & 0x3f;
                pc.env.sr = (b[4] >> 4) & 0x0f;
                pc.env.rr = (b[5] >> 4) & 0x0f;
                pc.env.dr = b[5] & 0x0f;
                pc.env.ar = (b[6] >> 4) & 0x0f;
                pc.env.sl = b[6] & 0x0f;
                pc.env.eg_type = pc.env.sr != 0;
                pc.loop_pt = (i32::from(b[11]) << 8) | i32::from(b[12]);
                pc.end_pt = (i32::from(b[13]) << 8) | i32::from(b[14]);
                pc.loop_ = b[15] & 0x80 != 0;
                pc.wave_id = i32::from(b[15] & 0x7f);
                if !(2000..=48_000).contains(&pc.fs) {
                    pc.fs = 8000;
                }
                out.valid = true;
            }
            return out;
        }

        let body = &p[10..];
        let alg_byte = if p[2] == 0x06 { 3 } else { 2 };
        let alg = if body.len() > alg_byte {
            i32::from(body[alg_byte] & 0x07)
        } else {
            0
        };
        let ops = op_count_from_alg(alg);
        if p[2] == 0x06 {
            apply_ma3_packed(body, ops, &mut out.patch);
        } else {
            apply_vm35_body(body, ops, &mut out.patch);
        }
        out.valid = true;
        return out;
    }

    // MA-5 short form: 43 05 01 [bankLSB] [pc] body
    if p.len() >= 6 && p[1] == 0x05 && p[2] == 0x01 {
        out.key.bank_msb = 0;
        out.key.bank_lsb = i32::from(p[3]);
        out.key.pc = i32::from(p[4]);
        let body = &p[5..];
        let alg = if body.len() >= 3 {
            i32::from(body[2] & 0x07)
        } else {
            0
        };
        apply_vm35_body(body, op_count_from_alg(alg), &mut out.patch);
        out.valid = true;
        return out;
    }

    // MA-1 / MA-2 (VMA): 43 03 [Enigma][Bank][PC] then 2 global + 5/op.
    if p.len() >= 7 && p[1] == 0x03 {
        out.key.bank_msb = 0;
        out.key.bank_lsb = i32::from(p[3] & 0x7f);
        out.key.pc = i32::from(p[4]);
        let g = &p[5..];
        if g.len() >= 2 {
            let fb = i32::from((g[0] >> 3) & 0x07);
            let alg = i32::from(g[0] & 0x07);
            apply_alg(&mut out.patch, alg);
            out.patch.feedback = fb as u8;
            out.patch.lfo = (g[0] >> 6) & 3;
            let count = op_count_from_alg(alg);
            let mut pos = 2;
            for i in 0..count {
                if pos + 5 > g.len() {
                    break;
                }
                let op = &g[pos..pos + 5];
                let o = &mut out.patch.ops[i];
                o.multi = (op[0] >> 4) & 0x0f;
                o.vib = op[0] & 0x08 != 0;
                let egt = op[0] & 0x04 != 0;
                o.ksr = op[0] & 0x01;
                o.rr = (op[1] >> 4) & 0x0f;
                o.dr = op[1] & 0x0f;
                o.ar = (op[2] >> 4) & 0x0f;
                o.sl = op[2] & 0x0f;
                o.tl = (op[3] >> 2) & 0x3f;
                o.ksl = op[3] & 0x03;
                o.am = op[4] & 0x08 != 0;
                o.dvb = (op[4] >> 6) & 3;
                o.dam = (op[4] >> 4) & 3;
                o.wave = op[4] & 0x07;
                o.sr = if egt { 0 } else { (op[1] >> 4) & 0x0f };
                o.eg_type = o.sr != 0;
                if i == 0 {
                    o.fb = fb as u8;
                }
                pos += 5;
            }
            out.valid = true;
        }
        return out;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_yamaha_payload_is_invalid() {
        assert!(!parse_voice_exclusive(b"").valid);
        assert!(!parse_voice_exclusive(&[0x41, 0x79, 0x06, 0x7f, 0x01]).valid);
    }

    #[test]
    fn the_vma_header_maps_key_algorithm_and_operators() {
        // 43 03 [enigma] [bank] [pc], then g0/g1 and two 5-byte operators.
        // g0 = (fb 3 << 3) | alg 1 (2-op parallel).
        let mut p = vec![0x43, 0x03, 0x00, 0x02, 0x05, 0x19, 0x01];
        p.extend_from_slice(&[0x20, 0x65, 0xa4, 0x12, 0x1b]); // op0
        p.extend_from_slice(&[0x11, 0x87, 0xc0, 0x00, 0x07]); // op1
        let v = parse_voice_exclusive(&p);
        assert!(v.valid);
        assert!(!v.is_pcm);
        assert_eq!(v.key.bank_lsb, 2);
        assert_eq!(v.key.pc, 5);
        assert_eq!(v.patch.algorithm, 1);
        assert!(!v.patch.four_op);
        assert_eq!(v.patch.feedback, 3);
        assert_eq!(v.patch.lfo, 0);

        let op0 = v.patch.ops[0];
        assert_eq!(
            (op0.multi, op0.rr, op0.dr, op0.ar, op0.sl),
            (2, 6, 5, 10, 4)
        );
        assert_eq!((op0.tl, op0.ksl), (4, 2));
        assert_eq!((op0.am, op0.dam, op0.dvb, op0.wave), (true, 1, 0, 3));
        assert_eq!((op0.sr, op0.eg_type, op0.fb), (6, true, 3));

        let op1 = v.patch.ops[1];
        assert_eq!((op1.multi, op1.ksr, op1.wave), (1, 1, 7));
        assert_eq!((op1.sr, op1.eg_type), (8, true));
    }

    #[test]
    fn a_real_payload_can_be_named_from_the_environment() {
        let Ok(path) = std::env::var("QUOIN_SMAF_VOICE") else {
            eprintln!("skipped: QUOIN_SMAF_VOICE is not set");
            return;
        };
        let bytes = std::fs::read(&path).expect("read the payload named by QUOIN_SMAF_VOICE");
        let v = parse_voice_exclusive(&bytes);
        assert!(
            v.valid,
            "the named payload does not parse as a Yamaha voice"
        );
    }
}
