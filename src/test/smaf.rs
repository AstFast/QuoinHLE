//! Tests for the SMAF decoder. The format parser is the MIT `smaf` crate and the synthesis is the
//! in-tree Rust port; these pin the host's side of the seam: what counts as SMAF, what a refusal
//! says, and — when a real file is named through the environment — that bytes become non-silent
//! PCM with a duration.

use super::*;

#[test]
fn the_magic_needs_a_plausible_declared_size() {
    let mut good = b"MMMD".to_vec();
    good.extend_from_slice(&16u32.to_be_bytes());
    good.extend_from_slice(&[0u8; 16]);
    assert!(looks_like(&good));

    assert!(!looks_like(b"MMMD"), "an 8-byte header is not a file");
    assert!(!looks_like(b"not a smaf file"));

    let mut bad = b"MMMD".to_vec();
    bad.extend_from_slice(&9999u32.to_be_bytes());
    bad.extend_from_slice(&[0u8; 16]);
    assert!(
        !looks_like(&bad),
        "a declared size past the buffer is not a file"
    );
}

#[test]
fn garbage_is_refused_by_name() {
    let error = decode(b"definitely not a ringtone").unwrap_err();
    assert!(
        error.contains("not a SMAF") || error.contains("not built"),
        "unexpected refusal: {error}"
    );
    assert_eq!(duration_ms(b"definitely not a ringtone"), None);
}

#[test]
fn a_named_sample_decodes_to_non_silent_pcm() {
    let Ok(path) = std::env::var("QUOIN_SMAF_SAMPLE") else {
        eprintln!("skipped: QUOIN_SMAF_SAMPLE is not set");
        return;
    };
    let bytes = std::fs::read(&path).expect("read the sample named by QUOIN_SMAF_SAMPLE");
    let length = duration_ms(&bytes).expect("the sample has a playable track");
    let (rate, samples) = decode(&bytes).expect("the sample decodes");
    assert_eq!(rate, SAMPLE_RATE);
    assert!(
        samples.len() > rate as usize,
        "one second of audio at least, got {} frames",
        samples.len()
    );
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(rms > 0.001, "the render is silent: rms={rms}");

    let rendered_ms = (samples.len() as u64 * 1000 / u64::from(rate)) as u32;
    assert!(
        rendered_ms.abs_diff(length) <= 10,
        "duration said {length} ms, rendering produced {rendered_ms} ms"
    );
}
