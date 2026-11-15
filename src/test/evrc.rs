#[test]
fn an_empty_evrc_packet_still_decodes_to_one_frame() {
    let mut decoder = super::EvrcDecoder::default();
    let pcm = decoder.decode(super::Rate::Quant, &[0, 0]);
    assert_eq!(pcm.len(), 160);
    assert!(pcm.iter().all(|value| value.is_finite()));
}
