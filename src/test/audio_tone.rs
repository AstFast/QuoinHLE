// Moved out of `src/audio/tone.rs` so that file is implementation only, the way the rest of the
// tree keeps its tests. The `#[path]` attribute on its `mod tests;` declaration is what keeps this a
// child module of that file, and that is what lets these reach its private items with `use super::*`.

use super::*;
use crate::abi::generated as g;

/// The ids whose sound is published rather than the OEM's, and what each of them is.
///
/// These numbers are checkable against documents this host did not write: the ring-tone range is
/// named after notes (A4 is 440 Hz by definition, and `AEE_TONE_RING_A7` is three octaves above it),
/// and the keypad is DTMF's grid. A table that drifted would show up here as a wrong frequency and
/// not as a beep somebody has to recognise by ear.
#[test]
fn the_published_ids_say_what_they_sound_like() {
    assert_eq!(frequencies(g::AEE_TONE_RING_A4), Some((440, 0)));
    assert_eq!(
        frequencies(g::AEE_TONE_RING_A7),
        Some((3520, 0)),
        "thirty-six semitones above A4 is three octaves"
    );
    assert_eq!(
        frequencies(g::AEE_TONE_RING_AS4),
        Some((466, 0)),
        "the next id up is the next semitone, not the next octave"
    );
    // The keypad is DTMF's four-by-four grid, read by the digit the name prints and **not** by the
    // id: `AEE_TONE_0` is 1 and `AEE_TONE_1` is 2, so a table indexed by value would sound every
    // key as the one next to it.
    assert_eq!(frequencies(g::AEE_TONE_1), Some((697, 1209)));
    assert_eq!(frequencies(g::AEE_TONE_0), Some((941, 1336)));
    assert_eq!(frequencies(g::AEE_TONE_9), Some((852, 1477)));
    assert_eq!(frequencies(g::AEE_TONE_STAR), Some((941, 1209)));
    assert_eq!(frequencies(g::AEE_TONE_POUND), Some((941, 1477)));
    assert_eq!(frequencies(g::AEE_TONE_C), Some((852, 1633)));
    // Call progress, in the form the networks publish.
    assert_eq!(frequencies(g::AEE_TONE_BUSY), Some((480, 620)));
    assert_eq!(frequencies(g::AEE_TONE_DIAL_TONE_TONE), Some((350, 440)));
    assert_eq!(frequencies(g::AEE_TONE_RBACK), Some((440, 480)));
}

/// Silence is a published answer too, and the honest one for an id whose sound was the handset's.
///
/// A guest that asks for `AEE_TONE_ERR` gets no sound from this host; that is a hole, and the point
/// of the hole is that it is *named* here rather than filled in with a beep nobody specified. The
/// request is still accepted and still reports `AEE_SOUND_PLAY_DONE` — the report is about the
/// request, never about this host's repertoire.
#[test]
fn the_ids_that_belong_to_the_oem_are_silent_rather_than_invented() {
    for id in [
        g::AEE_TONE_FIRST,
        g::AEE_TONE_CTRL,
        g::AEE_TONE_2ND,
        g::AEE_TONE_WARN,
        g::AEE_TONE_ERR,
        g::AEE_TONE_TIME,
        g::AEE_TONE_RING_A,
        g::AEE_TONE_SILENCE,
        g::AEE_TONE_PTT_SUCCESS,
        g::AEE_TONE_LAST,
    ] {
        assert_eq!(frequencies(id), None, "{id:#x} is not this host's to sound");
    }
}

/// The two halves of a tune: the record's centred value read back as a step, and the step as an
/// interval.
#[test]
fn the_tune_helpers_read_the_record_and_the_interval() {
    assert_eq!(
        tune_step(0),
        0,
        "a player that was never tuned is untuned, not -64 semitones"
    );
    assert_eq!(tune_step(0x40), 0, "0x40 is the centre `SetTune` writes");
    assert_eq!(tune_step(0x40 + 7), 7);
    assert_eq!(tune_step(0x40 - 12), -12);
    assert!((tune_factor(12) - 2.0).abs() < 1e-12, "an octave doubles");
    assert!((tune_factor(-12) - 0.5).abs() < 1e-12);
    assert_eq!(tune_factor(0), 1.0);
}

/// The generator sounds the frequency it was built with, at the level it was given, and leaves
/// nothing behind when it is released.
///
/// The pitch is asserted by counting zero crossings, which is the one measurement that can be made
/// of a sine without trusting the generator's own arithmetic: 440 Hz for 100 ms is 44 cycles and
/// therefore 88 crossings.
#[test]
fn a_tone_oscillates_at_its_frequency_and_fades_rather_than_stopping() {
    let rate = 44_100.0;
    let mut tone = ToneGen::new(440, 0, rate, g::AEE_MAX_VOLUME);
    let mut left = vec![0.0f32; 4_410];
    let mut right = vec![0.0f32; 4_410];
    assert!(!tone.render(&mut left, &mut right), "still sounding");
    assert_eq!(left, right, "a tone is the same sample on both sides");
    let crossings = left
        .windows(2)
        .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
        .count();
    assert!(
        (86..=90).contains(&crossings),
        "440 Hz over 100 ms is 44 cycles, so 88 crossings; counted {crossings}"
    );
    let peak = left
        .iter()
        .fold(0.0f32, |top, sample| top.max(sample.abs()));
    assert!(peak > 0.9, "full level reaches full scale: {peak}");

    // A release is a fade, not a cut: the block that ends the tone still has sound in it, and the
    // block after that has none.
    tone.release();
    let mut fade = vec![0.0f32; 512];
    let mut fade_right = vec![0.0f32; 512];
    assert!(
        tone.render(&mut fade, &mut fade_right),
        "5 ms of fade fits inside 512 samples at 44.1 kHz"
    );
    assert!(
        fade.iter().any(|sample| *sample != 0.0),
        "the fade is heard, which is what keeps a stop from clicking"
    );
    assert!(
        fade[fade.len() - 1] == 0.0,
        "and it has reached silence by the end of the block"
    );
}

/// A level scales the tone and nothing else, and a different pair of frequencies is a different
/// tone — the comparison `Mixer::tone` uses to tell a repeated request from a new one.
#[test]
fn the_level_is_the_packs_zero_to_a_hundred_and_the_pair_identifies_the_tone() {
    let rate = 44_100.0;
    let peak = |volume: u32| {
        let mut tone = ToneGen::new(1_000, 0, rate, volume);
        let mut left = vec![0.0f32; 4_410];
        let mut right = vec![0.0f32; 4_410];
        tone.render(&mut left, &mut right);
        left.iter()
            .fold(0.0f32, |top, sample| top.max(sample.abs()))
    };
    let full = peak(g::AEE_MAX_VOLUME);
    let half = peak(50);
    assert!(
        (half - full / 2.0).abs() < 0.02,
        "50 is half of 100: {full} -> {half}"
    );

    let mut quiet = ToneGen::new(1_000, 0, rate, 0);
    let mut left = vec![0.0f32; 441];
    let mut right = vec![0.0f32; 441];
    quiet.render(&mut left, &mut right);
    assert!(
        left.iter().all(|sample| *sample == 0.0),
        "level 0 is silence"
    );
    assert!(
        quiet.is(1_000, 0) && !quiet.is(440, 0),
        "the pair is the identity"
    );
}
