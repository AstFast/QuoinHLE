//! The canvas's only pure logic: an `AEERect` and the two guest words it travels as.

use crate::canvas::Clip;

/// The clip is stored as two `u32` because that is what a guest reads and writes, so the conversion
/// has to be exact in both directions — including the sign, which is the only part that can go wrong.
#[test]
fn words_round_trip_every_field() {
    let clip = Clip {
        x: -3,
        y: 7,
        dx: 240,
        dy: -1,
    };
    assert_eq!(Clip::from_words(clip.words()), clip);
}

/// Width and height are `int16` in `AEERect`, so a 240×320 screen fits and a 65536-pixel one does
/// not. Saturating is the honest answer: a clip clamped to `i16::MAX` paints less than the surface
/// but never wraps into a negative extent, which would paint nothing at all.
#[test]
fn whole_saturates_instead_of_wrapping() {
    let big = Clip::whole(70_000, 40);
    assert_eq!(big.dx, i16::MAX);
    assert_eq!(big.dy, 40);
    assert!(!big.is_empty());
}

/// `Clip::default()` is what a canvas with no bitmap reports, and it has to paint nothing — a
/// zero-extent rectangle is the "failed" answer rendered as geometry.
#[test]
fn default_paints_nothing() {
    assert!(Clip::default().is_empty());
    assert_eq!(Clip::from_words(Clip::default().words()), Clip::default());
}

/// A negative width is not a smaller rectangle, it is an empty one: `dx <= 0` is the test, and the
/// number is kept as written so `GetClipRect` gives back what `SetClipRect` was told.
#[test]
fn a_negative_extent_is_empty_but_kept() {
    let clip = Clip {
        x: 10,
        y: 10,
        dx: -5,
        dy: 4,
    };
    assert!(clip.is_empty());
    assert_eq!(Clip::from_words(clip.words()).dx, -5);
}
