//! Everything in the tree that turns bytes into noise, in one place.
//!
//! [`crate::runtime`] is deliberately not in here. It records what a pack *named* — `Set` and
//! `SetInfo` hand over an `SDT_*` and a pointer, and [`crate::runtime::AudioSource`] is that naming
//! turned into bytes — and stops there, because a run that has to be reproducible to the byte cannot
//! depend on a sound card existing. What is in here is the other half: [`requests`] reads the
//! runtime's state and says what should be playing, [`Speaker`] parses and decodes what that names,
//! and [`Mixer`] sums it. Nothing above this module knows a codec from a container.
//!
//! The layers, in the order a clip passes through them:
//!
//! - [`out`] — the speaker, the mixer, the eight voices, the request seam and the format sniffers.
//!   This is the only module here a front end touches.
//! - [`tone`] — the tone generator behind `ISound::PlayTone`, and the part of the OEM tone table
//!   that is not the OEM's to choose.
//! - [`smaf`] — Yamaha SMAF (`.mmf`) scores: `sequencer` maps the container's events onto the MA
//!   player's timeline, `voice` and `adpcm` decode the patch and sample banks, `fm` is the DSP.
//! - [`qcelp`] / `evrc` — the PureVoice container and the two vocoders a QCP can carry.
//! - `media_decode` — the Symphonia door for MP3, AAC, WAV and ADPCM.
//! - [`soundbank`] — the tone bank a run speaks with when it was given no `.sf2`.
//!
//! `qcelp_tables`, `evrc_tables` and `ffmpeg_decode` are support: generated constants, and the
//! optional second decoder backend behind the non-default `ffmpeg` feature.
//!
//! Nothing in this module opens a device unless [`Speaker::new`] is called. [`Speaker::without_device`]
//! is the same mixer with the caller as its clock, which is how a headless run renders a WAV and how
//! the libretro core hands back one frame's samples from inside the call that advanced it.
//!
//! A play costs two things — a parse and a decode — and neither is ever paid inside the audio
//! callback, which only sums what it was handed. A small clip is decoded where it is asked for, so
//! that a name resolves to a sound in the same frame; a large one is handed to a helper thread and
//! starts when its samples land, because a two-minute track is hundreds of milliseconds of codec
//! work and that is a freeze a person sees. Which of the two happens is
//! [`Speaker::set_async_decode`], and the reason it is a switch is reproducibility.

mod evrc;
mod evrc_tables;
#[cfg(feature = "ffmpeg")]
mod ffmpeg_decode;
mod media_decode;
pub mod out;
pub mod qcelp;
mod qcelp_tables;
pub mod smaf;
pub mod soundbank;
pub mod tone;

/// The speaker-facing half, re-exported so a caller writes `audio::Speaker` rather than
/// `audio::out::Speaker`: the file is an implementation detail of the layer, not a second level.
pub use out::*;
