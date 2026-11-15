//! Optional FFmpeg decode backend.
//!
//! This module is compiled only with `--features ffmpeg`, which links the system libav*
//! libraries through `ffmpeg-next` (the crate formerly called rust-ffmpeg). It is decode-only:
//! it returns mono f32 samples to the existing PCM mixer and never opens an output device or
//! takes over the run's clock.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use ffmpeg::{channel_layout::ChannelLayout, codec, format, frame, media};
use ffmpeg_next as ffmpeg;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Decode one media payload to mono f32 samples at its source rate.
pub fn decode(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init failed: {e}"))?;
    let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("quoin-ffmpeg-{}-{id}.bin", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| format!("ffmpeg temp file: {e}"))?;
    let result = decode_path(&path);
    let _ = std::fs::remove_file(&path);
    result
}

fn decode_path(path: &Path) -> Result<(u32, Vec<f32>), String> {
    let mut input = format::input(path).map_err(|e| format!("ffmpeg cannot open input: {e}"))?;
    let stream = input
        .streams()
        .best(media::Type::Audio)
        .ok_or_else(|| "ffmpeg found no audio stream".to_string())?;
    let stream_index = stream.index();
    let context = codec::context::Context::from_parameters(stream.parameters())
        .map_err(|e| format!("ffmpeg codec context: {e}"))?;
    let mut decoder = context
        .decoder()
        .audio()
        .map_err(|e| format!("ffmpeg audio decoder: {e}"))?;
    decoder
        .set_parameters(stream.parameters())
        .map_err(|e| format!("ffmpeg decoder parameters: {e}"))?;
    let rate = decoder.rate();
    if rate == 0 {
        return Err("ffmpeg decoder reported no sample rate".to_string());
    }
    let mut resampler = ffmpeg::software::resampling::Context::get(
        decoder.format(),
        decoder.channel_layout(),
        rate,
        format::Sample::F32(format::sample::Type::Packed),
        ChannelLayout::MONO,
        rate,
    )
    .map_err(|e| format!("ffmpeg resampler: {e}"))?;

    let mut out = Vec::new();
    let mut decoded = frame::Audio::empty();
    let mut converted = frame::Audio::empty();
    for (stream, packet) in input.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .map_err(|e| format!("ffmpeg packet: {e}"))?;
        receive_frames(
            &mut decoder,
            &mut resampler,
            &mut decoded,
            &mut converted,
            &mut out,
        )?;
    }
    decoder
        .send_eof()
        .map_err(|e| format!("ffmpeg decoder flush: {e}"))?;
    receive_frames(
        &mut decoder,
        &mut resampler,
        &mut decoded,
        &mut converted,
        &mut out,
    )?;
    if out.is_empty() {
        return Err("ffmpeg produced no audio samples".to_string());
    }
    Ok((rate, out))
}

fn receive_frames(
    decoder: &mut codec::decoder::Audio,
    resampler: &mut ffmpeg::software::resampling::Context,
    decoded: &mut frame::Audio,
    converted: &mut frame::Audio,
    out: &mut Vec<f32>,
) -> Result<(), String> {
    while decoder
        .receive_frame(decoded)
        .map_err(|e| format!("ffmpeg decode: {e}"))
        .is_ok()
    {
        *converted = frame::Audio::empty();
        resampler
            .run(decoded, converted)
            .map_err(|e| format!("ffmpeg resample: {e}"))?;
        let samples = converted.samples();
        let data = converted.plane::<f32>(0);
        out.extend_from_slice(&data[..samples]);
    }
    Ok(())
}
