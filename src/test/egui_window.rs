// Moved out of `src/egui_window.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.
//
// What is left here is the **game view**: which codes the keyboard and the keypad send, the one integer
// that scales the phone and its picture alike, and the fit that leaves room for the desktop. The
// manager's panes are checked in `src/test/manager.rs` — the library window that used to live in this
// file moved there when it was rewritten.

use super::*;

#[test]
fn the_keyboard_and_the_keypad_send_the_same_codes() {
    // Two tables — the keys [`crate::handset`] places, and the PC keys read from egui — for one
    // set of meanings. A player cannot tell them apart, so they must not disagree.
    let h = Handset::for_device(240, 320);
    let keypad = |name: &str| h.key(name).map(|p| p.key.avk());
    for (key, name) in [
        (egui::Key::Z, "AVK_SOFT1"),
        (egui::Key::X, "AVK_SOFT2"),
        (egui::Key::ArrowLeft, "AVK_LEFT"),
        (egui::Key::ArrowDown, "AVK_DOWN"),
        (egui::Key::Enter, "AVK_SELECT"),
        (egui::Key::Backspace, "AVK_CLR"),
        (egui::Key::Num7, "AVK_7"),
    ] {
        assert_eq!(handset_of(key), keypad(name), "{name} from the keyboard");
        assert!(keypad(name).is_some(), "{name} is on the keypad");
    }
    // Esc closes the window; it is not a key the handset has.
    assert_eq!(handset_of(egui::Key::Escape), None);
}

#[test]
fn one_integer_scales_the_phone_and_its_picture_alike() {
    // The screen must land on an exact multiple of the device's own size, and a key must move by
    // the same factor, or the keypad stops lining up with the picture it is answering.
    let h = Handset::for_device(176, 220);
    let origin = Pos2::new(7.5, 7.5);
    for scale in [1.0, 2.0, 3.0] {
        let screen = place(h.screen, origin, scale);
        assert_eq!(screen.width(), 176.0 * scale);
        assert_eq!(screen.height(), 220.0 * scale);
        let device = h.key("AVK_SELECT").unwrap().rect;
        let key = place(device, origin, scale);
        assert_eq!(key.width(), device.wide * scale, "same scale, same key");
        assert!(key.max.x <= place(h.body, origin, scale).max.x + 0.01);
    }
}

#[test]
fn the_fit_leaves_room_for_the_desktop_and_falls_back_to_one() {
    // A body that only just fits the *monitor* has its last row of keys under the taskbar, so the
    // scale is chosen against what is left once the title bar and taskbar have taken theirs.
    let body = Vec2::new(268.0, 500.0);
    assert_eq!(fit(Vec2::new(1024.0, 768.0), body), 1.0);
    assert_eq!(fit(Vec2::new(1600.0, 1200.0), body), 2.0);
    // 1100 points of monitor holds 500x2 only if the desktop's own chrome is ignored, which is
    // the bug this line exists to keep from coming back.
    assert_eq!(fit(Vec2::new(1024.0, 1120.0), body), 2.0);
    assert_eq!(fit(Vec2::new(1024.0, 1100.0), body), 1.0);
    // A body too big for the screen is still drawn at 1:1, never shrunk: a fractional scale is
    // what makes shipped art unreadable, so the window clips instead.
    assert_eq!(fit(Vec2::new(200.0, 200.0), Vec2::new(900.0, 700.0)), 1.0);
}

/// One PC key, as egui hands it over.
fn key_event(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

/// One frame's worth of input carrying those events.
fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(400.0, 200.0),
        )),
        events,
        ..Default::default()
    }
}

#[test]
fn the_phone_gets_the_keyboard_unless_something_else_is_typing() {
    // The manager draws a search box in the same window as this phone. A key that reaches both is a
    // search box nobody can type in and a game answering keystrokes aimed at a text field, so the phone
    // reads nothing at all while something else wants the keyboard.
    //
    // Two frames, because egui's focus lands on the frame after it is asked for: the first asks, the
    // second is the one carrying the keystroke.
    let ctx = egui::Context::default();
    let mut typed = String::new();
    let mut keys = Keys::default();
    let mut wanted = false;
    for frame in 0..2 {
        let events = if frame == 0 {
            Vec::new()
        } else {
            vec![
                key_event(egui::Key::Z, true),
                key_event(egui::Key::Z, false),
            ]
        };
        let _ = ctx.run(raw_input(events), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.text_edit_singleline(&mut typed).request_focus();
                wanted = ctx.wants_keyboard_input();
                keys = phone_keys(ctx, &Default::default());
            });
        });
    }
    assert!(wanted, "the box does have the keyboard");
    assert_eq!(
        keys,
        Keys::default(),
        "so the phone gets none of the keys aimed at it"
    );

    // The positive half, which is what keeps the assertion above from passing for the wrong reason: with
    // nothing else asking for the keyboard, the same press is the phone's.
    let ctx = egui::Context::default();
    let mut keys = Keys::default();
    let _ = ctx.run(
        raw_input(vec![
            key_event(egui::Key::Z, true),
            key_event(egui::Key::Z, false),
        ]),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |_ui| {
                keys = phone_keys(ctx, &Default::default());
            });
        },
    );
    assert_eq!(keys.pressed, vec![keymap::avk("AVK_SOFT1")]);
    assert_eq!(keys.released, vec![keymap::avk("AVK_SOFT1")]);
    assert!(!keys.leaving, "Z is not the key that leaves");

    // Esc is not a handset key: it is the player leaving, and it is reported as that rather than as a
    // press the applet could see.
    let ctx = egui::Context::default();
    let mut keys = Keys::default();
    let _ = ctx.run(raw_input(vec![key_event(egui::Key::Escape, true)]), |ctx| {
        egui::CentralPanel::default().show(ctx, |_ui| {
            keys = phone_keys(ctx, &Default::default());
        });
    });
    assert!(keys.leaving, "Esc leaves");
    assert!(keys.pressed.is_empty(), "and is not a key the handset has");

    // Auto-repeat: a held key being re-reported. egui does not take the flag from the input — it works
    // out "this key is already down" from its own state — so a repeat is two frames of the same press,
    // not one frame carrying `repeat: true`.
    let ctx = egui::Context::default();
    let mut handed = Vec::new();
    let mut per_frame = Vec::new();
    for frame in 0..2 {
        let mut keys = Keys::default();
        let _ = ctx.run(raw_input(vec![key_event(egui::Key::Z, true)]), |ctx| {
            egui::CentralPanel::default().show(ctx, |_ui| {
                if frame == 1 {
                    handed = ctx.input(|i| i.events.clone());
                }
                keys = phone_keys(ctx, &Default::default());
            });
        });
        per_frame.push(keys);
    }
    assert_eq!(
        per_frame[0].pressed,
        vec![keymap::avk("AVK_SOFT1")],
        "the first frame is a press"
    );
    assert!(
        handed
            .iter()
            .any(|e| matches!(e, egui::Event::Key { repeat: true, .. })),
        "and egui does mark the second frame's as a repeat: {handed:?}"
    );
    assert!(
        per_frame[1].pressed.is_empty(),
        "which the phone does not take for another press: {:?}",
        per_frame[1]
    );
}

#[test]
fn a_custom_binding_replaces_only_the_key_it_names() {
    let mut bindings = BTreeMap::new();
    bindings.insert("AVK_SOFT1".to_string(), "Q".to_string());
    assert_eq!(
        handset_of_with(egui::Key::Q, &bindings),
        Some(keymap::avk("AVK_SOFT1"))
    );
    assert_eq!(
        handset_of_with(egui::Key::Z, &bindings),
        None,
        "the old default is gone for the rebound key"
    );

    bindings.insert("AVK_SELECT".to_string(), "F1".to_string());
    assert_eq!(
        handset_of_with(egui::Key::F1, &bindings),
        Some(keymap::avk("AVK_SELECT"))
    );
    assert_eq!(
        handset_of_with(egui::Key::ArrowLeft, &bindings),
        Some(keymap::avk("AVK_LEFT")),
        "unbound keys keep their default"
    );
}

/// A long absence is not a slow frame. Folding a five-second gap into `1/gap` used to produce
/// `0 FPS`, and the EMA then took a dozen ordinary frames to climb back from it. A *steady* slow
/// cadence, on the other hand, is a real frame rate and must be quoted as one.
#[test]
fn a_long_absence_restarts_the_rate_but_a_steady_slow_rate_is_quoted() {
    use std::time::Duration;
    let ordinary = Duration::from_millis(16);
    assert!(
        (fold_fps(0.0, Some(ordinary)) - 62.5).abs() < 0.01,
        "the first frame seeds the rate from its own gap"
    );
    let folded = fold_fps(42.0, Some(ordinary));
    assert!(
        (folded - (42.0 * 0.9 + 62.5 * 0.1)).abs() < 0.01,
        "an ordinary gap is smoothed: {folded}"
    );
    let slow = Duration::from_millis(400);
    assert!(
        (fold_fps(0.0, Some(slow)) - 2.5).abs() < 0.01,
        "a pack presenting every 400 ms is running at 2.5 FPS, not at no rate at all"
    );
    assert!(
        fold_fps(42.0, Some(slow)) > 0.0,
        "a steady slow cadence must never be reset to zero"
    );
    assert_eq!(
        fold_fps(42.0, Some(Duration::from_secs(5))),
        0.0,
        "a five-second absence restarts the measurement rather than becoming 0.2 FPS"
    );
    assert_eq!(
        fold_fps(42.0, Some(Duration::from_millis(1001))),
        0.0,
        "just beyond the absence bound is an absence too"
    );
    assert_eq!(
        fold_fps(42.0, None),
        42.0,
        "no previous frame changes nothing"
    );
    // ...and on the way back the badge never prints "0 FPS".
    let reading = FpsReading {
        running: true,
        turn_active: false,
        advancing: true,
        busy_for: None,
    };
    assert_eq!(fps_badge(0.0, true, reading), "FPS -- (idle)");
}

/// Freshness follows the run's own rhythm, not one fixed frame budget. This is what stopped a pack
/// drawing a few times a second from being told it had no frame rate at all.
#[test]
fn freshness_follows_the_presentation_rhythm() {
    use std::time::{Duration, Instant};
    let now = Instant::now();
    let rhythm = Some(Duration::from_millis(400));
    assert!(
        presentation_fresh(now, Some(now - Duration::from_millis(400)), rhythm),
        "a pack presenting every 400 ms is still current 400 ms later"
    );
    assert!(
        !presentation_fresh(now, Some(now - Duration::from_secs(2)), rhythm),
        "a two-second freeze is not covered by a 400 ms rhythm"
    );
    assert!(
        presentation_fresh(now, Some(now - Duration::from_millis(100)), None),
        "before a rhythm exists, the floor still allows a fresh frame"
    );
    assert!(
        !presentation_fresh(now, Some(now - Duration::from_millis(300)), None),
        "and the floor does not let a stale number linger"
    );
    assert!(
        !presentation_fresh(now, None, rhythm),
        "no presentation at all is never fresh"
    );
}

/// The badge reports only what was *observed*. It never calls a healthy machine stalled: host work
/// inside a turn (screenshot, audio, network) retires no guest instructions either.
#[test]
fn the_fps_badge_reports_observations_not_a_diagnosis() {
    use std::time::Duration;
    let reading = |turn_active: bool, advancing: bool, busy_for: Option<Duration>| FpsReading {
        running: true,
        turn_active,
        advancing,
        busy_for,
    };
    assert_eq!(
        fps_badge(42.0, true, reading(false, true, None)),
        "42 FPS",
        "a fresh presentation quotes the rate"
    );
    assert_eq!(
        fps_badge(42.0, false, reading(false, true, None)),
        "FPS -- (idle)",
        "no turn in progress and no new picture is a still screen, not a slowdown"
    );
    assert_eq!(
        fps_badge(
            0.0,
            false,
            reading(false, true, Some(Duration::from_secs(5)))
        ),
        "FPS -- (idle)",
        "between turns the last turn's start is history, not a fake five-second stall timer"
    );
    assert_eq!(
        fps_badge(
            0.0,
            false,
            reading(true, true, Some(Duration::from_millis(320)))
        ),
        "FPS -- (busy 320 ms)",
        "a guest still retiring instructions inside one long turn is computing"
    );
    assert_eq!(
        fps_badge(
            0.0,
            false,
            reading(true, false, Some(Duration::from_millis(320)))
        ),
        "FPS -- (quiet 320 ms)",
        "no guest progress is stated as quiet, not blamed on a stalled machine: the host may be doing the work"
    );
    assert_eq!(
        fps_badge(
            0.0,
            false,
            reading(true, true, Some(Duration::from_millis(16)))
        ),
        "FPS -- (idle)",
        "a short turn is an ordinary frame, not a diagnosis"
    );
    assert_eq!(
        fps_badge(
            0.0,
            false,
            reading(true, false, Some(Duration::from_millis(16)))
        ),
        "FPS -- (idle)",
        "a guest waiting on a timer in a short turn retires no instructions and is not stalled"
    );
    let stopped = FpsReading {
        running: false,
        ..reading(true, true, None)
    };
    assert_eq!(
        fps_badge(42.0, true, stopped),
        "FPS --",
        "a run that ended has no rate and no reason worth naming"
    );
}
