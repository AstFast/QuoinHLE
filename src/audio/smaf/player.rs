//! The MA player: channel state, FM/PCM voice pools, mixing and the output stage.
//!
//! Ported from the yamaha-smaf-player C++ engine's `ma_player` (Apache-2.0, Andreas Wendorf). The
//! timeline comes from [`super::sequencer::Score`]; this file only decides which voice a note
//! binds to, how it is mixed, and when the song ends.

use std::f64::consts::PI;
use std::sync::Arc;

use ::smaf::{BaseBit, StreamWaveFormat};

use super::adpcm;
use super::fm::{Envelope, Voice, VoicePatch};
use super::sequencer::{Event, Kind, Score, Wave};
use super::voice::ParsedVoice;

const FM_POOL: usize = 32;
const PCM_POOL: usize = 16;
const MIX_GAIN: f64 = 0.32;

struct PcmSample {
    pcm: Vec<i16>,
    fs: u32,
}

#[derive(Clone, Copy, Debug)]
struct Chan {
    bank_msb: i32,
    bank_lsb: i32,
    program: i32,
    drum: bool,
    rhythm: bool,
    volume: f32,
    expression: f32,
    pan: f32,
    bend: f64,
    oct_shift: i32,
}

impl Default for Chan {
    fn default() -> Self {
        Self {
            bank_msb: 0,
            bank_lsb: 0,
            program: 0,
            drum: false,
            rhythm: false,
            volume: 1.0,
            expression: 1.0,
            pan: 0.0,
            bend: 0.0,
            oct_shift: 0,
        }
    }
}

#[derive(Clone, Default)]
struct PcmVoice {
    sample: Option<Arc<PcmSample>>,
    pos: f64,
    rate: f64,
    loop_start: usize,
    loop_end: usize,
    loop_: bool,
    env: Envelope,
    gain: f32,
    vel: f32,
    pan: f32,
    channel: i32,
    key_note: i32,
    active: bool,
}

impl PcmVoice {
    fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let Some(sample) = self.sample.clone() else {
            self.active = false;
            return 0.0;
        };
        let len = sample.pcm.len();
        if len == 0 {
            self.active = false;
            return 0.0;
        }
        let end = if self.loop_end > 0 && self.loop_end <= len {
            self.loop_end
        } else {
            len
        };
        if self.pos >= end as f64 {
            if self.loop_ && self.loop_end > self.loop_start {
                self.pos = self.loop_start as f64 + (self.pos - end as f64);
            } else {
                self.active = false;
                return 0.0;
            }
        }
        let i = self.pos as usize;
        if i >= len {
            self.active = false;
            return 0.0;
        }
        let frac = (self.pos - i as f64) as f32;
        let s0 = f32::from(sample.pcm[i]) / 32768.0;
        let s1 = if i + 1 < len {
            f32::from(sample.pcm[i + 1]) / 32768.0
        } else {
            s0
        };
        let s = s0 + (s1 - s0) * frac;
        self.pos += self.rate;
        let e = self.env.advance();
        if self.env.is_finished() {
            self.active = false;
        }
        s * e * self.gain * self.vel
    }
}

pub struct Player {
    rate: u32,
    rate_f: f64,
    score: Score,
    bank: Vec<Option<Arc<PcmSample>>>,
    chans: [Chan; 128],
    fm: [Voice; FM_POOL],
    fm_midi: [i32; FM_POOL],
    fm_pan: [f32; FM_POOL],
    fm_key: [i32; FM_POOL],
    pcm: Vec<PcmVoice>,
    fm_rr: usize,
    pcm_rr: usize,
    next_event: usize,
    cursor: u64,
    end_sample: u64,
    lp_coef: f64,
    lp1_l: f64,
    lp1_r: f64,
    lp2_l: f64,
    lp2_r: f64,
    lim_env: f64,
    lim_release: f64,
}

impl Player {
    pub fn new(score: Score, rate: u32) -> Self {
        let rate = if rate == 0 { 44_100 } else { rate };
        let rate_f = f64::from(rate);
        let mut chans = [Chan::default(); 128];
        for (i, rhythm) in score.rhythm.iter().enumerate() {
            chans[i].rhythm = *rhythm;
        }
        let end_sample = u64::from(score.length_ms) * u64::from(rate) / 1000;
        let bank = build_wave_bank(&score);
        let mut pcm = Vec::with_capacity(PCM_POOL);
        for _ in 0..PCM_POOL {
            pcm.push(PcmVoice::default());
        }
        Self {
            rate,
            rate_f,
            score,
            bank,
            chans,
            fm: [Voice::default(); FM_POOL],
            fm_midi: [0; FM_POOL],
            fm_pan: [0.0; FM_POOL],
            fm_key: [-1; FM_POOL],
            pcm,
            fm_rr: 0,
            pcm_rr: 0,
            next_event: 0,
            cursor: 0,
            end_sample,
            lp_coef: 1.0 - (-2.0 * PI * 10_000.0 / rate_f).exp(),
            lp1_l: 0.0,
            lp1_r: 0.0,
            lp2_l: 0.0,
            lp2_r: 0.0,
            lim_env: 0.0,
            lim_release: (-1.0 / (0.15 * rate_f)).exp(),
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn total_samples(&self) -> u64 {
        self.end_sample
    }

    pub fn position_ms(&self) -> u32 {
        (self.cursor.saturating_mul(1000) / u64::from(self.rate)) as u32
    }

    /// Rebuild the runtime state, keeping the parsed score and wave bank.
    fn reset_runtime(&mut self) {
        self.chans = [Chan::default(); 128];
        for (i, rhythm) in self.score.rhythm.iter().enumerate() {
            self.chans[i].rhythm = *rhythm;
        }
        self.fm = [Voice::default(); FM_POOL];
        self.fm_midi = [0; FM_POOL];
        self.fm_pan = [0.0; FM_POOL];
        self.fm_key = [-1; FM_POOL];
        for voice in &mut self.pcm {
            *voice = PcmVoice::default();
        }
        self.fm_rr = 0;
        self.pcm_rr = 0;
        self.next_event = 0;
        self.cursor = 0;
        self.lp1_l = 0.0;
        self.lp1_r = 0.0;
        self.lp2_l = 0.0;
        self.lp2_r = 0.0;
        self.lim_env = 0.0;
    }

    /// Move to `ms` by replaying the score's state up to that point. The skipped samples are
    /// rendered and discarded, so a sustaining note has the envelope it would have had in a
    /// continuous render — that is what makes the seek sample-accurate rather than a jump to a
    /// silent voice pool.
    pub fn seek_ms(&mut self, ms: u32) {
        let target = u64::from(ms)
            .saturating_mul(u64::from(self.rate))
            .saturating_div(1000)
            .min(self.end_sample);
        self.reset_runtime();
        let mut scratch = vec![0.0f32; 2 * 256];
        while self.cursor < target {
            let want = ((target - self.cursor) as usize).min(256);
            if self.render(&mut scratch[..want * 2]) == 0 {
                break;
            }
        }
    }

    pub fn finished(&self) -> bool {
        self.next_event >= self.score.events.len()
            && self.cursor >= self.end_sample
            && self.active_voices() == 0
    }

    fn active_voices(&self) -> usize {
        self.fm.iter().filter(|v| v.active()).count() + self.pcm.iter().filter(|v| v.active).count()
    }

    fn event_sample(&self, event: &Event) -> u64 {
        u64::from(event.time_ms) * u64::from(self.rate) / 1000
    }

    fn note_to_freq(midi: i32) -> f64 {
        440.0 * 2f64.powf((f64::from(midi) - 69.0) / 12.0)
    }

    fn resolve_voice(&self, channel: usize, raw_note: i32) -> Option<ParsedVoice> {
        let c = self.chans[channel];
        let mut melody = None;
        for voice in &self.score.voices {
            if voice.key.bank_msb == c.bank_msb
                && voice.key.bank_lsb == c.bank_lsb
                && voice.key.pc == c.program
            {
                if voice.key.drum_note != 0 {
                    if voice.key.drum_note == raw_note {
                        return Some(*voice);
                    }
                } else if melody.is_none() {
                    melody = Some(*voice);
                }
            }
        }
        melody
    }

    fn fire(&mut self, event: Event) {
        let channel = (event.channel & 0x7f) as usize;
        match event.kind {
            Kind::Program => self.chans[channel].program = event.a,
            Kind::BankMsb => self.chans[channel].bank_msb = event.a,
            Kind::BankLsb => {
                self.chans[channel].bank_lsb = event.a & 0x7f;
                self.chans[channel].drum = event.a & 0x80 != 0;
            }
            Kind::Volume => {
                self.chans[channel].volume = (event.a & 0x7f) as f32 / 127.0;
                let c = self.chans[channel];
                for voice in self.fm.iter_mut() {
                    if voice.active() && voice.channel == channel as i32 {
                        voice.set_volume(c.volume * c.expression);
                    }
                }
            }
            Kind::Expression => {
                self.chans[channel].expression = (event.a & 0x7f) as f32 / 127.0;
                let c = self.chans[channel];
                for voice in self.fm.iter_mut() {
                    if voice.active() && voice.channel == channel as i32 {
                        voice.set_volume(c.volume * c.expression);
                    }
                }
            }
            Kind::Pan => self.chans[channel].pan = ((event.a & 0x7f) - 64) as f32 / 64.0,
            Kind::OctaveShift => self.chans[channel].oct_shift = event.a,
            Kind::PitchBend => {
                self.chans[channel].bend = f64::from(event.a) / 8192.0 * 2.0;
                let bend = self.chans[channel].bend;
                for i in 0..FM_POOL {
                    if !self.fm[i].active() || self.fm[i].channel != channel as i32 {
                        continue;
                    }
                    let freq = Self::note_to_freq(self.fm_midi[i]) * 2f64.powf(bend / 12.0);
                    self.fm[i].set_pitch(freq);
                }
            }
            Kind::Modulation => {}
            Kind::NoteOff => {
                for i in 0..FM_POOL {
                    if self.fm[i].active()
                        && self.fm[i].channel == channel as i32
                        && self.fm_key[i] == event.a
                    {
                        self.fm[i].note_off();
                        break;
                    }
                }
                for voice in self.pcm.iter_mut() {
                    if voice.active && voice.channel == channel as i32 && voice.key_note == event.a
                    {
                        voice.env.key_off();
                    }
                }
            }
            Kind::NoteOn => {
                let c = self.chans[channel];
                let data_voice = self.resolve_voice(channel, event.a);
                let sounding = event.a.saturating_add(c.oct_shift);
                if let Some(voice) = data_voice {
                    if voice.is_pcm {
                        if voice.valid {
                            self.start_pcm(channel, event.a, sounding, voice, c);
                        }
                        return;
                    }
                }
                let patch = data_voice.map_or_else(
                    || {
                        if c.rhythm || c.drum {
                            *VoicePatch::drum_approx(event.a)
                        } else {
                            *VoicePatch::gm_approx(c.program)
                        }
                    },
                    |voice| voice.patch,
                );
                let midi = sounding.saturating_add(patch.note_shift);
                let freq = Self::note_to_freq(midi) * 2f64.powf(c.bend / 12.0);
                let slot = self.fm.iter().position(|v| !v.active()).unwrap_or_else(|| {
                    let slot = self.fm_rr % FM_POOL;
                    self.fm_rr = self.fm_rr.wrapping_add(1);
                    slot
                });
                self.fm[slot].set_sample_rate(self.rate_f);
                self.fm[slot].channel = channel as i32;
                self.fm[slot].note = midi;
                let velocity = if event.b != 0 { event.b } else { 100 } as f32 / 127.0;
                self.fm[slot].note_on(&patch, freq, velocity * velocity);
                self.fm[slot].set_volume(c.volume * c.expression);
                self.fm_midi[slot] = midi;
                self.fm_pan[slot] = if c.pan != 0.0 {
                    c.pan
                } else {
                    patch.pan_default
                };
                self.fm_key[slot] = event.a;
            }
        }
    }

    fn start_pcm(
        &mut self,
        channel: usize,
        raw_note: i32,
        sounding: i32,
        voice: ParsedVoice,
        c: Chan,
    ) {
        let mut sample = None;
        let wave_id = voice.pcm.wave_id;
        if wave_id >= 0 {
            if let Some(slot) = self.bank.get(wave_id as usize) {
                sample = slot.clone();
            }
        }
        if sample.is_none() {
            // A mis-indexed WaveID: prefer the shortest non-empty wave, so a drum hit is
            // chosen over a song-length melodic sample.
            for slot in &self.bank {
                if let Some(candidate) = slot {
                    if sample.as_ref().map_or(true, |current: &Arc<PcmSample>| {
                        candidate.pcm.len() < current.pcm.len()
                    }) {
                        sample = Some(candidate.clone());
                    }
                }
            }
        }
        let Some(sample) = sample else { return };
        let drum = voice.key.drum_note != 0;
        let base = f64::from(sample.fs) / self.rate_f;
        let semis = if drum { 0.0 } else { f64::from(sounding - 60) };
        let rate = base * 2f64.powf(semis / 12.0);
        let len = sample.pcm.len();
        let loop_end = if voice.pcm.end_pt > 0 && (voice.pcm.end_pt as usize) <= len {
            voice.pcm.end_pt as usize
        } else {
            len
        };
        let loop_start = if voice.pcm.loop_pt >= 0 && (voice.pcm.loop_pt as usize) < loop_end {
            voice.pcm.loop_pt as usize
        } else {
            0
        };
        let slot = self.pcm.iter().position(|v| !v.active).unwrap_or_else(|| {
            let slot = self.pcm_rr % PCM_POOL;
            self.pcm_rr = self.pcm_rr.wrapping_add(1);
            slot
        });
        let target = &mut self.pcm[slot];
        target.sample = Some(sample);
        target.pos = 0.0;
        target.rate = rate;
        target.loop_start = loop_start;
        target.loop_end = loop_end;
        target.loop_ = voice.pcm.loop_ && !drum;
        target.env.configure(&voice.pcm.env, self.rate_f, 1.0);
        target.env.key_on();
        target.gain = 10f64.powf(-0.75 * f64::from(voice.pcm.env.tl) / 20.0) as f32;
        target.vel = c.volume * c.expression;
        target.pan = c.pan;
        target.channel = channel as i32;
        target.key_note = raw_note;
        target.active = true;
    }

    /// Render interleaved stereo. Returns the number of frames written; a call after the song
    /// and its releases have settled returns 0.
    pub fn render(&mut self, out: &mut [f32]) -> usize {
        let frames = out.len() / 2;
        let mut produced = 0usize;
        while produced < frames {
            while self.next_event < self.score.events.len()
                && self.event_sample(&self.score.events[self.next_event]) <= self.cursor
            {
                let event = self.score.events[self.next_event];
                self.next_event += 1;
                self.fire(event);
            }
            if self.next_event >= self.score.events.len()
                && self.cursor >= self.end_sample
                && self.active_voices() == 0
            {
                break;
            }

            let mut left = 0.0f64;
            let mut right = 0.0f64;
            for i in 0..FM_POOL {
                if !self.fm[i].active() {
                    continue;
                }
                let pan = self.fm_pan[i];
                let sample = f64::from(self.fm[i].tick());
                left += sample * MIX_GAIN * f64::from(1.0 - pan);
                right += sample * MIX_GAIN * f64::from(1.0 + pan);
            }
            for voice in self.pcm.iter_mut() {
                if !voice.active {
                    continue;
                }
                let pan = voice.pan;
                let sample = f64::from(voice.tick());
                left += sample * MIX_GAIN * f64::from(1.0 - pan);
                right += sample * MIX_GAIN * f64::from(1.0 + pan);
            }

            self.lp1_l += self.lp_coef * (left - self.lp1_l);
            self.lp2_l += self.lp_coef * (self.lp1_l - self.lp2_l);
            self.lp1_r += self.lp_coef * (right - self.lp1_r);
            self.lp2_r += self.lp_coef * (self.lp1_r - self.lp2_r);
            let mut l = self.lp2_l;
            let mut r = self.lp2_r;
            let peak = l.abs().max(r.abs());
            self.lim_env = peak.max(self.lim_env * self.lim_release);
            if self.lim_env > 0.92 {
                let gain = 0.92 / self.lim_env;
                l *= gain;
                r *= gain;
            }
            out[produced * 2] = l as f32;
            out[produced * 2 + 1] = r as f32;
            produced += 1;
            self.cursor += 1;
            if self.cursor > self.end_sample.saturating_add(u64::from(self.rate)) {
                break;
            }
        }
        produced
    }
}

fn build_wave_bank(score: &Score) -> Vec<Option<Arc<PcmSample>>> {
    let max = score
        .waves
        .iter()
        .map(|w| usize::from(w.number))
        .max()
        .unwrap_or(0);
    let mut bank = vec![None; max + 1];
    for wave in &score.waves {
        let pcm = decode_wave(wave);
        if pcm.is_empty() {
            continue;
        }
        let fs = if wave.sampling_rate == 0 {
            8000
        } else {
            u32::from(wave.sampling_rate)
        };
        bank[usize::from(wave.number)] = Some(Arc::new(PcmSample { pcm, fs }));
    }
    bank
}

fn decode_wave(wave: &Wave) -> Vec<i16> {
    match (wave.format, wave.base_bit) {
        (StreamWaveFormat::YamahaADPCM, BaseBit::Bit4) => adpcm::decode_all(&wave.data, false),
        (StreamWaveFormat::TwosComplementPCM, BaseBit::Bit16) => {
            let mut out = Vec::with_capacity(wave.data.len() / 2);
            for pair in wave.data.chunks_exact(2) {
                out.push(i16::from_be_bytes([pair[0], pair[1]]));
            }
            out
        }
        (StreamWaveFormat::OffsetBinaryPCM, BaseBit::Bit8) => wave
            .data
            .iter()
            .map(|byte| (i16::from(*byte) - 128) << 8)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_score() -> Score {
        Score {
            events: Vec::new(),
            voices: Vec::new(),
            waves: Vec::new(),
            rhythm: [false; 128],
            length_ms: 0,
        }
    }

    #[test]
    fn an_empty_score_renders_silence() {
        let mut player = Player::new(empty_score(), 44_100);
        let mut out = vec![0.0f32; 256];
        assert_eq!(player.render(&mut out), 0);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn seek_replays_the_state_a_continuous_render_would_have() {
        let Ok(path) = std::env::var("QUOIN_SMAF_SAMPLE") else {
            eprintln!("skipped: QUOIN_SMAF_SAMPLE is not set");
            return;
        };
        let bytes = std::fs::read(&path).expect("read the sample named by QUOIN_SMAF_SAMPLE");
        let score = Score::parse(&bytes).expect("the sample parses");

        let mut direct = Player::new(score.clone(), 44_100);
        let mut sink = vec![0.0f32; 44_100 * 2 * 5];
        assert!(direct.render(&mut sink) > 0);
        let mut expected = vec![0.0f32; 44_100 * 2];
        direct.render(&mut expected);

        let mut seeked = Player::new(score, 44_100);
        seeked.seek_ms(5_000);
        let mut actual = vec![0.0f32; 44_100 * 2];
        seeked.render(&mut actual);

        for (index, (a, b)) in expected.iter().zip(actual.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-5,
                "seek diverged at sample {index}: continuous {a}, seeked {b}"
            );
        }
    }

    #[test]
    fn a_named_sample_renders_audible_frames() {
        let Ok(path) = std::env::var("QUOIN_SMAF_SAMPLE") else {
            eprintln!("skipped: QUOIN_SMAF_SAMPLE is not set");
            return;
        };
        let bytes = std::fs::read(&path).expect("read the sample named by QUOIN_SMAF_SAMPLE");
        let score = Score::parse(&bytes).expect("the sample parses");
        let mut player = Player::new(score, 44_100);
        // Five seconds: a score can open with sampled voices that are silent until their wave
        // bank is bound, so a two-second window can be legitimately empty.
        let mut out = vec![0.0f32; 44_100 * 2 * 5];
        let frames = player.render(&mut out);
        assert!(frames > 0);
        let peak = out.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.001, "the render is silent: peak={peak}");
    }
}
