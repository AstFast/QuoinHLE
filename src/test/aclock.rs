// The clock's own arithmetic: the reference hand geometry, the square the face becomes, and the two
// byte triples an `RGBVAL` turns into. No guest, no display — these are the numbers a screenshot is
// made of, so they are worth pinning without one.

use super::*;

#[test]
fn an_rgb_value_becomes_the_three_bytes_the_setters_keep() {
    // `MAKE_RGB(r, g, b)` puts red at bits 8-15, green at 16-23 and blue at 24-31, and the reference
    // macros read them back in that order.
    assert_eq!(rgb_parts(crate::device::make_rgb(1, 2, 3)), [1, 2, 3]);
    assert_eq!(rgb_parts(0), [0, 0, 0]);
}

#[test]
fn the_face_is_a_square_of_the_shorter_side_anchored_where_it_was_given() {
    // `AClock_CalcOffset` keeps x/y and makes both extents the smaller one.
    assert_eq!(
        square(Rect::new(5, 6, 80, 40)),
        Rect::new(5, 6, 40, 40),
        "the shorter side wins"
    );
    assert_eq!(square(Rect::new(0, 0, 30, 90)), Rect::new(0, 0, 30, 30));
}

#[test]
fn a_hand_points_at_the_clock_face_the_way_the_shipped_trigonometry_does() {
    // `AClock_GetX`/`_GetY` on a 100-pixel face at (10, 20), twelve ticks, factor 2:
    // noon is the top, three is the right edge, six the bottom, nine the left.
    let face = Rect::new(10, 20, 100, 100);
    assert_eq!(hand_point(face, 12, 0.0, 2.0), (60, 20), "twelve o'clock");
    assert_eq!(hand_point(face, 12, 3.0, 2.0), (110, 70), "three");
    assert_eq!(hand_point(face, 12, 6.0, 2.0), (60, 120), "six");
    assert_eq!(hand_point(face, 12, 9.0, 2.0), (10, 70), "nine");
    // Half a tick rounds nowhere: the tip of the second hand at 30 seconds is the bottom, and the
    // integer `dx / 2` in the body is why the centre is a whole pixel, not a half one.
    let (x, y) = hand_point(face, 60, 30.0, 2.1);
    assert_eq!((x, y), (60, 117), "diameter/2.1 is 47, truncated");
}

#[test]
fn a_fresh_clock_has_the_colours_and_properties_the_constructor_gives_it() {
    // `AClockCtl_New`: an 8-bit-or-better display gets a grey face, a black frame and
    // second hand, cyan minutes and a blue hour hand; a mono one gets a white frame and second hand.
    let clock = Clock::new(16);
    assert_eq!(clock.props, g::ACPROP_MINS | g::ACPROP_HOURS);
    assert_eq!(clock.face, [128, 128, 128]);
    assert_eq!(clock.frame, [0, 0, 0]);
    assert_eq!(clock.sec, [0, 0, 0]);
    assert_eq!(clock.min, [0, 255, 255]);
    assert_eq!(clock.hour, [0, 0, 255]);
    assert!(!clock.started);
    assert_eq!(clock.face_image, 0);

    let mono = Clock::new(1);
    assert_eq!(mono.frame, [255, 255, 255]);
    assert_eq!(mono.sec, mono.frame, "the second hand shares the frame");
    assert_eq!(mono.face, [0, 0, 0], "the mono face is left at zero");
}
