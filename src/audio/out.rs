//! The speaker, on a host that has one.
//!
//! [`crate::runtime`] follows the naming an `ISoundPlayer` is handed to the bytes behind it and
//! stops there, deliberately. Decoding is not the runtime's job, and a run that has to stay
//! reproducible to the byte cannot depend on a sound card existing. This module is the other half —
//! what turns those bytes into noise — and it sits beside [`crate::egui_window`] because it has the
//! same two properties that put the window there: it needs a real device, and none of it runs
//! unless a person is watching.
//!
//! # Why these crates
//!
//! [`rustysynth`] is the instrument. The sample set holds many payloads this host plays as `audio/mid` and not
//! one waveform (both corpora, 827 packs, counted from the pack census),
//! so
//! what had to be built was never a sample player: a Standard MIDI File is a score,
//! and something has to play it. `rustysynth` renders one through a SoundFont, is pure Rust, and
//! has no dependencies of its own — which is why it is here rather than one of the crates that wrap
//! a C library. This repository builds with mingw on Windows, and a C toolchain in the audio path
//! would be one more thing that can fail before a note is heard.
//!
//! [`midly`] is the decoder. rustysynth's own sequencer keeps its merged event list private, so this
//! host reads the file with midly and drives [`Synthesizer`] directly. Keeping the parsed timeline
//! here is what lets the forward window and a reverse one share a decoder; both crates are pure Rust,
//! so the audio path still needs no C toolchain.
//!
//! [`cpal`] is the device. It is the lower half of the pair on purpose: what this host has is a
//! stream it renders, not a file it opens, so the bottom of the stack wants a callback to fill
//! rather than a player with a queue. It reaches WASAPI, CoreAudio and ALSA from one piece of code,
//! and on Windows it needs no feature flag and no extra toolchain.
//!
//! # When a clip is parsed, which is the decision this file got wrong once
//!
//! A [`Speaker`] is built before the guest has run a single instruction, and therefore before any
//! `ISoundPlayer::SetInfo` has been made: at that moment [`Runtime::audio`] is empty and every clip
//! a game will ever name is still in its future. So the clips are **not** collected at
//! construction. Each one is parsed on the main thread the first time a play asks for it, and cached
//! from then on — parsed here rather than in the callback because parsing allocates, and the audio
//! thread is the one place that must not.
//!
//! Getting this backwards is a silence with no symptom: a speaker that snapshots the source list
//! when it is built holds an empty one, every play finds nothing to start, and every sound the run
//! asks for is dropped with nothing to read. That was this file's first shape, and
//! `a_clip_that_appears_after_the_speaker_is_built_is_still_played` is the test that keeps it from
//! being its second.
//!
//! # One voice per `ISoundPlayer`, which is the second decision this file got wrong
//!
//! An earlier shape of this module had a single sequencer, and a play replaced whatever was playing.
//! That reads as harmless — "the sample set starts one clip at a time" — and it is wrong the moment a
//! pack does anything more than that. A pack that turns its music off and on again makes a `Play`
//! each time; with one sequencer the music comes back as whichever effect happened to be started in
//! the same frame, chosen by the iteration order of a `HashMap`.
//!
//! A handset mixes per `ISoundPlayer`, not per shell: a pack keeps its music on one object and its
//! effects on others, and stopping the music stops that object. So a voice here is an object
//! address, the mixer is a sum of one [`Synthesizer`] per voice, and the test that pins it down is
//! `two_players_playing_at_once_are_two_voices_and_not_a_replacement`.
//!
//! # And the third: an edge is a change of *clip*, not of a flag
//!
//! Even with one voice per object, watching the object's `playing` flag is not enough. A pack brings
//! its music back by re-pointing the object it already has — `Stop`, `SetInfo` with the other file,
//! `Play` — and if those land in one turn the flag is `true` both before and after, so an edge
//! detector reading it sees nothing and the previous clip simply keeps playing. That is why its
//! music came back as the toggle effect, and no amount of mixing fixes it.
//!
//! [`Playing`] is the answer: what each voice is *playing*, compared by the three fields that name a
//! source. A voice still playing the same clip is left alone; one playing a different clip is a
//! play, whether or not the flag moved. `re_pointing_a_playing_player_at_another_clip_is_a_play` is
//! that test.
//!
//! # Where the tones come from
//!
//! The SoundFont is *not* vendored and cannot be: the usual General MIDI banks are GPL, and this
//! crate's `MIT OR Apache-2.0` cannot take one in. It is therefore a path a run is given
//! (`--soundfont`, or the `QUOIN_SOUNDFONT` the manager reads), and when a run is given none this
//! module loads [`crate::audio::soundbank`] instead — eight waveform tables generated at startup and
//! wrapped in the same container, so that the same parser, the same sequencer and the same mixer
//! answer either way and there is only one thing that decides what a run sounds like.
//!
//! That is a change of mind, and the old one is worth keeping: this file's first shape let a run
//! with no `--soundfont` have *no speaker at all*, on the reasoning that a bank is a third-party
//! file and a host cannot invent one. A handset settles it the other way, and it settles it with
//! what it **does**: the LG MX7000 image ships `.mid` files under its own `/usr/content/midi`,
//! advertises `General MIDI` in its UI, and takes no sound file from its owner — so the timbres came
//! from something in the machine. "You must supply a file or there is no sound" was our convenience,
//! not the platform's behaviour.
//!
//! ⚠ The stronger version of that sentence — "there is no sampled bank anywhere in its 32 MiB" — is
//! **not** something this host has established: it rests on `sfbk`/DLS magic being absent, and there
//! is no real `.sf2` on this machine to give that search a positive control. The source that measured
//! it labels that conclusion weak and says not to use it as one
//! (the source itself disclaims it), so nothing in this file may lean on it:
//! what the built-in bank claims is that a handset plays MIDI without a file from the user, which is
//! what the two strong records say, not what the instrument inside the phone was made of.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

fn audio_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("QUOIN_AUDIO_TRACE").is_some())
}

fn audio_trace_now() -> u128 {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_micros()
}

fn audio_trace(args: std::fmt::Arguments<'_>) {
    if audio_trace_enabled() {
        eprintln!("audio-trace at={}us {args}", audio_trace_now());
    }
}

/// Whether the development-only audio trace is on (`QUOIN_AUDIO_TRACE`).
///
/// One switch for both halves of the seam: what the mixer did with a clip, and what the front end
/// asked it for. That second half used to print unconditionally, which is not a trace but a leak —
/// a music-heavy pack writes a line per sound event per frame into a window that has no terminal to
/// scroll, and the `music` log channel on the control port is where those questions are meant to
/// be asked.
pub fn tracing() -> bool {
    audio_trace_enabled()
}

#[cfg(feature = "device")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "device")]
use cpal::{FromSample, SizedSample};
use midly::live::LiveEvent;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use rustysynth::{MidiFile, SoundFont, Synthesizer, SynthesizerSettings};

use crate::abi::generated as g;
use crate::audio::smaf::player::Player as SmafPlayer;
use crate::runtime::{Player, Runtime};

/// Frames rendered per pass of the callback's inner loop. Only a ceiling: the callback asks for
/// whatever the device wants, and a device that wants more than this gets it in several passes
/// rather than in one allocation it would have had to make on the audio thread.
const BLOCK: usize = 1024;

/// How many clips this speaker can have going at once.
///
/// A pack asks for as many `ISoundPlayer`s as it likes; a handset stops at whatever its audio
/// hardware manages, and this is that ceiling by another name. Eight is a music track, an ambient
/// loop and half a dozen effects, which is more than anything in the sample set uses at one moment, and
/// the voices cost nothing until they are used.
const VOICES: usize = 8;

/// The highest whole-mixer speed the resampler's scratch is sized for.
///
/// The command line clamps to the same figure (`--speed` takes 0.25 to 4), because a scratch that
/// silently ran out would either panic on the audio thread or cut a block short — both worse than
/// refusing a number the resampler was not built for.
const MAX_SPEED: usize = 4;

/// How many decoded samples this speaker keeps, across every clip it has played.
///
/// A per-clip limit would be the wrong shape: what runs out is memory, not files, and one game
/// plays thirty short effects while another plays two long tracks. The sum is what has a ceiling,
/// and the eviction order is what keeps the ceiling from falling on the clip in use.
///
/// 32 MiB of mono f32 is about five and a half minutes at 48 kHz — several music tracks plus every
/// effect a scene touches — and it is a few per cent of what the same pack's images already cost.
/// Nothing is lost when a clip goes: the mixer holds its own `Arc` to the samples, so a clip that is
/// playing keeps playing and only a *later* play of the evicted one decodes again.
const PCM_BUDGET_SAMPLES: usize = 8 << 20;

/// Sources at least this big are decoded on the helper thread rather than on the caller's.
///
/// The two costs a play can have are a parse and a decode. A MIDI or QCP effect is a few tens of
/// kilobytes and decodes inside a frame's worth of time, so it stays where it is and a play is
/// audible the moment it is asked for. A two-minute MP3 is hundreds of milliseconds of codec work,
/// and paying that between two frames is a visible freeze — which is what this threshold is for.
const ASYNC_DECODE_BYTES: usize = 256 << 10;

/// One channel message from a parsed clip, with its position in seconds.
#[derive(Debug, Clone, Copy)]
struct MidiEvent {
    time: f64,
    channel: u8,
    command: u8,
    data1: u8,
    data2: u8,
}

/// The raw channel-message bytes a `midly` message maps to.
fn midi_parts(message: MidiMessage) -> (u8, u8, u8) {
    match message {
        MidiMessage::NoteOff { key, vel } => (0x80, key.as_int(), vel.as_int()),
        MidiMessage::NoteOn { key, vel } => (0x90, key.as_int(), vel.as_int()),
        MidiMessage::Aftertouch { key, vel } => (0xA0, key.as_int(), vel.as_int()),
        MidiMessage::Controller { controller, value } => {
            (0xB0, controller.as_int(), value.as_int())
        }
        MidiMessage::ProgramChange { program } => (0xC0, program.as_int(), 0),
        MidiMessage::ChannelAftertouch { vel } => (0xD0, vel.as_int(), 0),
        MidiMessage::PitchBend { bend } => {
            let raw = bend.0.as_int();
            (0xE0, (raw & 0x7f) as u8, (raw >> 7) as u8)
        }
    }
}

/// A Standard MIDI File reduced to the channel messages and absolute times a synthesizer needs.
///
/// `midly` owns the decoder; this type exists because rustysynth's `MidiFile` keeps its merged event
/// list private, and a sequencer that cannot see the events cannot seek or reverse them. Tempo meta
/// events are consumed while the absolute times are built, so playback itself only has to look at
/// one sorted list.
#[derive(Debug)]
struct Clip {
    events: Vec<MidiEvent>,
    reversed: Vec<MidiEvent>,
    length: f64,
}

/// Build the event list a reverse window plays.
///
/// A note is not just its two messages mirrored: mirroring a note-on into a note-off would leave the
/// reversed timeline with no note at all. Notes are paired (first on with first off), then emitted as
/// an on at the mirrored off time and an off at the mirrored on time. Other channel messages keep
/// their kind and are mirrored in time.
fn reversed_events(events: &[MidiEvent], length: f64) -> Vec<MidiEvent> {
    let mut out = Vec::new();
    let mut open: HashMap<(u8, u8), Vec<(f64, u8)>> = HashMap::new();
    for event in events {
        let note_on = event.command == 0x90 && event.data2 > 0;
        let note_off = event.command == 0x80 || (event.command == 0x90 && event.data2 == 0);
        if note_on {
            open.entry((event.channel, event.data1))
                .or_default()
                .push((event.time, event.data2));
        } else if note_off {
            let list = open.entry((event.channel, event.data1)).or_default();
            if let Some((on_time, velocity)) = list.first().copied() {
                list.remove(0);
                out.push(MidiEvent {
                    time: (length - event.time).max(0.0),
                    channel: event.channel,
                    command: 0x90,
                    data1: event.data1,
                    data2: velocity,
                });
                out.push(MidiEvent {
                    time: (length - on_time).max(0.0),
                    channel: event.channel,
                    command: 0x80,
                    data1: event.data1,
                    data2: 0,
                });
            } else {
                out.push(MidiEvent {
                    time: (length - event.time).max(0.0),
                    ..*event
                });
            }
        } else {
            out.push(MidiEvent {
                time: (length - event.time).max(0.0),
                ..*event
            });
        }
    }
    for ((channel, key), list) in open {
        for (on_time, velocity) in list {
            out.push(MidiEvent {
                time: 0.0,
                channel,
                command: 0x90,
                data1: key,
                data2: velocity,
            });
            out.push(MidiEvent {
                time: (length - on_time).max(0.0),
                channel,
                command: 0x80,
                data1: key,
                data2: 0,
            });
        }
    }
    out.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

enum RawMidiEvent {
    Message {
        channel: u8,
        command: u8,
        data1: u8,
        data2: u8,
    },
    Tempo(u32),
}

impl Clip {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        let smf = Smf::parse(bytes).map_err(|e| e.to_string())?;
        let (ppqn, ticks_per_second) = match smf.header.timing {
            Timing::Metrical(value) => (Some(f64::from(value.as_int())), None),
            Timing::Timecode(fps, subframe) => {
                (None, Some(f64::from(fps.as_int()) * f64::from(subframe)))
            }
        };
        let mut raw = Vec::new();
        for track in &smf.tracks {
            let mut tick = 0u64;
            for event in track {
                tick += u64::from(event.delta.as_int());
                match &event.kind {
                    TrackEventKind::Midi { channel, message } => {
                        let channel = channel.as_int();
                        let (command, data1, data2) = match *message {
                            MidiMessage::NoteOff { key, vel } => (0x80, key.as_int(), vel.as_int()),
                            MidiMessage::NoteOn { key, vel } => (0x90, key.as_int(), vel.as_int()),
                            MidiMessage::Aftertouch { key, vel } => {
                                (0xA0, key.as_int(), vel.as_int())
                            }
                            MidiMessage::Controller { controller, value } => {
                                (0xB0, controller.as_int(), value.as_int())
                            }
                            MidiMessage::ProgramChange { program } => (0xC0, program.as_int(), 0),
                            MidiMessage::ChannelAftertouch { vel } => (0xD0, vel.as_int(), 0),
                            MidiMessage::PitchBend { bend } => {
                                let raw = bend.0.as_int();
                                (0xE0, (raw & 0x7f) as u8, (raw >> 7) as u8)
                            }
                        };
                        raw.push((
                            tick,
                            RawMidiEvent::Message {
                                channel,
                                command,
                                data1,
                                data2,
                            },
                        ));
                    }
                    TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                        raw.push((tick, RawMidiEvent::Tempo(tempo.as_int())));
                    }
                    _ => {}
                }
            }
        }
        raw.sort_by_key(|(tick, _)| *tick);

        let mut tempo = 500_000u32;
        let mut last_tick = 0u64;
        let mut now = 0.0f64;
        let mut events = Vec::new();
        for (tick, event) in raw {
            let delta = tick.saturating_sub(last_tick);
            if delta > 0 {
                now += if let Some(ticks_per_second) = ticks_per_second {
                    delta as f64 / ticks_per_second
                } else {
                    delta as f64 * f64::from(tempo)
                        / 1_000_000.0
                        / ppqn.expect("metrical timing has a tick base")
                };
                last_tick = tick;
            }
            match event {
                RawMidiEvent::Tempo(value) => tempo = value,
                RawMidiEvent::Message {
                    channel,
                    command,
                    data1,
                    data2,
                } => events.push(MidiEvent {
                    time: now,
                    channel,
                    command,
                    data1,
                    data2,
                }),
            }
        }
        let reversed = reversed_events(&events, now);
        Ok(Self {
            events,
            reversed,
            length: now,
        })
    }
}

/// A forward MIDI sequencer over rustysynth's `Synthesizer`.
///
/// This is the sequencer: times are already absolute seconds, tempo changes are baked in by
/// [`Clip::parse`], and the synth owns all voice state. Keeping the event list here is also what makes
/// a future reverse window possible without another decoder.
struct Sequencer {
    synthesizer: Synthesizer,
    clip: Option<Arc<Clip>>,
    speed: f64,
    /// Semitones to shift every sounding key by, from `ISoundPlayer::SetTune`.
    ///
    /// A tune moves *pitch*, not time, so it cannot be a rate: the notes have to stay where the
    /// score put them. Transposing the key number at the moment an event reaches the synthesizer is
    /// the only place that keeps the timeline intact, and it is why [`Self::set_tune`] has to
    /// release whatever is sounding: a note already held was started at the old key, and nothing
    /// tells a synthesiser to move a voice it is already running.
    tune: i32,
    block_wrote: usize,
    current_time: f64,
    event_index: usize,
    reverse: bool,
    window_start: f64,
    window_stop: f64,
    finished: bool,
}

impl Sequencer {
    fn new(synthesizer: Synthesizer) -> Self {
        let block_wrote = synthesizer.get_block_size();
        Self {
            synthesizer,
            clip: None,
            speed: 1.0,
            tune: 0,
            block_wrote,
            current_time: 0.0,
            event_index: 0,
            reverse: false,
            window_start: 0.0,
            window_stop: 0.0,
            finished: false,
        }
    }

    fn play(&mut self, clip: &Arc<Clip>) {
        self.clip = Some(Arc::clone(clip));
        self.block_wrote = self.synthesizer.get_block_size();
        self.current_time = 0.0;
        self.event_index = 0;
        self.reverse = false;
        self.window_start = 0.0;
        self.window_stop = clip.length;
        self.finished = false;
        self.synthesizer.reset();
    }

    /// Place the current clip on a media window, in milliseconds. Reverse windows play the mirrored
    /// event list from `start` down to `stop`, which is what `MM_PARM_POS` means when start > stop.
    fn set_window(&mut self, start_ms: u32, stop_ms: u32, reverse: bool) {
        let Some(clip) = self.clip.clone() else {
            return;
        };
        let length = clip.length;
        let (start, stop) = if reverse {
            (
                (length - f64::from(stop_ms) / 1000.0).clamp(0.0, length),
                (length - f64::from(start_ms) / 1000.0).clamp(0.0, length),
            )
        } else {
            (
                (f64::from(start_ms) / 1000.0).clamp(0.0, length),
                (f64::from(stop_ms) / 1000.0).clamp(0.0, length),
            )
        };
        let stop = stop.max(start);
        self.reverse = reverse;
        self.window_start = start;
        self.window_stop = stop;
        self.current_time = start;
        let events = if reverse {
            &clip.reversed
        } else {
            &clip.events
        };
        self.event_index = events.partition_point(|event| event.time < start);
        self.block_wrote = self.synthesizer.get_block_size();
        self.finished = false;
        self.synthesizer.reset();
    }

    fn stop(&mut self) {
        self.clip = None;
        self.synthesizer.reset();
    }

    #[cfg(test)]
    fn get_speed(&self) -> f64 {
        self.speed
    }

    /// Apply one live MIDI message from `MM_PARM_MIDI_MSG`.
    fn midi(&mut self, bytes: &[u8]) {
        if let Ok(LiveEvent::Midi { channel, message }) = LiveEvent::parse(bytes) {
            let (command, data1, data2) = midi_parts(message);
            let channel = channel.as_int();
            let Some(key) = shift_key(self.tune, channel, command, data1) else {
                return;
            };
            self.synthesizer.process_midi_message(
                i32::from(channel),
                i32::from(command),
                i32::from(key),
                i32::from(data2),
            );
        }
    }

    /// Shift every key this sequencer sends by `step` semitones, the `ISoundPlayer::SetTune` step.
    ///
    /// Whatever is sounding is released first, and that is not a shortcut: its note-on went out at
    /// the old key and a synthesiser has no message that moves a voice it is already running, so
    /// every note straddling the change would hang until its own note-off arrived at a key nothing
    /// is playing.
    fn set_tune(&mut self, step: i32) {
        if self.tune == step {
            return;
        }
        self.tune = step;
        self.synthesizer.note_off_all(false);
    }

    fn set_speed(&mut self, value: f64) {
        if value < 0.0 {
            panic!("The playback speed must be a non-negative value.");
        }
        self.speed = value;
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(
            left.len(),
            right.len(),
            "The output buffers for the left and right must have the same length."
        );
        let mut wrote = 0;
        while wrote < left.len() {
            if self.block_wrote == self.synthesizer.get_block_size() {
                // The window's stop is the only end condition there is, and a zero-length one is a
                // window that ends at once — the same rule the PCM sources follow. It used to
                // require `window_stop > window_start` here, which turned `MM_PARM_POS` saying
                // `start == stop` into a window that never ended: `current_time` was pinned at the
                // stop, the events at that instant were delivered, and the notes they started were
                // never taken off, because the `note_off_all` below is the only place that ends
                // them. A pack asking for nothing got a drone instead.
                if self.current_time >= self.window_stop {
                    if !self.finished {
                        self.synthesizer.note_off_all(false);
                        self.finished = true;
                    }
                } else {
                    self.process_events();
                }
                self.block_wrote = 0;
                if !self.finished {
                    self.current_time += self.speed * self.synthesizer.get_block_size() as f64
                        / self.synthesizer.get_sample_rate() as f64;
                    if self.current_time >= self.window_stop {
                        self.current_time = self.window_stop;
                    }
                }
            }
            let block = self.synthesizer.get_block_size();
            let rem = (block - self.block_wrote).min(left.len() - wrote);
            self.synthesizer.render(
                &mut left[wrote..wrote + rem],
                &mut right[wrote..wrote + rem],
            );
            self.block_wrote += rem;
            wrote += rem;
        }
    }

    fn process_events(&mut self) {
        let Some(clip) = self.clip.as_ref() else {
            return;
        };
        let events = if self.reverse {
            &clip.reversed
        } else {
            &clip.events
        };
        while self.event_index < events.len() {
            let event = events[self.event_index];
            if event.time > self.current_time || event.time > self.window_stop {
                break;
            }
            if event.time < self.window_start {
                self.event_index += 1;
                continue;
            }
            let Some(key) = shift_key(self.tune, event.channel, event.command, event.data1) else {
                self.event_index += 1;
                continue;
            };
            self.synthesizer.process_midi_message(
                i32::from(event.channel),
                i32::from(event.command),
                i32::from(key),
                i32::from(event.data2),
            );
            self.event_index += 1;
        }
    }
}

/// The MIDI channel whose `data1` is a drum number rather than a pitch.
///
/// Channel 10 is percussion, and a tune that shifted a percussion key would select a different
/// instrument instead of a higher one — which is not a tune at all.
const DRUM_CHANNEL: u8 = 9;

/// `key` shifted by `tune` semitones, or `None` when the shift takes it off the keyboard.
///
/// Only the three messages whose `data1` is a key are shifted: note-off, note-on and polyphonic
/// pressure. Program change's `data1` is a patch number, controller and channel pressure carry no
/// key, and pitch bend's is a 14-bit value — transposing any of them would be a different message.
///
/// An out-of-range result answers `None` and the event is dropped. Dropping a note-on is silence;
/// dropping a note-off would be a hang, but a note-off at the shifted key can only be out of range
/// if its note-on was too, and that one was already dropped.
fn shift_key(tune: i32, channel: u8, command: u8, key: u8) -> Option<u8> {
    if tune == 0 || channel == DRUM_CHANNEL || !matches!(command, 0x8 | 0x9 | 0xa) {
        return Some(key);
    }
    let shifted = i32::from(key) + tune;
    (0..=127).contains(&shifted).then_some(shifted as u8)
}

/// What each `ISoundPlayer` is currently playing, and the tempo last handed to the speaker.
///
/// The front end owns this — it is the record of what the speaker has been told — and it maps
/// voices to *clips* rather than being a set of objects on purpose. See the module docs: a pack
/// re-pointing an object at a different file while it plays is a play, and that object's `playing`
/// flag cannot say so.
///
/// The tempo here is the one the synthesizer is using, not a per-player setting: both ABI doors
/// write the runtime's single shared value, and [`requests`] only has to notice when it moves.
#[derive(Debug)]
pub struct Playing {
    voices: HashMap<u32, (u32, u32, u32, u32)>,
    /// The volume last handed to each voice, in the platform's 0..`AEE_MAX_VOLUME` scale.
    volumes: HashMap<u32, u32>,
    /// The tempo last handed to the speaker. Starts at the header's default, so a runtime that has
    /// never been told otherwise is not a change.
    tempo: u32,
    /// The `MM_PARM_RATE` numerator/denominator last handed to each voice.
    rates: HashMap<u32, (u32, u32)>,
    /// The tune step last handed to each voice, in semitones.
    tunes: HashMap<u32, i32>,
    /// The tone last handed to the mixer for each `ISound` object: its two frequencies and volume.
    ///
    /// The mixer has one generator and a tone is not a voice, so this records ownership rather than
    /// a slot: it is what keeps a hundred frames of `PlayTone` from restarting a beep, and what lets
    /// a `StopTone` name the object it ends.
    tones: HashMap<u32, (u16, u16, u32)>,
    /// How many queued `MM_PARM_MIDI_MSG` packets have already been sent to each voice.
    midi_sent: HashMap<u32, usize>,
    /// The window and direction last handed to each voice.
    windows: HashMap<u32, (u32, u32, bool)>,
}

impl Playing {
    pub fn new() -> Self {
        Self {
            voices: HashMap::new(),
            tempo: DEFAULT_TEMPO,
            volumes: HashMap::new(),
            rates: HashMap::new(),
            tunes: HashMap::new(),
            tones: HashMap::new(),
            midi_sent: HashMap::new(),
            windows: HashMap::new(),
        }
    }
}

impl Default for Playing {
    fn default() -> Self {
        Self::new()
    }
}

/// One thing a frame of a run implies the speaker should do.
///
/// `voice` is the `ISoundPlayer` the request came from — its address in the guest — because that is
/// the unit a pack mixes in and therefore the unit it stops in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Start the clip at this index into [`Runtime::audio`], on this voice.
    Play { voice: u32, index: usize },
    /// Silence this voice.
    Stop { voice: u32 },
    /// Set one voice's gain, in the platform's 0..`AEE_MAX_VOLUME` scale.
    Volume { voice: u32, volume: u32 },
    /// Set the tempo every voice plays at, as a percentage of normal speed.
    ///
    /// The tempo is a property of the synthesizer and not of a voice: a handset changes it once and
    /// every MIDI stream in flight follows. The request carries no voice for that reason.
    Tempo(u32),
    /// Set one voice's `MM_PARM_RATE` numerator/denominator. Unlike tempo, this is per media object,
    /// so the request names the voice.
    Rate { voice: u32, num: u32, den: u32 },
    /// Shift one voice's pitch by a `ISoundPlayer::SetTune` step, in semitones.
    ///
    /// A tuning is not a rate, and that is why it is its own request: rate moves time and pitch
    /// together, tune moves pitch alone.
    Tune { voice: u32, step: i32 },
    /// Sound a tone on behalf of this `ISound` object: two frequencies in hertz (`lo` 0 for one) at
    /// this volume. The object address is the owner, so a stop from another object cannot silence it.
    Tone {
        object: u32,
        hi: u16,
        lo: u16,
        volume: u32,
    },
    /// Hand one raw `MM_PARM_MIDI_MSG` packet to the voice's synthesizer.
    Midi { voice: u32, bytes: Vec<u8> },
    /// Set the media window and direction for a voice.
    Window {
        voice: u32,
        start: u32,
        stop: u32,
        reverse: bool,
    },
}

/// Which decoder backend turns compressed audio bytes into PCM.
///
/// The backend is independent of the output device and of the MIDI synthesizer: MIDI always goes
/// through `midly`/`rustysynth`, while this choice decides who decodes QCP/MP3/AAC/WAV/ADPCM. The
/// Rust backend is the default and the only one linked into this build; `Ffmpeg` is the reserved
/// optional backend and is reported unavailable rather than silently falling back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AudioBackend {
    #[default]
    Rust,
    Ffmpeg,
}

impl AudioBackend {
    pub const ALL: [AudioBackend; 2] = [Self::Rust, Self::Ffmpeg];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Ffmpeg => "ffmpeg",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust (Symphonia/QCP)",
            Self::Ffmpeg => "FFmpeg (not built)",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "rust" => Some(Self::Rust),
            "ffmpeg" => Some(Self::Ffmpeg),
            _ => None,
        }
    }

    /// Whether this binary actually contains the backend.
    pub const fn available(self) -> bool {
        matches!(self, Self::Rust) || cfg!(feature = "ffmpeg")
    }

    pub(crate) fn from_u8(value: u8) -> Self {
        if value == Self::Ffmpeg as u8 {
            Self::Ffmpeg
        } else {
            Self::Rust
        }
    }
}

/// What a player asks for when it has never been told otherwise. `MM_PARM_TEMPO` is a percentage,
/// and the header's default is 100.
pub const DEFAULT_TEMPO: u32 = 100;

/// The range `ISoundPlayer::SetTempo` documents, in percent: 1 % is the slowest factor and 500 %
/// the fastest.
const MIN_TEMPO: u32 = 1;
const MAX_TEMPO: u32 = 500;

/// The factor a caller asked for, when it is inside the documented range.
///
/// `ISoundPlayer::SetTempo` documents 1 % as the slowest factor and 500 % as the fastest. A value
/// outside that range is not a factor this host plays: the caller's old value stays, which is also
/// what the header's result table calls a failed set. Clamping instead would turn a reported 0 into
/// 1 %, a speed the caller never asked for.
pub fn tempo_percent(percent: u32) -> Option<u32> {
    (MIN_TEMPO..=MAX_TEMPO)
        .contains(&percent)
        .then_some(percent)
}

/// The requests one frame of a run implies, given what the front end has already answered.
///
/// This is the whole of the seam between the runtime and the speaker, and what matters is a change
/// of *clip* rather than a change of flag. `ISoundPlayer::Play` sets a boolean on an object, and
/// what a handset does with that is start — once, at that moment — so a player already playing the
/// same clip is already playing it, and a hundred frames of a window must not restart it. But a
/// player that has been *re-pointed* at another clip is starting something, which is how a pack
/// brings its music back; watching the boolean alone misses it, because the stop and the play
/// land in one turn and leave it `true` throughout.
///
/// `playing` is the caller's record of what each voice was last told to play, updated in place. It
/// lives in the front end rather than in the runtime for the same reason the clock does: what to do
/// about a flag is the front end's business, and the runtime has to stay a function of the guest
/// alone if a run is to be reproducible.
///
/// Split out of [`Speaker::follow`] because this is the part with the logic in it and it has
/// nothing to do with a sound card — which is what lets it be tested on a machine with no device.
pub fn requests(rt: &Runtime, playing: &mut Playing) -> Vec<Request> {
    let mut wanted = Vec::new();
    for (this, player) in &rt.players {
        let pass = (player.input, player.data, player.size, player.play_gen);
        let rate = player.rate_parts();
        match (player.playing, playing.voices.get(this)) {
            // Already playing this very source in the same pass. A repeated `Play` during a clip
            // must not restart it. `DONE`/`Stop` bump `play_gen`, so the next same-source `Play`
            // is a new pass and reaches the arm below.
            (true, Some(was)) if *was == pass => {}
            // Started, re-pointed, or resumed after completion. All are a play.
            (true, _) => {
                playing.voices.insert(*this, pass);
                playing.windows.remove(this);
                // A play is the one moment a slot may have changed hands — the mixer keeps only
                // eight, and a ninth voice takes the oldest — and a slot that changed hands starts
                // at the header's defaults. The record has to forget what it last sent, or it will
                // not send the pack's own level again and the voice plays at full scale for the
                // rest of the run. See `Mixer::assign`.
                playing.volumes.remove(this);
                playing.tunes.remove(this);
                if let Some(index) = source_of(rt, player) {
                    wanted.push(Request::Play {
                        voice: *this,
                        index,
                    });
                }
                playing.rates.insert(*this, rate);
                if rate != (1, 1) {
                    wanted.push(Request::Rate {
                        voice: *this,
                        num: rate.0,
                        den: rate.1,
                    });
                }
            }
            (false, Some(_)) => {
                playing.voices.remove(this);
                playing.rates.remove(this);
                playing.windows.remove(this);
                wanted.push(Request::Stop { voice: *this });
            }
            // Not playing, and not recorded as playing: nothing to say about it.
            (false, None) => {
                playing.rates.remove(this);
            }
        }
        if player.playing {
            match playing.rates.get(this) {
                Some(was) if *was == rate => {}
                Some(_) => {
                    playing.rates.insert(*this, rate);
                    wanted.push(Request::Rate {
                        voice: *this,
                        num: rate.0,
                        den: rate.1,
                    });
                }
                None => {
                    playing.rates.insert(*this, rate);
                    if rate != (1, 1) {
                        wanted.push(Request::Rate {
                            voice: *this,
                            num: rate.0,
                            den: rate.1,
                        });
                    }
                }
            }
        } else {
            playing.rates.remove(this);
        }
        if player.playing {
            let step = crate::audio::tone::tune_step(player.tune);
            // An untuned voice is what every slot starts as, and [`Mixer::assign`] puts a slot back
            // there when it changes hands, so an untuned voice with nothing on record has nothing
            // to say. Saying it anyway would put a `Tune` in front of every play in a run that
            // never tunes anything.
            let differs = match playing.tunes.get(this) {
                Some(was) => *was != step,
                None => step != 0,
            };
            if differs {
                playing.tunes.insert(*this, step);
                wanted.push(Request::Tune { voice: *this, step });
            }
        } else {
            playing.tunes.remove(this);
        }
        if player.playing {
            // `SetSoundDevice`'s ear mute folds into the gain rather than becoming a second control:
            // the mixer has one output, and a muted voice is a voice at no level. A player with no
            // level of its own and no mute wants the header's default, and says so *only* when this
            // end may have changed it — an unmute has to take back the zero the mute asked for.
            let want = if player.ear_mute == g::AEE_SOUND_MUTECTL_MUTED {
                Some(0)
            } else if player.volume_set {
                Some(player.volume.min(g::AEE_MAX_VOLUME))
            } else {
                None
            };
            match want {
                Some(volume) => {
                    if playing.volumes.get(this) != Some(&volume) {
                        playing.volumes.insert(*this, volume);
                        wanted.push(Request::Volume {
                            voice: *this,
                            volume,
                        });
                    }
                }
                None => {
                    if playing.volumes.remove(this).is_some() {
                        wanted.push(Request::Volume {
                            voice: *this,
                            volume: g::AEE_MAX_VOLUME,
                        });
                    }
                }
            }
        } else {
            playing.volumes.remove(this);
        }
        if let Some(packets) = rt.midi_out.get(this) {
            let sent = playing.midi_sent.entry(*this).or_insert(0);
            if *sent > packets.len() {
                *sent = packets.len();
            }
            for packet in &packets[*sent..] {
                wanted.push(Request::Midi {
                    voice: *this,
                    bytes: packet.clone(),
                });
            }
            *sent = packets.len();
        }
        if player.playing && player.pass_ms > 0 {
            let start = player.play_start;
            let stop = player.play_stop.unwrap_or(player.pass_ms);
            let reverse = player.reverse();
            let window = (start, stop, reverse);
            if playing.windows.get(this) != Some(&window) {
                playing.windows.insert(*this, window);
                wanted.push(Request::Window {
                    voice: *this,
                    start,
                    stop,
                    reverse,
                });
            }
        } else {
            playing.windows.remove(this);
        }
    }
    // Tones are not voices: a handset has one tone generator and every `ISound` object shares it,
    // so what the mixer needs is which object owns it — not a slot. Sorted by address because a
    // `HashMap` walk is not, and a scripted run has to repeat byte for byte.
    let mut sounding: Vec<u32> = rt
        .tones
        .iter()
        .filter(|(_, tone)| tone.playing)
        .map(|(object, _)| *object)
        .collect();
    sounding.sort_unstable();
    for object in &sounding {
        let tone = &rt.tones[object];
        if tone.freq_hi == 0 {
            // An id whose sound belongs to the OEM's tone table: the request was accepted, the
            // report is still owed, and there is nothing here to play. See `audio::tone`. If the
            // same object was sounding a mapped id a moment ago — a `PlayToneList` advancing into
            // the unmapped part of its list — the generator it owns has to be given up, or the
            // previous id would keep sounding for it.
            if playing.tones.remove(object).is_some() {
                wanted.push(Request::Stop { voice: *object });
            }
            continue;
        }
        let want = (
            tone.freq_hi,
            tone.freq_lo,
            tone.volume.min(g::AEE_MAX_VOLUME),
        );
        if playing.tones.get(object) != Some(&want) {
            playing.tones.insert(*object, want);
            wanted.push(Request::Tone {
                object: *object,
                hi: want.0,
                lo: want.1,
                volume: want.2,
            });
        }
    }
    // Whatever the mixer may still be holding for a tone that is no longer sounding: one whose
    // length ran out, one a `StopTone` ended, or one whose object the pack released.
    let mut ended: Vec<u32> = playing
        .tones
        .keys()
        .filter(|object| sounding.binary_search(object).is_err())
        .copied()
        .collect();
    ended.sort_unstable();
    for object in ended {
        playing.tones.remove(&object);
        wanted.push(Request::Stop { voice: object });
    }
    // Tempo belongs to the synthesizer, and the runtime keeps the one value both ABI doors write.
    // A player has no tempo setting of its own, so there is no per-object default that could be
    // mistaken for a command, and opening another sound object cannot undo a tempo already set.
    if rt.media_tempo != playing.tempo {
        playing.tempo = rt.media_tempo;
        wanted.push(Request::Tempo(rt.media_tempo));
    }
    // A voice whose object is gone. `Runtime::release` deletes the player record — which is right for
    // every other per-object table — but a sound that was sounding does not stop by being forgotten,
    // and the loop above cannot see a player it no longer holds. So `Release` on a playing object used
    // to leave that voice playing to the end of its clip while the pack, which is already on to the
    // next object, started the next tune: two musics at once. This ledger is the only place the fact
    // still exists, which is why the silence is said from here.
    let gone: Vec<u32> = playing
        .voices
        .keys()
        .filter(|voice| !rt.players.contains_key(voice))
        .copied()
        .collect();
    for voice in gone {
        playing.voices.remove(&voice);
        playing.rates.remove(&voice);
        playing.tunes.remove(&voice);
        playing.midi_sent.remove(&voice);
        playing.windows.remove(&voice);
        wanted.push(Request::Stop { voice });
    }
    wanted
}

fn source_fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Which of the run's recorded sources a player named, if it named one the run kept.
///
/// The same three fields `Set`/`SetInfo` stored and `GetInfo` hands back, which is what makes a play
/// request a lookup rather than a guess.
fn source_of(rt: &Runtime, player: &Player) -> Option<usize> {
    if player.input == g::SDT_FILE {
        let name = rt.cstring(player.data).ok()?;
        return rt
            .audio
            .iter()
            .position(|s| s.input == g::SDT_FILE && s.name == name);
    }
    // Buffers are reused by the guest heap. Prefer the record whose bytes are the buffer's
    // current bytes; an address-only lookup can otherwise select the clip that lived there
    // before and play it under the new naming.
    if player.size != 0 {
        if let Ok(bytes) = rt.cpu.mem.read_bytes(player.data, player.size as usize) {
            if let Some(index) = rt.audio.iter().position(|s| {
                s.input == player.input
                    && s.data == player.data
                    && s.size == player.size
                    && s.bytes == bytes
            }) {
                return Some(index);
            }
        }
    }
    rt.audio
        .iter()
        .position(|s| s.input == player.input && s.data == player.data && s.size == player.size)
}

/// Whether a voice is still waiting for this clip, asked long after it asked.
///
/// A decode on the helper thread runs for a while, and the guest keeps running while it does: it
/// can stop the sound, or point the same player at another clip, before the samples land. Starting
/// the voice then would be a sound the pack had already turned off, so the decision has to be made
/// against the runtime as it is now rather than as it was when the job was submitted.
fn still_wanted(rt: &Runtime, voice: u32, index: usize) -> bool {
    match rt.players.get(&voice) {
        Some(player) => player.playing && source_of(rt, player) == Some(index),
        None => false,
    }
}

/// One thing the run asked the audio thread to do.
///
/// Not `Copy`, unlike the requests above: a play carries the clip it starts, and that clip is an
/// [`Arc`] because the callback is being handed something the main thread parsed and must not free
/// while it is being rendered.
enum Command {
    Play {
        voice: u32,
        clip: Arc<Clip>,
    },
    /// Play a pre-decoded mono PCM clip. `rate` is the source sample rate; the mixer resamples it to
    /// its own rate and applies the same tempo/rate factors as a MIDI voice.
    Pcm {
        voice: u32,
        samples: Arc<Vec<f32>>,
        rate: u32,
    },
    /// A PCM source that renders itself in blocks (the SMAF player).
    PcmSource {
        voice: u32,
        source: PcmSource,
    },
    Stop {
        voice: u32,
    },
    /// Set one voice's gain, in the platform's 0..`AEE_MAX_VOLUME` scale.
    Volume {
        voice: u32,
        volume: u32,
    },
    /// The synthesizer-wide tempo, in percent. Every voice follows it.
    Tempo(u32),
    /// The whole mixer's playback rate, in thousandths (1000 = real time). Unlike [`Command::Tempo`],
    /// which moves note events and leaves pitch alone, this resamples every voice's output, so tempo
    /// and pitch move together — the tape-speed meaning of "speed".
    Speed(u32),
    /// One voice's `MM_PARM_RATE`, as numerator/denominator.
    Rate {
        voice: u32,
        num: u32,
        den: u32,
    },
    /// One voice's `ISoundPlayer::SetTune` step, in semitones from the untouched pitch.
    Tune {
        voice: u32,
        step: i32,
    },
    /// Sound a tone for this `ISound` object. The two frequencies are hertz, `lo` 0 for a single
    /// one; the object address is the ownership the single generator is handed to.
    Tone {
        object: u32,
        hi: u16,
        lo: u16,
        volume: u32,
    },
    /// One raw `MM_PARM_MIDI_MSG` packet for a voice's synthesizer.
    Midi {
        voice: u32,
        bytes: Vec<u8>,
    },
    /// One voice's media window and direction.
    Window {
        voice: u32,
        start: u32,
        stop: u32,
        reverse: bool,
    },
}

/// How many source frames a reverse window materialises per `Player::render` call while it is
/// built. Small enough that the scratch buffer stays tiny, large enough not to call the mixer once
/// per sample.
const WINDOW_CHUNK: usize = 256;

/// Where a PCM voice reads from. A decoded QCP/MP3 clip is a buffer; a SMAF file is a score player
/// that renders in blocks, so a long file never becomes one giant `Vec<f32>` on the main thread.
///
/// A media window ([`PcmSource::set_window`]) is part of the source rather than of the mixer
/// because the two kinds answer it differently: a decoded buffer is a sample offset and a
/// direction, while a score has to be replayed — its voices at 20 s are the ones 20 s of rendering
/// leave behind. Both answer it *here*, on the caller's thread, not on the audio callback.
enum PcmSource {
    Buffer {
        samples: Arc<Vec<f32>>,
        at: f64,
        rate: u32,
        /// The window's end in source samples; negative means "to the clip's end".
        stop: f64,
        /// Play from `at` down to `stop`, which is what `MM_PARM_POS` asks for when its start is
        /// past its stop.
        reverse: bool,
    },
    Smaf {
        player: Box<SmafPlayer>,
        /// Mono frames rendered ahead of the mixer's resampler.
        buffer: Vec<f32>,
        /// Interleaved stereo scratch for one `Player::render` call.
        scratch: Vec<f32>,
        /// Fractional read position into `buffer`.
        at: f64,
        rate: u32,
        /// The window's end in milliseconds. `None` is the source's own construction (play to the
        /// clip's end); `Some` is a window, and a zero-length one is a window that ends at once —
        /// the two must not share a value, because `MM_PARM_POS` may well say `start == stop`.
        stop: Option<u32>,
        /// The media position this source is about to emit, in source frames. A resampled read
        /// advances it by `step`, which is exactly how media time moves, so the stop test is a
        /// comparison against this and never against how much has been *rendered* ahead of it.
        pos: f64,
        ended: bool,
        /// `buffer` holds a whole window rendered forward and mirrored; play it once and stop.
        reversed: bool,
    },
}

impl PcmSource {
    fn smaf(player: SmafPlayer) -> Self {
        let rate = player.rate();
        Self::Smaf {
            player: Box::new(player),
            buffer: Vec::with_capacity(512),
            scratch: vec![0.0; 512 * 2],
            at: 0.0,
            rate,
            stop: None,
            pos: 0.0,
            ended: false,
            reversed: false,
        }
    }

    fn rate(&self) -> u32 {
        match self {
            PcmSource::Buffer { rate, .. } | PcmSource::Smaf { rate, .. } => *rate,
        }
    }

    /// Place this source on a media window: a start and stop in milliseconds, and `reverse` when
    /// `MM_PARM_POS`'s start is past its stop.
    ///
    /// **The replay this can cost belongs off the audio thread.** A score's window is achieved by
    /// rendering the score up to it (and, in reverse, by rendering the window and mirroring it), and
    /// only a deterministic render can put a sustaining voice's envelope where a continuous one
    /// would have left it. [`Speaker::follow`] applies it on the frame thread and hands the mixer a
    /// finished source; a window that somehow still reaches the callback is answered here, just not
    /// cheaply.
    fn set_window(&mut self, start_ms: u32, stop_ms: u32, reverse: bool) {
        match self {
            PcmSource::Buffer {
                at,
                rate,
                stop,
                reverse: dir,
                ..
            } => {
                let per_ms = f64::from(*rate) / 1000.0;
                *at = f64::from(start_ms) * per_ms;
                *stop = f64::from(stop_ms) * per_ms;
                *dir = reverse && stop_ms < start_ms;
            }
            PcmSource::Smaf {
                player,
                buffer,
                scratch,
                at,
                rate,
                stop,
                pos,
                ended,
                reversed,
            } => {
                buffer.clear();
                *at = 0.0;
                *pos = f64::from(start_ms) * f64::from(*rate) / 1000.0;
                *ended = false;
                if reverse && stop_ms < start_ms {
                    // The window as written is [stop, start]; render it forward from the low end
                    // and mirror it, which is the same samples a continuous forward render would
                    // have produced, played the other way round.
                    let want = ((u64::from(start_ms - stop_ms) * u64::from(*rate)) / 1000) as usize;
                    player.seek_ms(stop_ms);
                    while buffer.len() < want {
                        let chunk = (want - buffer.len()).min(WINDOW_CHUNK);
                        let made = player.render(&mut scratch[..chunk * 2]);
                        if made == 0 {
                            break;
                        }
                        for frame in 0..made {
                            buffer.push((scratch[frame * 2] + scratch[frame * 2 + 1]) * 0.5);
                        }
                    }
                    buffer.truncate(want);
                    buffer.reverse();
                    *reversed = true;
                    *stop = None;
                } else {
                    player.seek_ms(start_ms);
                    *reversed = false;
                    *stop = Some(stop_ms);
                }
            }
        }
    }

    /// One source sample, with the caller's resampling step. The mixer multiplies the device rate,
    /// tempo and per-voice rate into `step`; the source itself stays at its own rate.
    fn next(&mut self, step: f64) -> f32 {
        match self {
            PcmSource::Buffer {
                samples,
                at,
                stop,
                reverse,
                ..
            } => {
                if *reverse {
                    if *at < *stop {
                        return 0.0;
                    }
                } else if *stop >= 0.0 && *at >= *stop {
                    return 0.0;
                }
                let index = *at as usize;
                let value = if index + 1 < samples.len() {
                    let frac = (*at - index as f64) as f32;
                    samples[index] * (1.0 - frac) + samples[index + 1] * frac
                } else {
                    samples.get(index).copied().unwrap_or(0.0)
                };
                *at += if *reverse { -step } else { step };
                value
            }
            PcmSource::Smaf {
                player,
                buffer,
                scratch,
                at,
                rate,
                stop,
                pos,
                ended,
                reversed,
            } => {
                if *ended {
                    return 0.0;
                }
                if *reversed {
                    let index = *at as usize;
                    if index >= buffer.len() {
                        *ended = true;
                        return 0.0;
                    }
                    let value = if index + 1 < buffer.len() {
                        let frac = (*at - index as f64) as f32;
                        buffer[index] * (1.0 - frac) + buffer[index + 1] * frac
                    } else {
                        buffer[index]
                    };
                    *at += step;
                    return value;
                }
                // The stop is a point in media time, tested before a sample is emitted rather
                // than after a block is rendered: testing after would let the last rendered block
                // play past the window by up to its own length, and a zero-length window would
                // play a whole block instead of nothing.
                if let Some(stop) = *stop {
                    if *pos >= f64::from(stop) * f64::from(*rate) / 1000.0 {
                        *ended = true;
                        return 0.0;
                    }
                }
                if *at as usize + 1 >= buffer.len() {
                    buffer.clear();
                    *at = 0.0;
                    let produced = player.render(scratch);
                    if produced == 0 {
                        *ended = true;
                        return 0.0;
                    }
                    for frame in 0..produced {
                        buffer.push((scratch[frame * 2] + scratch[frame * 2 + 1]) * 0.5);
                    }
                }
                let index = *at as usize;
                let value = if index + 1 < buffer.len() {
                    let frac = (*at - index as f64) as f32;
                    buffer[index] * (1.0 - frac) + buffer[index + 1] * frac
                } else {
                    buffer.get(index).copied().unwrap_or(0.0)
                };
                *at += step;
                *pos += step;
                value
            }
        }
    }
}

/// A fresh SMAF source already sitting on a media window.
///
/// `MM_PARM_POS` names a point in the *media*, and for a score that point is state: an FM voice's
/// envelope at 20 s is whatever 20 s of rendering left it at. So the window is applied here, on the
/// thread that watched the run ask for it, and the audio thread receives a source with nothing left
/// to compute before its first sample.
fn smaf_window_source(
    score: &Arc<crate::audio::smaf::sequencer::Score>,
    start: u32,
    stop: u32,
    reverse: bool,
) -> PcmSource {
    let mut source = PcmSource::smaf(SmafPlayer::new(
        (**score).clone(),
        crate::audio::smaf::SAMPLE_RATE,
    ));
    source.set_window(start, stop, reverse);
    source
}

/// One slot of the mixer.
///
/// A [`Synthesizer`] per voice rather than one shared, because that is what mixing means here: two
/// clips are two sets of notes on two instruments, and the sum of their samples is what a handset's
/// audio hardware does. Every slot is built at construction rather than on demand, because building
/// one allocates and the audio thread is the one place that must not.
struct Voice {
    /// The `ISoundPlayer` this slot is speaking for, or 0 when the slot is free.
    id: u32,
    sequencer: Sequencer,
    /// This voice's `MM_PARM_RATE` factor; the shared tempo is multiplied by it.
    rate: f64,
    /// This voice's `ISoundPlayer::SetTune` pitch factor, 1.0 for an untouched pitch.
    ///
    /// A score moves its notes instead (see [`Sequencer::set_tune`]); a PCM or SMAF voice has no
    /// notes and is resampled by this, which is the same thing a handset's codec does with a tune.
    tune: f64,
    /// This player's `SetVolume` gain, already normalized to 0.0..=1.0.
    volume: f32,
    /// When this slot was assigned, for `QUOIN_AUDIO_TRACE`; the first non-silent frame clears it.
    trace_at: Option<Instant>,
    /// When this slot was last given a clip, so that a pack asking for more voices than there are
    /// slots takes the one furthest along. The oldest is the least audible choice available.
    age: u64,
    /// A decoded PCM clip or a streaming SMAF player, when this slot speaks PCM rather than MIDI.
    pcm: Option<PcmSource>,
    /// The resampler's fractional read position into the source stream, and the last source sample
    /// emitted on each side. Only read when the mixer's speed is not 1.0, where a block boundary
    /// must not interrupt the interpolation.
    phase: f64,
    last_l: f32,
    last_r: f32,
    /// The source sample just past [`Self::last_l`] on each side. Kept so the next block starts its
    /// interpolation between the two instead of jumping over a sample and clicking once per block.
    carry_l: f32,
    carry_r: f32,
    has_carry: bool,
}

/// The mixer: eight slots, and the arithmetic that turns them into samples.
///
/// **This is the half of the speaker that has nothing to do with a platform.** It was inside the
/// cpal callback, which cost two things: a run with no sound card could not reach it (`--dump-audio`
/// hands a person raw bytes and no way to listen, and `render_clip` — the one offline path — grew its
/// own synthesizer rather than sharing this math, so a change here would not show up there), and
/// every new backend had to re-implement the summing, the slot-stealing and the clamping. Both are
/// now one implementation with two drivers: [`build`] feeds it from a device callback, and
/// [`render_clip`] / [`render_wav`] feed it a fixed block with no device in sight.
///
/// It allocates everything up front and nothing while rendering, which is the property that lets the
/// live driver call it from inside a callback at all.
pub struct Mixer {
    voices: Vec<Voice>,
    /// What the run has asked for and this mixer has not yet acted on.
    ///
    /// The offline mixer is handed a receiver whose sender was dropped, so draining it finds nothing
    /// and returns - which is cheaper and clearer than an `Option` that every use of `self` would have
    /// to unwrap around while also mutably borrowing the slots.
    rx: mpsc::Receiver<Command>,
    /// Current shared tempo factor (1.0 = 100 %), multiplied by each voice's `rate`.
    tempo: f64,
    /// One shared scratch pair for a single voice's contribution, reused by every slot in turn and
    /// summed into the caller's destination. `BLOCK` samples on both drivers: `render_into` chunks, so
    /// a sixty-second render does not need a sixty-second scratch alongside it.
    one_left: Vec<f32>,
    one_right: Vec<f32>,
    clock: u64,
    /// The mixer's own sample rate, used to resample PCM voices.
    sample_rate: f64,
    /// Whole-mixer playback rate, 1.0 = real time. Anything else resamples each voice's output, so
    /// tempo and pitch move together.
    speed: f64,
    /// Source samples for one voice before resampling. Sized for [`MAX_SPEED`] plus the two samples
    /// a linear interpolation needs at the edges — allocated once, because the device callback must
    /// not allocate.
    src_left: Vec<f32>,
    src_right: Vec<f32>,
    /// Whether the development-only output timing trace is on.
    trace: bool,
    /// The tone generator, with the `ISound` object it belongs to.
    ///
    /// One gen, not one per object, because a handset has one tone path: a second object asking for
    /// a tone takes it from the first, exactly as a second `PlayTone` on a real handset does. The
    /// owner is what makes `StopTone` mean "the tone *I* started" rather than "any tone".
    tone: Option<(u32, crate::audio::tone::ToneGen)>,
}

impl Mixer {
    /// A mixer a device callback will drive: `VOICES` slots built eagerly, scratch at the block size.
    fn live(
        rx: mpsc::Receiver<Command>,
        sample_rate: i32,
        sound_font: &Arc<SoundFont>,
    ) -> Result<Self, String> {
        let settings = SynthesizerSettings::new(sample_rate);
        let mut voices = Vec::with_capacity(VOICES);
        for _ in 0..VOICES {
            let synthesizer = Synthesizer::new(sound_font, &settings)
                .map_err(|e| format!("cannot build a synthesizer: {e}"))?;
            voices.push(Voice {
                id: 0,
                sequencer: Sequencer::new(synthesizer),
                rate: 1.0,
                volume: 1.0,
                tune: 1.0,
                trace_at: None,
                age: 0,
                pcm: None,
                phase: 0.0,
                last_l: 0.0,
                last_r: 0.0,
                carry_l: 0.0,
                carry_r: 0.0,
                has_carry: false,
            });
        }
        Ok(Mixer {
            voices,
            rx,
            tempo: 1.0,
            trace: audio_trace_enabled(),
            tone: None,
            one_left: vec![0.0f32; BLOCK],
            one_right: vec![0.0f32; BLOCK],
            clock: 0,
            sample_rate: f64::from(sample_rate),
            speed: 1.0,
            src_left: vec![0.0; BLOCK * MAX_SPEED + 8],
            src_right: vec![0.0; BLOCK * MAX_SPEED + 8],
        })
    }

    /// A mixer with no device behind it: one slot, nothing to drain, and scratch big enough for the
    /// whole render. This is what lets "what does clip 7 sound like" be answered on a machine that has
    /// no sound card, through the same summing the live path uses.
    fn offline(sample_rate: i32, sound_font: &Arc<SoundFont>) -> Result<Self, String> {
        let synthesizer = Synthesizer::new(sound_font, &SynthesizerSettings::new(sample_rate))
            .map_err(|e| format!("cannot build a synthesizer: {e}"))?;
        Ok(Mixer {
            voices: vec![Voice {
                id: 0,
                sequencer: Sequencer::new(synthesizer),
                rate: 1.0,
                volume: 1.0,
                tune: 1.0,
                trace_at: None,
                age: 0,
                pcm: None,
                phase: 0.0,
                last_l: 0.0,
                last_r: 0.0,
                carry_l: 0.0,
                carry_r: 0.0,
                has_carry: false,
            }],
            // The sender is dropped on the spot: draining this finds a closed channel and stops.
            rx: mpsc::channel::<Command>().1,
            tempo: 1.0,
            trace: audio_trace_enabled(),
            tone: None,
            // `BLOCK`, not `frames`: `render_into` chunks, so a sixty-second render does not need a
            // sixty-second scratch alongside it. Sizing this to the render was my first shape here and
            // it quietly doubled the memory a `--dump-wav` of a long clip holds.
            one_left: vec![0.0f32; BLOCK],
            one_right: vec![0.0f32; BLOCK],
            clock: 0,
            sample_rate: f64::from(sample_rate),
            speed: 1.0,
            src_left: vec![0.0; BLOCK * MAX_SPEED + 8],
            src_right: vec![0.0; BLOCK * MAX_SPEED + 8],
        })
    }

    /// Take everything the run asked for since the last block. Called before rendering, so a play
    /// that arrived during the last buffer is heard from this one's first frame.
    fn drain(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Play { voice, clip } => {
                    if self.trace {
                        audio_trace(format_args!("drain play voice={voice:#010x}"));
                    }
                    self.assign(voice, &clip);
                }
                Command::Pcm {
                    voice,
                    samples,
                    rate,
                } => self.assign_source(
                    voice,
                    PcmSource::Buffer {
                        samples,
                        at: 0.0,
                        rate: rate.max(1),
                        stop: -1.0,
                        reverse: false,
                    },
                ),
                Command::PcmSource { voice, source } => self.assign_source(voice, source),
                Command::Stop { voice } => self.stop(voice),
                Command::Volume { voice, volume } => self.set_volume(voice, volume),
                Command::Tempo(percent) => self.set_tempo(percent),
                Command::Speed(permille) => self.set_speed(permille),
                Command::Rate { voice, num, den } => self.set_rate(voice, num, den),
                Command::Tune { voice, step } => self.set_tune(voice, step),
                Command::Tone {
                    object,
                    hi,
                    lo,
                    volume,
                } => self.tone(object, hi, lo, volume),
                Command::Midi { voice, bytes } => self.midi(voice, &bytes),
                Command::Window {
                    voice,
                    start,
                    stop,
                    reverse,
                } => self.set_window(voice, start, stop, reverse),
            }
        }
    }

    /// Set the factor every slot plays at, in percent with 100 as normal speed.
    ///
    /// The runtime has already filtered to the documented range before posting a command; the clamp
    /// here is only a guard for a value that did not come through that door. The shared factor is
    /// multiplied by each voice's own `MM_PARM_RATE`; the offline mixer has no rate commands and
    /// keeps its 1.0.
    fn set_tempo(&mut self, percent: u32) {
        self.tempo = percent.clamp(MIN_TEMPO, MAX_TEMPO) as f64 / 100.0;
        for slot in &mut self.voices {
            slot.sequencer.set_speed(self.tempo * slot.rate);
        }
    }

    /// Set the whole mixer's playback rate, in thousandths of real time.
    ///
    /// This is the tape-speed control, not the tempo one: it resamples every voice's output, so 2x
    /// plays twice as fast *and* an octave higher. [`Self::set_tempo`] moves note events and leaves
    /// pitch alone, which is why the two are separate commands rather than one factor. The
    /// resampler's phase is deliberately left where it is: the stream is continuous, only its rate
    /// changed.
    fn set_speed(&mut self, permille: u32) {
        self.speed = (f64::from(permille) / 1000.0).clamp(0.25, MAX_SPEED as f64);
    }

    /// Silence one voice, and give up the tone generator if this object owns it.
    ///
    /// A handset's tone path belongs to one `ISound` at a time, so a stop is only a stop for the
    /// object that started it: another object's `StopTone` must not silence a tone it never asked
    /// for.
    fn stop(&mut self, voice: u32) {
        for slot in self.voices.iter_mut() {
            if slot.id == voice {
                slot.sequencer.stop();
                slot.pcm = None;
                slot.id = 0;
            }
        }
        if let Some((owner, tone)) = self.tone.as_mut() {
            if *owner == voice {
                tone.release();
            }
        }
    }

    /// Set one voice's `ISoundPlayer::SetVolume` gain. The runtime has already clamped the value to
    /// `AEE_MAX_VOLUME`; the clamp here is the same guard for a value that came from another door.
    fn set_volume(&mut self, voice: u32, volume: u32) {
        let gain = volume.min(g::AEE_MAX_VOLUME) as f32 / 100.0;
        for slot in self.voices.iter_mut().filter(|slot| slot.id == voice) {
            slot.volume = gain;
        }
    }

    /// Set one voice's `MM_PARM_RATE`; zero denominator is the header's 1.
    fn set_rate(&mut self, voice: u32, num: u32, den: u32) {
        let rate = if den == 0 {
            1.0
        } else {
            f64::from(num) / f64::from(den)
        };
        if let Some(slot) = self.voices.iter_mut().find(|slot| slot.id == voice) {
            slot.rate = rate;
            slot.sequencer.set_speed(self.tempo * rate);
        }
    }

    /// Hand one live MIDI packet to a voice's synthesizer, if that voice is assigned.
    fn midi(&mut self, voice: u32, bytes: &[u8]) {
        if let Some(slot) = self.voices.iter_mut().find(|slot| slot.id == voice) {
            slot.sequencer.midi(bytes);
        }
    }

    /// Set one voice's `ISoundPlayer::SetTune` step, in semitones from the untouched pitch.
    ///
    /// The two halves of the mixer answer this in the two ways that are exact for them. A score is
    /// **transposed**: the key number of every note event moves and the timeline does not, which is
    /// what a tune is and why it cannot be a rate. A PCM or SMAF voice has no notes to move — its
    /// pitch is its sample rate — so the step becomes a resampling factor, and the clip runs at the
    /// interval's speed as well as its pitch. That is what a handset's codec does with a tune; a
    /// host that wanted the pitch *without* the time would have to be a pitch shifter, and this is
    /// not one.
    fn set_tune(&mut self, voice: u32, step: i32) {
        let factor = crate::audio::tone::tune_factor(step);
        if let Some(slot) = self.voices.iter_mut().find(|slot| slot.id == voice) {
            slot.tune = factor;
            slot.sequencer.set_tune(step);
        }
    }

    /// Sound a tone, or re-level the one already sounding.
    ///
    /// The same frequencies from the same owner are a level change, not a new tone: a pack that
    /// re-asks on every tick, or a `PlayToneList` repeating an id, must not restart the beep. A
    /// different pair is a different tone and starts its phase from the top.
    fn tone(&mut self, object: u32, hi: u16, lo: u16, volume: u32) {
        match self.tone.as_mut() {
            Some((owner, gen)) if *owner == object && gen.is(hi, lo) => gen.set_volume(volume),
            _ => {
                self.tone = Some((
                    object,
                    crate::audio::tone::ToneGen::new(hi, lo, self.sample_rate, volume),
                ));
            }
        }
    }

    /// Set one voice's media window and direction.
    ///
    /// A PCM source answers this itself: a decoded buffer seeks, and a score replays. The score
    /// case normally never reaches this thread — [`Speaker::follow`] builds the windowed source on
    /// the frame thread and sends it whole, so the callback only swaps an enum.
    fn set_window(&mut self, voice: u32, start: u32, stop: u32, reverse: bool) {
        if let Some(slot) = self.voices.iter_mut().find(|slot| slot.id == voice) {
            if let Some(source) = slot.pcm.as_mut() {
                source.set_window(start, stop, reverse);
            } else {
                slot.sequencer.set_window(start, stop, reverse);
            }
        }
    }

    /// Forget where the resampler was, for a slot that has just been given a different stream.
    ///
    /// `last_*` and `carry_*` are the *previous* stream's samples, and the anything-but-1× path
    /// interpolates the new stream's first frame between them. That is a click at every change of
    /// clip, and it is a leftover rather than a crossfade: nothing here ever meant to blend two
    /// clips, only to keep one clip's own block boundaries smooth.
    fn reset_resampler(&mut self, slot: usize) {
        let voice = &mut self.voices[slot];
        voice.phase = 0.0;
        voice.last_l = 0.0;
        voice.last_r = 0.0;
        voice.carry_l = 0.0;
        voice.carry_r = 0.0;
        voice.has_carry = false;
    }

    /// Put a clip on a voice: the slot already holding that `ISoundPlayer`, or a free one, or the one
    /// that has been playing longest.
    ///
    /// One pass, not a loop, because that is what a handset does and not a choice this end gets to
    /// make: `Play` takes nothing but the object, so there is no flag a pack could set to ask for one
    /// or the other. A pack that wants its music to continue asks again — one `Play` per `TICK_UPDATE`,
    /// once a second — and `requests` is what keeps those repeats from restarting a
    /// clip that is already playing.
    fn assign(&mut self, voice: u32, clip: &Arc<Clip>) {
        let slot = match self.voices.iter().position(|v| v.id == voice) {
            Some(i) => i,
            None => {
                let free = self.voices.iter().position(|v| v.id == 0);
                let i = free.unwrap_or_else(|| {
                    let mut oldest = 0;
                    for j in 1..self.voices.len() {
                        if self.voices[j].age < self.voices[oldest].age {
                            oldest = j;
                        }
                    }
                    oldest
                });
                self.voices[i].sequencer.stop();
                i
            }
        };
        self.clock += 1;
        if self.voices[slot].id != voice {
            // A slot taken over from another voice starts at the header's defaults, which is what a
            // handset does with a stolen channel. The front end cannot see this — it happens inside
            // the callback — so `requests` drops its own record of the gain and the pitch at the
            // same moment it asks for a play, or a voice that was stolen would go on playing at the
            // previous owner's level and tuning.
            self.voices[slot].rate = 1.0;
            self.voices[slot].volume = 1.0;
            self.voices[slot].tune = 1.0;
            self.voices[slot].sequencer.set_tune(0);
        }
        self.voices[slot].id = voice;
        self.voices[slot].age = self.clock;
        let rate = self.voices[slot].rate;
        self.voices[slot].pcm = None;
        self.reset_resampler(slot);
        self.voices[slot].sequencer.set_speed(self.tempo * rate);
        self.voices[slot].sequencer.play(clip);
        if self.trace {
            let at = Instant::now();
            self.voices[slot].trace_at = Some(at);
            audio_trace(format_args!(
                "assign midi voice={voice:#010x} slot={slot} at={}",
                at.elapsed().as_micros()
            ));
        }
    }

    /// Put decoded PCM on a voice. The same slot policy as [`Mixer::assign`], but this stops the
    /// sequencer: a QCP clip is not a score and must not leave a previous MIDI note ringing.
    fn assign_source(&mut self, voice: u32, source: PcmSource) {
        let slot = match self.voices.iter().position(|v| v.id == voice) {
            Some(i) => i,
            None => {
                let free = self.voices.iter().position(|v| v.id == 0);
                free.unwrap_or_else(|| {
                    let mut oldest = 0;
                    for j in 1..self.voices.len() {
                        if self.voices[j].age < self.voices[oldest].age {
                            oldest = j;
                        }
                    }
                    oldest
                })
            }
        };
        self.clock += 1;
        // A slot taken over from another voice starts at the header's default rate. A source
        // replaced on the voice that already owns the slot keeps its `MM_PARM_RATE`: the speaker
        // hands a windowed SMAF source to the very slot it is playing on, after the rate was set,
        // and resetting here would silently undo it.
        let same_voice = self.voices[slot].id == voice;
        self.voices[slot].sequencer.stop();
        self.voices[slot].id = voice;
        self.voices[slot].age = self.clock;
        if !same_voice {
            self.voices[slot].rate = 1.0;
            self.voices[slot].volume = 1.0;
            self.voices[slot].tune = 1.0;
            self.voices[slot].sequencer.set_tune(0);
        }
        self.voices[slot].pcm = Some(source);
        self.reset_resampler(slot);
        if self.trace {
            let at = Instant::now();
            self.voices[slot].trace_at = Some(at);
            audio_trace(format_args!(
                "assign pcm voice={voice:#010x} slot={slot} at={}",
                at.elapsed().as_micros()
            ));
        }
    }

    /// Sum the active slots into the caller's destination, which must be the same length on both
    /// sides. A voice that has run out of clip renders silence and stays in the pool: it is stopped
    /// by the pack, not by the end of its track.
    ///
    /// **The output is clamped, and that belongs here rather than in either driver** — several voices
    /// at once can add up past full scale, and a device handed more than that is a device that
    /// crackles. Clamping in the callback only would leave the offline path writing values that
    /// saturate differently, which is exactly the two-authorities split this struct exists to close.
    ///
    /// Saturating rather than limiting is deliberate, and it is the *emulated* behaviour rather than
    /// the convenient one: a handset's mixer saturates, so a pack that mixes eight voices at full
    /// scale distorted on the device it was written for, and a host that compressed instead would
    /// sound better than the thing it is standing in for — which is the one thing an HLE must not
    /// do. A compressor is a feature, not a correction: it would need an attack, a release and a
    /// threshold to argue about, and it would change the sample values every existing render test
    /// pins. What would make it worth adding is a pack whose normal mix clips, and no sample pack
    /// has been measured doing that.
    fn render_into(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(
            left.len(),
            right.len(),
            "one interleaved frame count both ways"
        );
        let mut done = 0;
        while done < left.len() {
            let n = (left.len() - done).min(self.one_left.len());
            let (l, r) = (&mut left[done..done + n], &mut right[done..done + n]);
            l.fill(0.0);
            r.fill(0.0);
            for slot in self.voices.iter_mut().filter(|v| v.id != 0) {
                let gain = slot.volume;
                if self.speed == 1.0 {
                    let (ol, or) = (&mut self.one_left[..n], &mut self.one_right[..n]);
                    if let Some(source) = slot.pcm.as_mut() {
                        let step = f64::from(source.rate()) / self.sample_rate
                            * self.tempo
                            * slot.rate
                            * slot.tune;
                        for i in 0..n {
                            let value = source.next(step);
                            ol[i] = value;
                            or[i] = value;
                        }
                    } else {
                        slot.sequencer.render(ol, or);
                    }
                } else {
                    // The path for anything but 1×: render the source samples first, then send them
                    // back through `one_*` by linear interpolation at the global rate. Unlike `tempo`
                    // (which moves events and leaves pitch alone) this is tape-style: speed and pitch
                    // move together.
                    //
                    // `src[0]` is the last sample of the previous block; with `has_carry` `src[1]` is
                    // the one right after it, so interpolation across a block boundary stays at the same
                    // fractional position instead of clicking once per block. `need` is `n * speed` plus
                    // the interpolation edge; `src` is already sized for `MAX_SPEED`, so the clamp is
                    // insurance rather than a budget.
                    let base = if slot.has_carry { 2usize } else { 1 };
                    let need = ((slot.phase + n as f64 * self.speed).floor() as usize + 2)
                        .min(self.src_left.len() - base);
                    self.src_left[0] = slot.last_l;
                    self.src_right[0] = slot.last_r;
                    if slot.has_carry {
                        self.src_left[1] = slot.carry_l;
                        self.src_right[1] = slot.carry_r;
                    }
                    {
                        let (sl, sr) = (
                            &mut self.src_left[base..base + need],
                            &mut self.src_right[base..base + need],
                        );
                        if let Some(source) = slot.pcm.as_mut() {
                            let step = f64::from(source.rate()) / self.sample_rate
                                * self.tempo
                                * slot.rate
                                * slot.tune;
                            for i in 0..need {
                                let value = source.next(step);
                                sl[i] = value;
                                sr[i] = value;
                            }
                        } else {
                            slot.sequencer.render(sl, sr);
                        }
                    }
                    // With a carry the start lands between `last` (src[0]) and `carry` (src[1]), so
                    // indexing starts at 0; the first block has no `last` and starts at the first real
                    // sample (src[1]).
                    let mut idx = if slot.has_carry { 0 } else { 1 };
                    let mut pos = slot.phase;
                    for i in 0..n {
                        while pos >= 1.0 {
                            idx += 1;
                            pos -= 1.0;
                        }
                        let (a_l, b_l) = (self.src_left[idx], self.src_left[idx + 1]);
                        let (a_r, b_r) = (self.src_right[idx], self.src_right[idx + 1]);
                        self.one_left[i] = a_l + (b_l - a_l) * pos as f32;
                        self.one_right[i] = a_r + (b_r - a_r) * pos as f32;
                        pos += self.speed;
                    }
                    while pos >= 1.0 {
                        idx += 1;
                        pos -= 1.0;
                    }
                    slot.last_l = self.src_left[idx];
                    slot.last_r = self.src_right[idx];
                    slot.carry_l = self.src_left[idx + 1];
                    slot.carry_r = self.src_right[idx + 1];
                    slot.has_carry = true;
                    slot.phase = pos;
                }
                if let Some(started) = slot.trace_at {
                    let audible = (0..n).any(|i| {
                        (self.one_left[i] * gain).abs() > 1.0e-4
                            || (self.one_right[i] * gain).abs() > 1.0e-4
                    });
                    if audible {
                        slot.trace_at = None;
                        audio_trace(format_args!(
                            "first sound voice={:#010x} slot_voice={} since_assign={}us",
                            slot.id,
                            slot.id,
                            started.elapsed().as_micros()
                        ));
                    }
                }
                for i in 0..n {
                    self.one_left[i] *= gain;
                    self.one_right[i] *= gain;
                    l[i] += self.one_left[i];
                    r[i] += self.one_right[i];
                }
            }
            // The tone generator comes after the voices because it is not one of them — a handset's
            // beeper is a separate path summed into the same output — and before the clamp because
            // the clamp is the one place that defines full scale for both drivers.
            let faded = match self.tone.as_mut() {
                Some((_, tone)) => tone.render(l, r),
                None => false,
            };
            if faded {
                self.tone = None;
            }
            for (a, b) in l.iter_mut().zip(r.iter_mut()) {
                *a = a.clamp(-1.0, 1.0);
                *b = b.clamp(-1.0, 1.0);
            }
            done += n;
        }
    }
}

/// Load the tone bank a run renders with.
///
/// `None` is not "no bank": it is [`crate::audio::soundbank`], the tables this crate generates itself. A
/// real handset needed no file from its owner to play a `.mid` — the LG MX7000 image carries no
/// sampled bank at all and still plays MIDI out of tables inside the flash — and a host that can
/// only make sound when handed a third-party `.sf2` is a host that has made the player's convenience
/// a platform requirement. `--soundfont` still wins when given, because a person who has a real GM
/// bank should hear the pack through it.
fn load(soundfont: Option<&Path>) -> Result<(Arc<SoundFont>, String), String> {
    match soundfont {
        Some(path) => {
            let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let font = SoundFont::new(&mut BufReader::new(file))
                .map_err(|e| format!("{}: not a SoundFont: {e}", path.display()))?;
            Ok((Arc::new(font), path.display().to_string()))
        }
        None => {
            let image = crate::audio::soundbank::image();
            let font = SoundFont::new(&mut Cursor::new(image))
                .map_err(|e| format!("the built-in tone bank does not parse: {e}"))?;
            Ok((Arc::new(font), "built-in tone tables".to_string()))
        }
    }
}

/// What the helper thread found in a clip, so that the frame thread only has to file it.
enum DecodeOutcome {
    /// The bytes were audio, and the decoder produced samples.
    Pcm(crate::audio::media_decode::Decoded),
    /// The bytes were not audio but were a Standard MIDI File — the other thing a play can name,
    /// and the reason a failed decode is not a failed clip.
    Midi(Arc<Clip>),
    /// Neither. One sentence naming both attempts, because a person reading a report wants to know
    /// that the audio decoder was tried and what it said, not only that the clip did not play.
    Failed(String),
}

/// One clip handed to the helper thread.
struct DecodeJob {
    index: usize,
    name: String,
    /// The source fingerprint at the moment of submission. The runtime's audio vector can be
    /// renamed under a play, and a result that no longer describes the index must not be filed.
    fingerprint: u64,
    bytes: Vec<u8>,
    backend: AudioBackend,
}

/// One finished decode, carrying enough to be filed under the right clip.
struct DecodeDone {
    index: usize,
    name: String,
    fingerprint: u64,
    outcome: DecodeOutcome,
}

/// The thread that decodes large sources, and the two channels that reach it.
///
/// Dropping this closes the job channel, which is what ends the thread: it is not joined, because
/// the speaker is dropped on the frame thread and a decode in flight is not worth a stall at the
/// end of a run. What that decode would have produced is never read, and the process is usually
/// about to exit anyway.
struct DecodeWorker {
    jobs: mpsc::Sender<DecodeJob>,
    done: mpsc::Receiver<DecodeDone>,
}

impl DecodeWorker {
    fn new() -> Option<DecodeWorker> {
        let (jobs, job_rx) = mpsc::channel::<DecodeJob>();
        let (done_tx, done) = mpsc::channel::<DecodeDone>();
        std::thread::Builder::new()
            .name("quoin-decode".to_string())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let finished = DecodeDone {
                        index: job.index,
                        name: job.name.clone(),
                        fingerprint: job.fingerprint,
                        outcome: decode_job(&job),
                    };
                    // The speaker is gone: nothing will read this, and nothing else is waiting.
                    if done_tx.send(finished).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(DecodeWorker { jobs, done })
    }
}

/// Decode one job the way a play does: audio first, then MIDI.
///
/// The order is the play's own, kept here rather than written twice. A source is tried as audio and
/// only a source that is not audio is tried as a score — the other order would send every MP3
/// through a MIDI parser first, and `Clip::parse` is happy to accept a container's first bytes as
/// a header often enough for that to matter.
fn decode_job(job: &DecodeJob) -> DecodeOutcome {
    let audio = decode_with(job.backend, &job.bytes);
    match audio {
        Ok(decoded) if !decoded.samples.is_empty() => DecodeOutcome::Pcm(decoded),
        other => match Clip::parse(&job.bytes) {
            Ok(clip) => DecodeOutcome::Midi(Arc::new(clip)),
            Err(midi) => {
                let audio = match other {
                    Ok(_) => "the decoder produced no audio samples".to_string(),
                    Err(error) => error,
                };
                DecodeOutcome::Failed(format!(
                    "not a MIDI/QCP or {} audio file: MIDI: {midi}; audio: {audio}",
                    job.backend.name()
                ))
            }
        },
    }
}

/// Decode through the selected backend, with no speaker in the picture.
///
/// The Rust backend tries QCP first and Symphonia second; FFmpeg is compile-time gated and never
/// silently falls back to Rust. SMAF is not a Symphonia or FFmpeg format: the vendored engine
/// renders it directly, and the audio-backend choice has nothing to say about it.
fn decode_with(
    backend: AudioBackend,
    bytes: &[u8],
) -> Result<crate::audio::media_decode::Decoded, String> {
    use crate::audio::media_decode::Decoded;
    if crate::audio::smaf::looks_like(bytes) {
        return crate::audio::smaf::decode(bytes)
            .map(|(rate, samples)| Decoded::whole(rate, samples));
    }
    match backend {
        AudioBackend::Rust => match crate::audio::qcelp::decode_qcp(bytes) {
            Ok((rate, samples)) => Ok(Decoded::whole(rate, samples)),
            Err(_) => crate::audio::media_decode::decode(bytes),
        },
        AudioBackend::Ffmpeg => {
            #[cfg(feature = "ffmpeg")]
            {
                crate::audio::ffmpeg_decode::decode(bytes)
                    .map(|(rate, samples)| Decoded::whole(rate, samples))
            }
            #[cfg(not(feature = "ffmpeg"))]
            {
                Err("ffmpeg backend is not built into this binary".to_string())
            }
        }
    }
}

/// A run's clips, and the device they come out of.
///
/// Held for as long as the sound is wanted: the stream inside stops the moment this is dropped,
/// which is the usual reason a program that "plays nothing" turns out to have built a speaker and
/// let it go at the end of the function that made it.
pub struct Speaker {
    /// The only way to reach the mixer, which lives on the audio thread. The callback drains this
    /// once per buffer, so a play that arrives mid-buffer is heard at the start of the next one — a
    /// few milliseconds, and what a real device's queue does anyway.
    ///
    /// A channel rather than a `Mutex<[Voice]>`, and that is not a style choice: a lock taken in a
    /// realtime callback is a priority inversion waiting for a busy UI thread to happen, and this is
    /// the one place in the tree where that is true.
    tx: mpsc::Sender<Command>,
    /// The clips parsed so far, keyed by their index in [`Runtime::audio`].
    ///
    /// Filled lazily, on the main thread, the first time one is played — see the module docs for why
    /// it cannot be filled at construction.
    clips: HashMap<usize, Arc<Clip>>,
    /// QCP/MP3/WAV/… clips decoded to mono PCM, keyed the same way as `clips`, with their source
    /// sample rate.
    pcm: HashMap<usize, (Arc<Vec<f32>>, u32)>,
    /// The same clips in the order they were last asked for, oldest first. What [`PCM_BUDGET_SAMPLES`]
    /// leaves out of the cache is taken from the front.
    pcm_order: VecDeque<usize>,
    /// How many samples `pcm` holds altogether, kept in step with every insert and removal so the
    /// budget is a comparison rather than a walk of the map.
    pcm_samples: usize,
    /// The thread that decodes large sources, created the first time one is played.
    ///
    /// A thread rather than a queue the caller drains, because the caller *is* the frame clock: a
    /// decode that has to be pumped from the frame loop is a decode that still stops it.
    decoder: Option<DecodeWorker>,
    /// Whether a first play of a large source decodes off the caller's thread. See
    /// [`Speaker::set_async_decode`] for why this is a switch at all.
    async_decode: bool,
    /// Voices that asked for a clip the helper thread is still decoding, in the order they asked.
    waiting: Vec<(u32, usize)>,
    /// Clips already handed to the helper thread, so two voices asking at once submit one job and
    /// a second play of the same index does not queue behind itself.
    in_flight: HashSet<usize>,
    /// Parsed SMAF scores, cached by `Runtime::audio` index. Playing one makes a fresh `Player`
    /// from this score, so a repeated `Play` does not re-parse and no rendered buffer is kept.
    smaf_scores: HashMap<usize, Arc<crate::audio::smaf::sequencer::Score>>,
    /// Scores bound to live SMAF voices, keyed by the `ISoundPlayer` the sound is on. This is what
    /// lets a media window be built on this thread instead of the callback's: the score is the one
    /// thing the frame thread needs and the mixer already has.
    smaf_voices: HashMap<u32, Arc<crate::audio::smaf::sequencer::Score>>,
    /// Sources that turned out not to be MIDI files, so a parse is attempted once rather than once
    /// per play. A play of one of these is still a play the run made; this end simply has nothing.
    failed: HashSet<usize>,
    /// How many clips have been handed to the audio thread. Counted on this side rather than in the
    /// callback, so that a test can see it without waiting on a device that may not be there.
    requested: usize,
    /// Which tone bank is speaking, for the run's report: a file's path, or the built-in tables.
    /// Said out loud because "the game makes no sound" and "the game has no sound for the bank you
    /// meant to give it" look identical from outside a silent window.
    source: String,
    /// Fingerprint of the bytes last parsed for each audio index. The runtime's audio vector is a
    /// `Vec`, and a naming can remove/replace an entry; an index alone is not a stable clip identity.
    seen_sources: HashMap<usize, u64>,
    /// The decoder backend this speaker was built with; the GUI may change it for later clips.
    backend: Arc<AtomicU8>,
    /// What drives the mixer: the device's callback, or the caller. See [`Driver`].
    driver: Driver,
    /// What the speaker could not use, for the run's report. Collected rather than printed here,
    /// because a library has no business writing to a terminal.
    notes: Vec<String>,
}

impl std::fmt::Debug for Speaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Speaker")
            .field("clips", &self.clips.len())
            .field("requested", &self.requested)
            .field("notes", &self.notes)
            .finish_non_exhaustive()
    }
}

impl Speaker {
    /// Open a device, load the SoundFont, build the mixer, and start the stream.
    ///
    /// No clips are named here and none could be: this is called before the guest has run, so the
    /// run has not named a single source yet. See the module docs.
    ///
    /// The device is asked first on purpose: the synthesizers' sample rate has to be the device's
    /// own, or every clip comes out at the wrong pitch and the wrong speed, and the device is the
    /// only thing that knows what it runs at. Every failure after that is something the caller can
    /// report, because a run without sound is still a run.
    pub fn new(soundfont: Option<&Path>) -> Result<Speaker, String> {
        Self::new_with_backend(soundfont, AudioBackend::Rust)
    }

    /// Open a device with an explicit decoder backend. `backend` must be available in this build;
    /// the FFmpeg backend is reserved but not linked here.
    pub fn new_with_backend(
        soundfont: Option<&Path>,
        backend: AudioBackend,
    ) -> Result<Speaker, String> {
        if !backend.available() {
            return Err(format!(
                "audio backend {} is not built into this binary (available: rust)",
                backend.name()
            ));
        }
        Self::new_with_backend_handle(soundfont, Arc::new(AtomicU8::new(backend as u8)))
    }

    #[cfg(feature = "device")]
    pub(crate) fn new_with_backend_handle(
        soundfont: Option<&Path>,
        backend: Arc<AtomicU8>,
    ) -> Result<Speaker, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "this host has no default output device".to_string())?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("no default output config: {e}"))?;
        let sample_rate = supported.sample_rate() as i32;
        let channels = supported.channels() as usize;
        let config = supported.config();

        let (sound_font, source) = load(soundfont)?;
        let (tx, rx) = mpsc::channel::<Command>();
        let mixer = Mixer::live(rx, sample_rate, &sound_font)?;

        // Three arms rather than a generic `match` inside the callback: the conversion is the only
        // part that differs, and `FromSample` is what knows how to do it. `config` is cloned per arm
        // because `build_output_stream` takes it by value.
        let audible = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, config.clone(), channels, mixer),
            cpal::SampleFormat::I16 => build::<i16>(&device, config.clone(), channels, mixer),
            cpal::SampleFormat::U16 => build::<u16>(&device, config.clone(), channels, mixer),
            other => {
                return Err(format!(
                    "this host's output format {other:?} is not one this speaker renders"
                ))
            }
        }
        .map_err(|e| format!("cannot open an output stream: {e}"))?;
        audible
            .play()
            .map_err(|e| format!("cannot start the output stream: {e}"))?;

        Ok(Speaker {
            tx,
            clips: HashMap::new(),
            pcm: HashMap::new(),
            pcm_order: VecDeque::new(),
            pcm_samples: 0,
            decoder: None,
            // A real device means a real frame clock, and the thread that owns it is the guest's.
            // See `set_async_decode`.
            async_decode: true,
            waiting: Vec::new(),
            in_flight: HashSet::new(),
            smaf_scores: HashMap::new(),
            smaf_voices: HashMap::new(),
            failed: HashSet::new(),
            requested: 0,
            source,
            seen_sources: HashMap::new(),
            backend,
            driver: Driver::Device(audible),
            notes: Vec::new(),
        })
    }

    /// The same constructor in a build with no device backend.
    ///
    /// An `Err` rather than a panic or a silent no-device speaker: every caller already treats "no
    /// sound" as a state it can print a reason for and run on (`session` drops the `Option` and says
    /// so on stderr), and a speaker that rendered into a void would be a second kind of silence that
    /// nobody asked for. Builds that need a device turn the feature back on.
    #[cfg(not(feature = "device"))]
    pub(crate) fn new_with_backend_handle(
        _soundfont: Option<&Path>,
        _backend: Arc<AtomicU8>,
    ) -> Result<Speaker, String> {
        Err(
            "this build has no device backend (built without the `device` feature); \
             use Speaker::without_device and render the frames yourself"
                .to_string(),
        )
    }

    /// Start one of the run's clips on one voice, parsing it if this is the first time it has been
    /// asked for.
    ///
    /// Returns whether the clip went to the audio thread. A source this end cannot parse is not an
    /// error and is not retried: the request already happened, and "this host has no sound for clip
    /// 7" belongs in the run's own report rather than in a failure.
    pub fn play(&mut self, rt: &Runtime, voice: u32, index: usize) -> bool {
        audio_trace(format_args!(
            "enqueue play voice={voice:#010x} index={index}"
        ));
        // Whatever this voice played before no longer describes it, and a window that arrives for
        // the new clip must not be answered from the old score.
        self.smaf_voices.remove(&voice);
        let backend = self.backend();
        // An audio index is not a stable identity: `Runtime::note_audio_source` and
        // `withdraw_audio` can replace or remove entries, and a new clip can land on an index whose
        // cached clip was another source. Compare the bytes before trusting any per-index cache.
        if let Some(source) = rt.audio.get(index) {
            let fingerprint = source_fingerprint(&source.bytes);
            if self.seen_sources.get(&index) != Some(&fingerprint) {
                self.forget_clip(index);
                self.seen_sources.insert(index, fingerprint);
            }
        }
        if !self.failed.contains(&index) {
            if let Some(source) = rt.audio.get(index) {
                if crate::audio::smaf::looks_like(&source.bytes) {
                    let score = match self.smaf_scores.get(&index) {
                        Some(score) => score.clone(),
                        None => match crate::audio::smaf::sequencer::Score::parse(&source.bytes) {
                            Ok(score) => {
                                let score = Arc::new(score);
                                self.smaf_scores.insert(index, score.clone());
                                score
                            }
                            Err(e) => {
                                self.notes.push(format!(
                                    "clip {} ({}): SMAF does not parse: {e}",
                                    index + 1,
                                    source.name
                                ));
                                self.failed.insert(index);
                                return false;
                            }
                        },
                    };
                    let player = SmafPlayer::new((*score).clone(), crate::audio::smaf::SAMPLE_RATE);
                    self.smaf_voices.insert(voice, score);
                    let _ = self.tx.send(Command::PcmSource {
                        voice,
                        source: PcmSource::smaf(player),
                    });
                    self.requested += 1;
                    return true;
                }
            }
        }
        if self.start_cached(voice, index) {
            return true;
        }
        if self.failed.contains(&index) {
            return false;
        }
        let Some(source) = rt.audio.get(index) else {
            return false;
        };
        // The one play that can cost a frame. A long clip's decode is codec work measured in
        // hundreds of milliseconds; doing it here would stop the guest for that long, so it goes to
        // the helper thread and the voice starts when the samples arrive. `follow` is what starts
        // it, because that is the loop that runs once a frame whatever the guest is doing.
        if self.async_decode && source.bytes.len() >= ASYNC_DECODE_BYTES {
            if !self.waiting.contains(&(voice, index)) {
                self.waiting.push((voice, index));
            }
            if self.submit_decode(index, &source.name, &source.bytes, backend) {
                return false;
            }
            // The thread could not be started. A decode where it stands is slower but correct,
            // and the wait is dropped rather than left to be retried every frame.
            self.waiting.retain(|held| *held != (voice, index));
        }
        let audio = match decode_with(backend, &source.bytes) {
            Ok(decoded) if !decoded.samples.is_empty() => {
                self.note_decode(index, &source.name, &decoded);
                let (samples, rate) = (Arc::new(decoded.samples), decoded.rate);
                self.store_pcm(index, samples.clone(), rate);
                let _ = self.tx.send(Command::Pcm {
                    voice,
                    samples,
                    rate,
                });
                self.requested += 1;
                return true;
            }
            Ok(_) => "the decoder produced no audio samples".to_string(),
            Err(error) => error,
        };
        match Clip::parse(&source.bytes) {
            Ok(clip) => {
                let clip = Arc::new(clip);
                self.clips.insert(index, clip.clone());
                let _ = self.tx.send(Command::Play { voice, clip });
                self.requested += 1;
                true
            }
            Err(midi) => {
                self.notes.push(format!(
                    "clip {} ({}): not a MIDI/QCP or {} audio file: MIDI: {midi}; audio: {audio}",
                    index + 1,
                    source.name,
                    backend.name()
                ));
                self.failed.insert(index);
                false
            }
        }
    }

    /// Hand an already-decoded clip to the mixer, touching nothing on disk and decoding nothing.
    ///
    /// Returns false when neither cache holds this index, which is the frame thread's cue that the
    /// clip needs a decode rather than that it cannot be played.
    fn start_cached(&mut self, voice: u32, index: usize) -> bool {
        if let Some((samples, rate)) = self.pcm.get(&index).cloned() {
            self.touch_pcm(index);
            let _ = self.tx.send(Command::Pcm {
                voice,
                samples,
                rate,
            });
            self.requested += 1;
            return true;
        }
        if let Some(clip) = self.clips.get(&index).cloned() {
            let _ = self.tx.send(Command::Play { voice, clip });
            self.requested += 1;
            return true;
        }
        false
    }

    /// File decoded samples and hold the cache to [`PCM_BUDGET_SAMPLES`].
    ///
    /// Eviction here cannot interrupt anything: the mixer was handed the same `Arc`, so a clip that
    /// is playing stays in memory for as long as the slot wants it. What an eviction costs is a
    /// second decode the next time the clip is played, which is the price of the ceiling.
    fn store_pcm(&mut self, index: usize, samples: Arc<Vec<f32>>, rate: u32) {
        self.store_pcm_within(index, samples, rate, PCM_BUDGET_SAMPLES);
    }

    /// [`Self::store_pcm`] with the ceiling passed in, so a test can fill a cache with a thousand
    /// samples instead of thirty-two megabytes.
    fn store_pcm_within(&mut self, index: usize, samples: Arc<Vec<f32>>, rate: u32, budget: usize) {
        if let Some((old, _)) = self.pcm.insert(index, (samples.clone(), rate)) {
            self.pcm_samples = self.pcm_samples.saturating_sub(old.len());
            self.pcm_order.retain(|held| *held != index);
        }
        self.pcm_samples += samples.len();
        self.pcm_order.push_back(index);
        // One entry that is larger than the whole budget stays: there has to be a cache, and
        // dropping the clip that was just asked for would mean decoding it again every play.
        while self.pcm_samples > budget && self.pcm_order.len() > 1 {
            let Some(victim) = self.pcm_order.pop_front() else {
                break;
            };
            if let Some((gone, _)) = self.pcm.remove(&victim) {
                self.pcm_samples = self.pcm_samples.saturating_sub(gone.len());
            }
        }
    }

    /// Mark a cached clip as the most recently used one.
    fn touch_pcm(&mut self, index: usize) {
        if self.pcm_order.iter().any(|held| *held == index) {
            self.pcm_order.retain(|held| *held != index);
            self.pcm_order.push_back(index);
        }
    }

    /// Drop everything cached for one audio index, and keep the sample count honest.
    fn forget_clip(&mut self, index: usize) {
        self.clips.remove(&index);
        if let Some((gone, _)) = self.pcm.remove(&index) {
            self.pcm_samples = self.pcm_samples.saturating_sub(gone.len());
        }
        self.pcm_order.retain(|held| *held != index);
        self.smaf_scores.remove(&index);
        self.failed.remove(&index);
    }

    /// Record what a decode lost, if it lost anything.
    ///
    /// Silence here is the failure mode this is for: a file with a corrupt middle sounds like a
    /// shorter file, and nothing in the samples says otherwise.
    fn note_decode(
        &mut self,
        index: usize,
        name: &str,
        decoded: &crate::audio::media_decode::Decoded,
    ) {
        if decoded.dropped > 0 {
            self.notes.push(format!(
                "clip {} ({}): {} frame(s) of audio did not decode and were skipped",
                index + 1,
                name,
                decoded.dropped
            ));
        }
        if decoded.chained {
            self.notes.push(format!(
                "clip {} ({}): chained stream, only its first segment plays",
                index + 1,
                name
            ));
        }
    }

    /// Hand one clip to the helper thread, if it is not already there.
    ///
    /// Returns whether a job was submitted. A thread that cannot be spawned is not an error the
    /// run should fail on: `play` falls back to decoding where it stands, which is what every
    /// build did before this thread existed.
    fn submit_decode(
        &mut self,
        index: usize,
        name: &str,
        bytes: &[u8],
        backend: AudioBackend,
    ) -> bool {
        if self.in_flight.contains(&index) {
            return true;
        }
        let worker = match self.decoder.as_mut() {
            Some(worker) => worker,
            None => match DecodeWorker::new() {
                Some(worker) => self.decoder.insert(worker),
                None => return false,
            },
        };
        let fingerprint = self.seen_sources.get(&index).copied().unwrap_or(0);
        let job = DecodeJob {
            index,
            name: name.to_string(),
            fingerprint,
            bytes: bytes.to_vec(),
            backend,
        };
        if worker.jobs.send(job).is_err() {
            self.decoder = None;
            return false;
        }
        self.in_flight.insert(index);
        true
    }

    /// File every decode the helper thread has finished, then start the voices that were waiting.
    ///
    /// Called at the top of [`Speaker::follow`], so a clip becomes audible on the frame after its
    /// samples land. A voice that stopped or was re-pointed while the decode ran is not started:
    /// the guest asked for something else in the meantime, and a late start would be a sound the
    /// pack had already turned off.
    fn collect_decodes(&mut self, rt: &Runtime) {
        let mut finished = Vec::new();
        let mut dead = false;
        if let Some(worker) = self.decoder.as_mut() {
            loop {
                match worker.done.try_recv() {
                    Ok(done) => finished.push(done),
                    Err(mpsc::TryRecvError::Empty) => break,
                    // The thread is gone — it panicked, or the process is tearing the speaker down.
                    // Whatever was in flight will never be reported, so nothing may keep waiting on
                    // it: the flag is cleared and the next frame asks again.
                    Err(mpsc::TryRecvError::Disconnected) => {
                        dead = true;
                        break;
                    }
                }
            }
        }
        if dead {
            self.in_flight.clear();
            self.decoder = None;
        }
        for done in finished {
            self.in_flight.remove(&done.index);
            // The index was renamed while this was decoding: the samples describe something the
            // runtime no longer has there.
            if self.seen_sources.get(&done.index).copied() != Some(done.fingerprint) {
                continue;
            }
            match done.outcome {
                DecodeOutcome::Pcm(decoded) => {
                    if decoded.samples.is_empty() {
                        continue;
                    }
                    self.note_decode(done.index, &done.name, &decoded);
                    let rate = decoded.rate;
                    self.store_pcm(done.index, Arc::new(decoded.samples), rate);
                }
                DecodeOutcome::Midi(clip) => {
                    self.clips.insert(done.index, clip);
                }
                DecodeOutcome::Failed(message) => {
                    self.notes.push(format!(
                        "clip {} ({}): {message}",
                        done.index + 1,
                        done.name
                    ));
                    self.failed.insert(done.index);
                }
            }
        }
        if self.waiting.is_empty() {
            return;
        }
        let mut still = Vec::new();
        for (voice, index) in std::mem::take(&mut self.waiting) {
            if self.in_flight.contains(&index) {
                still.push((voice, index));
                continue;
            }
            if !still_wanted(rt, voice, index) {
                continue;
            }
            if self.start_cached(voice, index) || self.failed.contains(&index) {
                continue;
            }
            // Nothing cached, nothing in flight, nothing failed: the decode that was going to answer
            // this either described other bytes (the index was renamed under it) or never happened.
            // Ask again for what the runtime holds now; `submit_decode` refusing is the end of it.
            let resubmitted = match rt.audio.get(index) {
                Some(source) => {
                    self.submit_decode(index, &source.name, &source.bytes, self.backend())
                }
                None => false,
            };
            if resubmitted {
                still.push((voice, index));
            }
        }
        self.waiting = still;
    }

    /// The decoder backend this speaker is currently using.
    pub fn backend(&self) -> AudioBackend {
        AudioBackend::from_u8(self.backend.load(Ordering::Relaxed))
    }

    /// Change the decoder backend for subsequent ordinary clips. A backend that is not built is
    /// still reported by [`AudioBackend::available`]; it is not silently replaced by Rust.
    pub fn set_backend(&self, backend: AudioBackend) {
        self.backend.store(backend as u8, Ordering::Relaxed);
    }

    /// Whether a first play of a large source decodes on the helper thread.
    ///
    /// On for a speaker with a device behind it: the thread that renders frames is the guest's
    /// thread, and a hundred-millisecond decode between two frames is a freeze a person sees. Off
    /// for a speaker the caller drives with [`Speaker::mix`], because that caller chose to be the
    /// clock: a libretro core and a `--serve` run hand back a fixed number of frames per call, and
    /// a start that lands on whichever frame the operating system finished a thread on is not
    /// reproducible. A front end that pulls its own frames and would rather have the smoothness can
    /// turn it back on — the cost is that a run's audio is then no longer byte-identical run to run.
    ///
    /// What it never changes is *whether* a clip plays. With this on, the first play of one large
    /// source is silent for as long as its decode takes and starts by itself afterwards; with it
    /// off, the caller waits and the clip starts immediately.
    pub fn set_async_decode(&mut self, on: bool) {
        self.async_decode = on;
    }

    /// Whether the helper thread is on, for a report or a test.
    pub fn async_decode(&self) -> bool {
        self.async_decode
    }

    /// How many samples of decoded PCM are held right now, across every cached clip.
    pub fn cached_samples(&self) -> usize {
        self.pcm_samples
    }

    /// How many clips are cached, decoded or parsed.
    pub fn cached_clips(&self) -> usize {
        self.pcm.len() + self.clips.len()
    }

    /// Silence one voice.
    pub fn stop(&mut self, voice: u32) {
        self.smaf_voices.remove(&voice);
        let _ = self.tx.send(Command::Stop { voice });
    }

    /// Silence every voice and tone this speaker was told to play, and forget what it was told.
    ///
    /// For a host that has to cut the mixer without asking the run: a front end loading a different
    /// pack, or restarting the one it has. The bookkeeping goes with it — [`Playing`] is the memory
    /// of "what I last told the mixer", and keeping it across a machine that no longer exists is how
    /// a voice number from the old run holds a voice in the new one. What this does **not** do is
    /// stop the run: the next [`Speaker::follow`] derives from the runtime again, so a pack that
    /// still names a sound is heard again on the next frame. That is the split — this call owns the
    /// mixer, the runtime owns whether anything should be playing.
    pub fn stop_all(&mut self, playing: &mut Playing) {
        let mut sounding: Vec<u32> = playing.voices.keys().copied().collect();
        // A tone is stopped by the address of the object that owns it, the same word a voice uses
        // (see `requests`), so one list serves both.
        sounding.extend(playing.tones.keys().copied());
        sounding.sort_unstable();
        sounding.dedup();
        for voice in sounding {
            self.stop(voice);
        }
        *playing = Playing::new();
    }

    /// How many clips have been handed to the audio thread.
    pub fn requested(&self) -> usize {
        self.requested
    }

    /// Which tone bank this speaker speaks with, in the words the run's report should use.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// What the speaker could not use. Empty on a sample set it can play whole.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Answer whatever the run asked for since the last frame, returning what it asked for.
    ///
    /// The return value is what makes this seam visible from outside: a front end can say what the
    /// run requested, in order, and then "the game never asked" is distinguishable from "the speaker
    /// dropped it". It returns the *requests* rather than only the clips that started for exactly
    /// that reason — a `Stop` that arrives with no `Play` after it is invisible if only starts are
    /// reported, and that distinction is what the music bug that widened this return value turned
    /// on.
    /// Set the whole speaker's playback rate, in thousandths of real time (1000 = 1x).
    ///
    /// Best effort like the other commands: a closed channel means the mixer is gone, and a run
    /// that is over has nothing left to change. The rate is applied inside the mixer, so every
    /// voice is resampled and tempo and pitch move together.
    pub fn set_speed(&mut self, permille: u32) {
        let _ = self.tx.send(Command::Speed(permille));
    }

    pub fn follow(&mut self, rt: &Runtime, playing: &mut Playing) -> Vec<Request> {
        // Before this frame's requests, not after: a decode that finished while the guest was
        // running should be audible now, and a voice waiting on one should start on the earliest
        // frame it can rather than one frame later.
        self.collect_decodes(rt);
        let asked = requests(rt, playing);
        for request in &asked {
            match request {
                Request::Play { voice, index } => {
                    self.play(rt, *voice, *index);
                }
                Request::Stop { voice } => self.stop(*voice),
                Request::Volume { voice, volume } => {
                    let _ = self.tx.send(Command::Volume {
                        voice: *voice,
                        volume: *volume,
                    });
                }
                Request::Tempo(percent) => {
                    // Best effort, like `play` and `stop`: a closed channel means the mixer is gone,
                    // and a run that is over has nothing left to change.
                    let _ = self.tx.send(Command::Tempo(*percent));
                }
                Request::Rate { voice, num, den } => {
                    let _ = self.tx.send(Command::Rate {
                        voice: *voice,
                        num: *num,
                        den: *den,
                    });
                }
                Request::Tune { voice, step } => {
                    let _ = self.tx.send(Command::Tune {
                        voice: *voice,
                        step: *step,
                    });
                }
                Request::Tone {
                    object,
                    hi,
                    lo,
                    volume,
                } => {
                    let _ = self.tx.send(Command::Tone {
                        object: *object,
                        hi: *hi,
                        lo: *lo,
                        volume: *volume,
                    });
                }
                Request::Midi { voice, bytes } => {
                    let _ = self.tx.send(Command::Midi {
                        voice: *voice,
                        bytes: bytes.clone(),
                    });
                }
                Request::Window {
                    voice,
                    start,
                    stop,
                    reverse,
                } => {
                    // A SMAF window is a replay, so it happens here; a MIDI or decoded-PCM window
                    // is a table position or a sample offset and stays a mixer command.
                    match self.smaf_voices.get(voice).cloned() {
                        Some(score) => {
                            let source = smaf_window_source(&score, *start, *stop, *reverse);
                            let _ = self.tx.send(Command::PcmSource {
                                voice: *voice,
                                source,
                            });
                        }
                        None => {
                            let _ = self.tx.send(Command::Window {
                                voice: *voice,
                                start: *start,
                                stop: *stop,
                                reverse: *reverse,
                            });
                        }
                    }
                }
            }
        }
        asked
    }
}

/// Hand a mixer to the device, and give the stream a closure that drains it and copies what it
/// renders into whatever the device asks for.
///
/// **This is the whole platform half of the speaker**: opening the stream, knowing the sample format,
/// and interleaving mono pairs into the device's channel count. Everything that decides *what the
/// sound is* lives in [`Mixer`], so a backend for another platform is a function of this shape and
/// nothing else — and the offline path in [`render_clip`] needs no version of it at all.
///
/// The buffers are allocated here, once, and never on the audio thread.
#[cfg(feature = "device")]
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    mut mixer: Mixer,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let mut left = vec![0.0f32; BLOCK];
    let mut right = vec![0.0f32; BLOCK];
    device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            // Commands first, so a play that arrived during the last buffer is heard from this
            // one's first frame rather than a buffer later.
            mixer.drain();
            let frames = out.len() / channels;
            let mut done = 0;
            while done < frames {
                let n = (frames - done).min(left.len());
                let (l, r) = (&mut left[..n], &mut right[..n]);
                // The sum, already clamped to full scale by the mixer.
                mixer.render_into(l, r);
                for i in 0..n {
                    let (a, b) = (l[i], r[i]);
                    for c in 0..channels {
                        out[(done + i) * channels + c] =
                            T::from_sample(if c % 2 == 0 { a } else { b });
                    }
                }
                done += n;
            }
        },
        |e| eprintln!("audio stream error: {e}"),
        None,
    )
}

/// Render one clip on its own, with no device in sight.
///
/// This is what lets the layer be tested on a machine with no sound card, and it is not a second
/// implementation: it is the same [`Synthesizer`] and the same sequencer the callback drives, asked for
/// its samples directly instead of through a buffer the operating system owns.
/// What it cannot show is the device, and the device is the one part with nothing to get wrong.
/// How long one named clip runs, in the run's own milliseconds, or `None` when the bytes are not a
/// sound this host can time.
///
/// `midly` reports absolute seconds, so the MIDI conversion lives here and nowhere else. The runtime
/// arms a pack's status callback from this, because a handset tells a pack when its clip ended and
/// some packs wait for that telling before doing anything else — a pack brings its music back by
/// re-arming its theme from the callback that ends the one-second jingle.
///
/// The three families answer differently and all three are asked: SMAF and QCP carry their own
/// length, MIDI's is the end of its merged timeline, and a container Symphonia recognises answers
/// from its header's frame count ([`crate::audio::media_decode::duration_ms`]). A media clip used to
/// fall through all of this and be timed at 0 ms, which is the one answer a pack cannot work with:
/// `DONE` never fires, `GetTotalTime` answers 0, and a window into the clip has nowhere to land.
pub fn clip_length_ms(bytes: &[u8]) -> Option<u32> {
    if crate::audio::smaf::looks_like(bytes) {
        return crate::audio::smaf::duration_ms(bytes);
    }
    if let Ok(qcp) = crate::audio::qcelp::Qcp::parse(bytes) {
        if matches!(
            qcp.codec,
            crate::audio::qcelp::Codec::Qcelp13 | crate::audio::qcelp::Codec::Evrc
        ) {
            return qcp.duration_ms().ok().and_then(|ms| u32::try_from(ms).ok());
        }
    }
    // A Standard MIDI File is asked first and by its own header, because the probe below would
    // otherwise be pointed at every score a pack names and would fail on all of them.
    if bytes.starts_with(b"MThd") {
        let count: [u8; 2] = bytes.get(10..12)?.try_into().ok()?;
        if i16::from_be_bytes(count) <= 0 {
            return None;
        }
        let clip = Clip::parse(bytes).ok()?;
        return (clip.length.is_finite() && clip.length > 0.0)
            .then(|| (clip.length * 1000.0) as u32);
    }
    crate::audio::media_decode::duration_ms(bytes)
}

/// The one audio format this host makes sound out of, spelled the way BREW spells it.
///
/// the header's own format table gives MIDI the MIME `audio/mid` and the file
/// extension `mid`, and this constant is the single place that pair is written down: it is the string
/// `IShell::DetectType` writes back, and the key [`IShell::GetHandler`](crate::Runtime::handlers)
/// files the media class under. A name invented here would be a name no handset answer matches.
pub const MIME_MIDI: &str = "audio/mid";

/// Yamaha SMAF (`MMMD` container). The media format table in the header files
/// `AEECLSID_MEDIAMMF` under this exact string, and the Carrier extension module registers
/// `audio/mmf` for it; the host follows the published pair rather than inventing an extension
/// spelling. (`AEEMedia.bid` carries the class id only, not the MIME.)
pub const MIME_MMF: &str = "audio/mmf";

/// Yamaha SPF, the third face of the same Carrier media module.
///
/// `IMediaMA3.dll` registers `audio/spf` against `AEECLSID_MEDIAPHR` beside the MIDI and MMF rows
/// and implements all three with one 14-slot `IMedia` body, differing only in a format flag
/// (`BREW_JP_ABI.md` §6.1). It is a separate string because `GetHandler` is asked by name; the
/// player behind it is the same SMAF engine the `audio/mmf` row names.
pub const MIME_SPF: &str = "audio/spf";

/// The QCELP/EVRC container under the name the header's format table gives it.
pub const MIME_QCP: &str = "audio/qcp";

/// The BREW 1.x spelling of the same container (`MT_QCP "snd/qcp"`).
pub const MIME_QCP_LEGACY: &str = "snd/qcp";

/// RIFF/WAV, including the ADPCM payloads the media samples use.
pub const MIME_WAV: &str = "audio/wav";

/// Raw AAC/ADTS. MP4-carried AAC needs a container handler before it can be named here.
pub const MIME_AAC: &str = "audio/aac";

/// The same format under the spelling BREW 1.x left behind: the header defines
/// `SOUND_MIME_BASE "snd/"` and spells the MIDI type `MT_MIDI "snd/midi"`, beside
///'s `MT_AUDIO_MIDI "audio/mid"`.
///
/// A registry row and nothing else — [`sniff`] never answers with it, because the header's format
/// table names one MIME per class and that is the one a caller of `IShell::DetectType` is told. But
/// `IShell::GetHandler` is a *lookup*, and a module that was written against the older constant asks
/// with `"snd/midi"` for the same file. Answering that with 0 would be a spelling coin-flip standing
/// between a pack and its own playlist, which is not the kind of no this host has been burned by
/// making.
pub const MIME_MIDI_LEGACY: &str = "snd/midi";

/// The two prefixes a BREW MIME has to carry to name a **sound** rather than a picture or a video:
/// The header `SOUND_MIME_BASE "snd/"`, `AUDIO_MIME_BASE "audio/"`, `VIDEO_MIME_BASE "video/"` (the same four are
/// repeated at) — and the middle two are the sound pair. The older `snd/` one is not
/// hypothetical: spell the whole ring tone set with it (`MT_MIDI "snd/midi"`,
/// `MT_QCP "snd/qcp"`, …) and spell those **same twelve formats** under `audio/`;
/// then add a set (`hvs`, `saf`, `xmf`…, `dls`) that has **no** `snd/` twin, so the two bases are not
/// mirror images of each other.
///
/// **This is not a claim about what this host can play** — [`sniff`] is that claim, and it is made by
/// parsing. This one answers "would a handset route this MIME to a media/sound handler at all",
/// which is a fact about the *string*; it lives here rather than at each caller because
/// one answer belongs here: two callers each writing their own
/// `starts_with("audio/")` is how `--info` came to report no audio for a pack whose ring tones are
/// registered under the 1.x spelling.
///
/// Trimmed and case-folded, because that is how the same names are matched everywhere else this host
/// keeps them: both `Runtime::register_handler` and the `IShell::GetHandler` lookup fold the name
/// with `trim` + ASCII lowercase, and the reason written there is that MIME types and URL scheme
/// names are case-insensitive by definition. A caller that forgot this would answer "that names no
/// sound" for `SND/MIDI` while the registry answered with the media class for the same text.
/// **No such declaration exists in either sample set** (all 827 packs, every declared MIME: only
/// lowercase `audio/…` shows up, and the header itself spells its constants without a trailing
/// space), so this is consistency with the rest of the host, not a correction to a measured number.
pub fn names_a_sound(mime: &str) -> bool {
    let mime = mime.trim();
    ["audio/", "snd/"].iter().any(|base| {
        mime.get(..base.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(base))
    })
}

/// What these bytes are, in the only sense this host can honestly mean: the name the audio front end
/// will then take them under, or `None`.
///
/// This is the audio half of what [`crate::resource::sniff`] is for pictures, and it defers to the
/// same rule for the same reason: a caller of `IShell::DetectType` takes the string and goes off to
/// *load* the payload with it, so a name produced by a four-byte `MThd` prefix test is a claim this
/// module then has to honour. What it runs is literally the test [`Speaker::play`] applies — the
/// same `MidiFile::new` on the same bytes — which is the only answer that cannot drift out of sync
/// with the thing it describes. It is *not* [`clip_length_ms`]: the front end accepts a file whose
/// merged timeline has a single end-of-track on it and plays a beat of silence, and that is a clip
/// this host does render even though it has no length to report.
///
/// Which is also why a format with no decoder below is not named at all. What is listed here is
/// exactly what [`Speaker::play`] can turn into sound: SMAF, QCP at either rate, MPEG audio and ADTS
/// through `media_decode`, RIFF/WAVE, and a Standard MIDI File. FLAC and Ogg/Vorbis
/// are deliberately absent — `symphonia`'s default feature set would decode both, this build does not
/// enable them, and naming one would turn "this handset has no player for it" into a file that opens
/// and plays nothing. That rule is what keeps [`MIME_MP3`] honest rather than what used to forbid it:
/// the name is here because Symphonia converts the frames, not because a class id was served.
///
/// What it costs is a parse, and what it gives up is precision on a partial buffer — the same price
/// [`crate::resource::sniff`] documents for a PNG whose bytes ran out. MIDI is the one format whose
/// *complete* parse cannot answer from the window `IShell::DetectType` offers (32 bytes, and the
/// documented media probe offers exactly that much), so its header is checked instead; the full
/// parse below remains as the fallback for a header this test refuses.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if crate::audio::smaf::looks_like(bytes) {
        return Some(MIME_MMF);
    }
    if let Ok(qcp) = crate::audio::qcelp::Qcp::parse(bytes) {
        if matches!(
            qcp.codec,
            crate::audio::qcelp::Codec::Qcelp13 | crate::audio::qcelp::Codec::Evrc
        ) {
            return Some(MIME_QCP);
        }
    }
    if mpeg_audio(bytes) {
        return Some(MIME_MP3);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Some(MIME_WAV);
    }
    if adts_audio(bytes) {
        return Some(MIME_AAC);
    }
    if midi_header(bytes) || MidiFile::new(&mut Cursor::new(bytes)).is_ok() {
        return Some(MIME_MIDI);
    }
    None
}

/// The 14-byte Standard MIDI File header, checked well enough to name the format from the
/// detection window alone.
///
/// `IShell::DetectType` answers from at most `TYPE_DETECT_WINDOW` (32) bytes, and the documented
/// media probe (`AEEMediaUtil_FindClass`) offers exactly that much. A track of any real score is
/// longer, so [`MidiFile::new`] — which reads the whole file — refuses every score that arrives
/// through that door. The header is the format's own definition and carries the fields that
/// separate a MIDI from bytes that merely begin with `MThd`; the player still parses the whole file
/// when it plays, and a malformed track is its failure to report, not detection's.
fn midi_header(bytes: &[u8]) -> bool {
    if bytes.len() < 14 || &bytes[..4] != b"MThd" {
        return false;
    }
    let header_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let format = u16::from_be_bytes([bytes[8], bytes[9]]);
    let tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
    let division = u16::from_be_bytes([bytes[12], bytes[13]]);
    if !(header_len >= 6 && format <= 2 && tracks > 0 && division != 0) {
        return false;
    }
    // A header alone is a shape, not a file: the first track chunk has to be behind it. The
    // detection window (32 bytes) holds `MTrk` and its length word, but never the track itself.
    let track = 8usize.saturating_add(header_len as usize);
    bytes.len() >= track + 8 && &bytes[track..track + 4] == b"MTrk"
}

/// A raw AAC ADTS header: twelve sync bits, layer `00`, and a legal sampling-frequency index.
fn adts_audio(bytes: &[u8]) -> bool {
    bytes.len() >= 7 && bytes[0] == 0xFF && bytes[1] & 0xF6 == 0xF0 && bytes[2] >> 2 != 0xF
}

/// The name BREW gives an MP3, spelled the way the platform spells it.
///
/// the header's format table row is `MP3 / AEECLSID_MEDIAMP3 / IMediaMP3 /
/// `**`audio/mp3`**` / mp3`, and the header defines `MT_AUDIO_MP3 "audio/mp3"` with
/// the 1.x twin at (`MT_MP3 "snd/mp3"`) and `MP3_EXTENSION "mp3"` at.
///
/// **No container in either sample set writes this spelling.** The sound payloads that carry an MPEG family
/// name declare `audio/mpeg` — 205 records of it in the pack census, which is
/// not a string in either header. That is why this constant exists separately from the declared text
/// rather than replacing it: a census row keyed by what the container *said* stays keyed by that, and
/// this name is what the *bytes* answer. Conflating the two is how a report ends up claiming a format
/// the platform never named, or hides the fact that the pack's tools spelled it their own way.
pub const MIME_MP3: &str = "audio/mp3";

/// What these bytes **are**, for the caller that is taking inventory rather than playing them.
///
/// This was once the *wide* question and [`sniff`] the narrow one: sniff withheld `audio/mp3` because
/// nothing below it turned MPEG frames into samples, so a census asking what those 205 `audio/mpeg`
/// records actually held needed an answer that was not also a promise of playback. `symphonia` closed
/// that gap — every format [`sniff`] names is now one this host can sound — so the two questions have
/// collapsed into the same one. The name stays because its callers ask it for a different purpose:
/// [`crate::probe`] uses it to say what a pack provides, and resource loading reads a `None` as "this
/// is not audio at all" rather than as "not playable".
///
/// That history is worth keeping because it is the shape a second decoder entry takes: a declared MIME
/// is not the payload's family, and neither is the absence of one — 186 clips of real MIDI sit inside
/// containers that called them `image/mid*`. One
/// delegation and not a second parser: two lists of formats is
/// how a report ends up claiming one thing while the player does another.
pub fn recognises(bytes: &[u8]) -> Option<&'static str> {
    sniff(bytes)
}

/// Whether these bytes begin with MPEG audio — as a frame, or as an ID3v2 tag that hands over to one.
///
/// The MPEG-audio test both names above ultimately rest on, split out because the frame header is the
/// part with a judgment in it.
///
/// Cheap by construction: a buffer that is not one fails on the first four bytes, so asking it about
/// something already known not to be MIDI costs nothing worth naming.
fn mpeg_audio(bytes: &[u8]) -> bool {
    if frame_header(bytes) {
        return true;
    }
    if !bytes.starts_with(b"ID3") || bytes.len() < 10 {
        return false;
    }
    // The tag's own length, in the synchsafe form both 2.3 and 2.4 write: four bytes of seven bits.
    let size = ((bytes[6] as usize) << 21)
        | ((bytes[7] as usize) << 14)
        | ((bytes[8] as usize) << 7)
        | bytes[9] as usize;
    // A tag with no frame behind it is a tag, not audio — which is the difference between this answer
    // and the two-byte prefix test that counts a cover-art-only file as a song.
    bytes.get(10 + size..).is_some_and(frame_header)
}

/// One MPEG audio frame header, read by its own validity fields rather than by a prefix.
///
/// Eleven sync bits, then four fields each of which has a value a real encoder never writes: version
/// `01` is "unspecified", layer `00` and sample-rate `11` are reserved, and bitrate index `1111` is
/// defined as "bad". That is the whole point of testing them instead of matching `FF FB`: four bytes of
/// `FF` — which is what padding, a run of ones, or a truncated plane looks like — pass any prefix test
/// and are refused here by the bitrate field. Bitrate index `0000` is left alone, because free-bitrate
/// streams are a legal thing an encoder produces rather than a sign these are not audio.
fn frame_header(bytes: &[u8]) -> bool {
    let Some(h) = bytes.get(..4) else {
        return false;
    };
    h[0] == 0xFF
        && h[1] & 0xE0 == 0xE0
        && (h[1] >> 3) & 0x3 != 1
        && (h[1] >> 1) & 0x3 != 0
        && h[2] >> 4 != 0xF
        && (h[2] >> 1) & 0x3 != 3
}

/// What a *name* is, for the caller that has the one and not the other.
///
/// No bytes, so nothing can be parsed and the suffix is all there is — which is why it is read out
/// of [`MIME_MIDI`] rather than listed beside it, exactly as [`crate::resource::sniff_name`] does:
/// one list of formats this host serves, not two a later commit can update separately. The header's
/// own table says the extension for MIDI is `mid`, so `song.mid` names and `song.midi` does not;
/// guessing past the spelled list is how a sniffer ends up promising a decode.
pub fn sniff_name(name: &str) -> Option<&'static str> {
    let file = name.rsplit(['/', '\\']).next()?;
    let (_, ext) = file.rsplit_once('.')?;
    if ext.eq_ignore_ascii_case("qcp") {
        return Some(MIME_QCP);
    }
    if ext.eq_ignore_ascii_case("mp3") {
        return Some(MIME_MP3);
    }
    if ext.eq_ignore_ascii_case("wav") {
        return Some(MIME_WAV);
    }
    if ext.eq_ignore_ascii_case("aac") {
        return Some(MIME_AAC);
    }
    if ext.eq_ignore_ascii_case("mmf") {
        return Some(MIME_MMF);
    }
    MIME_MIDI
        .rsplit_once('/')
        .map(|(_, subtype)| subtype)
        .filter(|subtype| subtype.eq_ignore_ascii_case(ext))
        .map(|_| MIME_MIDI)
}

/// Render one clip on its own, with no device in sight.
///
/// This is the offline driver for the same [`Mixer`] the device callback drives, and that is the
/// point: it is what lets the layer be tested on a machine with no sound card *and* what lets a
/// headless run hand a person something they can listen to ([`render_wav`]) — through one rendering
/// path, so a change to the mixing shows up in the window and here together. The shape this function
/// used to have built its own `Synthesizer` and summed nothing, which made it a second authority over
/// "what does clip 7 sound like".
///
/// What it cannot show is the device, and the device is the one part with nothing to get wrong.
pub fn render_clip(
    soundfont: Option<&Path>,
    bytes: &[u8],
    sample_rate: i32,
    frames: usize,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let (sound_font, _) = load(soundfont)?;
    let clip = Arc::new(Clip::parse(bytes)?);
    let mut mixer = Mixer::offline(sample_rate, &sound_font)?;
    // Any non-zero voice: this mixer has one slot, and `0` is the mark for a free one.
    mixer.assign(1, &clip);
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    mixer.render_into(&mut left, &mut right);
    Ok((left, right))
}

/// The most audio one [`render_wav`] call carries. A clip longer than this is cut, and said to be
/// cut: the number has to be in the report, not in the person's ears.
pub const WAV_SECONDS: u64 = 60;

/// One clip, as a WAV file's bytes, with no sound card and no platform in the picture.
///
/// `frames` is how many to render — the caller decides, because only the caller knows whether it wants
/// the clip's own length or a bounded slice of it ([`WAV_SECONDS`] is what `--dump-wav` chose) — and
/// the answer comes back as the second half of the result rather than being assumed. 44 100 Hz,
/// 16-bit signed, stereo: what every player and every phone takes.
pub fn render_wav(
    soundfont: Option<&Path>,
    bytes: &[u8],
    rate: u32,
    frames: usize,
) -> Result<(Vec<u8>, usize), String> {
    let (left, right) = render_clip(soundfont, bytes, rate as i32, frames)?;
    Ok((wav(rate, &left, &right), left.len()))
}

/// Decode any audio this host can play into mono PCM: QCP first, then the Symphonia formats.
///
/// The tuple rather than [`crate::audio::media_decode::Decoded`] because this is the offline face
/// of the decoder — `--dump-wav` and the tools that want samples and nothing else. What a decode
/// lost is only meaningful to a run's report, and a run reports through [`Speaker`].
pub fn decode_audio(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    decode_with(AudioBackend::Rust, bytes).map(|decoded| (decoded.rate, decoded.samples))
}

/// Wrap mono f32 PCM as a stereo 16-bit WAV file.
pub fn pcm_wav(rate: u32, samples: &[f32]) -> Vec<u8> {
    wav(rate, samples, samples)
}

/// PCM16 little-endian stereo, wrapped in the 44-byte RIFF header that describes it.
///
/// Hand-assembled rather than through a container crate: the file is four `u32`s and four `u16`s of
/// arithmetic, and a dependency here would be one more authority over bytes this module already
/// decided. Both sizes are derived from the same `data` the loop writes, so a header that lies about
/// its own body is not a thing this shape can produce — and `a_wav_describes_the_bytes_it_carries` is
/// the test that keeps it that way.
fn wav(rate: u32, left: &[f32], right: &[f32]) -> Vec<u8> {
    let frames = left.len().min(right.len());
    let data = frames * 4;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    out.extend_from_slice(&2u16.to_le_bytes()); // two channels
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 4).to_le_bytes()); // bytes per second: 2 channels x 2 bytes
    out.extend_from_slice(&4u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    for i in 0..frames {
        out.extend_from_slice(&(to_pcm16(left[i])).to_le_bytes());
        out.extend_from_slice(&(to_pcm16(right[i])).to_le_bytes());
    }
    out
}

/// Full scale, saturating rather than wrapping: a value at or past the rail must land on the rail and
/// not on the opposite side, which is what casting an out-of-range float to `i16` does.
fn to_pcm16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32_767.0).round() as i16
}

/// A one-note Standard MIDI File, written out here rather than read from anywhere: the question is
/// whether the chain from bytes to samples works, and a file a test had to go and find would make a
/// failure mean two things at once.
///
/// `MThd` with format 0, one track and 480 ticks per quarter; one `MTrk` holding a note-on for
/// middle C, a note-off 480 ticks later, and an end-of-track. The length is the number of bytes that
/// follow it, which is the one part a hand-built file gets wrong.
///
/// Shared with `runtime`'s tests because that half needs a clip with a knowable end, and a second
/// copy of these twenty-odd bytes is a second thing to get wrong.
#[cfg(test)]
pub(crate) fn one_note_midi() -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes()); // format 0
    file.extend_from_slice(&1u16.to_be_bytes()); // one track
    file.extend_from_slice(&480u16.to_be_bytes()); // ticks per quarter note
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&12u32.to_be_bytes());
    file.extend_from_slice(&[0x00, 0x90, 60, 100]); // delta 0: note on, middle C
    file.extend_from_slice(&[0x83, 0x60, 0x80, 60, 0]); // delta 480: note off
    file.extend_from_slice(&[0x00, 0xff, 0x2f, 0x00]); // end of track
    file
}

/// What drives a [`Speaker`]'s mixer.
///
/// The mixer has one owner and no lock: either the sound card's callback pumps it — the device case,
/// where the stream is kept alive here and never read — or the caller pumps it by hand, which is what
/// a front end needs when *it* owns the frame clock and there may be no sound card at all. Naming the
/// choice is what keeps a second speaker type from existing: everything that decides what the sound
/// *is* lives in [`Mixer`], and this is only about who says "now".
pub enum Driver {
    /// The device's own thread renders, on the device's schedule. Dropping the stream ends it.
    #[cfg(feature = "device")]
    Device(cpal::Stream),
    /// The caller renders, on its own schedule: it owns the mixer and calls [`Speaker::mix`].
    Pull(Box<Mixer>),
}

impl Speaker {
    /// The same speaker with no device behind it.
    ///
    /// A front end that owns the frame clock cannot use a callback arriving on the sound card's
    /// schedule — a libretro core is called once a frame and has to hand back that frame's samples
    /// from *inside* the call — and a machine with no sound card cannot use one at all. Both are the
    /// case [`Mixer`] was already built for: `render_clip` renders through the same summing with no
    /// device in sight. `sample_rate` is the rate the caller will render at, and it must be the rate
    /// it reports to its own front end: one number, because two of them drift apart.
    pub fn without_device(sample_rate: i32, soundfont: Option<&Path>) -> Result<Speaker, String> {
        Self::without_device_with_backend(sample_rate, soundfont, AudioBackend::Rust)
    }

    /// The same speaker with no device behind it and an explicit decoder backend.
    pub fn without_device_with_backend(
        sample_rate: i32,
        soundfont: Option<&Path>,
        backend: AudioBackend,
    ) -> Result<Speaker, String> {
        if !backend.available() {
            return Err(format!(
                "audio backend {} is not built into this binary (available: rust)",
                backend.name()
            ));
        }
        Self::without_device_with_backend_handle(
            sample_rate,
            soundfont,
            Arc::new(AtomicU8::new(backend as u8)),
        )
    }

    pub(crate) fn without_device_with_backend_handle(
        sample_rate: i32,
        soundfont: Option<&Path>,
        backend: Arc<AtomicU8>,
    ) -> Result<Speaker, String> {
        let (sound_font, source) = load(soundfont)?;
        let (tx, rx) = mpsc::channel::<Command>();
        let mixer = Mixer::live(rx, sample_rate, &sound_font)?;
        Ok(Speaker {
            tx,
            clips: HashMap::new(),
            pcm: HashMap::new(),
            pcm_order: VecDeque::new(),
            pcm_samples: 0,
            decoder: None,
            // A caller that pulls its own frames is a caller that asked for a reproducible run:
            // `--serve`, a libretro core and every test in this file mix on their own schedule, and
            // a helper thread would make *when* a clip starts depend on the operating system.
            async_decode: false,
            waiting: Vec::new(),
            in_flight: HashSet::new(),
            smaf_scores: HashMap::new(),
            smaf_voices: HashMap::new(),
            failed: HashSet::new(),
            requested: 0,
            source,
            seen_sources: HashMap::new(),
            backend,
            driver: Driver::Pull(Box::new(mixer)),
            notes: Vec::new(),
        })
    }

    /// Follow the run and render the next `left.len` frames into the two sides.
    ///
    /// Only meaningful on a speaker built by [`Speaker::without_device`]: with a device behind it the
    /// mixer belongs to the callback's thread and is not here to be rendered, so this returns silence
    /// rather than reaching for it — a lock on the audio path is the one thing this file refuses.
    /// Both buffers must be the same length, and what comes back is already clamped to full scale by
    /// the mixer, so it is safe to convert.
    pub fn mix(
        &mut self,
        rt: &Runtime,
        playing: &mut Playing,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        self.follow(rt, playing);
        match &mut self.driver {
            Driver::Pull(mixer) => {
                mixer.drain();
                mixer.render_into(left, right);
            }
            #[cfg(feature = "device")]
            Driver::Device(_) => {
                left.fill(0.0);
                right.fill(0.0);
            }
        }
    }

    /// One rendered frame in the PCM form an API that takes samples wants: interleaved left/right.
    ///
    /// Here rather than in the caller so that the conversion is one place: `to_pcm16` is the only
    /// thing that knows how a float becomes a sample, and a second copy of it in a front end is how
    /// the two ends start disagreeing about what full scale is.
    pub fn interleave(left: &[f32], right: &[f32]) -> Vec<i16> {
        let mut out = Vec::with_capacity(left.len().min(right.len()) * 2);
        Self::interleave_into(left, right, &mut out);
        out
    }

    /// The same conversion into a buffer the caller keeps.
    ///
    /// A front end calls this once a frame for the life of a run, so the buffer is the caller's to
    /// hold: at 44.1 kHz a sixteen-millisecond turn is 1410 samples, and a fresh `Vec` for each of
    /// sixty turns a second is churn nobody needs. `out` is cleared and refilled, and its capacity is
    /// kept across calls.
    pub fn interleave_into(left: &[f32], right: &[f32], out: &mut Vec<i16>) {
        let n = left.len().min(right.len());
        out.clear();
        if out.capacity() < n * 2 {
            out.reserve(n * 2 - out.capacity());
        }
        for i in 0..n {
            out.push(to_pcm16(left[i]));
            out.push(to_pcm16(right[i]));
        }
    }
}

#[cfg(test)]
#[path = "../test/audio_out.rs"]
mod tests;
