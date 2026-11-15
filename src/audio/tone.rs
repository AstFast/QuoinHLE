//! The tone generator: what `ISound::PlayTone` and `ISound::PlayFreqTone` actually ask a handset to
//! do, and the small part of it this host can answer without inventing anything.
//!
//! The header generator lived in the handset's firmware and every OEM's tone table was its own. So a host that
//! wants a beep has to pick the ids whose sound is not the OEM's to choose, and stay silent on the
//! rest — which is what [`frequencies`] does. Three groups qualify, and each is checkable against a
//! published document rather than against taste:
//!
//! * the DTMF keypad, whose four-by-four grid is ITU-T Q.23's;
//! * the ring-tone range, whose ids are **named after notes** (`AEE_TONE_RING_A4` is A4, and the
//!   next thirty-six ids are the chromatic run up from it), so the frequency is printed on the
//!   name and the table only has to read it;
//! * four call-progress tones (dial, ringback, busy, reorder) at the frequencies the networks this
//!   host would meet actually use.
//!
//! Everything else answers `None`, and `None` is silence rather than a guess. A pack that beeps
//! with `AEE_TONE_ERR` hears nothing here; that is a documented hole, not a silent failure — the
//! request is still accepted and still reports `AEE_SOUND_PLAY_DONE`, because the report is about
//! the request and never about this host's repertoire.
//!
//! The generator itself ([`ToneGen`]) is one sine pair with a five-millisecond fade on both ends.
//! The fade is not polish: `StopTone` and the end of a `PlayToneList` both cut a tone mid-cycle, and
//! a mid-cycle cut is a click on every beep a pack makes.

use crate::abi::generated as g;

/// How long a tone takes to reach full level, and to leave it.
///
/// Short enough that a hundred-millisecond beep still sounds like a beep rather than a swell, long
/// enough that stopping one does not step the waveform. Below about a millisecond the click comes
/// back; above about twenty the beep's own attack starts to dominate its length.
const FADE_SECONDS: f64 = 0.005;

/// What a tone id sounds like, in whole hertz, or `None` for an id whose sound belongs to the OEM.
///
/// The second value is 0 for a single-frequency tone, which is how this writes a "pair" of one.
pub fn frequencies(id: u32) -> Option<(u16, u16)> {
    Some(match id {
        // DTMF, ITU-T Q.23. `AEE_TONE_*`'s values are the digit plus one — `AEE_TONE_0` is 1 and
        // `AEE_TONE_9` is 0x0a — and then a, b, c, d, pound, star, in the order the header lists
        // them. The grid is 697/770/852/941 Hz across and 1209/1336/1477/1633 Hz down.
        0x01 => (941, 1336), // 0
        0x02 => (697, 1209), // 1
        0x03 => (697, 1336), // 2
        0x04 => (697, 1477), // 3
        0x05 => (770, 1209), // 4
        0x06 => (770, 1336), // 5
        0x07 => (770, 1477), // 6
        0x08 => (852, 1209), // 7
        0x09 => (852, 1336), // 8
        0x0a => (852, 1477), // 9
        0x0b => (697, 1633), // a
        0x0c => (770, 1633), // b
        0x0d => (852, 1633), // c
        0x0e => (941, 1633), // d
        0x0f => (941, 1477), // pound
        0x10 => (941, 1209), // star
        // The ring-tone range prints its own answer: `AEE_TONE_RING_A4` is A4 and each id up to
        // `AEE_TONE_RING_A7` is one semitone above the one before. The octave numbers count from A,
        // which is why `RING_C4` follows `RING_B4` — read as scientific pitch it would be a
        // seventh *below* the B before it, and the run would stop ascending. The progression the
        // ids describe is the one that lands 0x3e on 3520 Hz, three octaves above the 0x1a.
        0x1a..=0x3e => {
            let step = f64::from(id - 0x1a);
            ((440.0 * 2f64.powf(step / 12.0)).round() as u16, 0)
        }
        // Call progress in the form the networks this host is likely to meet use. The header names
        // the tones but not their pitches, and these four are the same in every handset that ever
        // shipped on those networks.
        0x3f => (440, 480), // ringback
        0x40 => (480, 620), // busy
        0x43 => (480, 620), // reorder
        0x47 => (350, 440), // dial
        _ => return None,
    })
}

/// The step `ISoundPlayer::SetTune` was given, from the `0x40`-centred value the record keeps.
///
/// `0` in the record is *not* a tune of −64: it is a player that has never been tuned, because
/// [`crate::runtime::Player::default`] leaves the field alone. The centre is `0x40`, which the
/// shipped body writes for a step of zero.
pub fn tune_step(tune: u32) -> i32 {
    if tune == 0 {
        0
    } else {
        tune as i32 - 0x40
    }
}

/// The pitch factor of a tune step: one equal-tempered semitone is a factor of `2^(1/12)`.
pub fn tune_factor(step: i32) -> f64 {
    2f64.powf(f64::from(step) / 12.0)
}

/// A two-sine tone, rendered on the audio thread.
///
/// One of these is the mixer's whole tone generator, because that is the hardware: a handset has a
/// single tone path shared by every `ISound` object, so a second object asking for a tone replaces
/// the first rather than adding to it.
pub(crate) struct ToneGen {
    hi: f64,
    lo: f64,
    /// Phase in cycles, 0.0..1.0, one per oscillator.
    phase_hi: f64,
    phase_lo: f64,
    /// The mixer's sample rate.
    rate: f64,
    /// Current and target gain, 0.0..=1.0. Finished when both are zero.
    gain: f32,
    target: f32,
    /// Gain change per sample, the fade's slope.
    ramp: f32,
}

impl ToneGen {
    pub(crate) fn new(hi: u16, lo: u16, rate: f64, volume: u32) -> Self {
        let mut tone = Self {
            hi: f64::from(hi),
            lo: f64::from(lo),
            phase_hi: 0.0,
            phase_lo: 0.0,
            rate,
            gain: 0.0,
            target: 0.0,
            ramp: (1.0 / (FADE_SECONDS * rate)) as f32,
        };
        tone.set_volume(volume);
        tone
    }

    /// Whether this generator already sounds `(hi, lo)`.
    ///
    /// A repeated request for the same tone keeps its phase instead of restarting it, which is what
    /// makes a `TICK_UPDATE`-driven pack's beep continuous rather than a buzz at whatever rate it
    /// asks. A different pair is a different tone and starts from the top.
    pub(crate) fn is(&self, hi: u16, lo: u16) -> bool {
        self.hi == f64::from(hi) && self.lo == f64::from(lo)
    }

    /// The pack's `SetVolume`, on the 0..[`g::AEE_MAX_VOLUME`] scale.
    ///
    /// A level also takes back a release: a `StopTone` followed by the same tone inside one turn is
    /// one beep, and the second request must not be read as "keep fading".
    pub(crate) fn set_volume(&mut self, volume: u32) {
        self.target = volume.min(g::AEE_MAX_VOLUME) as f32 / 100.0;
    }

    /// Fade out. The caller drops this generator once [`Self::render`] says it has.
    pub(crate) fn release(&mut self) {
        self.target = 0.0;
    }

    /// Add this tone to the caller's block. `true` once it has faded to nothing.
    pub(crate) fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> bool {
        debug_assert_eq!(left.len(), right.len(), "one frame count both ways");
        let two_pi = std::f64::consts::TAU;
        let (step_hi, step_lo) = (self.hi / self.rate, self.lo / self.rate);
        for i in 0..left.len() {
            let sample = if self.hi > 0.0 {
                let a = (self.phase_hi * two_pi).sin();
                let wave = if self.lo > 0.0 {
                    // Two sines at half level each, so a pair and a single tone peak at the same
                    // place and a pack's `SetVolume` means the same thing on both.
                    0.5 * (a + (self.phase_lo * two_pi).sin())
                } else {
                    a
                };
                (wave * f64::from(self.gain)) as f32
            } else {
                0.0
            };
            left[i] += sample;
            right[i] += sample;
            self.phase_hi += step_hi;
            if self.phase_hi >= 1.0 {
                self.phase_hi -= 1.0;
            }
            if self.lo > 0.0 {
                self.phase_lo += step_lo;
                if self.phase_lo >= 1.0 {
                    self.phase_lo -= 1.0;
                }
            }
            if self.gain < self.target {
                self.gain = (self.gain + self.ramp).min(self.target);
            } else if self.gain > self.target {
                self.gain = (self.gain - self.ramp).max(self.target);
            }
        }
        self.target == 0.0 && self.gain <= 0.0
    }
}

#[cfg(test)]
#[path = "../test/audio_tone.rs"]
mod tests;
