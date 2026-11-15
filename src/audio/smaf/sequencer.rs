//! Turn the `smaf` crate's raw score events into the timeline the MA player consumes.
//!
//! `smaf` gives the container and the two Mobile dialects (including the Huffman body) but
//! stops at raw events. The mapping that matters is the player's: which channel base a track
//! uses, the HandyPhone +36 note base, the Mobile running velocity, a zero gate meaning "no
//! note", and the setup/voice exclusives that bind a bank/program to an FM patch. Those rules
//! are ported from the reference engine's `decodeTrack_`/`decodeMobile_`/`decodeHandyPhone_`,
//! not from `smaf-player` (which maps to GM and would lose the file's own voices).

use ::smaf::{
    BaseBit, Channel, ChannelType, FormatType, PCMAudioTrackChunk, PCMDataChunk, PcmWaveFormat,
    ScoreTrack, ScoreTrackChunk, ScoreTrackSequenceEvent, Smaf, SmafChunk, StreamWaveFormat,
};

use super::voice::{parse_voice_exclusive, ParsedVoice};

/// One timeline event. `channel` is already offset by the track base, as in the reference
/// engine; `a`/`b` keep the register units (note, velocity, controller value, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    NoteOn,
    NoteOff,
    Program,
    BankMsb,
    BankLsb,
    Volume,
    Pan,
    Expression,
    Modulation,
    PitchBend,
    OctaveShift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub time_ms: u32,
    pub kind: Kind,
    pub channel: u8,
    pub a: i32,
    pub b: i32,
}

#[derive(Clone, Debug)]
pub struct Wave {
    pub number: u8,
    pub channel: Channel,
    pub format: StreamWaveFormat,
    pub base_bit: BaseBit,
    pub sampling_rate: u16,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct Score {
    pub events: Vec<Event>,
    pub voices: Vec<ParsedVoice>,
    pub waves: Vec<Wave>,
    /// Channels whose MTR channel-status type is 3 (rhythm).
    pub rhythm: [bool; 128],
    /// Last event plus the reference engine's one-second release tail.
    pub length_ms: u32,
}

impl Score {
    pub fn parse(bytes: &[u8]) -> Result<Score, String> {
        let smaf = Smaf::parse(bytes).map_err(|e| format!("not a SMAF file: {e:?}"))?;
        let mut events = Vec::new();
        let mut voices = Vec::new();
        let mut waves = Vec::new();
        let mut rhythm = [false; 128];

        for chunk in &smaf.chunks {
            match chunk {
                SmafChunk::ScoreTrack(track_number, track) => {
                    let handy = track.format_type == FormatType::HandyPhoneStandard;
                    let base = if handy {
                        (*track_number).wrapping_mul(4)
                    } else {
                        (*track_number).wrapping_mul(16)
                    };
                    let count = if handy { 4 } else { 32 };
                    for (i, status) in track.channel_status.iter().take(count).enumerate() {
                        rhythm[(base + i as u8) as usize & 0x7f] =
                            matches!(status.channel_type, ChannelType::Rhythm);
                    }
                    for part in &track.chunks {
                        match part {
                            ScoreTrackChunk::SetupData(data) => {
                                add_setup_exclusives(data, !handy, &mut voices)
                            }
                            ScoreTrackChunk::SequenceData(sequence) => append_sequence(
                                track,
                                sequence,
                                base,
                                handy,
                                &mut events,
                                &mut voices,
                            ),
                            ScoreTrackChunk::PCMData(chunks) => {
                                for chunk in chunks {
                                    let PCMDataChunk::WaveData(number, wave) = chunk;
                                    waves.push(Wave {
                                        number: *number,
                                        channel: wave.channel,
                                        format: wave.format,
                                        base_bit: wave.base_bit,
                                        sampling_rate: wave.sampling_freq,
                                        data: wave.wave_data.to_vec(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                SmafChunk::PCMAudioTrack(_, track) => {
                    let format = match track.format {
                        PcmWaveFormat::Adpcm => StreamWaveFormat::YamahaADPCM,
                        PcmWaveFormat::TwosComplementPCM => StreamWaveFormat::TwosComplementPCM,
                        _ => continue,
                    };
                    for part in &track.chunks {
                        if let PCMAudioTrackChunk::WaveData(number, data) = part {
                            waves.push(Wave {
                                number: *number,
                                channel: track.channel,
                                format,
                                base_bit: track.base_bit,
                                sampling_rate: track.sampling_freq,
                                data: data.to_vec(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Controllers first at equal timestamps, as the reference engine sorts them: a bank
        // or program change must land before the note-on that depends on it.
        events.sort_by(|left, right| {
            left.time_ms.cmp(&right.time_ms).then_with(|| {
                let left_note = matches!(left.kind, Kind::NoteOn | Kind::NoteOff);
                let right_note = matches!(right.kind, Kind::NoteOn | Kind::NoteOff);
                left_note.cmp(&right_note)
            })
        });
        if events.len() > 400_000 {
            events.truncate(400_000);
        }
        let last = events.iter().map(|e| e.time_ms).max().unwrap_or(0);

        Ok(Score {
            length_ms: last.saturating_add(1_000),
            events,
            voices,
            waves,
            rhythm,
        })
    }
}

fn append_sequence(
    track: &ScoreTrack,
    sequence: &[::smaf::SequenceData],
    base: u8,
    handy: bool,
    events: &mut Vec<Event>,
    voices: &mut Vec<ParsedVoice>,
) {
    let mut now = 0u32;
    let mut running_velocity = [64u8; 16];
    for item in sequence {
        now = now.saturating_add(item.duration.saturating_mul(u32::from(track.timebase_d)));
        let channel = |local: u8| base.wrapping_add(local) & 0x7f;
        match &item.event {
            ScoreTrackSequenceEvent::NoteMessage {
                channel: local,
                note,
                velocity,
                gate_time,
            } => {
                let (local, note, gate_time) = (*local, i32::from(*note), *gate_time);
                // A zero gate is "no note" on the reference engine; it also explains the
                // 16-event difference between the raw stream and the rendered timeline of a
                // sample with percussion hits.
                if gate_time == 0 {
                    continue;
                }
                let ch = channel(local);
                let velocity = if handy {
                    127
                } else {
                    match *velocity {
                        Some(value) => {
                            let value = value & 0x7f;
                            running_velocity[(local & 0x0f) as usize] = value;
                            value
                        }
                        None => running_velocity[(local & 0x0f) as usize],
                    }
                };
                let note = if handy { note.saturating_add(36) } else { note };
                let off_ms =
                    now.saturating_add(gate_time.saturating_mul(u32::from(track.timebase_g)));
                events.push(Event {
                    time_ms: now,
                    kind: Kind::NoteOn,
                    channel: ch,
                    a: note,
                    b: i32::from(velocity),
                });
                events.push(Event {
                    time_ms: off_ms,
                    kind: Kind::NoteOff,
                    channel: ch,
                    a: note,
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::ProgramChange {
                channel: local,
                program,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::Program,
                    channel: channel(*local),
                    a: i32::from(*program & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::BankSelect {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::BankLsb,
                    channel: channel(*local),
                    a: i32::from(*value),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::OctaveShift {
                channel: local,
                value,
            } => {
                let value = i32::from(*value);
                let semitones = if (0x81..=0x84).contains(&value) {
                    -12 * (value - 0x80)
                } else if (1..=4).contains(&value) {
                    12 * value
                } else {
                    0
                };
                events.push(Event {
                    time_ms: now,
                    kind: Kind::OctaveShift,
                    channel: channel(*local),
                    a: semitones,
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::PitchBend {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::PitchBend,
                    channel: channel(*local),
                    a: i32::from(*value) - 8192,
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::Volume {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::Volume,
                    channel: channel(*local),
                    a: i32::from(*value & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::Pan {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::Pan,
                    channel: channel(*local),
                    a: i32::from(*value & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::Expression {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::Expression,
                    channel: channel(*local),
                    a: i32::from(*value & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::Modulation {
                channel: local,
                value,
            } => {
                events.push(Event {
                    time_ms: now,
                    kind: Kind::Modulation,
                    channel: channel(*local),
                    a: i32::from(*value & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::ControlChange {
                channel: local,
                control,
                value,
            } => {
                // The reference decoder consumes controllers 0/32/7/10/11/1 and ignores the
                // rest. Emitting the rest would make the timeline longer than the engine's
                // (bell has 20 such events, and they are exactly its 882-vs-862 difference).
                let kind = match *control {
                    0 => Kind::BankMsb,
                    32 => Kind::BankLsb,
                    7 => Kind::Volume,
                    10 => Kind::Pan,
                    11 => Kind::Expression,
                    1 => Kind::Modulation,
                    _ => continue,
                };
                events.push(Event {
                    time_ms: now,
                    kind,
                    channel: channel(*local),
                    a: i32::from(*value & 0x7f),
                    b: 0,
                });
            }
            ScoreTrackSequenceEvent::Exclusive(data) => {
                let payload = data.as_slice();
                let payload = payload.strip_suffix(&[0xf7]).unwrap_or(payload);
                if payload.first() == Some(&0x43) {
                    let voice = parse_voice_exclusive(payload);
                    if voice.valid || voice.is_pcm {
                        voices.push(voice);
                    }
                }
            }
            ScoreTrackSequenceEvent::Nop => {}
        }
    }
}

fn add_setup_exclusives(data: &[u8], mobile: bool, voices: &mut Vec<ParsedVoice>) {
    let mut at = 0usize;
    while at < data.len() {
        if data[at] == 0xff {
            at += 1;
            continue;
        }
        if data[at] != 0xf0 {
            at += 1;
            continue;
        }
        at += 1;
        let length = if mobile {
            read_vlq(data, &mut at)
        } else {
            read_hps(data, &mut at)
        };
        let Some(length) = length else { break };
        let length = length as usize;
        if length == 0 || at + length > data.len() {
            break;
        }
        let mut payload = &data[at..at + length];
        if payload.last() == Some(&0xf7) {
            payload = &payload[..payload.len() - 1];
        }
        if payload.first() == Some(&0x43) {
            let voice = parse_voice_exclusive(payload);
            if voice.valid || voice.is_pcm {
                voices.push(voice);
            }
        }
        at += length;
    }
}

fn read_vlq(data: &[u8], at: &mut usize) -> Option<u32> {
    let mut value = 0u32;
    for _ in 0..5 {
        let byte = *data.get(*at)?;
        *at += 1;
        value = (value << 7) | u32::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    Some(value)
}

fn read_hps(data: &[u8], at: &mut usize) -> Option<u32> {
    let first = *data.get(*at)?;
    *at += 1;
    if first < 0x80 {
        return Some(u32::from(first));
    }
    let second = *data.get(*at)?;
    *at += 1;
    Some(((u32::from(first & 0x7f) + 1) << 7) | u32::from(second))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_smaf_buffer_is_refused() {
        assert!(Score::parse(b"definitely not a ringtone").is_err());
    }

    #[test]
    fn a_named_sample_produces_notes_and_a_length() {
        let Ok(path) = std::env::var("QUOIN_SMAF_SAMPLE") else {
            eprintln!("skipped: QUOIN_SMAF_SAMPLE is not set");
            return;
        };
        let bytes = std::fs::read(&path).expect("read the sample named by QUOIN_SMAF_SAMPLE");
        let score = Score::parse(&bytes).expect("the sample parses");
        assert!(
            score.events.iter().any(|e| e.kind == Kind::NoteOn),
            "the score has no note-on"
        );
        assert!(score.length_ms > 1_000);
    }
}
