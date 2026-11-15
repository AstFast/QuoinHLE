//! Decode the audio containers the MIDI/QCP paths do not cover.
//!
//! Symphonia is a pure-Rust decoder: MP3, AAC, RIFF/WAV and ADPCM. It returns interleaved samples;
//! this module downmixes them to the mono stream the mixer already renders. A format this module
//! cannot decode stays unnamed and unplayed — it is not a licence to guess at bytes.

use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// One clip's worth of mono PCM, plus everything the decode knows that the samples do not say.
///
/// The counts are here because a decode that loses part of a file is the one failure a caller
/// cannot see in the samples: a stream whose tenth frame is corrupt plays nine tenths of itself and
/// sounds like a short file. Carrying the count out of here is what lets the report say so.
pub struct Decoded {
    /// The source sample rate the samples are at.
    pub rate: u32,
    /// Mono f32 samples, one per frame.
    pub samples: Vec<f32>,
    /// Frames the container handed over that the codec refused. Playback skips them, which is the
    /// only thing a whole-file decode can do with them, but the run's report says how many.
    pub dropped: usize,
    /// Whether the container asked to be re-probed part-way through — a chained stream, which this
    /// decoder does not follow. What came back is the first segment, and the report says so.
    pub chained: bool,
}

impl Decoded {
    /// A decode that lost nothing, for the paths that cannot lose anything (QCP, SMAF).
    pub fn whole(rate: u32, samples: Vec<f32>) -> Decoded {
        Decoded {
            rate,
            samples,
            dropped: 0,
            chained: false,
        }
    }
}

/// Decode a supported container to mono f32 samples and its source sample rate.
///
/// `Err` means "this decoder did not produce audio"; it is deliberately not a promise that the bytes
/// are not some other format. The caller falls back to MIDI/QCP and reports the joined failure.
///
/// A packet the codec rejects is skipped rather than fatal — one corrupt frame in the middle of a
/// two-minute track is not a reason to refuse the track — but it is counted, and a file where
/// *nothing* decoded has to say how many frames it tried, because "this is not audio" and "the
/// codec choked on every frame" are different sentences in a report.
pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("not a format supported by Symphonia: {e}"))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "the container has no decodable audio track".to_string())?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("no decoder for this track: {e}"))?;

    let mut rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut out = Vec::new();
    let mut dropped: usize = 0;
    let mut chained = false;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(Error::ResetRequired) => {
                // A chained stream: the container has a second segment this decoder cannot be
                // handed, because re-probing mid-file would need a fresh `FormatReader`. Playing
                // the first segment and saying so beats playing it and pretending it is the whole.
                chained = true;
                break;
            }
            Err(error) => return Err(format!("container read failed: {error}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                let channels = spec.channels.count();
                if channels == 0 {
                    continue;
                }
                let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                buffer.copy_interleaved_ref(decoded);
                let samples = buffer.samples();
                for frame in samples.chunks_exact(channels) {
                    let sum: f32 = frame.iter().copied().sum();
                    out.push(sum / channels as f32);
                }
            }
            Err(Error::DecodeError(_)) => dropped += 1,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(error) => return Err(format!("audio decode failed: {error}")),
        }
    }

    if out.is_empty() {
        return Err(if dropped > 0 {
            format!("the codec rejected all {dropped} frames in the file")
        } else {
            "the decoder produced no audio samples".to_string()
        });
    }
    if rate == 0 {
        return Err("the decoder did not report a sample rate".to_string());
    }
    Ok(Decoded {
        rate,
        samples: out,
        dropped,
        chained,
    })
}

/// How long a container says it is, in milliseconds, without decoding it.
///
/// The frame count is the container's own answer — a RIFF header's `data` size, an MP3's Xing/Info
/// header — so this costs a header read rather than a decode, which matters because the callers ask
/// often: `ISoundPlayer::Set` measures a clip as it names it, and `GetTotalTime` measures again on
/// every call rather than trusting a cached figure.
///
/// `None` is "this container does not say", which is not the same as zero: a stream with no frame
/// count (a VBR MP3 with no Xing header, say) is one whose length this host genuinely does not know
/// before it has played it, and a wrong number here would make a pack's `DONE` fire in the wrong
/// place. The runtime already distinguishes "0 ms" from "no length" elsewhere; this keeps the
/// distinction honest at the source.
pub fn duration_ms(bytes: &[u8]) -> Option<u32> {
    // The copy is the decoder's own shape, not a choice made here: `MediaSourceStream` takes a
    // `Box<dyn MediaSource>` and a boxed trait object has to be `'static`, so a borrowed cursor
    // cannot be handed over.
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let track = probed
        .format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)?;
    let rate = track.codec_params.sample_rate?;
    if rate == 0 {
        return None;
    }
    let frames = track.codec_params.n_frames?;
    // `u32` milliseconds is 49 days; a saturating conversion would quietly turn a nonsense length
    // into a plausible one, so an oversized frame count is "unknown" rather than a guess.
    u32::try_from(frames.saturating_mul(1_000) / u64::from(rate)).ok()
}

#[cfg(test)]
#[path = "../test/media_decode.rs"]
mod tests;
