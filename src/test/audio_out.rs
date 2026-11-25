// Moved out of `src/audio_out.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;
use crate::abi::generated as g;
use crate::runtime::AudioSource;

fn soundfont() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("BREW_SOUNDFONT")?;
    if path.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(&path);
    assert!(
        path.is_file(),
        "BREW_SOUNDFONT={path:?} has to name a readable .sf2"
    );
    Some(path)
}

/// A file whose merged timeline is empty has no length to report, and rustysynth 1.3.6 says so by
/// unwrapping an empty `Vec`.
///
/// `MidiFile::get_length` is `*self.times.last().unwrap()`, and the track loop runs once per track
/// the header declares — so a file declaring **zero** tracks parses `Ok`, merges to nothing, and
/// **panics the host** when asked how long it is. That count is bytes a pack hands over:
/// `ISoundPlayer::Set` names a source and `arm_notify` times it on every following call, and
/// `IShell::DetectType` sniffs any buffer the guest points at. The control at the bottom is what
/// keeps this from passing on a guard that simply returns `None` for everything.
#[test]
fn audio_backend_names_are_explicit_and_ffmpeg_is_not_silently_substituted() {
    assert_eq!(AudioBackend::default(), AudioBackend::Rust);
    assert_eq!(AudioBackend::parse("rust"), Some(AudioBackend::Rust));
    assert_eq!(AudioBackend::parse(" FFMPEG "), Some(AudioBackend::Ffmpeg));
    assert_eq!(AudioBackend::parse("vlc"), None);
    assert!(AudioBackend::Rust.available());
    assert_eq!(
        AudioBackend::Ffmpeg.available(),
        cfg!(feature = "ffmpeg"),
        "FFmpeg is available exactly when the feature is built"
    );
}

fn qcp_full_frame() -> Vec<u8> {
    let mut format = vec![0u8; 0x96];
    format[20..29].copy_from_slice(b"Qcelp 13K");
    let data = [&[4u8][..], &[0u8; 34]].concat();
    let chunk = |id: &[u8; 4], payload: &[u8]| {
        let mut out = Vec::new();
        out.extend_from_slice(id);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        if payload.len() & 1 != 0 {
            out.push(0);
        }
        out
    };
    let body = [
        &b"QLCM"[..],
        &chunk(b"fmt ", &format),
        &chunk(b"data", &data),
    ]
    .concat();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[test]
fn a_qcp_clip_is_sniffed_and_decoded_into_a_pcm_voice() {
    let bytes = qcp_full_frame();
    assert_eq!(sniff(&bytes), Some(MIME_QCP));
    let (rate, samples) = crate::audio::qcelp::decode_qcp(&bytes).expect("QCP decodes");
    assert_eq!(rate, 8_000);
    assert_eq!(samples.len(), 160);

    let (font, _) = load(None).expect("built-in tone bank");
    let mut mixer = Mixer::offline(16_000, &font).expect("offline mixer");
    mixer.assign_source(
        1,
        PcmSource::Buffer {
            samples: Arc::new(vec![0.25f32; 160]),
            at: 0.0,
            rate: 8_000,
            stop: -1.0,
            reverse: false,
        },
    );
    let mut left = [0.0f32; 160];
    let mut right = [0.0f32; 160];
    mixer.render_into(&mut left, &mut right);
    assert!(left.iter().all(|sample| (*sample - 0.25).abs() < 1e-6));
}

#[test]
fn a_midi_file_with_nothing_on_the_timeline_has_no_length_rather_than_a_panic() {
    // MThd, header length 6, format 0, **track count 0**, division 0.
    let no_tracks = [b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        clip_length_ms(&no_tracks),
        None,
        "a score with no track has no length"
    );
    // MThd, format 0, one track, division 480, holding a tempo change and an end-of-track. The
    // end-of-track does land on the timeline, so this one never reached the `unwrap` — it just has
    // nothing to time, because both of its events sit at tick 0.
    let no_ticks = [
        b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0, b'M', b'T', b'r', b'k', 0, 0,
        0, 11, 0, 0xFF, 0x51, 3, 0x07, 0xA1, 0x20, 0, 0xFF, 0x2F, 0,
    ];
    assert_eq!(
        clip_length_ms(&no_ticks),
        None,
        "and a score that ends where it started"
    );
    // Both are still MIDI. The front end's own acceptance test is `MidiFile::new(..).is_ok()` (see
    // [`Speaker::play`]), and it takes either one and plays a beat of silence — so a sniffer that
    // refused them would be *stricter* than the thing it describes, which is the drift this pair of
    // functions exists to prevent.
    assert_eq!(sniff(&no_tracks), Some(MIME_MIDI));
    assert_eq!(sniff(&no_ticks), Some(MIME_MIDI));
    // The control: a file with a note on it has a length, so the two refusals above are the guard
    // biting and not a dead path.
    let played = one_note_midi();
    assert!(
        clip_length_ms(&played).is_some(),
        "a one-note score has a length"
    );
    assert_eq!(sniff(&played), Some(MIME_MIDI));
    assert_eq!(
        sniff(&[b'M', b'T', b'h', b'd']),
        None,
        "and a four-byte stub is not a file"
    );
}

#[test]
fn a_score_that_arrives_as_the_detection_window_is_still_named_midi() {
    // `IShell::DetectType` promises a caller the first 32 bytes (`TYPE_DETECT_WINDOW`) and takes
    // its answer from those; the documented media probe (`AEEMediaUtil_FindClass`) offers exactly
    // that much. Every real track is longer, so a sniffer that parses the whole file refuses a
    // score the handset names — which is how a 3D title's load was aborted.
    let mut window = Vec::new();
    window.extend_from_slice(b"MThd");
    window.extend_from_slice(&6u32.to_be_bytes());
    window.extend_from_slice(&1u16.to_be_bytes()); // format 1
    window.extend_from_slice(&3u16.to_be_bytes()); // three tracks
    window.extend_from_slice(&192u16.to_be_bytes());
    window.extend_from_slice(b"MTrk");
    window.extend_from_slice(&2490u32.to_be_bytes()); // the track runs past the window
    assert_eq!(window.len(), 22);
    window.resize(crate::runtime::TYPE_DETECT_WINDOW as usize, 0);
    assert_eq!(sniff(&window), Some(MIME_MIDI));
    // The magic alone is not the format: the header must carry values a MIDI allows.
    let mut not_midi = window.clone();
    not_midi[0] = b'X';
    assert_eq!(sniff(&not_midi), None);
}

/// A runtime holding one recorded source, and a player pointed at it. The player's three source
/// fields are set to match, which is the state a real run is in by the time anything plays.
fn run_with_one_clip() -> (Runtime, u32) {
    let mut rt = Runtime::new();
    let bytes = one_note_midi();
    let len = bytes.len() as u32;
    let clip = rt.cpu.mem.alloc(len, 4).unwrap();
    rt.cpu.mem.write_bytes(clip, &bytes).unwrap();
    rt.audio.push(AudioSource {
        input: g::SDT_BUFFER,
        data: clip,
        size: len,
        name: format!("{clip:#010x}"),
        bytes,
        problem: None,
    });
    let object = 0x0400_0050;
    let player = rt.players.entry(object).or_default();
    player.input = g::SDT_BUFFER;
    player.data = clip;
    player.size = len;
    (rt, object)
}

/// A second recorded source on the same runtime, so a player can be moved between two clips the
/// way a pack moves its music player between two scores.
fn add_second_clip(rt: &mut Runtime) -> (u32, u32) {
    let bytes = one_note_midi();
    let len = bytes.len() as u32;
    let clip = rt.cpu.mem.alloc(len, 4).unwrap();
    rt.cpu.mem.write_bytes(clip, &bytes).unwrap();
    rt.audio.push(AudioSource {
        input: g::SDT_BUFFER,
        data: clip,
        size: len,
        name: format!("{clip:#010x}"),
        bytes,
        problem: None,
    });
    (clip, len)
}

#[test]
fn a_clip_renders_into_samples_that_are_not_silence() {
    let Some(font) = soundfont() else {
        eprintln!("skipped: set BREW_SOUNDFONT to a .sf2 to render a clip");
        return;
    };
    let (left, right) =
        render_clip(Some(&font), &one_note_midi(), 44_100, 44_100).expect("bytes to samples");
    assert_eq!(left.len(), 44_100, "one second per channel");
    assert_eq!(right.len(), 44_100);
    let peak = left
        .iter()
        .chain(right.iter())
        .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak > 0.01, "the note is audible, peak {peak}");
}

#[test]
fn a_file_that_is_not_a_soundfont_is_refused_by_name() {
    let mut path = std::env::temp_dir();
    path.push("QuoinHLE-not-a-soundfont.sf2");
    std::fs::write(&path, b"this is not a RIFF sfbk at all").unwrap();
    let err = render_clip(Some(&path), &one_note_midi(), 44_100, 16).unwrap_err();
    let _ = std::fs::remove_file(&path);
    assert!(err.contains("not a SoundFont"), "{err}");
}

#[test]
fn a_payload_that_is_not_midi_is_refused_by_name() {
    let Some(font) = soundfont() else {
        eprintln!("skipped: set BREW_SOUNDFONT to a .sf2 to render a clip");
        return;
    };
    let err = render_clip(Some(&font), b"not a midi file either", 44_100, 16).unwrap_err();
    assert!(err.contains("not a MIDI file"), "{err}");
}

/// `names_a_sound` is the string half of the audio question, and it is the half `--info`'s census
/// fell back on. It has to fold the same way `IShell::GetHandler` folds, or the two authorities
/// disagree about one declaration: the census would say "this container names no sound" while the
/// registry would happily hand out the media class for the same text.
#[test]
fn a_declaration_is_a_sound_by_the_same_folding_the_registry_uses() {
    // Both platform sound bases, in the spellings the header actually ships.
    assert!(names_a_sound("audio/mid"), "{MIME_MIDI}");
    assert!(names_a_sound(MIME_MIDI_LEGACY), "{MIME_MIDI_LEGACY}");
    assert!(names_a_sound("audio/qcelp"));
    assert!(names_a_sound("snd/imy"));
    // A surrounding space is not a different MIME: `IShell::RegisterHandler` takes a comma-delimited
    // list, so `"audio/mid, image/bmp"` arrives at the lookup as `" image/bmp"`. The registry trims
    // there; this must too.
    assert!(
        names_a_sound("audio/mid "),
        "trimmed like the handler list is"
    );
    // Case is folded, like `Runtime::handlers` folds it on both sides of every lookup.
    assert!(names_a_sound("AUDIO/MIDI"));
    assert!(names_a_sound("Snd/QCP"));
    // What is not a sound: the other two bases, a bare prefix that names nothing, and a name that
    // merely contains the base. The last two are what a `contains` would have let through.
    assert!(!names_a_sound("image/png"));
    assert!(!names_a_sound("video/3gpp"));
    assert!(!names_a_sound("application/octet-stream"));
    assert!(!names_a_sound(""));
    assert!(!names_a_sound("audio"));
    assert!(!names_a_sound("snd"));
    assert!(!names_a_sound("x-audio/mid"));
    assert!(!names_a_sound("not audio/mid"));
}

#[test]
fn a_player_that_stays_playing_is_started_once_and_not_once_per_frame() {
    let (mut rt, object) = run_with_one_clip();
    let mut playing = Playing::new();
    rt.players.get_mut(&object).unwrap().playing = true;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "the frame it starts on"
    );
    assert!(requests(&rt, &mut playing).is_empty(), "already playing");
    assert!(requests(&rt, &mut playing).is_empty(), "and still");
    rt.players.get_mut(&object).unwrap().playing = false;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Stop { voice: object }],
        "a player that stops is a stop"
    );
    rt.players.get_mut(&object).unwrap().playing = true;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "and starting again is a play again, which is what a repeated effect is"
    );
}

#[test]
fn a_reused_buffer_resolves_to_the_bytes_that_are_there_now() {
    let (mut rt, object) = run_with_one_clip();
    let player = rt.players[&object];
    let current = rt.audio[0].bytes.clone();
    let (data, size) = (player.data, player.size);
    // The first record has the same address and length but the old clip; the second has the bytes
    // the buffer holds now. An address-only lookup would play the first one under the new naming.
    rt.audio.clear();
    rt.audio.push(AudioSource {
        input: g::SDT_BUFFER,
        data,
        size,
        name: "old".to_string(),
        bytes: b"old clip".to_vec(),
        problem: None,
    });
    rt.audio.push(AudioSource {
        input: g::SDT_BUFFER,
        data,
        size,
        name: "current".to_string(),
        bytes: current,
        problem: None,
    });
    assert_eq!(
        source_of(&rt, &player),
        Some(1),
        "the clip whose bytes are in the buffer now wins over the stale record at the same address"
    );
}

#[test]
fn a_play_after_the_pass_ended_starts_the_same_source_again() {
    let (mut rt, object) = run_with_one_clip();
    let mut playing = Playing::new();
    rt.players.get_mut(&object).unwrap().playing = true;

    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "the first pass starts"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "a repeat inside the same pass does not restart the clip"
    );

    // The delivered body clears `m_bPlaying` before it delivers `PLAY_CB DONE`; the pack may
    // re-play the same source from that callback. The generation bump is the host-side edge that
    // makes that re-play a new pass rather than news of one already playing.
    rt.players.get_mut(&object).unwrap().play_gen += 1;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "after DONE/Stop the same source is a new pass"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "and repeats inside that new pass are still suppressed"
    );
}

#[test]
fn a_player_that_named_nothing_the_run_recorded_asks_for_nothing() {
    let (mut rt, object) = run_with_one_clip();
    // The same player, pointed somewhere the run never recorded. A handset would have nothing to
    // play here either, and asking for a clip that does not exist is worse than silence.
    rt.players.get_mut(&object).unwrap().data = 0xdead_beef;
    rt.players.get_mut(&object).unwrap().playing = true;
    let mut playing = Playing::new();
    assert!(requests(&rt, &mut playing).is_empty());
    assert!(
        playing.voices.contains_key(&object),
        "and the voice is still marked as answered"
    );
}

/// Releasing a player is a stop, once, and then nothing more.
///
/// `Runtime::release` deletes the player's own record, which is the right thing for a file handle or a
/// bitmap and the wrong thing for a sound: the mixer holds the voice, not the runtime, so a deleted
/// record left the clip sounding to its end while the pack moved on to the next object and started the
/// next tune there. That is the two-musics-at-once shape, and it is invisible to the loop that watches
/// `rt.players` — the entry it would have to compare is gone.
#[test]
fn releasing_a_playing_object_silences_its_voice_once() {
    let (mut rt, object) = run_with_one_clip();
    rt.players.get_mut(&object).unwrap().playing = true;
    let mut playing = Playing::new();
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "the pack started it"
    );

    // What `IBase::Release` leaves behind: no record, a voice still on the front end's ledger.
    rt.players.remove(&object);
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Stop { voice: object }],
        "so the next poll owes the silence"
    );
    assert!(
        !playing.voices.contains_key(&object),
        "and it forgets the voice, rather than stopping it again every frame"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "a second poll says nothing"
    );
}

/// The address comes back, and the tune comes with it.
///
/// Shell objects are recycled, so the same address reappearing with the *same* clip is the case the
/// ledger would otherwise read as "already playing this very clip, leave it alone" — and having
/// silenced the voice on release, that reading is now a silent game rather than an overlapping one.
/// Both outcomes come from the same forgotten byte, so the forgetting is what this pins.
#[test]
fn a_recycled_object_address_asks_for_its_clip_again() {
    let (mut rt, object) = run_with_one_clip();
    rt.players.get_mut(&object).unwrap().playing = true;
    let named = rt.players[&object];
    let mut playing = Playing::new();
    requests(&rt, &mut playing);
    rt.players.remove(&object);
    requests(&rt, &mut playing);

    // The shell hands the same address out again, and the new owner names the same clip.
    let mut fresh = named;
    fresh.playing = true;
    rt.players.insert(object, fresh);
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
        "the new owner of that address is a new play, not an already-handled voice"
    );
}

/// Two players that start in the same frame are two voices, not one replacing the other, and
/// each request names the object it came from — so the mixer can keep them apart, and a pack
/// that stops its music stops only that object.
///
/// With one sequencer and a request that carried only a clip index, which is what this file had
/// first, the two collapsed into whichever the `HashMap` happened to reach last.
#[test]
fn two_players_playing_at_once_are_two_voices_and_not_a_replacement() {
    let (mut rt, object) = run_with_one_clip();
    // A second player, on its own object, pointed at the same clip: a pack that wants its music
    // and an effect at the same moment hands them to two objects.
    let second = 0x0400_0058;
    let first = rt.players[&object].clone();
    rt.players.insert(second, first);
    rt.players.get_mut(&object).unwrap().playing = true;
    rt.players.get_mut(&second).unwrap().playing = true;

    let mut playing = Playing::new();
    let wanted = requests(&rt, &mut playing);
    assert_eq!(wanted.len(), 2, "both of them started: {wanted:?}");
    assert!(
        wanted.contains(&Request::Play {
            voice: object,
            index: 0
        }),
        "{wanted:?}"
    );
    assert!(
        wanted.contains(&Request::Play {
            voice: second,
            index: 0
        }),
        "{wanted:?}"
    );
    // And one of them stopping leaves the other alone, which is what stopping the music while an
    // effect is still playing has to do.
    rt.players.get_mut(&second).unwrap().playing = false;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Stop { voice: second }],
        "only the object that stopped"
    );
}

/// The bug that made music come back as an effect, in the form that survived giving every player
/// its own voice.
///
/// A pack brings its music back by re-pointing the object it already has: `Stop`, `SetInfo` with
/// the other file, `Play`. When the stop and the play land in one turn, `playing` is `true`
/// before and after, so an edge detector watching that flag sees nothing and the toggle effect
/// started in between simply keeps playing. Comparing what the voice is *playing* catches it.
#[test]
fn re_pointing_a_playing_player_at_another_clip_is_a_play() {
    let (mut rt, object) = run_with_one_clip();
    let (other, other_len) = add_second_clip(&mut rt);

    let mut playing = Playing::new();
    rt.players.get_mut(&object).unwrap().playing = true;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 0
        }],
    );
    // Still playing, still the same clip: not a play. This is the frame-after-frame case.
    assert!(requests(&rt, &mut playing).is_empty());

    // The same object moved to the other clip, with `playing` never going back to `false` from
    // here — which is exactly what `Stop` + `SetInfo` + `Play` inside one turn amounts to.
    {
        let player = rt.players.get_mut(&object).unwrap();
        player.data = other;
        player.size = other_len;
    }
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: object,
            index: 1
        }],
        "the clip changed, so this is a play and not a repetition"
    );
    assert!(requests(&rt, &mut playing).is_empty(), "and only once");
}

/// The bug this file has already had once. A speaker is built before the guest runs, so it holds
/// no clips at all; a play that arrives later has to parse its clip on the spot. The first shape
/// of this file snapshotted the source list instead, which was empty, so every play returned
/// `false` and the run made no sound and said nothing about why.
#[test]
#[cfg(feature = "device")]
fn a_clip_that_appears_after_the_speaker_is_built_is_still_played() {
    let Some(font) = soundfont() else {
        eprintln!("skipped: set BREW_SOUNDFONT to a .sf2 to build a speaker");
        return;
    };
    // Built against a run that has named nothing yet, which is the state a window opens in.
    let mut speaker = Speaker::new(Some(&font)).expect("a device to play through");
    assert_eq!(speaker.requested(), 0);
    // Now the run names something, the way a guest does once it is running.
    let (mut rt, object) = run_with_one_clip();
    rt.players.get_mut(&object).unwrap().playing = true;
    let mut playing = Playing::new();
    speaker.follow(&rt, &mut playing);
    assert_eq!(
        speaker.requested(),
        1,
        "the clip that did not exist when the speaker was built"
    );
    assert!(speaker.notes().is_empty(), "{:?}", speaker.notes());
    // And the next frame does not play it again, whichever way round it was parsed.
    speaker.follow(&rt, &mut playing);
    assert_eq!(speaker.requested(), 1);
}

/// Two voices through the real device, which is the part the requests test cannot reach: a second
/// play must not silence the first. Only the count is asserted, because what comes out of a sound
/// card is not something a test can hear.
#[test]
#[cfg(feature = "device")]
fn a_second_voice_does_not_silence_the_first() {
    let Some(font) = soundfont() else {
        eprintln!("skipped: set BREW_SOUNDFONT to a .sf2 to build a speaker");
        return;
    };
    let mut speaker = Speaker::new(Some(&font)).expect("a device to play through");
    let (mut rt, object) = run_with_one_clip();
    let second = 0x0400_0058;
    let first = rt.players[&object].clone();
    rt.players.insert(second, first);
    let mut playing = Playing::new();

    rt.players.get_mut(&object).unwrap().playing = true;
    speaker.follow(&rt, &mut playing);
    rt.players.get_mut(&second).unwrap().playing = true;
    speaker.follow(&rt, &mut playing);
    assert_eq!(speaker.requested(), 2, "both went to the device");

    // Stopping the second one is a stop on that voice, not on the mixer.
    rt.players.get_mut(&second).unwrap().playing = false;
    speaker.follow(&rt, &mut playing);
    assert_eq!(speaker.requested(), 2, "and the first was not restarted");
}

/// A Layer III frame header: the smallest thing that says "MPEG audio" without a parser behind it.
///
/// `FF FB` is the 11 sync bits with MPEG 1 and Layer III, and `90` is bitrate index 9 (128 k) over a
/// sample-rate index of 0 (44.1 kHz). The fourth byte carries channel mode and padding, which the
/// header test does not read.
fn layer_iii() -> Vec<u8> {
    vec![0xFF, 0xFB, 0x90, 0x00, 0x00, 0x00]
}

#[test]
fn an_mp3_is_recognised_and_named() {
    let mp3 = layer_iii();
    assert!(mpeg_audio(&mp3), "the frame header says what it is");
    assert_eq!(
        recognises(&mp3),
        Some(MIME_MP3),
        "...and the authority names it"
    );
    assert_eq!(
        sniff(&mp3),
        Some(MIME_MP3),
        "Symphonia now gives the sniff a decoder to back the name"
    );
    assert_eq!(
        clip_length_ms(&mp3),
        None,
        "a six-byte header alone still has no length to arm a status callback with"
    );
    // One authority, with the narrow question a subset of the wide one.
    assert_eq!(recognises(&one_note_midi()), Some(MIME_MIDI));
    assert!(
        !mpeg_audio(&one_note_midi()),
        "and a score is not mistaken for the other family"
    );
}

#[test]
fn four_bytes_of_ones_are_not_an_mp3_however_you_read_them() {
    // `FF FF FF FF` passes any two-byte prefix test: the sync bits are set, and so are version and
    // layer. What refuses it is the bitrate field, whose `1111` is defined as "bad". That is the whole
    // reason this reads the fields instead of matching a prefix — padding, a run of ones, or a
    // truncated plane would otherwise be counted as audio, in exactly the direction this census
    // exists to avoid.
    assert!(!mpeg_audio(&[0xFF, 0xFF, 0xFF, 0xFF]));
    // Each of the other three reserved values on its own, with the rest of the header valid.
    assert!(
        !mpeg_audio(&[0xFF, 0xF1, 0x90, 0x00]),
        "layer 00 is reserved"
    );
    assert!(
        !mpeg_audio(&[0xFF, 0xEE, 0x90, 0x00]),
        "version 01 is 'unspecified'"
    );
    assert!(
        !mpeg_audio(&[0xFF, 0xFB, 0x9E, 0x00]),
        "sample-rate index 11 is reserved"
    );
    assert!(mpeg_audio(&layer_iii()), "the control: this one is a frame");
}

#[test]
fn an_id3_tag_is_audio_only_when_a_frame_follows_it() {
    // "ID3", version 2.4, flags 0, and a synchsafe size of 0, so the audio starts where the tag ends.
    let mut tagged = vec![b'I', b'D', b'3', 4, 0, 0, 0, 0, 0, 0];
    tagged.extend_from_slice(&layer_iii());
    assert_eq!(recognises(&tagged), Some(MIME_MP3));
    assert_eq!(
        recognises(&tagged[..10]),
        None,
        "a tag with nothing behind it is a tag, not a song"
    );
    // A tag declaring 64 bytes of its own and running out at 16: the frame it promises is not there to
    // be asked, and reading past the end is not what decides that.
    let mut lying = tagged;
    lying[9] = 64;
    assert_eq!(
        recognises(&lying[..16]),
        None,
        "the size is the tag's claim"
    );
}

/// The WAV wrapper, checked against the bytes it claims to be.
///
/// A RIFF header is four sizes and four shorts of arithmetic, and every one of them is a claim about
/// the body: get `data` wrong and a player reads silence or runs off the end, get the byte rate wrong
/// and it plays at the wrong speed. So the test parses the file back with the same arithmetic the
/// writer used, and then proves there is a note in it — the second half is what keeps a header bug and
/// a silent render from both being called "it wrote a file".
#[test]
fn a_wav_describes_the_bytes_it_carries() {
    let (bytes, frames) = render_wav(None, &one_note_midi(), 44_100, 44_100).expect("a render");
    assert_eq!(frames, 44_100, "one second of audio asked for");
    assert_eq!(bytes.len(), 44 + frames * 4, "and nothing else in the file");
    let u16le =
        |r: std::ops::Range<usize>| u16::from_le_bytes(bytes[r].try_into().unwrap()) as usize;
    let u32le =
        |r: std::ops::Range<usize>| u32::from_le_bytes(bytes[r].try_into().unwrap()) as usize;
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..16], b"WAVEfmt ");
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32le(4..8),
        bytes.len() - 8,
        "RIFF size is the file minus its own header"
    );
    assert_eq!(
        u32le(16..20),
        16,
        "the fmt chunk is the 16 bytes this writer emits"
    );
    assert_eq!(u16le(20..22), 1, "PCM, not floating point");
    assert_eq!(u16le(22..24), 2, "stereo: the mixer renders two channels");
    assert_eq!(u32le(24..28), 44_100, "at the rate it was asked for");
    assert_eq!(
        u32le(28..32),
        44_100 * 4,
        "bytes per second: rate times frame size"
    );
    assert_eq!(u16le(32..34), 4, "two channels of two bytes each");
    assert_eq!(u16le(34..36), 16, "signed 16-bit");
    assert_eq!(
        u32le(40..44),
        frames * 4,
        "and `data` is exactly the samples"
    );
    let peak = bytes[44..]
        .chunks(2)
        .map(|b| i16::from_le_bytes(b.try_into().unwrap()).abs() as i32)
        .max()
        .unwrap_or(0);
    assert!(
        peak > 1_000,
        "middle C is audible in there, not silent: peak {peak}"
    );
}

#[test]
fn the_pcm_conversion_saturates_rather_than_wrapping() {
    // Casting an out-of-range float to `i16` wraps to the OPPOSITE sign, which in a waveform is a
    // click and in a file is a sample that denies what the mix did. `render_into` clamps first, so
    // this is the second guard on the same rail - and the one a caller outside the mixer cannot skip.
    assert_eq!(to_pcm16(0.0), 0);
    assert_eq!(to_pcm16(1.0), 32_767);
    assert_eq!(to_pcm16(2.0), 32_767, "past full scale stays at full scale");
    assert_eq!(to_pcm16(-1.0), -32_767);
    assert_eq!(to_pcm16(-2.0), -32_767);
}

/// A format-0 chord: `notes` all on at velocity 127, all off a quarter note later.
///
/// Needed because one note of the built-in bank peaks at 0.11, and eight of those never reach full
/// scale - so a test that asserted "nothing past the rail" on single notes would pass with the clamp
/// deleted. A chord is how a pack actually gets loud.
fn chord_midi(notes: &[u8]) -> Vec<u8> {
    let mut events: Vec<u8> = Vec::new();
    for n in notes {
        events.extend_from_slice(&[0x00, 0x90, *n, 127]);
    }
    let mut first = true;
    for n in notes {
        if first {
            events.extend_from_slice(&[0x83, 0x60]); // delta 480, written once
            first = false;
        } else {
            events.push(0x00);
        }
        events.extend_from_slice(&[0x80, *n, 0]);
    }
    events.extend_from_slice(&[0x00, 0xff, 0x2f, 0x00]);
    let mut file = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&1u16.to_be_bytes());
    file.extend_from_slice(&480u16.to_be_bytes());
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&(events.len() as u32).to_be_bytes());
    file.extend_from_slice(&events);
    file
}

#[test]
fn the_mixer_sums_every_slot_it_is_given_and_clamps_what_it_hands_back() {
    // The device callback and the offline path are one mixer now, so both properties have to hold on
    // the offline one: slots add (a pack that names a clip on two `ISoundPlayer`s hears two voices, not
    // a replacement), and the sum is clamped where BOTH drivers can see it. The clamp used to live in
    // the callback's loop, which left the offline path writing unclamped samples - a second authority
    // on "how loud is this clip", which is what this struct exists to close.
    let (font, _) = load(None).expect("the built-in bank loads");
    let notes = [60u8, 64, 67, 72, 55, 59, 62, 65];
    let midi = Arc::new(Clip::parse(&chord_midi(&notes)).expect("parses"));
    let frames = 4_410;
    let peak = |v: &[f32]| v.iter().fold(0.0f32, |a, x| a.max(x.abs()));

    let mut one = Mixer::offline(44_100, &font).expect("one mixer");
    one.assign(1, &midi);
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    one.drain();
    one.render_into(&mut left, &mut right);
    let single = peak(&left);
    assert!(single > 0.0, "the chord is audible on one voice");
    // The precondition that makes the clamp assertion below mean something: if eight copies of this
    // still summed under full scale, `all <= 1.0` would prove nothing and the clamp could be deleted
    // without going red. Inert assertions are the failure this repository tracks by name.
    assert!(
        single * VOICES as f32 > 1.0,
        "the fixture has to be loud enough that {VOICES} voices reach the rail: peak {single}"
    );

    let (tx, rx) = mpsc::channel();
    let mut many = Mixer::live(rx, 44_100, &font).expect("a mixer a callback could drive");
    for voice in 1..=(VOICES as u32) {
        tx.send(Command::Play {
            voice,
            clip: midi.clone(),
        })
        .expect("the receiver is alive");
    }
    let mut mixed = vec![0.0f32; frames];
    let mut quiet = vec![0.0f32; frames];
    many.drain();
    many.render_into(&mut mixed, &mut quiet);
    assert_eq!(
        many.voices.iter().filter(|v| v.id != 0).count(),
        VOICES,
        "{VOICES} `ISoundPlayer`s are {VOICES} slots, not one slot overwritten {VOICES} times"
    );
    assert!(peak(&mixed) > single, "and the mix is louder");
    assert!(
        mixed.iter().all(|x| *x >= -1.0 && *x <= 1.0),
        "with the voices full the sum passes full scale, and nothing leaves the rail"
    );
}

/// `render_into` zeroes the destination before summing into it, and that is what lets a driver reuse
/// one buffer forever instead of allocating inside the callback.
///
/// Without this case the zeroing is untested in the only place it can be seen: the live driver hands
/// it freshly-zeroed buffers of its own, so deleting the `fill(0.0)` changes nothing a current test
/// observes. Here the destination arrives pre-filled with a value no render can produce, and the
/// result has to be the same render into a clean destination - which is exactly what a leftover from
/// the previous buffer would break.
#[test]
fn render_into_overwrites_the_destination_rather_than_adding_to_it() {
    let (font, _) = load(None).expect("the built-in bank loads");
    let notes = [60u8, 64, 67, 72];
    let midi = Arc::new(Clip::parse(&chord_midi(&notes)).expect("parses"));
    let frames = 3_000;

    let mut clean = vec![0.0f32; frames];
    let mut clean_r = vec![0.0f32; frames];
    let mut m = Mixer::offline(44_100, &font).expect("an offline mixer");
    m.assign(1, &midi);
    m.render_into(&mut clean, &mut clean_r);

    // The same mixer, the same clip, a destination that arrives dirty.
    let mut dirty = vec![7.5f32; frames];
    let mut dirty_r = vec![-7.5f32; frames];
    let mut m2 = Mixer::offline(44_100, &font).expect("a second mixer");
    m2.assign(1, &midi);
    m2.render_into(&mut dirty, &mut dirty_r);

    assert!(
        !dirty.iter().any(|x| *x == 7.5) && !dirty_r.iter().any(|x| *x == -7.5),
        "no sample survived from the caller's previous buffer"
    );
    assert_eq!(
        dirty, clean,
        "and the render is identical to the one into a clean destination, sample for sample"
    );
    assert!(peak_of(&clean) > 0.0, "the control: not silence");
}

fn peak_of(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |a, x| a.max(x.abs()))
}

/// The path a front end with its own frame clock needs: a speaker with no device behind it, rendered
/// on demand.
///
/// Two things have to be true for that to be worth having. Nothing may touch a sound card — this
/// machine may not have one, and the person picking the front end has no say in that. And the
/// rendering has to be the *same* chain the device path uses, which is what makes a clip sound the
/// same either way: silence until the run asks for something, and sound afterwards.
#[test]
fn a_speaker_with_no_device_renders_when_asked_and_is_silent_until_then() {
    let mut speaker = match Speaker::without_device(44_100, None) {
        Ok(speaker) => speaker,
        // No bank to build is the same answer `--dump-wav` gives on such a host, and it is not what
        // this test is about.
        Err(e) => {
            eprintln!("skipped: no bank on this host ({e})");
            return;
        }
    };
    let (mut rt, object) = run_with_one_clip();
    let mut playing = Playing::new();
    let mut left = vec![0.0f32; 64];
    let mut right = vec![0.0f32; 64];

    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    assert_eq!(peak_of(&left), 0.0, "nothing has been asked to play yet");
    let pcm = Speaker::interleave(&left, &right);
    assert_eq!(pcm.len(), 128, "one i16 per side per frame, interleaved");
    assert!(pcm.iter().all(|s| *s == 0), "silence converts to silence");

    // And now the run asks for it. The same call has to reach the same mixer the device path reaches.
    rt.players.get_mut(&object).unwrap().playing = true;
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    assert!(
        peak_of(&left) > 0.0 || peak_of(&right) > 0.0,
        "the clip was asked for, so the mixer has something to render"
    );
}

/// Cutting the mixer on a front end's command: every voice and tone goes, and so does the memory of
/// what was told, so the next frame derives from the run rather than diffing against a dead one.
#[test]
fn stopping_everything_leaves_the_mixer_with_nothing_to_render() {
    let mut speaker = match Speaker::without_device(44_100, None) {
        Ok(speaker) => speaker,
        Err(e) => {
            eprintln!("skipped: no bank on this host ({e})");
            return;
        }
    };
    let (mut rt, object) = run_with_one_clip();
    let mut playing = Playing::new();
    let mut left = vec![0.0f32; 64];
    let mut right = vec![0.0f32; 64];

    rt.players.get_mut(&object).unwrap().playing = true;
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    assert!(
        peak_of(&left) > 0.0 || peak_of(&right) > 0.0,
        "the clip was asked for, so there is something to cut"
    );

    // The run stops naming it — that is what a front end resetting or loading another pack means for
    // this voice — and the front end cuts what the mixer still holds.
    rt.players.get_mut(&object).unwrap().playing = false;
    speaker.stop_all(&mut playing);
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    assert_eq!(
        (peak_of(&left), peak_of(&right)),
        (0.0, 0.0),
        "the mixer was cut and the run asks for nothing, so the turn is silent"
    );
}

/// A tempo change asked for through one object is one shared request, and repeating the value is
/// not a change.
///
/// `ISoundPlayer::SetTempo` and `MM_PARM_TEMPO` both name a percentage of normal speed, and on this
/// host that factor belongs to the synthesizer: changing it once moves every MIDI stream in flight.
/// Both calls write one shared value, and the front end only has to notice when it moves.
#[test]
fn one_players_tempo_change_is_one_shared_request_and_a_repeat_is_not_a_change() {
    let (mut rt, object) = run_with_one_clip();
    let second = 0x0400_0058;
    let first = rt.players[&object];
    rt.players.insert(second, first);
    rt.players.get_mut(&object).unwrap().playing = true;
    rt.players.get_mut(&second).unwrap().playing = true;

    let mut playing = Playing::new();
    assert_eq!(requests(&rt, &mut playing).len(), 2, "both voices started");
    // Both sit at the header's default, which is where the speaker already is: saying so every
    // frame would be noise, and a player handed out later must not cause it either.
    assert!(requests(&rt, &mut playing).is_empty());

    rt.media_tempo = 50;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Tempo(50)],
        "one change, one request, and it names no voice because it moves all of them"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "and not once per frame"
    );

    // Repeating the value already in force is not a second change...
    rt.media_tempo = 50;
    assert!(requests(&rt, &mut playing).is_empty());
    // ...while a different one is.
    rt.media_tempo = 200;
    assert_eq!(requests(&rt, &mut playing), vec![Request::Tempo(200)]);
}

/// Two voices already playing when the tempo changes both follow it, and a voice started afterwards
/// inherits the same factor.
///
/// This is the property the per-player field cannot show on its own: the mixer holds one factor and
/// applies it to every slot, so the second player's sound changes without that player being told
/// anything. The sequencer's `play` keeps the speed it was given, which is why the late voice's
/// speed is asserted too.
#[test]
fn a_tempo_change_reaches_every_voice_that_is_already_playing() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let (mut rt, object) = run_with_one_clip();
    let second = 0x0400_0058;
    let first = rt.players[&object];
    rt.players.insert(second, first);
    rt.players.get_mut(&object).unwrap().playing = true;
    rt.players.get_mut(&second).unwrap().playing = true;

    let mut playing = Playing::new();
    let mut left = vec![0.0f32; 64];
    let mut right = vec![0.0f32; 64];
    // The first mix sends both plays through the channel; the mixer assigns them on drain.
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);

    rt.media_tempo = 50;

    // A voice that starts after the change inherits it, and it is started in a second pass so that
    // no `Tempo` command travels with its `Play`: the sequencer's `play` does not reset the
    // speed, and that is the property this half pins down.
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    let third = 0x0400_0060;
    let mut late = Player::default();
    late.input = first.input;
    late.data = first.data;
    late.size = first.size;
    late.playing = true;
    rt.players.insert(third, late);

    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);

    let mixer = match &mut speaker.driver {
        Driver::Pull(mixer) => mixer,
        #[cfg(feature = "device")]
        Driver::Device(_) => panic!("without_device is the pull driver"),
    };
    let speeds: Vec<f64> = mixer
        .voices
        .iter()
        .filter(|voice| voice.id != 0)
        .map(|voice| voice.sequencer.get_speed())
        .collect();
    assert_eq!(
        speeds,
        vec![0.5, 0.5, 0.5],
        "every voice, including the one that started later"
    );
}

#[test]
fn a_reverse_window_is_sent_to_the_speaker_once() {
    let (mut rt, object) = run_with_one_clip();
    let player = rt.players.get_mut(&object).unwrap();
    player.playing = true;
    player.play_start = 6_000;
    player.play_stop = Some(2_000);
    player.pass_ms = 10_000;

    let mut playing = Playing::new();
    let asked = requests(&rt, &mut playing);
    assert!(
        asked.iter().any(|request| matches!(
            request,
            Request::Window {
                voice,
                start: 6_000,
                stop: 2_000,
                reverse: true,
            } if *voice == object
        )),
        "the reverse window reaches the mixer: {asked:?}"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "an unchanged window is not sent again"
    );
}

#[test]
fn a_midi_message_is_sent_once() {
    let mut rt = Runtime::new();
    let object = 0x0400_0050;
    rt.players.insert(object, Player::default());
    rt.midi_out.insert(object, vec![vec![0x90, 60, 100]]);

    let mut playing = Playing::new();
    let asked = requests(&rt, &mut playing);
    assert!(
        asked.iter().any(|request| matches!(
            request,
            Request::Midi { voice, bytes }
                if *voice == object && bytes == &[0x90, 60, 100]
        )),
        "the queued packet is handed to the speaker once: {asked:?}"
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "a packet already sent is not sent again"
    );
}

#[test]
fn a_player_rate_multiplies_the_shared_tempo() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let (mut rt, object) = run_with_one_clip();
    let player = rt.players.get_mut(&object).unwrap();
    player.playing = true;
    player.rate_num = 2;
    player.rate_den = 1;

    let mut playing = Playing::new();
    let mut left = vec![0.0f32; 64];
    let mut right = vec![0.0f32; 64];
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    let speed = match &mut speaker.driver {
        Driver::Pull(mixer) => mixer
            .voices
            .iter()
            .find(|voice| voice.id != 0)
            .expect("the clip was assigned")
            .sequencer
            .get_speed(),
        #[cfg(feature = "device")]
        Driver::Device(_) => panic!("without_device is the pull driver"),
    };
    assert_eq!(speed, 2.0, "2/1 is twice normal speed at 100 % tempo");

    rt.media_tempo = 200;
    speaker.mix(&mut rt, &mut playing, &mut left, &mut right);
    let speed = match &mut speaker.driver {
        Driver::Pull(mixer) => mixer
            .voices
            .iter()
            .find(|voice| voice.id != 0)
            .expect("the clip is still assigned")
            .sequencer
            .get_speed(),
        #[cfg(feature = "device")]
        Driver::Device(_) => panic!("without_device is the pull driver"),
    };
    assert_eq!(speed, 4.0, "the per-voice rate multiplies the shared tempo");
}

/// A player handed out after the tempo changed must not drag the shared value back to its own
/// default.
///
/// Both ABI doors write the one shared value, while a fresh player starts with no tempo setting of
/// its own — so there is no per-object default that could look like a command. Treating one as a
/// command would undo the tempo of every stream already playing.
#[test]
fn a_player_handed_out_after_a_tempo_change_does_not_reset_it() {
    let (mut rt, object) = run_with_one_clip();
    rt.players.get_mut(&object).unwrap().playing = true;
    let mut playing = Playing::new();
    assert_eq!(
        requests(&rt, &mut playing).len(),
        1,
        "the first voice starts"
    );

    rt.media_tempo = 50;
    assert_eq!(requests(&rt, &mut playing), vec![Request::Tempo(50)]);

    // A second object is created and pointed at the same clip. Its own field is the default, which
    // is the state every freshly handed-out player is in.
    let first = rt.players[&object];
    let second = 0x0400_0058;
    rt.players.insert(
        second,
        Player {
            input: first.input,
            data: first.data,
            size: first.size,
            playing: true,
            ..Player::default()
        },
    );
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Play {
            voice: second,
            index: 0
        }],
        "the new player's default is not a tempo change"
    );
}

#[test]
fn a_smaf_source_renders_blocks_and_resumes_from_a_seek() {
    let Ok(path) = std::env::var("QUOIN_SMAF_SAMPLE") else {
        eprintln!("skipped: QUOIN_SMAF_SAMPLE is not set");
        return;
    };
    let bytes = std::fs::read(&path).expect("read the sample named by QUOIN_SMAF_SAMPLE");
    let score = crate::audio::smaf::sequencer::Score::parse(&bytes).expect("the sample parses");
    let player = crate::audio::smaf::player::Player::new(score, 44_100);
    let mut source = PcmSource::smaf(player);

    let mut peak = 0.0f32;
    for _ in 0..44_100 * 5 {
        peak = peak.max(source.next(1.0).abs());
    }
    assert!(peak > 0.001, "streaming source was silent: peak={peak}");

    source.set_window(10_000, 12_000, false);
    let mut resumed = 0.0f32;
    for _ in 0..44_100 {
        resumed = resumed.max(source.next(1.0).abs());
    }
    assert!(resumed > 0.001, "seeked source was silent: peak={resumed}");
}

/// `MM_PARM_POS` takes its direction from the two positions, and for a decoded clip that is the
/// same samples read the other way round. Both halves matter: a reverse window whose start is past
/// its stop used to be treated as an empty forward range and went silent after one block, and a
/// forward window's stop was never applied at all.
#[test]
fn a_pcm_window_obeys_its_stop_and_plays_backwards_when_its_start_is_past_it() {
    let ramp: Arc<Vec<f32>> = Arc::new((0..10).map(|i| i as f32).collect());
    let source = || PcmSource::Buffer {
        samples: ramp.clone(),
        at: 0.0,
        rate: 1_000,
        stop: -1.0,
        reverse: false,
    };

    // One millisecond is one sample at 1 kHz, so the window reads as sample indices.
    let mut forward = source();
    forward.set_window(2, 5, false);
    let got: Vec<f32> = (0..5).map(|_| forward.next(1.0)).collect();
    assert_eq!(got, vec![2.0, 3.0, 4.0, 0.0, 0.0], "forward window [2, 5)");

    let mut reverse = source();
    reverse.set_window(8, 2, true);
    let got: Vec<f32> = (0..8).map(|_| reverse.next(1.0)).collect();
    assert_eq!(
        got,
        vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 0.0],
        "reverse window [8 down to 2)"
    );

    // The control: with no window the clip still plays whole, so the two assertions above are
    // about the window and not about a buffer that stops early for some other reason.
    let mut whole = source();
    let got: Vec<f32> = (0..10).map(|_| whole.next(1.0)).collect();
    assert_eq!(got, (0..10).map(|i| i as f32).collect::<Vec<_>>());
}

/// A note played on the built-in bank, so the window tests need no sample set file.
fn tone_score() -> crate::audio::smaf::sequencer::Score {
    use crate::audio::smaf::sequencer::{Event, Kind, Score};
    Score {
        events: vec![
            Event {
                time_ms: 0,
                kind: Kind::NoteOn,
                channel: 0,
                a: 60,
                b: 100,
            },
            Event {
                time_ms: 400,
                kind: Kind::NoteOff,
                channel: 0,
                a: 60,
                b: 0,
            },
        ],
        voices: Vec::new(),
        waves: Vec::new(),
        rhythm: [false; 128],
        length_ms: 1_400,
    }
}

/// A reverse window over a score is the forward window mirrored, sample for sample.
///
/// The score is rendered forward from the window's low end and the result is reversed, so this
/// compares against exactly that: one player rendering the window forward, one window built from
/// the other end. If the materialisation ever drifts (a chunk boundary that resets a voice, a
/// stop that lands a sample late) the difference shows up here rather than in someone's ear.
#[test]
fn a_reverse_smaf_window_is_the_forward_window_mirrored() {
    let score = tone_score();
    let frames = (450 * 44_100) / 1000;
    let mut reference = SmafPlayer::new(score.clone(), 44_100);
    let mut stereo = vec![0.0f32; frames * 2];
    assert_eq!(reference.render(&mut stereo), frames, "the window renders");
    let mut want: Vec<f32> = (0..frames)
        .map(|i| (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5)
        .collect();
    want.reverse();
    assert!(
        want.iter().fold(0.0f32, |p, s| p.max(s.abs())) > 0.001,
        "the window is silent, so mirroring it would prove nothing"
    );

    let mut source = smaf_window_source(&Arc::new(score), 450, 0, true);
    let got: Vec<f32> = (0..frames).map(|_| source.next(1.0)).collect();
    for (i, (a, b)) in want.iter().zip(got.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1e-6,
            "sample {i}: forward window {a}, reversed window {b}"
        );
    }
    assert_eq!(source.next(1.0), 0.0, "and it stops at the window's end");
}

/// The window is applied before the mixer ever sees the source, which is what keeps a long replay
/// off the audio callback.
#[test]
fn a_smaf_window_is_ready_before_the_mixer_sees_it() {
    let score = tone_score();
    let mut reference = SmafPlayer::new(score.clone(), 44_100);
    reference.seek_ms(200);
    let mut out = vec![0.0f32; 2 * 1_024];
    assert!(reference.render(&mut out) > 0);
    let first = (out[0] + out[1]) * 0.5;
    assert!(first.abs() > 0.0, "the control: not silence");

    let mut source = smaf_window_source(&Arc::new(score), 200, 5_000, false);
    assert!(
        (source.next(1.0) - first).abs() < 1e-6,
        "the first sample is already the window's, so nothing is left to replay in the callback"
    );
}

/// The window reaches the mixer as a prepared source rather than as a command the callback has to
/// work through.
///
/// This is the end-to-end half of [`a_smaf_window_is_ready_before_the_mixer_sees_it`]: the run asks
/// for a window on a voice the speaker knows is SMAF, the frame thread builds the source, and the
/// mixer's first frame is already the window's. If the interception ever falls back to
/// `Command::Window` the mixer would try to replay on the audio thread — correct, but the thing this
/// path exists to avoid.
#[test]
fn a_smaf_window_reaches_the_mixer_as_a_prepared_source() {
    let score = Arc::new(tone_score());
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let mut rt = Runtime::new();
    let object = 0x0400_0050;
    let mut player = Player::default();
    player.input = g::SDT_BUFFER;
    player.data = 0x1000;
    player.size = 4;
    player.playing = true;
    player.pass_ms = 1_000;
    player.play_start = 200;
    rt.players.insert(object, player);
    speaker.smaf_voices.insert(object, score.clone());

    // The front end already knows this voice plays this clip, so the only request this frame is
    // the window.
    let mut playing = Playing::new();
    playing.voices.insert(object, (g::SDT_BUFFER, 0x1000, 4, 0));
    let asked = speaker.follow(&rt, &mut playing);
    assert!(
        asked
            .iter()
            .any(|r| matches!(r, Request::Window { voice, .. } if *voice == object)),
        "the run did ask for a window: {asked:?}"
    );

    let mut left = vec![0.0f32; 64];
    let mut right = vec![0.0f32; 64];
    match &mut speaker.driver {
        Driver::Pull(mixer) => {
            mixer.drain();
            mixer.render_into(&mut left, &mut right);
        }
        #[cfg(feature = "device")]
        Driver::Device(_) => unreachable!("without_device builds a Pull driver"),
    }

    let mut reference = SmafPlayer::new((*score).clone(), 44_100);
    reference.seek_ms(200);
    let mut want = vec![0.0f32; 2 * left.len()];
    assert!(reference.render(&mut want) > 0);
    for i in 0..left.len() {
        let expected = (want[i * 2] + want[i * 2 + 1]) * 0.5;
        assert!(
            (left[i] - expected).abs() < 1e-6,
            "frame {i}: the window starts at {expected}, the mixer played {}",
            left[i]
        );
    }
}

/// A window's stop is a point in media time, checked before a sample is emitted.
///
/// The stop used to be tested after a render block was produced, which let the last block play up
/// to 512 frames past the window and made a zero-length window play a whole block. `MM_PARM_POS`
/// can legitimately say `start == stop`, so "nothing" and "to the clip's end" must not share a
/// sentinel.
#[test]
fn a_smaf_window_stops_at_its_stop_and_a_zero_length_one_is_silent() {
    let score = Arc::new(tone_score());

    let mut zero = smaf_window_source(&score, 0, 0, false);
    for i in 0..1_024 {
        assert_eq!(
            zero.next(1.0),
            0.0,
            "a zero-length window has nothing to play ({i})"
        );
    }

    let mut short = smaf_window_source(&score, 0, 100, false);
    let frames = 100 * 44_100 / 1000;
    let mut audible = 0;
    for i in 0..frames + 1_024 {
        let sample = short.next(1.0);
        if i >= frames {
            assert_eq!(sample, 0.0, "past the stop at frame {i}");
        } else if sample != 0.0 {
            audible += 1;
        }
    }
    assert!(audible > 0, "the control: the window itself is not silence");
}

/// Replacing a playing source with the same voice's window keeps `MM_PARM_RATE`.
///
/// The speaker hands the windowed SMAF source to the slot the voice is already on, after the rate
/// command. The slot's rate is reset only when the slot changes hands, so this is the one property
/// that keeps a half-speed voice half-speed across a window.
#[test]
fn putting_a_window_on_a_smaf_voice_keeps_the_rate_it_was_given() {
    let score = Arc::new(tone_score());
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let voice = 0x0400_0050;
    let Driver::Pull(mixer) = &mut speaker.driver else {
        unreachable!("without_device builds a Pull driver")
    };
    mixer.assign_source(
        voice,
        PcmSource::smaf(SmafPlayer::new((*score).clone(), 44_100)),
    );
    mixer.set_rate(voice, 2, 1);
    mixer.assign_source(voice, smaf_window_source(&score, 200, 0, false));
    let slot = mixer
        .voices
        .iter()
        .find(|slot| slot.id == voice)
        .expect("the voice kept its slot");
    assert!(
        (slot.rate - 2.0).abs() < 1e-9,
        "the window reset the voice's rate to {}",
        slot.rate
    );
}

/// A play that replaces a voice's clip drops the old score binding, even when the new clip turns
/// out to be one this end cannot use.
///
/// A window that arrived afterwards must not be answered from the score the voice no longer plays.
#[test]
fn playing_another_clip_clears_the_smaf_binding_for_that_voice() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let voice = 0x0400_0050;
    speaker.smaf_voices.insert(voice, Arc::new(tone_score()));
    let rt = Runtime::new();
    assert!(!speaker.play(&rt, voice, 0), "there is no clip 0 to play");
    assert!(
        !speaker.smaf_voices.contains_key(&voice),
        "the old score is no longer what this voice plays"
    );
}

/// The global rate is tape-style: at 2× the same number of output samples covers twice as much source
/// time and pitch rises with it; at 1× the original path runs and the points are still the original
/// sequence — the evidence that the default path has not regressed.
#[test]
fn mixer_speed_resamples_every_voice_so_tempo_and_pitch_move_together() {
    let ramp = || Arc::new((0..256).map(|i| i as f32 / 256.0).collect::<Vec<f32>>());

    let mut mixer = pcm_mixer(1000, ramp());
    let (mut l1, mut r1) = (vec![0.0; 6], vec![0.0; 6]);
    mixer.render_into(&mut l1, &mut r1);
    assert!(
        (l1[1] - 1.0 / 256.0).abs() < 1.0e-5,
        "at 1x each output sample advances one source sample"
    );
    assert!((l1[2] - 2.0 / 256.0).abs() < 1.0e-5);

    let mut mixer = pcm_mixer(2000, ramp());
    let (mut l2, mut r2) = (vec![0.0; 6], vec![0.0; 6]);
    mixer.render_into(&mut l2, &mut r2);
    assert!(
        (l2[1] - 2.0 / 256.0).abs() < 1.0e-4,
        "at 2x each output sample advances two source samples: {l2:?}"
    );
    assert!((l2[2] - 4.0 / 256.0).abs() < 1.0e-4);
    assert_eq!(l2, r2, "mono PCM is identical on both sides");

    let mut mixer = pcm_mixer(500, ramp());
    let (mut l3, mut r3) = (vec![0.0; 4], vec![0.0; 4]);
    mixer.render_into(&mut l3, &mut r3);
    assert!(
        (l3[2] - 1.0 / 256.0).abs() < 1.0e-5,
        "at 0.5x one source sample advances only every two output samples: {l3:?}"
    );
}

/// Phase has to stay continuous across a block boundary: rendering the same source stretch twice in a
/// row matches rendering it in one go to within interpolation error, or every block boundary clicks.
#[test]
fn mixer_speed_keeps_the_phase_across_block_boundaries() {
    let ramp = || Arc::new((0..512).map(|i| i as f32 / 512.0).collect::<Vec<f32>>());

    let mut whole = pcm_mixer(2000, ramp());
    let (mut lw, mut rw) = (vec![0.0; 64], vec![0.0; 64]);
    whole.render_into(&mut lw, &mut rw);

    let mut split = pcm_mixer(2000, ramp());
    let mut ls = vec![0.0; 64];
    let mut rs = vec![0.0; 64];
    for half in 0..2 {
        let (mut l, mut r) = (vec![0.0; 32], vec![0.0; 32]);
        split.render_into(&mut l, &mut r);
        ls[half * 32..half * 32 + 32].copy_from_slice(&l);
        rs[half * 32..half * 32 + 32].copy_from_slice(&r);
    }
    for (i, (a, b)) in lw.iter().zip(ls.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1.0e-5,
            "sample {i} jumped between two renders: {a} vs {b}"
        );
    }
}

fn pcm_mixer(permille: u32, samples: Arc<Vec<f32>>) -> Mixer {
    let (tx, rx) = mpsc::channel();
    let (font, _) = load(None).expect("the tone bank this crate generates parses");
    let mut mixer = Mixer::live(rx, 44_100, &font).expect("a mixer with eight slots");
    tx.send(Command::Pcm {
        voice: 1,
        samples,
        rate: 44_100,
    })
    .expect("the receiver lives in the mixer");
    mixer.drain();
    mixer.set_speed(permille);
    mixer
}

/// A score's media window is [start, stop), and `MM_PARM_POS` may legitimately say `start == stop`.
///
/// The two readings of that are "nothing" and "everything", and the sequencer used to pick the
/// second: its end test required `window_stop > window_start`, so a zero-length window never ended,
/// `current_time` was pinned at the stop, and the events at that instant were delivered and never
/// taken off — a note struck and never released, ringing until the voice was stolen. The PCM sources
/// already had the right rule (`a_smaf_window_stops_at_its_stop_and_a_zero_length_one_is_silent`);
/// this is the same rule on the other kind of clip.
#[test]
fn a_zero_length_midi_window_is_silent_and_leaves_no_note_sounding() {
    let (font, _) = load(None).expect("the tone bank this crate generates parses");
    let mut sequencer = Sequencer::new(
        Synthesizer::new(&font, &SynthesizerSettings::new(44_100)).expect("a synthesizer"),
    );
    let clip = Arc::new(Clip::parse(&one_note_midi()).expect("the score parses"));
    sequencer.play(&clip);

    sequencer.set_window(0, 0, false);
    let mut left = vec![0.0f32; 4_410];
    let mut right = vec![0.0f32; 4_410];
    sequencer.render(&mut left, &mut right);
    assert!(
        left.iter().all(|sample| *sample == 0.0),
        "a window of no length has nothing in it"
    );

    // The control: the same clip with a window that has a length is audible, so the silence above
    // is the window and not a score nothing can play.
    sequencer.set_window(0, 100, false);
    let mut left = vec![0.0f32; 4_410];
    let mut right = vec![0.0f32; 4_410];
    sequencer.render(&mut left, &mut right);
    assert!(
        left.iter().any(|sample| sample.abs() > 0.01),
        "the note inside the window is heard"
    );
}

/// A tune moves the key a note is struck at, and moves nothing else.
///
/// The three messages whose `data1` is a key are the ones shifted; a program change names a patch
/// and percussion channel 10's `data1` names a drum, so shifting either would be a different
/// message rather than a higher one.
#[test]
fn a_tune_moves_keys_and_leaves_drums_and_other_messages_alone() {
    assert_eq!(
        shift_key(3, 0, 0x9, 60),
        Some(63),
        "note on, three semitones up"
    );
    assert_eq!(
        shift_key(3, 0, 0x8, 60),
        Some(63),
        "the note off has to move with its on, or the note hangs"
    );
    assert_eq!(
        shift_key(0xa, 0, 0xa, 60),
        Some(70),
        "aftertouch names a key too"
    );
    assert_eq!(
        shift_key(-12, 0, 0x9, 4),
        None,
        "a note pushed off the keyboard is dropped, not wrapped"
    );
    assert_eq!(
        shift_key(3, DRUM_CHANNEL, 0x9, 60),
        Some(60),
        "channel 10 is percussion, where the number is an instrument"
    );
    assert_eq!(
        shift_key(3, 0, 0xc, 12),
        Some(12),
        "program change is not a key"
    );
    assert_eq!(
        shift_key(0, 0, 0x9, 60),
        Some(60),
        "an untuned voice costs nothing"
    );
}

/// A player whose slot was taken has to be told its own level again.
///
/// The mixer keeps eight slots and a ninth voice takes the oldest, whose slot starts at the header's
/// default gain — inside the callback, where the front end cannot see it. The record of what was sent
/// used to survive that, so `requests` answered "already at 40" for a voice that was at 100 and the
/// pack's music came back at full scale. The next play for that voice is the one moment the record
/// can be dropped, and this is that moment.
#[test]
fn a_play_gives_a_voice_its_own_volume_back() {
    let (mut rt, object) = run_with_one_clip();
    {
        let player = rt.players.get_mut(&object).unwrap();
        player.playing = true;
        player.volume_set = true;
        player.volume = 40;
    }
    let mut playing = Playing::new();
    let first = requests(&rt, &mut playing);
    assert!(
        first.contains(&Request::Volume {
            voice: object,
            volume: 40
        }),
        "{first:?}"
    );
    assert!(
        !requests(&rt, &mut playing)
            .iter()
            .any(|request| matches!(request, Request::Volume { .. })),
        "the level is not re-sent every frame"
    );

    // A new pass: `DONE` and `Stop` bump `play_gen`, and a re-pointed or resumed object does too.
    rt.players.get_mut(&object).unwrap().play_gen += 1;
    let again = requests(&rt, &mut playing);
    assert!(
        again.contains(&Request::Volume {
            voice: object,
            volume: 40
        }),
        "the play asks for the level again: {again:?}"
    );
}

/// `SetSoundDevice`'s ear mute is a gain, and the header's `MUTECTL` values are 0 and 1.
///
/// It used to be a field nothing read, so a pack muting the earpiece went on playing at full scale.
#[test]
fn an_ear_muted_player_asks_for_silence_and_a_bare_player_gets_its_default_back() {
    let (mut rt, object) = run_with_one_clip();
    {
        let player = rt.players.get_mut(&object).unwrap();
        player.playing = true;
        player.ear_mute = g::AEE_SOUND_MUTECTL_MUTED;
    }
    let mut playing = Playing::new();
    let asked = requests(&rt, &mut playing);
    assert!(
        asked.contains(&Request::Volume {
            voice: object,
            volume: 0
        }),
        "a mute is a level of zero even with no `SetVolume` behind it: {asked:?}"
    );

    // Unmuting has to take the zero back. The player never set a level of its own, so what it goes
    // back to is the header's default and not "whatever the slot happens to hold".
    rt.players.get_mut(&object).unwrap().ear_mute = g::AEE_SOUND_MUTECTL_UNMUTED;
    let asked = requests(&rt, &mut playing);
    assert!(
        asked.contains(&Request::Volume {
            voice: object,
            volume: g::AEE_MAX_VOLUME
        }),
        "the default level comes back: {asked:?}"
    );
}

/// A tone is a request of its own, sent once, and taken back when it ends.
#[test]
fn a_tone_is_asked_for_once_and_stopped_when_it_ends() {
    let mut rt = Runtime::new();
    let object = 0x0400_0060;
    rt.tones.insert(
        object,
        crate::runtime::Tone {
            playing: true,
            freq_hi: 440,
            freq_lo: 0,
            volume: 60,
            ..Default::default()
        },
    );
    let mut playing = Playing::new();
    let asked = requests(&rt, &mut playing);
    assert!(
        asked.contains(&Request::Tone {
            object,
            hi: 440,
            lo: 0,
            volume: 60
        }),
        "{asked:?}"
    );
    assert!(
        !requests(&rt, &mut playing)
            .iter()
            .any(|request| matches!(request, Request::Tone { .. })),
        "a tone that is already sounding is not restarted"
    );

    // Its length ran out; `StopTone` is the same edge, because both clear `playing`.
    rt.tones.get_mut(&object).unwrap().playing = false;
    assert_eq!(
        requests(&rt, &mut playing),
        vec![Request::Stop { voice: object }]
    );
    assert!(
        requests(&rt, &mut playing).is_empty(),
        "and the silence is said once"
    );
}

/// An id whose sound belongs to the OEM's tone table is accepted and never reaches the mixer.
#[test]
fn a_tone_this_host_has_no_sound_for_is_never_asked_of_the_mixer() {
    let mut rt = Runtime::new();
    let object = 0x0400_0060;
    rt.tones.insert(
        object,
        crate::runtime::Tone {
            playing: true,
            freq_hi: 0,
            freq_lo: 0,
            ..Default::default()
        },
    );
    let mut playing = Playing::new();
    assert!(requests(&rt, &mut playing).is_empty());
}

/// The generator is heard, at the pitch it was given, and a stop fades it out.
#[test]
fn a_tone_is_audible_and_a_stop_takes_it_away() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let Driver::Pull(mixer) = &mut speaker.driver else {
        unreachable!("without_device builds a Pull driver")
    };
    let object = 0x0400_0060;
    mixer.tone(object, 440, 0, g::AEE_MAX_VOLUME);
    let mut left = vec![0.0f32; 4_410];
    let mut right = vec![0.0f32; 4_410];
    mixer.render_into(&mut left, &mut right);
    let crossings = left
        .windows(2)
        .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
        .count();
    assert!(
        (86..=90).contains(&crossings),
        "440 Hz for 100 ms is 88 crossings; counted {crossings}"
    );
    assert_eq!(left, right, "the tone is centred");

    // A stop from another object is not this object's stop: the generator has one owner.
    mixer.stop(0x0400_0070);
    let mut left = vec![0.0f32; 512];
    let mut right = vec![0.0f32; 512];
    mixer.render_into(&mut left, &mut right);
    assert!(
        left.iter().any(|sample| sample.abs() > 0.1),
        "another object's stop did not silence it"
    );

    mixer.stop(object);
    let mut fade = vec![0.0f32; 1_024];
    let mut fade_right = vec![0.0f32; 1_024];
    mixer.render_into(&mut fade, &mut fade_right);
    let mut after = vec![0.0f32; 512];
    let mut after_right = vec![0.0f32; 512];
    mixer.render_into(&mut after, &mut after_right);
    assert!(
        after.iter().all(|sample| *sample == 0.0),
        "the tone is gone once its fade is over"
    );
    assert!(
        mixer.tone.is_none(),
        "and the generator was given up rather than left silent in place"
    );
}

/// Two `ISound` objects asking for tones get one generator between them, which is the hardware: a
/// handset has a single tone path, so the second request takes it from the first.
#[test]
fn a_second_object_asking_for_a_tone_takes_the_generator() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let Driver::Pull(mixer) = &mut speaker.driver else {
        unreachable!("without_device builds a Pull driver")
    };
    mixer.tone(0x0400_0060, 440, 0, 100);
    mixer.tone(0x0400_0070, 880, 0, 100);
    let (owner, tone) = mixer.tone.as_ref().expect("a tone is sounding");
    assert_eq!(*owner, 0x0400_0070, "the last object to ask owns it");
    assert!(tone.is(880, 0), "and the pair is the one it asked for");
    // The first object's stop finds nothing of its own to stop.
    mixer.stop(0x0400_0060);
    assert!(mixer
        .tone
        .as_ref()
        .is_some_and(|(owner, _)| *owner == 0x0400_0070));
}

/// The cache has a ceiling, and the clip it gives up is the one used longest ago.
///
/// A run plays thirty short effects and two long tracks; without a sum there is nothing that says
/// when the decoded copies stop fitting in a machine. The order matters as much as the ceiling: a
/// scene re-plays its own few effects every second, and evicting one of those would decode it again
/// on the next play — a cache that costs more than it saves.
#[test]
fn a_cache_over_its_budget_gives_up_the_clip_used_longest_ago() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    let clip = |samples: usize| Arc::new(vec![0.0f32; samples]);
    speaker.store_pcm_within(0, clip(400), 44_100, 1_000);
    speaker.store_pcm_within(1, clip(400), 44_100, 1_000);
    assert_eq!(speaker.cached_samples(), 800, "both fit");
    // Clip 0 is used again, so clip 1 is now the oldest.
    speaker.touch_pcm(0);
    speaker.store_pcm_within(2, clip(400), 44_100, 1_000);
    assert_eq!(
        speaker.cached_samples(),
        800,
        "the sum stays under the ceiling"
    );
    assert!(speaker.pcm.contains_key(&0), "the freshly used clip stays");
    assert!(speaker.pcm.contains_key(&2), "the new clip stays");
    assert!(!speaker.pcm.contains_key(&1), "the untouched one went");
    assert_eq!(speaker.cached_clips(), 2);
}

/// One clip larger than the whole budget is still kept.
///
/// The alternative is a cache that holds nothing for the one file a run is playing on a loop, and
/// that would mean decoding a two-minute track between every pair of frames.
#[test]
fn a_clip_larger_than_the_budget_is_still_kept() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    speaker.store_pcm_within(0, Arc::new(vec![0.0f32; 5_000]), 44_100, 1_000);
    assert!(speaker.pcm.contains_key(&0), "the only clip is not evicted");
    assert_eq!(speaker.cached_samples(), 5_000);
}

/// A source that is renamed to different bytes drops what was cached under its index.
///
/// An index is a slot in the runtime's audio vector, not an identity: `withdraw_audio` and a new
/// naming can put another file there, and playing the old samples under the new name is the one
/// way a report and the sound disagree. The sample count has to follow the removal, or the ceiling
/// drifts and starts evicting clips that are still there.
#[test]
fn a_renamed_index_drops_its_cached_samples_and_the_count_with_them() {
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    speaker.store_pcm(0, Arc::new(vec![0.0f32; 300]), 44_100);
    assert_eq!(speaker.cached_samples(), 300);
    speaker.forget_clip(0);
    assert!(speaker.pcm.is_empty());
    assert_eq!(
        speaker.cached_samples(),
        0,
        "the budget follows the removal"
    );
}

/// A large source is decoded off the caller's thread when that is asked for, and the voice starts
/// by itself once the samples land.
///
/// This is the play that used to stop the frame: a minute of MP3 is hundreds of milliseconds of
/// codec work, and paying it between two frames is a freeze a person sees. The test uses a WAV big
/// enough to cross the threshold rather than a real MP3 — the threshold is about bytes, and a
/// generated file keeps the test free of a sample set.
#[test]
fn a_large_source_decodes_off_the_calling_thread_and_starts_when_ready() {
    let (rt, voice, _) = run_with_a_large_wav();
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    speaker.set_async_decode(true);
    assert!(speaker.async_decode());
    assert!(
        !speaker.play(&rt, voice, 0),
        "a large source is not decoded between two frames"
    );
    assert_eq!(
        speaker.requested(),
        0,
        "nothing was handed to the mixer yet"
    );
    assert_eq!(
        speaker.cached_samples(),
        0,
        "and nothing is cached while it decodes"
    );

    // The frame loop keeps running while the helper thread works; the start lands on whichever
    // frame the samples arrive before.
    let mut playing = Playing::new();
    let (mut left, mut right) = (vec![0.0f32; 64], vec![0.0f32; 64]);
    let mut started = false;
    for _ in 0..200 {
        speaker.mix(&rt, &mut playing, &mut left, &mut right);
        if speaker.requested() > 0 {
            started = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(started, "the clip started once its samples arrived");
    assert_eq!(speaker.cached_samples(), 176_000, "and it is cached");
    assert!(speaker.notes().is_empty(), "{:?}", speaker.notes());
}

/// The same source, on a speaker that asked for the synchronous path, is decoded before `play`
/// returns — which is what makes a pulled run reproducible and every other test in this file
/// independent of thread scheduling.
#[test]
fn a_pulled_speaker_decodes_large_sources_where_it_stands() {
    let (rt, voice, _) = run_with_a_large_wav();
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    assert!(
        !speaker.async_decode(),
        "a speaker the caller drives is synchronous by default"
    );
    assert!(
        speaker.play(&rt, voice, 0),
        "the decode happens before the play returns"
    );
    assert_eq!(speaker.cached_samples(), 176_000);
}

/// A voice that stopped while its clip was decoding is not started when the samples land.
///
/// The guest keeps running during the decode, so `ISoundPlayer::Stop` can arrive first. Starting
/// the voice then would be a sound the pack had already turned off — the same edge the play/stop
/// bookkeeping exists for, one thread later.
#[test]
fn a_voice_that_stopped_during_the_decode_is_not_started() {
    let (mut rt, voice, _) = run_with_a_large_wav();
    let mut speaker = Speaker::without_device(44_100, None)
        .expect("the tone bank this crate generates always parses");
    speaker.set_async_decode(true);
    assert!(!speaker.play(&rt, voice, 0));
    // The pack turns it off before the decode is done.
    if let Some(player) = rt.players.get_mut(&voice) {
        player.playing = false;
    }
    let mut playing = Playing::new();
    let (mut left, mut right) = (vec![0.0f32; 64], vec![0.0f32; 64]);
    for _ in 0..100 {
        speaker.mix(&rt, &mut playing, &mut left, &mut right);
        if speaker.cached_samples() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // One more pass with the samples cached: the wait is over and must not start anything.
    speaker.mix(&rt, &mut playing, &mut left, &mut right);
    assert_eq!(speaker.requested(), 0, "a stopped voice stays stopped");
}

/// A runtime holding one WAV larger than [`ASYNC_DECODE_BYTES`], a player pointed at it, and that
/// player marked playing — the state a run is in the moment it asks for a track.
///
/// 8 kHz mono 16-bit: 176 000 samples is a 352 000-byte file, and the same number of samples the
/// decode has to produce, which is what the assertions compare against.
fn run_with_a_large_wav() -> (Runtime, u32, usize) {
    let mut rt = Runtime::new();
    let samples = 176_000usize;
    let bytes = pcm_wav(8_000, &vec![0.25f32; samples]);
    assert!(bytes.len() >= ASYNC_DECODE_BYTES);
    let len = bytes.len() as u32;
    let clip = rt.cpu.mem.alloc(len, 4).unwrap();
    rt.cpu.mem.write_bytes(clip, &bytes).unwrap();
    rt.audio.push(AudioSource {
        input: g::SDT_BUFFER,
        data: clip,
        size: len,
        name: format!("{clip:#010x}"),
        bytes,
        problem: None,
    });
    let voice = 0x0400_0058;
    let player = rt.players.entry(voice).or_default();
    player.input = g::SDT_BUFFER;
    player.data = clip;
    player.size = len;
    player.playing = true;
    (rt, voice, samples)
}

/// A container's own frame count is the length this host reports.
///
/// A media clip used to be timed at 0 ms — `clip_length_ms` only understood SMAF, QCP and MIDI — and
/// every consequence of that is a behaviour a pack can see: `DONE` never fires, `GetTotalTime`
/// answers 0, and a `SetBytes`/`Set` window has no length to land inside.
#[test]
fn a_wav_is_timed_from_its_own_header() {
    // One second at 8 kHz: the header says 8 000 frames, and the answer has to be 1 000 ms.
    let one_second = pcm_wav(8_000, &vec![0.25f32; 8_000]);
    assert_eq!(clip_length_ms(&one_second), Some(1_000));
    // The control: the same builder at a different rate says a different number of milliseconds,
    // so this is reading the header rather than returning a constant.
    let half_a_second = pcm_wav(16_000, &vec![0.25f32; 8_000]);
    assert_eq!(clip_length_ms(&half_a_second), Some(500));
}

/// Bytes that no container claims have no length, which is not the same answer as zero.
#[test]
fn noise_has_no_length_rather_than_a_length_of_zero() {
    assert_eq!(clip_length_ms(&[0u8; 400]), None);
    assert_eq!(clip_length_ms(b"this is not audio at all"), None);
}

/// A score is still timed by its own timeline, not by the container probe.
///
/// The probe runs after the MIDI branch precisely so that a pack's every score does not go through
/// a Symphonia format scan; this is the test that keeps that order from being rearranged.
#[test]
fn a_score_is_still_timed_by_its_own_timeline() {
    let single = one_note_midi();
    let length = clip_length_ms(&single).expect("a one-note score has a length");
    assert!(length > 0, "and it is not zero: {length}");
}
