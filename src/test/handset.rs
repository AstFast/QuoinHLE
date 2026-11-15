// Moved out of `src/handset.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn every_key_the_table_offers_has_exactly_one_place_on_the_body() {
    // The two files split one claim: `keymap` knows the keys that exist, this knows the keys
    // that are on the phone. A key missing here is a key a player can only reach from the PC
    // keyboard, and a duplicate is a phone with two of one button.
    let h = handset();
    for key in keymap::KEYS {
        let hits = h.keys.iter().filter(|p| p.key.name == key.name).count();
        assert_eq!(hits, 1, "{} placed once", key.name);
    }
    assert_eq!(h.keys.len(), keymap::KEYS.len());
}

fn handset() -> Handset {
    Handset::for_device(240, 320)
}

#[test]
fn nothing_is_outside_the_body_and_nothing_overlaps() {
    let h = handset();
    assert!(h.body.contains(&h.screen), "the screen is inside");
    for a in &h.keys {
        assert!(h.body.contains(&a.rect), "{} outside the body", a.key.name);
        assert!(
            a.rect.wide > 4.0 && a.rect.high > 4.0,
            "{} too small",
            a.key.name
        );
        for b in &h.keys {
            if std::ptr::eq(a, b) {
                continue;
            }
            assert!(
                !a.rect.overlaps(&b.rect),
                "{} overlaps {}",
                a.key.name,
                b.key.name
            );
        }
    }
    // The keypad is under the screen, not over it: a key that covered a pixel of the device
    // would be a key the player cannot see the answer to.
    for a in &h.keys {
        assert!(
            !a.rect.overlaps(&h.screen),
            "{} covers the screen",
            a.key.name
        );
    }
}

#[test]
fn the_screen_keeps_the_size_the_device_reported() {
    // The whole reason the geometry is in device pixels: the frontend multiplies by an integer,
    // so a screen rect that is not exactly the device's own size would resample the game.
    for (wide, high) in [(176, 220), (240, 320), (320, 240), (480, 240)] {
        let h = Handset::for_device(wide, high);
        assert_eq!((h.screen.wide as u32, h.screen.high as u32), (wide, high));
        assert!(h.body.wide >= wide as f32);
        assert!(
            h.body.high > high as f32,
            "a body is taller than its screen"
        );
        // Every key has to fit the window the body asks for, however wide the pack is.
        for a in &h.keys {
            assert!(h.body.contains(&a.rect), "{wide}x{high}: {}", a.key.name);
        }
    }
}

#[test]
fn the_rocker_is_a_cross_and_not_a_list_of_squares() {
    // The five navigation keys share one centre. If a layout change ever scatters them a
    // player's thumb finds it first, but the drawing depends on it too: it paints one ring
    // behind the five cells, and a ring only reads as a rocker if the arms are on it.
    let h = handset();
    let (cx, cy) = h.key("AVK_SELECT").unwrap().rect.centre();
    let pitch = h.key("AVK_UP").unwrap().rect.high + 2.0;
    let centre = |name: &str| h.key(name).unwrap().rect.centre();
    for (name, dx, dy) in [
        ("AVK_UP", 0.0, -1.0),
        ("AVK_DOWN", 0.0, 1.0),
        ("AVK_LEFT", -1.0, 0.0),
        ("AVK_RIGHT", 1.0, 0.0),
    ] {
        let (ax, ay) = centre(name);
        let across = if dx != 0.0 {
            (ay - cy).abs()
        } else {
            (ax - cx).abs()
        };
        let along = (ax - cx) * dx + (ay - cy) * dy;
        assert!(across < 1.0, "{name} is off its row or column by {across}");
        assert!(
            (along - pitch).abs() < 1.0,
            "{name} is {along} from the centre, not one pitch's {pitch}"
        );
    }
}

#[test]
fn the_keypad_costs_a_fixed_height_below_the_screen() {
    // What put a shipped pack's digits off the bottom of the desktop was not its screen: the
    // keypad is a fixed cost, and a layout that let it grow with the screen's *width* spends
    // more phone below a landscape pack than any window has room for.
    for (wide, high) in [(176, 220), (240, 320), (320, 240), (480, 240)] {
        let h = Handset::for_device(wide, high);
        let below = h.body.high - h.screen.bottom();
        assert!(
            below < 360.0,
            "{wide}x{high} spends {below} below its screen"
        );
    }
}

#[test]
fn a_skin_name_either_resolves_or_says_what_would() {
    assert_eq!(skin("Graphite").map(|s| s.name), Some("graphite"));
    assert_eq!(
        skin("").map(|s| s.name),
        Some("graphite"),
        "empty is the default"
    );
    assert!(skin("banana").is_none());
    assert!(skin_names().contains("candy"));
    for s in SKINS {
        assert_ne!(s.body, s.bezel, "{}'s glass must read as recessed", s.name);
        assert_ne!(s.key, s.key_down, "{}'s keys must light up", s.name);
    }
}
