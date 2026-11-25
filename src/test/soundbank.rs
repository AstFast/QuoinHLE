//! Tests for the built-in tone bank. None of them reads a sample set file or opens a device: the bank is
//! bytes this crate makes, and the render goes through `audio::render_clip`, which is the same
//! synthesizer the callback drives with the sound card taken out of the picture.

use crate::audio::render_clip;
use crate::audio::soundbank::{self, Class, CLASSES, CYCLE, TABLE};

const RATE: usize = 44_100;

/// Middle C, in the equal-tempered pitch that names it: `A4 = 440`, twelve to the octave.
const MIDDLE_C: f64 = 261.625_6;

/// One format-0 Standard MIDI File: a program change, one note, its release, end of track.
///
/// Written out here rather than found, for the reason `one_note_midi` gives: a test whose fixture can
/// be wrong is a test whose failure means two things. The tempo is the engine's own default of 120
/// BPM against a 480-tick quarter, so a second is 960 ticks.
fn one_note(channel: u8, program: u8, key: u8, held: f32) -> Vec<u8> {
    let mut track = Vec::new();
    track.extend_from_slice(&[0x00, 0xc0 | channel, program]);
    track.extend_from_slice(&[0x00, 0x90 | channel, key, 100]);
    variable_length((held * 960.0).round() as u32, &mut track);
    track.extend_from_slice(&[0x80 | channel, key, 0x00]);
    track.extend_from_slice(&[0x00, 0xff, 0x2f, 0x00]);
    let mut file = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&1u16.to_be_bytes());
    file.extend_from_slice(&480u16.to_be_bytes());
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&(track.len() as u32).to_be_bytes());
    file.extend_from_slice(&track);
    file
}

/// A MIDI delta time: seven bits per byte, high bit set on every byte but the last.
fn variable_length(mut value: u32, out: &mut Vec<u8>) {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.insert(0, ((value & 0x7f) as u8) | 0x80);
        value >>= 7;
    }
    out.extend_from_slice(&bytes);
}

/// Render one note for `held` seconds and then `tail` seconds more.
///
/// The tail is a parameter because it is not free: every second asked for is a second of synthesis,
/// and a sweep over 128 programs that each renders 2.3 s rather than 0.3 adds about fifteen seconds of
/// busy cores to the suite — which was measured to be enough to starve `net`'s two-second
/// reply-waiting loop in the same binary. Ask for the tail only where the release is what is being
/// looked at.
fn play(channel: u8, program: u8, key: u8, held: f32, tail: f32) -> Vec<f32> {
    let frames = ((held + tail) * RATE as f32) as usize;
    let (left, _) = render_clip(
        None,
        &one_note(channel, program, key, held),
        RATE as i32,
        frames,
    )
    .expect("the built-in bank to load and the hand-written clip to parse");
    left
}

/// A note with two seconds after its release, for the tests that look at the envelope's end.
fn render(channel: u8, program: u8, key: u8, held: f32) -> Vec<f32> {
    play(channel, program, key, held, 2.0)
}

fn window(samples: &[f32], from: f32, to: f32) -> &[f32] {
    let a = (from * RATE as f32) as usize;
    let b = ((to * RATE as f32) as usize).min(samples.len());
    &samples[a..b]
}

/// The loudest excursion in a block.
fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0_f32, |m, v| if v.abs() > m { v.abs() } else { m })
}

/// How loud a block is, phase ignored.
fn level(samples: &[f32]) -> f32 {
    (samples.iter().map(|v| v * v).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

/// A block with its mean taken off, as f64 so a long window does not drift.
fn detrend(samples: &[f32]) -> Vec<f64> {
    let mean = samples.iter().map(|v| *v as f64).sum::<f64>() / samples.len() as f64;
    samples.iter().map(|v| *v as f64 - mean).collect()
}

/// How strongly a block repeats itself shifted by `lag` samples: 1.0 for an exact repeat, 0.0 for
/// silence or for a block that carries no relation to its shifted self.
///
/// Both halves are mean-subtracted before the products are taken, because a block that has decayed to
/// a constant would otherwise score a perfect match at *every* lag — and a meter that always answers
/// "yes" is the failure this repository has been burned by before, not a corner case.
fn repeats(samples: &[f32], lag: usize) -> f32 {
    if lag == 0 || lag >= samples.len() {
        return 0.0;
    }
    let a = detrend(&samples[..samples.len() - lag]);
    let b = detrend(&samples[lag..]);
    let (mut ab, mut aa, mut bb) = (0.0_f64, 0.0_f64, 0.0_f64);
    for i in 0..a.len() {
        ab += a[i] * b[i];
        aa += a[i] * a[i];
        bb += b[i] * b[i];
    }
    if aa == 0.0 || bb == 0.0 {
        return 0.0;
    }
    (ab / (aa * bb).sqrt()) as f32
}

/// The period `samples` repeats at, nearest to `expected_hz`, as (hertz, how sure the match was).
///
/// Only the neighbourhood of the expected period is searched, which is what lets this say "not at that
/// pitch" rather than "at some pitch": a block an octave away repeats perfectly at half the expected
/// period and not at the expected one, so it comes back with a low certainty instead of a pass.
///
/// Know what it reports when it is wrong. A block whose true period has left the window still answers
/// with an in-window lag — the correlation of a near-miss stays high (a 172-sample lag on a wave whose
/// true period is 173.14 still scores 0.9987), so the *certainty* is not the part with teeth. The test
/// below asserts the reported hertz against the expected one for that reason, and the wrong-sign
/// mutation reads "256.40 Hz" off a true 254.70 Hz precisely because 173 was outside the window.
///
/// `rate` is a parameter because the device decides it: `Speaker::new` asks the sound card what it
/// runs at and builds its synthesizers at *that*, so the tables' own declared rate is the only thing
/// keeping a 48 kHz output at the same pitch. That is measured below rather than assumed.
fn measured_hz(samples: &[f32], rate: usize, expected_hz: f64) -> (f64, f32) {
    let nominal = (rate as f64 / expected_hz).round() as i64;
    let mut best = (0_i64, -2.0_f32);
    for lag in (nominal - 3).max(1)..=nominal + 3 {
        let score = repeats(samples, lag as usize);
        if score > best.1 {
            best = (lag, score);
        }
    }
    (rate as f64 / best.0 as f64, best.1)
}

/// A reference block at a known pitch, made here so that the meter has something it cannot agree with.
fn synthetic(hz: f64, seconds: f32, harmonics: u32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / RATE as f64;
            (1..=harmonics)
                .map(|k| ((k as f64 * t * hz * std::f64::consts::TAU).sin()) / k as f64)
                .sum::<f64>() as f32
        })
        .collect()
}

#[test]
fn the_bank_loads_and_no_instrument_is_left_without_a_voice() {
    // What this proves beyond "`SoundFont::new` said Ok": that every instrument kept at least one
    // region. A zone whose last generator is not `sampleID` is read as a global prefix instead of as
    // a voice, the instrument ends up with no regions, `sanity_check` has nothing to look at, and the
    // bank loads perfectly and plays nothing. `Ok` alone cannot tell those two apart.
    let font = rustysynth::SoundFont::new(&mut std::io::Cursor::new(soundbank::image()))
        .expect("the built-in bank to parse");
    assert_eq!(font.get_instruments().len(), CLASSES.len());
    assert_eq!(font.get_presets().len(), 129);
    for instrument in font.get_instruments() {
        assert!(
            !instrument.get_regions().is_empty(),
            "instrument {:?} has no regions, so every note it is asked for is silent",
            instrument.get_name()
        );
    }
    for preset in font.get_presets() {
        assert!(
            !preset.get_regions().is_empty(),
            "preset {:?} has no regions",
            preset.get_name()
        );
    }
}

#[test]
fn the_pitch_meter_reads_a_tone_it_was_handed_and_refuses_one_it_was_not() {
    // The control, first: if this cannot tell 440 from 523 then every pitch claim below is noise.
    // A plain sine and a 32-harmonic saw at the same pitch both have to read 440 — the saw is the
    // one that could fool a zero-crossing counter, which is why this meter correlates instead.
    for harmonics in [1_u32, 32] {
        let block = synthetic(440.0, 0.2, harmonics);
        let (hz, sure) = measured_hz(&block, RATE, 440.0);
        assert!(
            sure > 0.99 && (hz - 440.0).abs() < 2.0,
            "the meter read {hz} Hz at certainty {sure} from a {harmonics}-partial 440 Hz reference"
        );
        let wrong = measured_hz(&block, RATE, 523.25);
        assert!(
            wrong.1 < 0.9,
            "the meter called a 440 Hz block an accurate 523 Hz one (certainty {})",
            wrong.1
        );
    }
    // And the floor of the instrument: a block of nothing matches nothing.
    assert_eq!(repeats(&vec![0.0_f32; 4000], 169), 0.0);
}

#[test]
fn a_note_plays_at_the_pitch_its_key_names() {
    // The claim this whole file exists to be able to make: the correction that carries the table's
    // 172.27 Hz fundamental to the key it is declared under has the right *sign*. Read the other way
    // it is a bank a quarter tone flat, which loads, parses and passes every structural test.
    for (key, hz) in [(60, MIDDLE_C), (72, MIDDLE_C * 2.0), (53, 174.61)] {
        let clip = play(0, 0, key, 0.5, 0.0);
        let block = window(&clip, 0.06, 0.26);
        let (measured, sure) = measured_hz(block, RATE, hz);
        assert!(
            sure > 0.9,
            "key {key} did not repeat near {hz} Hz at all (best match {sure})"
        );
        assert!(
            (measured - hz).abs() / hz < 0.01,
            "key {key} measures {measured} Hz against the {hz} Hz it names"
        );
    }
    // The *device* decides the rate a window renders at — `Speaker::new` asks the sound card and
    // builds its synthesizers at whatever it answers — so a run on a 48 kHz output is the same bank
    // played through another rate. The tables carry their own declared rate, which is what makes
    // that come out in tune rather than a semitone sharp, and nothing above can show it: those all
    // render at the tables' own 44.1 kHz. A pure tone is used here because a 32-partial saw at
    // 22.05 kHz is genuinely aliasing, and that is a different question than whose pitch this asks.
    for rate in [22_050_usize, 48_000] {
        let frames = (0.5 * rate as f32) as usize;
        let (left, _) = render_clip(None, &one_note(0, 8, 60, 0.5), rate as i32, frames)
            .expect("a render at a device's own rate");
        let block = &left[(0.06 * rate as f32) as usize..(0.26 * rate as f32) as usize];
        let (measured, sure) = measured_hz(block, rate, MIDDLE_C);
        assert!(
            sure > 0.9 && (measured - MIDDLE_C).abs() / MIDDLE_C < 0.01,
            "middle C through a {rate} Hz device measures {measured} Hz (certainty {sure})"
        );
    }
}

#[test]
fn every_general_midi_program_makes_a_sound() {
    // One program per voice is not enough: the mapping from 128 names to eight tables is a table in
    // this file, and a program that points at a preset that points at nothing is silent in a way
    // that only shows up on that one song. Measured spread across the 128: 0.024 at the quietest
    // (program 112, the noise voice) to 0.142 at the loudest, so this catches a missing voice and
    // not a soft one — which is the right shape for the claim "there is something to hear".
    for program in 0..=127u8 {
        let clip = play(0, program, 60, 0.3, 0.0);
        let loud = peak(window(&clip, 0.02, 0.25));
        assert!(
            loud > 0.01,
            "program {program} renders nothing louder than {loud}"
        );
    }
    // Channel 10 (index 9) is the one the engine pins to bank 128 without being told.
    let drums = play(9, 0, 36, 0.2, 0.0);
    assert!(peak(window(&drums, 0.0, 0.15)) > 0.01, "no drums");
}
#[test]
fn a_sustaining_voice_holds_and_a_struck_voice_falls_away() {
    // The two readings of `sustainVolEnv` this engine actually implements: a floor, in 0.1 dB of
    // attenuation, that the decay falls to and holds at. Organ is the case where that floor is 0 dB,
    // i.e. nowhere to fall to; program 0 is the case where the floor is pushed under the engine's own
    // audibility cut, so the decay is what ends the note. Put the spec's "default" 0 in the struck
    // voice's slot — which is what an author following the document rather than this engine would
    // write — and the second assertion below fails, because a decay that can only fall to full scale
    // never falls at all.
    //
    // Both keys stay down for two seconds, so everything here happens *before* any release. The
    // comparison is between the two voices at the same instant rather than each against itself,
    // because the engine's default reverb keeps ringing for about a second after a voice ends and a
    // window placed on that tail measures the reverb, not the envelope.
    let organ = render(0, 16, 60, 2.0);
    let pluck = render(0, 0, 60, 2.0);
    let started = level(window(&organ, 0.05, 0.2));
    let held = level(window(&organ, 0.4, 0.5));
    assert!(
        held > 0.5 * started,
        "an organ note had fallen to {held} from {started} while its key was still down"
    );
    let struck = level(window(&pluck, 0.05, 0.2));
    let after = level(window(&pluck, 0.4, 0.5));
    assert!(
        after < 0.1 * struck,
        "a struck note was still at {after} 0.4 s in, against its own {struck}"
    );
    assert!(
        after < 0.1 * held,
        "a struck note and a held organ note were {after} and {held} in the same second"
    );

    // And both end when the key is let go, which is the release stage being wired to anything at all.
    // Measured here: 7e-6 against 0.079, so this margin is five orders of magnitude, and the one
    // thing that could legitimately tighten it is a reverb that rings longer than the clip is played.
    for (name, clip, loud) in [("organ", &organ, held), ("pluck", &pluck, struck)] {
        let gone = level(window(clip, 3.4, 3.7));
        assert!(
            gone < 0.01 * loud,
            "the {name} was still at {gone} 1.4 s after its key was let go, against {loud}"
        );
    }
}

#[test]
fn the_eight_tables_are_eight_different_shapes() {
    // `Class` decides one thing: which harmonic stack becomes which waveform. A `partials` arm copied
    // from its neighbour leaves a bank that loads, parses, tunes and passes every test above with one
    // of its eight voices missing — so this looks at the tables themselves.
    //
    // The discriminator is the correlation between a cycle and its own second half. Shifting a cycle
    // by half turns harmonic k into (-1)^k of itself, so a wave built only of odd harmonics is its own
    // negative (-1), a saw — which has the even ones too, against the fundamental — is a copy of
    // itself (+1), and noise is neither.
    let half = |class: Class| {
        let cycle: Vec<f32> = class
            .table()
            .into_iter()
            .take(CYCLE)
            .map(|v| v as f32 / 32_768.0)
            .collect();
        repeats(&cycle, CYCLE / 2)
    };
    for class in [Class::Sine, Class::Triangle, Class::Square] {
        assert!(
            half(class) < -0.99,
            "{class:?} is not the negative of its own half-cycle: {}",
            half(class)
        );
    }
    // Measured, not guessed: -1.0 for the three odd-harmonic waves, +0.918 for the saw, +0.169 for
    // the noise. The saw is not nearer to 1.0 because truncating it to 32 harmonics leaves Gibbs
    // ripple at its discontinuity, and the discontinuity sits at the two ends of the cycle — so the
    // ripple is in one half-window and not the other.
    assert!(
        half(Class::Saw) > 0.9,
        "a saw is not a repeat of its own half-cycle: {}",
        half(Class::Saw)
    );
    assert!(
        half(Class::Perc).abs() < 0.3,
        "the noise voice repeats itself like a tone: {}",
        half(Class::Perc)
    );
    // Distinctness, the blunt version: no two of the eight are the same bytes.
    for class in CLASSES {
        assert_eq!(
            class.table().len(),
            TABLE,
            "{class:?} is not one cycle plus its interpolation head"
        );
    }
    let tables: Vec<(&str, Vec<i16>)> = CLASSES
        .iter()
        .map(|class| (class.tag(), class.table()))
        .collect();
    for (a, (_, left)) in tables.iter().enumerate() {
        for (b, (_, right)) in tables.iter().enumerate() {
            if a != b {
                assert_ne!(left, right, "two of the eight tables are identical");
            }
        }
    }
}
