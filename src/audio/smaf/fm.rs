// SPDX-License-Identifier: Apache-2.0
//
// Ported from the yamaha-smaf-player C++ engine (Copyright (c) Andreas Wendorf,
// Apache-2.0), `ma_fm_core`. The DSP structure is kept:
// exponential ADSR, 2-op/4-op operators, the eight MA connection algorithms,
// voice LFO and the ROM-free GM approximation bank. The port is intentionally
// dependency-free so it can be exercised without the rest of the host.

use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

/// One full turn, spelled with the standard constant: the literal this stood in for was the same
/// value to the last bit, and a hand-written approximation only invites a linter to say so.
const TWO_PI: f64 = TAU;
const MOD_DEPTH: f64 = 0.42;

/// One operator's patch, in the chip's own register units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpPatch {
    pub multi: u8,
    pub tl: u8,
    pub ar: u8,
    pub dr: u8,
    pub sr: u8,
    pub rr: u8,
    pub sl: u8,
    pub ksl: u8,
    pub ksr: u8,
    pub wave: u8,
    pub dt: u8,
    pub fb: u8,
    pub am: bool,
    pub vib: bool,
    pub eg_type: bool,
    pub dvb: u8,
    pub dam: u8,
    pub xof: bool,
}

impl Default for OpPatch {
    fn default() -> Self {
        Self {
            multi: 1,
            tl: 0,
            ar: 15,
            dr: 0,
            sr: 0,
            rr: 7,
            sl: 0,
            ksl: 0,
            ksr: 0,
            wave: 0,
            dt: 0,
            fb: 0,
            am: false,
            vib: false,
            eg_type: true,
            dvb: 0,
            dam: 0,
            xof: false,
        }
    }
}

/// A voice patch: 2-op or 4-op plus one connection algorithm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoicePatch {
    pub four_op: bool,
    pub algorithm: u8,
    pub feedback: u8,
    pub note_shift: i32,
    pub pan_default: f32,
    pub lfo: u8,
    pub ops: [OpPatch; 4],
}

impl Default for VoicePatch {
    fn default() -> Self {
        Self {
            four_op: false,
            algorithm: 0,
            feedback: 0,
            note_shift: 0,
            pan_default: 0.0,
            lfo: 0,
            ops: [OpPatch::default(); 4],
        }
    }
}

impl VoicePatch {
    pub fn default_patch() -> Self {
        let mut v = Self::default();
        v.ops[0] = OpPatch {
            multi: 2,
            tl: 20,
            ar: 15,
            dr: 6,
            sr: 2,
            rr: 7,
            sl: 4,
            ..OpPatch::default()
        };
        v.ops[1] = OpPatch {
            multi: 1,
            tl: 0,
            ar: 15,
            dr: 4,
            sr: 1,
            rr: 7,
            sl: 2,
            ..OpPatch::default()
        };
        v
    }

    /// The built-in ROM-free GM approximation, indexed by program number.
    pub fn gm_approx(program: i32) -> &'static VoicePatch {
        static BANK: OnceLock<[VoicePatch; 128]> = OnceLock::new();
        let bank = BANK.get_or_init(|| {
            let mut bank = [VoicePatch::default(); 128];
            for (i, slot) in bank.iter_mut().enumerate() {
                let mut v = VoicePatch::default_patch();
                match i / 8 {
                    0 => {
                        v.ops[0].multi = 1;
                        v.ops[0].dr = 7;
                    }
                    1 => {
                        v.ops[0].multi = 2;
                        v.ops[0].tl = 14;
                    }
                    2 => {
                        v.ops[0].multi = 1;
                        v.ops[1].sr = 0;
                        v.ops[0].dr = 2;
                    }
                    3 => {
                        v.ops[0].multi = 1;
                        v.ops[0].dr = 5;
                    }
                    4 => {
                        v.ops[0].multi = 2;
                        v.ops[0].dr = 4;
                    }
                    5 => {
                        v.ops[0].multi = 1;
                        v.ops[1].sr = 0;
                        v.ops[0].dr = 1;
                    }
                    6 => {
                        v.ops[0].multi = 1;
                        v.ops[1].sr = 0;
                    }
                    7 => {
                        v.ops[0].multi = 3;
                        v.ops[1].sr = 0;
                    }
                    8 => {
                        v.ops[0].multi = 1;
                        v.ops[1].sr = 0;
                        v.ops[0].tl = 26;
                    }
                    9 => {
                        v.ops[0].multi = 1;
                        v.ops[1].sr = 0;
                        v.ops[0].tl = 28;
                    }
                    10 => {
                        v.ops[0].multi = 4;
                        v.ops[0].wave = 1;
                    }
                    11 => {
                        v.ops[0].multi = 1;
                        v.ops[0].wave = 2;
                        v.ops[1].sr = 0;
                    }
                    12 => {
                        v.ops[0].multi = 6;
                        v.ops[0].wave = 3;
                    }
                    13 => {
                        v.ops[0].multi = 2;
                        v.ops[0].dr = 6;
                    }
                    14 => {
                        v.ops[0].multi = 8;
                        v.ops[0].dr = 10;
                        v.ops[0].wave = 1;
                    }
                    _ => {
                        v.ops[0].multi = 12;
                        v.ops[0].wave = 4;
                        v.ops[0].dr = 12;
                    }
                }
                *slot = v;
            }
            bank
        });
        &bank[program.clamp(0, 127) as usize]
    }

    /// Three short percussive hits for rhythm notes with no bound voice.
    pub fn drum_approx(note: i32) -> &'static VoicePatch {
        static KIT: OnceLock<[VoicePatch; 3]> = OnceLock::new();
        let kit = KIT.get_or_init(|| {
            let mut k = [VoicePatch::default(); 3];

            k[0] = VoicePatch::default_patch();
            k[0].ops[0] = OpPatch {
                multi: 0,
                tl: 8,
                ar: 15,
                dr: 10,
                sr: 0,
                rr: 12,
                sl: 15,
                eg_type: false,
                ..OpPatch::default()
            };
            k[0].ops[1] = OpPatch {
                multi: 0,
                tl: 0,
                ar: 15,
                dr: 9,
                sr: 0,
                rr: 12,
                sl: 15,
                fb: 5,
                eg_type: false,
                ..OpPatch::default()
            };

            k[1] = VoicePatch::default_patch();
            k[1].ops[0] = OpPatch {
                multi: 11,
                tl: 4,
                ar: 15,
                dr: 9,
                sr: 0,
                rr: 11,
                sl: 15,
                fb: 7,
                eg_type: false,
                ..OpPatch::default()
            };
            k[1].ops[1] = OpPatch {
                multi: 1,
                tl: 2,
                ar: 15,
                dr: 8,
                sr: 0,
                rr: 11,
                sl: 15,
                eg_type: false,
                ..OpPatch::default()
            };

            k[2] = VoicePatch::default_patch();
            k[2].ops[0] = OpPatch {
                multi: 15,
                tl: 6,
                ar: 15,
                dr: 12,
                sr: 0,
                rr: 13,
                sl: 15,
                fb: 7,
                eg_type: false,
                ..OpPatch::default()
            };
            k[2].ops[1] = OpPatch {
                multi: 9,
                tl: 8,
                ar: 15,
                dr: 12,
                sr: 0,
                rr: 13,
                sl: 15,
                eg_type: false,
                ..OpPatch::default()
            };
            k
        });
        &kit[if note < 44 {
            0
        } else if note < 52 {
            1
        } else {
            2
        }]
    }
}

fn tl_to_gain(tl: u8) -> f64 {
    if tl >= 63 {
        0.0
    } else {
        10f64.powf((-0.75 * f64::from(tl)) / 20.0)
    }
}

fn rate_to_step(rate: u8, sample_rate: f64, attack: bool) -> f64 {
    if rate == 0 {
        return if attack {
            0.0
        } else {
            1.0 / (sample_rate * 20.0)
        };
    }
    let fast: f64 = if attack { 0.0008 } else { 0.004 };
    let slow: f64 = if attack { 0.35 } else { 4.0 };
    let t = slow * (fast / slow).powf((f64::from(rate) - 1.0) / 14.0);
    let samples = (t * sample_rate).max(1.0);
    1.0 / samples
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Exponential ADSR with the chip's discrete rate steps.
#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    phase: Phase,
    level: f64,
    atk_step: f64,
    dec_step: f64,
    sus_step: f64,
    rel_step: f64,
    sus_level: f64,
    sustaining: bool,
    xof: bool,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            level: 0.0,
            atk_step: 0.0,
            dec_step: 0.0,
            sus_step: 0.0,
            rel_step: 0.0,
            sus_level: 0.0,
            sustaining: true,
            xof: false,
        }
    }
}

impl Envelope {
    pub fn configure(&mut self, p: &OpPatch, sample_rate: f64, rate_scale: f64) {
        self.atk_step = rate_to_step(p.ar, sample_rate, true) * rate_scale;
        self.dec_step = rate_to_step(p.dr, sample_rate, false) * rate_scale;
        self.sus_step = rate_to_step(p.sr, sample_rate, false) * rate_scale;
        self.rel_step = rate_to_step(p.rr, sample_rate, false) * rate_scale;
        self.sus_level = 1.0 - f64::from(p.sl) / 15.0;
        self.sustaining = p.eg_type;
        self.xof = p.xof;
        self.phase = Phase::Idle;
        self.level = 0.0;
    }

    pub fn key_on(&mut self) {
        self.phase = Phase::Attack;
    }

    pub fn key_off(&mut self) {
        if self.xof && !self.sustaining {
            return;
        }
        if self.phase != Phase::Idle {
            self.phase = Phase::Release;
        }
    }

    pub fn advance(&mut self) -> f32 {
        match self.phase {
            Phase::Idle => 0.0,
            Phase::Attack => {
                self.level += if self.atk_step <= 0.0 {
                    1.0
                } else {
                    self.atk_step
                };
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.phase = Phase::Decay;
                }
                self.level as f32
            }
            Phase::Decay => {
                self.level -= self.dec_step;
                if self.level <= self.sus_level {
                    self.level = self.sus_level;
                    self.phase = if self.sustaining {
                        Phase::Sustain
                    } else {
                        Phase::Release
                    };
                }
                self.level as f32
            }
            Phase::Sustain => {
                self.level -= self.sus_step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.phase = Phase::Idle;
                }
                self.level as f32
            }
            Phase::Release => {
                self.level -= self.rel_step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.phase = Phase::Idle;
                }
                self.level as f32
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.phase == Phase::Idle
    }
}

/// Phase generator + waveform + envelope for one operator.
#[derive(Clone, Copy, Debug)]
pub struct Operator {
    patch: OpPatch,
    env: Envelope,
    sample_rate: f64,
    freq_hz: f64,
    phase: f64,
    phase_inc: f64,
    tl_gain: f64,
    ksl_gain: f64,
    vib_amt: f64,
    am_amt: f32,
    ny_mute: bool,
    wave: u8,
}

impl Default for Operator {
    fn default() -> Self {
        Self {
            patch: OpPatch::default(),
            env: Envelope::default(),
            sample_rate: 48_000.0,
            freq_hz: 440.0,
            phase: 0.0,
            phase_inc: 0.0,
            tl_gain: 1.0,
            ksl_gain: 1.0,
            vib_amt: 0.0,
            am_amt: 0.0,
            ny_mute: false,
            wave: 0,
        }
    }
}

impl Operator {
    pub fn configure(&mut self, p: OpPatch, sample_rate: f64, rate_scale: f64) {
        self.patch = p;
        self.sample_rate = sample_rate;
        self.wave = p.wave;
        self.tl_gain = tl_to_gain(p.tl);
        const VIB: [f64; 4] = [0.00196, 0.00387, 0.00774, 0.01548];
        const TREM: [f32; 4] = [0.129, 0.242, 0.424, 0.669];
        self.vib_amt = if p.vib {
            VIB[(p.dvb & 3) as usize]
        } else {
            0.0
        };
        self.am_amt = if p.am {
            TREM[(p.dam & 3) as usize]
        } else {
            0.0
        };
        self.env.configure(&p, sample_rate, rate_scale);
    }

    fn recalc(&mut self) {
        let m = if self.patch.multi == 0 {
            0.5
        } else {
            f64::from(self.patch.multi & 15)
        };
        let detune = 1.0 + (f64::from(self.patch.dt) - 3.5) * 0.0006;
        self.phase_inc = (self.freq_hz * m * detune) / self.sample_rate;
        const KSL_DB: [f64; 4] = [0.0, 1.5, 3.0, 6.0];
        let oct = (self.freq_hz.max(1.0) / 261.63).log2();
        let att = KSL_DB[(self.patch.ksl & 3) as usize] * oct.max(0.0);
        self.ksl_gain = 10f64.powf(-att / 20.0);
        self.ny_mute = self.phase_inc >= 0.5;
    }

    pub fn note_on(&mut self, freq_hz: f64) {
        self.freq_hz = freq_hz;
        self.phase = 0.0;
        self.recalc();
        self.env.key_on();
    }

    pub fn note_off(&mut self) {
        self.env.key_off();
    }

    pub fn set_base_freq(&mut self, freq: f64) {
        self.freq_hz = freq;
        self.recalc();
    }

    pub fn finished(&self) -> bool {
        self.env.is_finished()
    }

    pub fn tick(&mut self, mod_cycles: f64, lfo_sin: f64) -> f32 {
        let env_gain = self.env.advance();
        if self.ny_mute {
            return 0.0;
        }
        let mut inc = self.phase_inc;
        if self.vib_amt != 0.0 {
            inc *= 1.0 + self.vib_amt * lfo_sin;
        }
        self.phase += inc;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        let out = waveform(self.wave, self.phase + mod_cycles);
        let am_gain = if self.am_amt != 0.0 {
            1.0 - self.am_amt * (0.5 + 0.5 * lfo_sin) as f32
        } else {
            1.0
        };
        (out * self.tl_gain * self.ksl_gain * f64::from(env_gain) * f64::from(am_gain)) as f32
    }
}

fn waveform(wave: u8, phase01: f64) -> f64 {
    let x = phase01 - phase01.floor();
    let s = (TWO_PI * x).sin();
    match wave & 7 {
        0 => s,
        1 => {
            if s > 0.0 {
                s
            } else {
                0.0
            }
        }
        2 => s.abs(),
        3 => {
            let q = x - (x * 2.0).floor() * 0.5;
            if q < 0.25 {
                (TWO_PI * q).sin().abs()
            } else {
                0.0
            }
        }
        4 => {
            if x < 0.5 {
                (TWO_PI * 2.0 * x).sin()
            } else {
                0.0
            }
        }
        5 => {
            if x < 0.5 {
                (TWO_PI * 2.0 * x).sin().abs()
            } else {
                0.0
            }
        }
        6 => (s * 3.0).tanh(),
        _ => {
            let v = (TWO_PI * x).sin()
                + 0.5 * (TWO_PI * 2.0 * x).sin()
                + 0.333 * (TWO_PI * 3.0 * x).sin();
            v * 0.6
        }
    }
}

/// A playing voice: up to four operators and the voice LFO.
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    patch: VoicePatch,
    ops: [Operator; 4],
    sample_rate: f64,
    velocity: f32,
    volume: f32,
    active: bool,
    four_op: bool,
    algo: u8,
    feedback: u8,
    op_fb: [u8; 4],
    fb_mem: [[f64; 2]; 4],
    lfo_phase: f64,
    lfo_inc: f64,
    lfo_sin: f64,
    pub channel: i32,
    pub note: i32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            patch: VoicePatch::default(),
            ops: [Operator::default(); 4],
            sample_rate: 48_000.0,
            velocity: 1.0,
            volume: 1.0,
            active: false,
            four_op: false,
            algo: 0,
            feedback: 0,
            op_fb: [0; 4],
            fb_mem: [[0.0; 2]; 4],
            lfo_phase: 0.0,
            lfo_inc: 0.0,
            lfo_sin: 0.0,
            channel: -1,
            note: -1,
        }
    }
}

impl Voice {
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    pub fn note_on(&mut self, patch: &VoicePatch, freq_hz: f64, velocity: f32) {
        self.patch = *patch;
        self.four_op = patch.four_op;
        self.algo = patch.algorithm & 7;
        self.feedback = patch.feedback;
        self.velocity = velocity.clamp(0.0, 1.0);
        for i in 0..4 {
            self.fb_mem[i] = [0.0, 0.0];
            self.op_fb[i] = patch.ops[i].fb;
        }
        if patch.feedback != 0 && self.op_fb[0] == 0 {
            self.op_fb[0] = patch.feedback;
        }
        const LFO_HZ: [f64; 4] = [1.8, 4.0, 6.0, 9.7];
        self.lfo_inc = LFO_HZ[(patch.lfo & 3) as usize] / self.sample_rate;
        self.lfo_phase = 0.0;
        self.lfo_sin = 0.0;

        let semis_up = 12.0 * (freq_hz.max(1.0) / 261.63).log2();
        let nops = if self.four_op { 4 } else { 2 };
        for i in 0..nops {
            let div = if patch.ops[i].ksr != 0 { 24.0 } else { 96.0 };
            let rs = 2f64.powf(semis_up / div).clamp(0.5, 4.0);
            self.ops[i].configure(patch.ops[i], self.sample_rate, rs);
            self.ops[i].note_on(freq_hz);
        }
        self.active = true;
    }

    pub fn note_off(&mut self) {
        let nops = if self.four_op { 4 } else { 2 };
        for op in self.ops.iter_mut().take(nops) {
            op.note_off();
        }
    }

    pub fn set_pitch(&mut self, freq_hz: f64) {
        let nops = if self.four_op { 4 } else { 2 };
        for op in self.ops.iter_mut().take(nops) {
            op.set_base_freq(freq_hz);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    fn mod_op(&mut self, i: usize, ext_mod_cycles: f64) -> f32 {
        let mut fbc = 0.0;
        if self.op_fb[i] > 0 {
            let avg = (self.fb_mem[i][0] + self.fb_mem[i][1]) * 0.5;
            fbc = avg * (f64::from(self.op_fb[i]) / 24.0);
        }
        let o = self.ops[i].tick(ext_mod_cycles + fbc, self.lfo_sin);
        self.fb_mem[i][1] = self.fb_mem[i][0];
        self.fb_mem[i][0] = f64::from(o);
        o
    }

    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.lfo_phase += self.lfo_inc;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        self.lfo_sin = (2.0 * PI * self.lfo_phase).sin();
        let d = MOD_DEPTH;
        let out = match self.algo {
            0 => {
                let m = self.mod_op(0, 0.0);
                f64::from(self.mod_op(1, f64::from(m) * d))
            }
            1 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, 0.0);
                (f64::from(a) + f64::from(b)) * 0.5
            }
            2 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, 0.0);
                let c = self.mod_op(2, 0.0);
                let e = self.mod_op(3, 0.0);
                (f64::from(a) + f64::from(b) + f64::from(c) + f64::from(e)) * 0.35
            }
            3 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, 0.0);
                let c = self.mod_op(2, (f64::from(a) + f64::from(b)) * d);
                f64::from(self.mod_op(3, f64::from(c) * d))
            }
            4 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, f64::from(a) * d);
                let c = self.mod_op(2, f64::from(b) * d);
                f64::from(self.mod_op(3, f64::from(c) * d))
            }
            5 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, f64::from(a) * d);
                let c = self.mod_op(2, 0.0);
                let e = self.mod_op(3, f64::from(c) * d);
                (f64::from(b) + f64::from(e)) * 0.5
            }
            6 => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, 0.0);
                let c = self.mod_op(2, f64::from(b) * d);
                let e = self.mod_op(3, f64::from(c) * d);
                (f64::from(a) + f64::from(e)) * 0.5
            }
            _ => {
                let a = self.mod_op(0, 0.0);
                let b = self.mod_op(1, 0.0);
                let c = self.mod_op(2, f64::from(b) * d);
                let e = self.mod_op(3, 0.0);
                (f64::from(a) + f64::from(c) + f64::from(e)) * 0.4
            }
        };
        let nops = if self.four_op { 4 } else { 2 };
        if self.ops.iter().take(nops).all(|op| op.finished()) {
            self.active = false;
        }
        (out * f64::from(self.velocity) * f64::from(self.volume) * 0.7) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(patch: &VoicePatch, freq: f64, frames: usize) -> (Vec<f32>, f64, f32) {
        let mut voice = Voice::default();
        voice.set_sample_rate(44_100.0);
        voice.note_on(patch, freq, 1.0);
        let mut samples = Vec::with_capacity(frames);
        let mut sum = 0.0;
        let mut peak = 0.0f32;
        for _ in 0..frames {
            let s = voice.tick();
            assert!(s.is_finite(), "the FM core produced a non-finite sample");
            samples.push(s);
            sum += f64::from(s) * f64::from(s);
            peak = peak.max(s.abs());
        }
        (samples, (sum / frames as f64).sqrt(), peak)
    }

    #[test]
    fn the_gm_piano_is_audible_and_finite() {
        let patch = *VoicePatch::gm_approx(0);
        let (_, rms, peak) = render(&patch, 440.0, 4_410);
        assert!(rms > 0.05, "rms={rms}");
        assert!(peak > 0.1, "peak={peak}");
    }

    #[test]
    fn every_connection_algorithm_renders_a_finite_audible_signal() {
        let mut patch = VoicePatch::default_patch();
        patch.four_op = true;
        patch.feedback = 5;
        patch.lfo = 2;
        patch.ops[0] = OpPatch {
            multi: 2,
            tl: 18,
            ar: 15,
            dr: 5,
            sr: 2,
            rr: 8,
            sl: 4,
            vib: true,
            dvb: 2,
            am: true,
            dam: 1,
            fb: 6,
            ..OpPatch::default()
        };
        patch.ops[1] = OpPatch {
            multi: 3,
            tl: 24,
            ar: 13,
            dr: 6,
            sr: 3,
            rr: 7,
            sl: 6,
            ..OpPatch::default()
        };
        patch.ops[2] = OpPatch {
            multi: 1,
            tl: 8,
            ar: 14,
            dr: 4,
            sr: 1,
            rr: 9,
            sl: 2,
            ..OpPatch::default()
        };
        patch.ops[3] = OpPatch {
            multi: 4,
            ar: 15,
            dr: 3,
            rr: 7,
            ..OpPatch::default()
        };
        for algorithm in 0..8 {
            patch.algorithm = algorithm;
            let (_, rms, peak) = render(&patch, 440.0, 4_410);
            assert!(rms > 0.005, "algorithm {algorithm} is silent: rms={rms}");
            assert!(peak > 0.01, "algorithm {algorithm} peak={peak}");
        }
    }

    #[test]
    fn key_off_lets_a_sustaining_voice_retire() {
        let patch = *VoicePatch::gm_approx(0);
        let mut voice = Voice::default();
        voice.set_sample_rate(44_100.0);
        voice.note_on(&patch, 440.0, 1.0);
        for _ in 0..1_000 {
            let _ = voice.tick();
        }
        voice.note_off();
        let mut frames = 0;
        while voice.active() && frames < 441_000 {
            let _ = voice.tick();
            frames += 1;
        }
        assert!(
            !voice.active(),
            "the voice never reached Idle after key-off"
        );
    }
}
