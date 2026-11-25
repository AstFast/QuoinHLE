// Moved out of `src/disasm.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

fn text_of(bytes: &[u8], base: u32, thumb: bool, limit: usize) -> Vec<String> {
    let mut out: Vec<String> = disassemble(bytes, base, thumb, limit)
        .into_iter()
        .map(|l| l.text)
        .collect();
    out.dedup_by(|a, b| a.is_empty() && b.is_empty());
    out
}

#[test]
fn arm_words_read_back_as_the_instructions_they_are() {
    // The first two words of every reference `.mod` in the sample set: a branch to the entry point and
    // a branch to its neighbour. Both were read off real images, so a decoder that disagrees with
    // these disagrees with the sample set.
    let words = [0xea00_000eu32, 0xea00_000fu32];
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let lines = disassemble(&bytes, 0x1000_0000, false, 8);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].addr, 0x1000_0000);
    assert_eq!(lines[0].bytes, vec![0x0e, 0x00, 0x00, 0xea]);
    assert!(lines[0].text.starts_with("b "), "{}", lines[0].text);
    assert!(lines[1].text.starts_with("b "), "{}", lines[1].text);
    assert_ne!(lines[0].text, lines[1].text, "the two branches differ");
}

#[test]
fn the_prologue_a_module_opens_its_load_with_is_named() {
    // `push {r0, r1, r2, r4, lr}`, the word a `.mod` starts with after its branch word. This is the
    // single line a listing of a `.mod` should get right first.
    let bytes = 0xe92d4017u32.to_le_bytes();
    let lines = disassemble(&bytes, 0, false, 1);
    assert_eq!(lines.len(), 1);
    let text = &lines[0].text;
    assert!(text.contains("push"), "{text}");
    assert!(text.contains("lr"), "{text}");
}

#[test]
fn thumb_halfwords_are_decoded_in_thumb_state() {
    // `push {r4, lr}` as Thumb-1 spells it, which is the first instruction of a module a pack has
    // just inflated.
    let bytes = 0xb510u16.to_le_bytes();
    let lines = disassemble(&bytes, 0x2001_2344, true, 1);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].text.contains("push"), "{}", lines[0].text);
    // The same bytes in ARM state are not that instruction, which is the point of having the
    // state in the signature at all.
    let arm = disassemble(&bytes, 0x2001_2344, false, 1);
    assert!(!arm[0].text.contains("push"), "{}", arm[0].text);
}

#[test]
fn a_listing_that_walks_into_a_decoder_panic_reports_it_and_keeps_going() {
    let bytes: Vec<u8> = [
        0x26u8, 0x98, 0x01, 0x68, 0x0a, 0x69, 0x29, 0x49, 0x79, 0x44, 0xed, 0xf7, 0xf1, 0xfb, 0x02,
        0xe0, 0x00, 0x20, 0x11, 0x90, 0xb0, 0x62, 0xb0, 0x6a, 0x2e, 0x99, 0x88, 0x42, 0x05, 0xda,
        0x26, 0x98, 0x01, 0x68, 0x49, 0x68, 0xed, 0xf7, 0xe3, 0xfb, 0x75, 0xe6, 0x2e, 0x99, 0x88,
        0x42, 0x08, 0xd1, 0x01, 0x30, 0xb0, 0x62, 0xe2, 0x6b, 0x41, 0x01, 0xf0, 0x6a, 0x52, 0x6f,
        0xed, 0xf7, 0xd8, 0xfb, 0xf0, 0x62, 0xe0, 0x6b, 0x0c, 0xa9, 0x80, 0x30, 0x82, 0x6b, 0x00,
        0x20, 0xed, 0xf7, 0xd0, 0xfb, 0x27, 0x98, 0x08, 0xab, 0x0a, 0x90, 0x38, 0x68, 0x00, 0x6d,
        0x40, 0x6a, 0x18, 0x70, 0x70, 0x6a, 0x0b, 0x90, 0xe0, 0x6b, 0x80, 0x30, 0xc1, 0x69, 0x11,
        0x48, 0x78, 0x44, 0xed, 0xf7, 0xbf, 0xfb, 0x26, 0x98, 0x04, 0x22, 0x01, 0x68, 0x8b, 0x68,
        0x0e, 0x49, 0x79, 0x44, 0xed, 0xf7, 0xb9, 0xfb, 0x07, 0x1c, 0x00, 0x68, 0x01, 0x22, 0x43,
        0x69, 0x03, 0x48, 0x2d, 0x99, 0x01, 0x30, 0x13, 0xe0, 0xca, 0xda, 0xfe, 0xff, 0x86, 0x81,
        0x00, 0x00, 0x0a, 0x02, 0x00, 0x00, 0x03, 0x10, 0x00, 0x01, 0xae, 0x7e, 0x00, 0x00, 0x9a,
        0x7e, 0x00, 0x00,
    ]
    .to_vec();
    let lines = disassemble(&bytes, 0x1001_2840, false, 45);
    assert!(
        lines.len() > 30,
        "the walk kept going: {} lines",
        lines.len()
    );
    assert!(
        lines
            .iter()
            .any(|l| l.error.as_deref().is_some_and(|e| e.contains("panicked"))),
        "and the word it fell over on is named: {:?}",
        lines
            .iter()
            .filter(|l| l.text.is_empty())
            .map(|l| (l.addr, &l.error))
            .collect::<Vec<_>>()
    );
    // Read in the state it was actually running in, the instruction the fault report is about is
    // the one that says everything: the handle the host handed back was null, and the pack's next
    // move was to load its vtable.
    let thumb = disassemble(&bytes, 0x1001_2840, true, 200);
    let fault = thumb
        .iter()
        .find(|l| l.addr == 0x1001_28c2)
        .expect("the faulting halfword is inside the listing");
    assert_eq!(fault.text, "ldr r0, [r0]", "{}", fault.text);
}

#[test]
fn bytes_too_short_for_an_instruction_are_a_line_and_not_the_end() {
    // One byte with no halfword after it: there is no Thumb instruction there, and the listing has
    // to say so rather than panic or invent one.
    let lines = disassemble(&[0x00u8], 0x1000, true, 4);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].error.is_some(), "{}", lines[0].render());
    assert!(lines[0].render().contains(';'), "{}", lines[0].render());

    // A refusal in the middle of a listing is a line, not the end of it: `nop` then half a
    // halfword, and the walk carries on to report the second one.
    let bytes = vec![0x00u8, 0xbf, 0x00];
    let lines = disassemble(&bytes, 0x1000, true, 4);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].error.is_none(), "{}", lines[0].render());
    assert!(lines[1].error.is_some(), "{}", lines[1].render());
    assert_eq!(
        lines[1].addr, 0x1002,
        "the refused bytes are the ones after nop"
    );
}

#[test]
fn the_listing_stops_at_the_limit_and_at_the_end_of_the_bytes() {
    let bytes: Vec<u8> = std::iter::repeat(0xea00_000eu32.to_le_bytes())
        .take(8)
        .flatten()
        .collect();
    assert_eq!(disassemble(&bytes, 0, false, 3).len(), 3);
    assert_eq!(disassemble(&bytes, 0, false, 100).len(), 8);
    assert!(disassemble(&[], 0, false, 4).is_empty());
}

#[test]
fn text_of_is_only_used_by_these_tests() {
    // Keeps the helper honest: an empty limit still walks nothing.
    assert!(text_of(&[], 0, false, 4).is_empty());
}
