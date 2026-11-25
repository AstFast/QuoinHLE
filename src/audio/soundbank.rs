//! A built-in tone bank: waveform tables we generate, wrapped in the container the synth engine reads.
//!
//! ## Why this exists
//!
//! Until now the host could only make sound if the player pointed it at a `.sf2` file, because
//! `Speaker::new` opened one before it could build a single voice. A real handset never asked for
//! that: the LG MX7000 firmware image ships plain `.mid` files under `/usr/content/midi`, advertises
//! `General MIDI` in its own UI, and takes no sound file from its owner — so on the device the timbre
//! came from something inside the machine. The evidence, and the three ways I mis-measured it,
//! are summarised in the module notes below.
//!
//! ⚠ What that record does **not** establish is the stronger claim this file used to make in its own
//! voice — "there is no sampled instrument bank in the whole 32 MiB flash". That rests on `sfbk`/DLS
//! magic being absent from a scan with **no positive control** (there is no real `.sf2` on this box to
//! search for), and the entry that measured it labels the conclusion weak and says not to use it. The
//! decision to carry a bank stands on the two strong facts — the songs are in the image, and no file
//! comes from the user — not on what the phone's instrument was made of.
//!
//! ## Why a bank rather than a synthesizer
//!
//! The obvious reading of "built-in table synthesis" is "write our own sequencer and renderer". That
//! was rejected for one specific reason: `rustysynth::MidiFile` keeps its tracks private (its public
//! surface is `new`, `new_with_loop_type`, `get_length`), so rendering ourselves means writing a
//! second Standard MIDI File parser — and then two authorities would decide "is this playable":
//! `audio::sniff`, which gates what the run believes it can play, and the new parser, which decides
//! what actually sounds. Two such authorities would disagree about what is playable.
//! So the tables go **into** the engine that already sequences, mixes and
//! renders offline (`--dump-audio` keeps working, and `--soundfont` stays as an explicit override), and
//! the new code is only the bank itself.
//!
//! ## What the format demands
//!
//! `rustysynth` 1.3.6 parses a SoundFont with a strictness worth stating, because every number below
//! exists to satisfy one of its checks rather than to look like a real bank:
//! - `RIFF` + `sfbk`, then `LIST INFO`, then **`LIST sdta` before `LIST pdta`** (`SoundFont::new` reads
//!   info, sample data, parameters in that order).
//! - Inside `pdta`, the seven sub-chunks `phdr`, `pbag`, `pgen`, `inst`, `ibag`, `igen`, `shdr` must all
//!   be present, and an id it does not know is a hard error. `pmod`/`imod` *are* known and would be
//!   skipped if sent empty (`soundfont_parameters.rs:58/62` + `binary_reader.rs:118`), so leaving them
//!   out is this bank's choice, not a requirement — there is nothing to route them to.
//! - Record widths come from its readers: `phdr` 38 (name 20, program, bank, **zone index as the third
//!   u16**, then three i32), `inst` 22, `shdr` 46, bag entry 4, generator 4.
//! - The last record of each list is a terminator and is dropped; zone and generator counts are derived
//!   from the *next* record's index, so each bag list has exactly one more entry than it has zones and
//!   its last entry points at the generator terminator. An extra entry whose index runs past the end of
//!   the generator list is not ignored — it becomes a zone whose span is read out of bounds, and that
//!   is a panic in the dependency rather than an `Err` (measured: `zone.rs:21`, "len is 129 but the
//!   index is 129"). An extra entry with the *same* index as the terminator is merely a dead zone.
//! - **An instrument zone must end with `sampleID`.** Both zone builders ask "is the first zone a
//!   global one?" by checking whether its *last* generator is the id that closes a real zone (`INSTRUMENT`
//!   for a preset, `SAMPLE_ID` for an instrument); if it is not, the only zone is consumed as a global
//!   prefix and the instrument is left with **zero regions**. `sanity_check` walks regions, so it
//!   cannot see that: the bank loads without complaint and every note it is asked for is silent. That
//!   asymmetry is why the tests here measure sound rather than an `Ok`.
//! - `SoundFont::sanity_check` rejects `start < 0`, `start_loop < 0`, `end >= wave_data.len()`,
//!   `end_loop >= wave_data.len()`, `end <= start`, `end_loop < start_loop`, and `start_loop >= end_loop`
//!   for any looping region, so the loop bounds are arithmetic, not taste. It walks **regions**, which
//!   is why the bullet above is the dangerous one: an instrument with no regions has nothing to reject.
//! - Five of the six `*VolEnv` generators — everything but `sustainVolEnv`, which is the next bullet —
//!   are **timecents**: the engine reads them as `2^(x/1200)` seconds, so a written `0` is one second
//!   of silence, not "none". The value this engine *defaults* an omitted stage to is -12000, which is
//!   one millisecond. That is why no delay stage is written below: writing the spec's "default" 0 there
//!   would put a full second between every note-on and its sound. The engine also refuses to make a
//!   release shorter than 10 ms, on its own initiative (`region_ex.rs:62`).
//! - `sustainVolEnv` is a **level the decay falls to and holds at, in 0.1 dB of attenuation** — the
//!   engine computes it as `decibels_to_linear(-0.1 × raw)`. So the spec's default of 0 is a voice that
//!   never falls at all (that is the organ below), and a struck voice has to ask for the floor to be
//!   pushed out of the way instead of relying on 0. What sets how long a struck note rings is
//!   `decayVolEnv`, which this engine reads as the time to fall 80 dB and then cuts off at -60 dB.
//! - `shdr`'s `pitchCorrection` is *added* to fine tune (`region_pair.rs:217`) and fine tune is in
//!   cents, so an integer-length table can sit between two semitones without being sharp — by the
//!   correction the `root_key` function works out, which is positive when the table's own pitch is
//!   *flat* of the key it is declared at.
//!
//! All of that is what this build's parser does with the bytes written below, checked against
//! `rustysynth` 1.3.6's own sources rather than against a specification document — which matters most
//! for the two unit readings, because a generator in the wrong unit is not an error to the parser. It
//! is a note that never ends, or a bank that loads and plays nothing.

use std::sync::OnceLock;

/// The rate every table declares itself at.
///
/// A SoundFont sample carries its own rate and the engine resamples, so this does not have to be the
/// device's rate — it has to be the rate the tables were built at, which is the only way the declared
/// pitch and the audible pitch can agree.
const RATE: i32 = 44_100;

/// Samples in one cycle of every table.
///
/// The power of two is incidental; the **integer cycle** is the requirement. A loop whose length is
/// not a whole number of the fundamental leaves a step at the seam, and that step is one click per
/// cycle — at this bank's root that is 172 clicks a second, heard as a buzz under every held note.
const CYCLE: usize = 256;

/// Extra samples after the loop, repeating its head.
///
/// Not what makes the seam clean — the oscillator wraps its own second interpolation sample, so a
/// table of exactly one cycle would loop without a click. What these are for is the *declaration*: a
/// looping region's `end` has to sit past `endLoop` with real samples behind it, and a reader that
/// interpolates at `end - 1` rather than wrapping would otherwise read the next instrument's wave.
const PAD: usize = 8;

/// One table's length in samples.
const TABLE: usize = CYCLE + PAD;

/// The fundamental every table is built at, in hertz.
const ROOT_HZ: f32 = (RATE as f32) / (CYCLE as f32);

/// The MIDI note that fundamental sits inside of, and its cent offset from that note.
///
/// Neither is chosen: the exact frequency is rounded to the nearest equal-tempered note and the
/// remainder travels as `pitchCorrection`. The sign is the part worth writing down, because the
/// engine *adds* the correction to the pitch and a table whose fundamental is flat of its key needs a
/// positive number: 44100/256 is 172.27 Hz, the nearest key is 53 at 174.61 Hz, and so the answer is
/// +23 cents. Read the other way it is a bank that is a quarter tone flat on every note.
fn root_key() -> (u8, i8) {
    let note = 69.0 + 12.0 * (ROOT_HZ / 440.0).log2();
    let key = note.round();
    (key as u8, ((key - note) * 100.0).round() as i8)
}

/// How many harmonics a table is allowed to carry.
///
/// One looped table serves the whole keyboard, and the engine resamples it by stepping a fixed-point
/// position, so anything above the output's Nyquist at the *played* pitch folds back down rather than
/// disappearing. With this bank's root at note 53 (174.6 Hz), the 32nd partial stays under 22.05 kHz
/// up to note 76 — the whole range a game melody lives in — and above that the fold set is the
/// quietest partials the wave has (1/k, so -30 dB at k=32). Going to the cycle's own limit of 128
/// buys brightness that is worth a few dB at the bottom of the range and pays for it with audible
/// inharmonic fizz at the top. This is the compromise; nothing here measures which end of it a
/// person can hear over a real handset, because there is no real handset here.
const MAX_PARTIAL: u32 = 32;

/// One voice shape: which harmonics are present and how loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Sine,
    Triangle,
    Square,
    Saw,
    Organ,
    Reed,
    Pluck,
    Perc,
}

/// Every class in table order. The `as u16` casts below rely on this order and on nothing else, so the
/// enum's declaration *is* the instrument index list.
pub const CLASSES: [Class; 8] = [
    Class::Sine,
    Class::Triangle,
    Class::Square,
    Class::Saw,
    Class::Organ,
    Class::Reed,
    Class::Pluck,
    Class::Perc,
];

impl Class {
    /// The name written into the bank's records, so a dump says which voice is which.
    fn tag(self) -> &'static str {
        match self {
            Class::Sine => "sine",
            Class::Triangle => "triangle",
            Class::Square => "square",
            Class::Saw => "saw",
            Class::Organ => "organ",
            Class::Reed => "reed",
            Class::Pluck => "pluck",
            Class::Perc => "perc",
        }
    }

    /// Harmonic number and amplitude, with the signs that make each wave its own shape.
    fn partials(self) -> Vec<(u32, f32)> {
        let limit = (CYCLE as u32 / 2).min(MAX_PARTIAL);
        let mut out: Vec<(u32, f32)> = Vec::new();
        match self {
            Class::Sine => out.push((1, 1.0)),
            // Odd harmonics, 1/k², alternating sign: the triangle's turns.
            Class::Triangle => {
                let mut k = 1_u32;
                let mut flip = 1.0_f32;
                while k <= limit {
                    out.push((k, flip / (k * k) as f32));
                    flip = -flip;
                    k += 2;
                }
            }
            Class::Square => {
                let mut k = 1_u32;
                while k <= limit {
                    out.push((k, 1.0 / k as f32));
                    k += 2;
                }
            }
            Class::Saw => {
                let mut k = 1_u32;
                while k <= limit {
                    out.push((k, (1.0 / k as f32) * if k % 2 == 1 { 1.0 } else { -1.0 }));
                    k += 1;
                }
            }
            Class::Organ => out.extend([(1, 1.0), (2, 0.5), (4, 0.25), (8, 0.12)]),
            Class::Reed => out.extend([
                (1, 1.0),
                (2, 0.6),
                (3, 0.35),
                (4, 0.2),
                (5, 0.12),
                (6, 0.07),
            ]),
            Class::Pluck => out.extend((1..=limit).map(|k| (k, 1.0 / (k as f32).powf(0.8)))),
            // The noise voice is built elsewhere; it has no harmonic content by design.
            Class::Perc => {}
        }
        out
    }

    /// This class's samples: one whole cycle, plus the head copy `PAD` argues for.
    ///
    /// The noise voice comes from a small integer congruence, not from anything with a seed the run
    /// could depend on, so the table is byte-identical on every start — which is what keeps
    /// "same pack, same rendered WAV" true.
    fn table(self) -> Vec<i16> {
        let mut body: Vec<f32> = Vec::with_capacity(CYCLE);
        if self == Class::Perc {
            let mut s: u32 = 0x2545_F491;
            for i in 0..CYCLE {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = (s >> 8) as f32 / 8_388_608.0 - 1.0;
                // The decay is inside the table: the loop then sits in the tail of a drum hit, quiet
                // enough that hearing it repeat is not heard as a repetition.
                body.push(v * (1.0 - i as f32 / CYCLE as f32).powi(3));
            }
        } else {
            let partials = self.partials();
            for i in 0..CYCLE {
                let phase = std::f32::consts::TAU * i as f32 / CYCLE as f32;
                body.push(
                    partials
                        .iter()
                        .map(|(k, a)| a * (phase * *k as f32).sin())
                        .sum(),
                );
            }
        }
        let peak = body
            .iter()
            .fold(0.0_f32, |m, v| if v.abs() > m { v.abs() } else { m })
            .max(1.0e-6);
        // 0.85 of full scale. There is no compressor anywhere in this path: `audio::build` mixes
        // the voices by adding them and clamps the sum at ±1.0, so a dense chord does clip, and the
        // way this bank keeps that from being heard on an ordinary melody is a lower peak per table
        // plus the per-class `attenuate` below. Whether the ceiling should be a limiter instead is an
        // open question, not a settled one.
        let scale = 0.85 / peak;
        let to_i16 = |v: f32| (v * scale * 32_767.0).round() as i16;
        let mut out: Vec<i16> = body.iter().map(|v| to_i16(*v)).collect();
        out.extend(body.iter().take(PAD).map(|v| to_i16(*v)));
        out
    }

    /// Volume envelope: the four stages this engine needs, plus the two levels.
    fn voice(self) -> Voice {
        match self {
            // The sustaining voices fall to a floor and sit there until the note is let go. Where that
            // floor is decides how much of the note is "the shape" and how much is "the hold".
            Class::Sine => Voice {
                attack: 0.010,
                decay: 1.2,
                sustain: 250,
                release: 0.100,
                attenuate: 0,
            },
            Class::Triangle => Voice {
                attack: 0.012,
                decay: 1.5,
                sustain: 280,
                release: 0.120,
                attenuate: 0,
            },
            Class::Square => Voice {
                attack: 0.008,
                decay: 1.0,
                sustain: 300,
                release: 0.080,
                // Square and saw carry more energy above the fundamental than the rest of the bank;
                // 4 dB of that difference is heard as "less brash", which is what a MIDI square lead
                // needs to sit in a game mix rather than sit on it.
                attenuate: 60,
            },
            Class::Saw => Voice {
                attack: 0.050,
                decay: 1.8,
                sustain: 320,
                release: 0.150,
                attenuate: 40,
            },
            // The one voice whose floor is *nothing*: `sustain: 0` asks for 0 dB of attenuation, which
            // the engine reads as a level of 1.0, and a decay that can only fall to where the attack
            // already is holds at full until the note is released. That is an organ.
            Class::Organ => Voice {
                attack: 0.006,
                decay: 4.0,
                sustain: 0,
                release: 0.020,
                attenuate: 0,
            },
            Class::Reed => Voice {
                attack: 0.030,
                decay: 0.8,
                sustain: 150,
                release: 0.080,
                attenuate: 30,
            },
            // Struck voices have no floor to hold at, so the decay *is* the note. Anything at or above
            // 600 (60 dB) already puts the floor under the level the engine stops a voice at, so this
            // is far out of the way rather than tuned; what ends these is the clock, not the level.
            Class::Pluck => Voice {
                attack: 0.002,
                decay: 0.40,
                sustain: 4608,
                release: 0.080,
                attenuate: 0,
            },
            Class::Perc => Voice {
                attack: 0.001,
                decay: 0.15,
                sustain: 4608,
                release: 0.020,
                attenuate: 0,
            },
        }
    }
}

/// One voice's shape in time and in level.
///
/// The times are seconds and become timecents on the way to the generator; the levels are already in
/// the generator's own unit, 0.1 dB of attenuation. `sustain` is the floor the decay falls to, so 0
/// means "never falls" and 4608 means "there is no floor". See the module docs for why those two
/// readings are the opposite of what the SoundFont spec's defaults suggest.
struct Voice {
    attack: f32,
    decay: f32,
    sustain: i16,
    release: f32,
    attenuate: i16,
}

/// Seconds into the timecents this engine raises 2 to the power of.
fn timecents(seconds: f32) -> i16 {
    (1200.0_f32 * seconds.log2()).round() as i16
}

/// Which class answers a General-MIDI program.
///
/// This is **our** mapping, not the register's: GM names 128 instruments and this bank has eight
/// voices, so the honest form is one place that says which family got which voice and lets a reader
/// argue with it. Anyone comparing this to a real handset should expect disagreement *here* first, and
/// never in pitch.
fn class_of(program: u8) -> Class {
    match program {
        0..=7 => Class::Pluck,
        8..=15 => Class::Sine,
        16..=23 => Class::Organ,
        24..=31 => Class::Pluck,
        32..=39 => Class::Saw,
        40..=55 => Class::Saw,
        56..=63 => Class::Square,
        64..=71 => Class::Reed,
        72..=79 => Class::Sine,
        80..=87 => Class::Square,
        88..=95 => Class::Saw,
        96..=103 => Class::Organ,
        104..=111 => Class::Triangle,
        112..=119 => Class::Perc,
        // A MIDI program number is seven bits, and the preset list this table is read from stops at
        // 127, so the spare half of a `u8` is a case the compiler still makes me answer for. Answering
        // it is one voice more than answering it with a panic.
        120..=u8::MAX => Class::Sine,
    }
}

/// Generator opcodes, as this build's parser numbers them (`generator_type.rs`).
///
/// Only the ones written into the bank are named. The envelope stages missing here — `delayVolEnv`
/// and `holdVolEnv` — are deliberately *not* emitted: their defaults in the engine are one
/// millisecond, which is right, while the value an author might reach for first (`0`) is one second,
/// which would put a second of silence in front of every note.
mod gen {
    pub const ATTACK_VOL: u16 = 34;
    pub const DECAY_VOL: u16 = 36;
    pub const SUSTAIN_VOL: u16 = 37;
    pub const RELEASE_VOL: u16 = 38;
    pub const INSTRUMENT: u16 = 41;
    pub const INITIAL_ATTENUATION: u16 = 48;
    pub const SAMPLE_ID: u16 = 53;
    pub const SAMPLE_MODES: u16 = 54;
}

/// The bank as the bytes the engine reads.
pub fn image() -> &'static [u8] {
    static BANK: OnceLock<Vec<u8>> = OnceLock::new();
    BANK.get_or_init(build)
}

/// The number of presets: the 128 GM programs plus one percussion map.
const PRESETS: usize = 129;

/// Assemble it.
///
/// One function rather than several because the lists are mutually index-addressed — a `phdr` entry
/// names a position in `pbag`, which names a position in `pgen`, and each list's terminator is the next
/// list's length. Splitting that across functions means carrying the counters and re-deriving the same
/// arithmetic twice, which is how an off-by-one in a SoundFont gets born.
fn build() -> Vec<u8> {
    let (root, correction) = root_key();

    let mut wave: Vec<i16> = Vec::with_capacity(CLASSES.len() * TABLE);
    for class in CLASSES {
        wave.extend_from_slice(&class.table());
    }

    let mut shdr: Vec<u8> = Vec::new();
    for (i, class) in CLASSES.iter().enumerate() {
        let start = (i * TABLE) as i32;
        push_name(&mut shdr, class.tag());
        push_i32(&mut shdr, start);
        push_i32(&mut shdr, start + TABLE as i32 - 1);
        push_i32(&mut shdr, start);
        push_i32(&mut shdr, start + CYCLE as i32);
        push_i32(&mut shdr, RATE);
        shdr.push(root);
        shdr.push(correction as u8);
        push_u16(&mut shdr, 0);
        push_u16(&mut shdr, 1);
    }
    push_name(&mut shdr, "EOS");
    for _ in 0..5 {
        push_i32(&mut shdr, 0);
    }
    shdr.extend_from_slice(&[0; 6]);

    let mut inst: Vec<u8> = Vec::new();
    let mut ibag: Vec<u8> = Vec::new();
    let mut igen: Vec<u8> = Vec::new();
    for (i, class) in CLASSES.iter().enumerate() {
        push_name(&mut inst, class.tag());
        push_u16(&mut inst, i as u16);
        bag(&mut ibag, igen.len() as u16 / 4);
        let v = class.voice();
        push_gen(&mut igen, gen::ATTACK_VOL, timecents(v.attack));
        push_gen(&mut igen, gen::DECAY_VOL, timecents(v.decay));
        push_gen(&mut igen, gen::SUSTAIN_VOL, v.sustain);
        push_gen(&mut igen, gen::RELEASE_VOL, timecents(v.release));
        push_gen(&mut igen, gen::INITIAL_ATTENUATION, v.attenuate);
        // 1 = loop continuously, which is the only mode a one-cycle table can be played in.
        push_gen(&mut igen, gen::SAMPLE_MODES, 1);
        // Last, and not by taste: the reader decides whether this zone is a real one or a global
        // prefix by looking at the generator that closes it. See the module docs.
        push_gen(&mut igen, gen::SAMPLE_ID, i as i16);
    }
    push_name(&mut inst, "EOI");
    push_u16(&mut inst, CLASSES.len() as u16);
    bag(&mut ibag, igen.len() as u16 / 4);
    push_gen(&mut igen, 0, 0);

    let mut phdr: Vec<u8> = Vec::new();
    let mut pbag: Vec<u8> = Vec::new();
    let mut pgen: Vec<u8> = Vec::new();
    for program in 0..PRESETS {
        // Bank 0 holds the GM programs; the one bank-128 preset answers channel 10 with the noise
        // voice for the whole key range — the drum *map* is deliberately not invented here, because a
        // per-note kit is a guess at 128 sounds this bank does not have.
        let (name, patch, bank, class) = if program < 128 {
            let class = class_of(program as u8);
            (
                format!("{} {}", class.tag(), program),
                program as u16,
                0u16,
                class,
            )
        } else {
            ("percussion".to_string(), 0, 128u16, Class::Perc)
        };
        push_name(&mut phdr, &name);
        push_u16(&mut phdr, patch);
        push_u16(&mut phdr, bank);
        // The third u16 is the index of this preset's first zone, and one zone per preset means it
        // is the preset's own number.
        push_u16(&mut phdr, program as u16);
        for _ in 0..3 {
            push_i32(&mut phdr, 0);
        }
        bag(&mut pbag, pgen.len() as u16 / 4);
        push_gen(&mut pgen, gen::INSTRUMENT, class as i16);
    }
    push_name(&mut phdr, "EOP");
    push_u16(&mut phdr, 0);
    push_u16(&mut phdr, 0);
    push_u16(&mut phdr, PRESETS as u16);
    for _ in 0..3 {
        push_i32(&mut phdr, 0);
    }
    bag(&mut pbag, pgen.len() as u16 / 4);
    push_gen(&mut pgen, 0, 0);

    let mut pdta: Vec<u8> = Vec::new();
    chunk(&mut pdta, b"phdr", &phdr);
    chunk(&mut pdta, b"pbag", &pbag);
    chunk(&mut pdta, b"pgen", &pgen);
    chunk(&mut pdta, b"inst", &inst);
    chunk(&mut pdta, b"ibag", &ibag);
    chunk(&mut pdta, b"igen", &igen);
    chunk(&mut pdta, b"shdr", &shdr);

    let mut info: Vec<u8> = Vec::new();
    chunk(&mut info, b"ifil", &[2, 0, 1, 0]);
    text(&mut info, b"isng", "EMU8000");
    text(&mut info, b"INAM", "Quoin built-in tone tables");
    text(&mut info, b"irom", "QuoinHLE");
    chunk(&mut info, b"iver", &[2, 0, 1, 0]);

    let samples: Vec<u8> = wave
        .iter()
        .flat_map(|s| (*s as u16).to_le_bytes())
        .collect();
    let mut sdta: Vec<u8> = Vec::new();
    chunk(&mut sdta, b"smpl", &samples);

    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"sfbk");
    body.extend_from_slice(&list(b"INFO", &info));
    body.extend_from_slice(&list(b"sdta", &sdta));
    body.extend_from_slice(&list(b"pdta", &pdta));

    let mut out: Vec<u8> = Vec::with_capacity(12 + body.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// One INFO text chunk: the string, its terminator, and a second terminator if that leaves an odd
/// count.
///
/// RIFF pads a chunk to an even size with a byte that is **not** inside the declared length, and this
/// parser reads exactly the declared number of bytes and then looks for the next id — so an odd text
/// chunk makes the following id start one byte early, and the whole `INFO` list comes back as an
/// unknown-id error. Padding the string is what real banks do, and it keeps the size honest.
fn text(out: &mut Vec<u8>, id: &[u8; 4], body: &str) {
    let mut bytes = body.as_bytes().to_vec();
    bytes.push(0);
    if bytes.len() % 2 != 0 {
        bytes.push(0);
    }
    chunk(out, id, &bytes);
}

/// `LIST` + length + kind + body, ready to be concatenated into the RIFF payload.
fn list(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(b"LIST");
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 != 0 {
        out.push(0);
    }
}

fn push_name(out: &mut Vec<u8>, name: &str) {
    let mut bytes = [0u8; 20];
    let src = name.as_bytes();
    bytes[..src.len().min(20)].copy_from_slice(&src[..src.len().min(20)]);
    out.extend_from_slice(&bytes);
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_gen(out: &mut Vec<u8>, kind: u16, amount: i16) {
    push_u16(out, kind);
    push_u16(out, amount as u16);
}

/// One bag entry: a generator index, and a modulator index this bank never uses.
fn bag(out: &mut Vec<u8>, generator_index: u16) {
    push_u16(out, generator_index);
    push_u16(out, 0);
}

#[cfg(test)]
#[path = "../test/soundbank.rs"]
mod tests;
