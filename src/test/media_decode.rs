#[test]
fn a_real_audio_sample_decodes_when_asked() {
    let Ok(path) = std::env::var("QUOIN_MEDIA_SAMPLE") else {
        return;
    };
    let bytes = std::fs::read(path).expect("read media sample");
    let decoded = super::decode(&bytes).expect("decode");
    assert!(decoded.rate >= 8_000);
    assert!(!decoded.samples.is_empty());
    assert!(decoded.samples.iter().all(|sample| sample.is_finite()));
    assert!(decoded.samples.iter().any(|sample| sample.abs() > 1e-9));
}

/// A file the codec cannot read at all has to say which of the two failures it was: bytes no
/// container claimed, or a container whose every frame the codec rejected.
#[test]
fn a_failed_decode_says_what_it_was_reading() {
    let garbage = [0u8; 512];
    let error = match super::decode(&garbage) {
        Err(error) => error,
        Ok(decoded) => panic!("noise decoded to {} samples", decoded.samples.len()),
    };
    assert!(
        error.contains("not a format supported by Symphonia"),
        "a bare buffer is not a container: {error}"
    );
}
