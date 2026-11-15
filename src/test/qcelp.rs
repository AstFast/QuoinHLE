use super::*;

fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 != 0 {
        out.push(0);
    }
    out
}

fn fixture(codec: &[u8], data: &[u8]) -> Vec<u8> {
    let mut format = vec![0u8; 0x96];
    format[20..20 + codec.len()].copy_from_slice(codec);
    let body = [
        &b"QLCM"[..],
        &chunk(b"fmt ", &format),
        &chunk(b"data", data),
    ]
    .concat();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[test]
fn a_full_rate_qcp_is_split_into_rate_prefixed_packets() {
    let mut data = vec![4u8];
    data.extend_from_slice(&[0x11; 34]);
    let bytes = fixture(b"Qcelp 13K", &data);
    let qcp = Qcp::parse(&bytes).expect("QCP parses");
    assert_eq!(qcp.codec, Codec::Qcelp13);
    let frames = qcp.frames().expect("frames");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].rate, FrameRate::Full);
    assert_eq!(frames[0].payload.len(), 34);
    assert_eq!(qcp.duration_ms().unwrap(), 20);
}

#[test]
fn evrc_is_recognised_from_the_format_name() {
    let bytes = fixture(b"EVRC", &[1, 1, 2, 3, 4]);
    assert_eq!(Qcp::parse(&bytes).unwrap().codec, Codec::Evrc);
}

#[test]
fn a_full_rate_frame_decodes_lsp_gain_and_pitch_parameters() {
    let mut frame = QcelpFrame::default();
    frame.lspv[..5].copy_from_slice(&[41, 91, 92, 40, 20]);
    frame.plag[0] = 30;
    frame.pgain[0] = 3;
    let mut decoder = QcelpDecoder::default();
    let params = decoder.decode(&mut frame, FrameRate::Full).expect("params");
    assert!(params.gain.iter().all(|value| value.is_finite()));
    assert_eq!(params.pitch_lag[0], 46);
    assert!(params.pitch_gain[0] > 0.0);
}

#[test]
fn a_full_rate_frame_produces_160_pcm_samples() {
    let mut frame = QcelpFrame::default();
    frame.lspv[..5].copy_from_slice(&[41, 91, 92, 40, 20]);
    let mut decoder = QcelpDecoder::default();
    let pcm = decoder
        .decode_pcm(&mut frame, FrameRate::Full, 0x1234)
        .expect("PCM");
    assert_eq!(pcm.len(), 160);
    assert!(pcm.iter().all(|value| value.is_finite()));
    assert!(pcm.iter().any(|value| value.abs() > 1e-9));
}

#[test]
fn a_real_qcp_sample_decodes_when_asked() {
    let Ok(path) = std::env::var("QUOIN_QCP_SAMPLE") else {
        return;
    };
    let bytes = std::fs::read(path).expect("read QCP sample");
    let qcp = Qcp::parse(&bytes).expect("QCP parses");
    let mut decoder = QcelpDecoder::default();
    let frames = qcp.frames().expect("frames");
    let expected_samples = frames.len() * 160;
    let mut samples = 0usize;
    let mut energy = 0.0f32;
    for frame in frames {
        let mut unpacked = QcelpFrame::unpack(frame.rate, frame.payload).expect("frame");
        let seed = if frame.payload.len() >= 2 {
            u16::from_be_bytes([frame.payload[0], frame.payload[1]])
        } else {
            0
        };
        let pcm = decoder
            .decode_pcm(&mut unpacked, frame.rate, seed)
            .expect("PCM");
        assert_eq!(pcm.len(), 160);
        samples += pcm.len();
        energy += pcm.iter().map(|value| value * value).sum::<f32>();
    }
    assert_eq!(samples, expected_samples);
    assert!(energy > 0.0, "a real QCP clip should not decode to silence");
}

#[test]
fn full_rate_lsp_frequencies_are_stable_and_monotonic() {
    let mut frame = QcelpFrame::default();
    frame.lspv[..5].copy_from_slice(&[41, 91, 92, 40, 20]);
    let mut state = LspState::default();
    let lsp = state
        .decode(&frame, FrameRate::Full)
        .expect("full-rate LSP");
    assert!(lsp.windows(2).all(|pair| pair[0] < pair[1]), "{lsp:?}");
    assert!(lsp[9] > 0.66 && lsp[9] < 0.985);
}

#[test]
fn octave_lsp_prediction_stays_inside_the_unit_interval() {
    let frame = QcelpFrame::default();
    let mut state = LspState::default();
    let lsp = state.decode(&frame, FrameRate::Eighth).expect("octave LSP");
    assert!(lsp.windows(2).all(|pair| pair[0] < pair[1]), "{lsp:?}");
    assert!(lsp[0] >= LSP_SPREAD_FACTOR);
    assert!(lsp[9] <= 1.0 - LSP_SPREAD_FACTOR);
}

#[test]
fn a_full_rate_payload_unpacks_into_a_frame() {
    let frame =
        QcelpFrame::unpack(FrameRate::Full, &[0u8; 34]).expect("34 bytes hold a full frame");
    assert_eq!(frame, QcelpFrame::default());
    let short = QcelpFrame::unpack(FrameRate::Full, &[0u8; 1]).unwrap_err();
    assert_eq!(short, Error::Truncated);
}

#[test]
fn a_zeroed_full_payload_is_not_a_silence_packet() {
    // SILENCE is a one-byte payload with rate byte 0; a zeroed full payload still goes through
    // the full-rate bitmap and is a (zero-valued) frame.
    let frame = QcelpFrame::unpack(FrameRate::Full, &[0u8; 34]).unwrap();
    assert_eq!(frame.reserved, 0);
    assert_eq!(frame.lspv, [0; 10]);
}
