// Moved out of `src/keymap.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

#[test]
fn every_key_the_table_offers_exists_in_the_abi() {
    // The names are the interface to the generated constants, so a typo is caught here rather
    // than as a panic the first time a player clicks the button.
    for key in KEYS {
        assert_ne!(key.avk(), 0, "{} resolves", key.name);
    }
}

#[test]
fn the_soft_keys_are_not_the_centre_key() {
    // The whole reason a session needs a soft key at all.
    assert_eq!(physical(VK_Z), Some(avk("AVK_SOFT1")));
    assert_eq!(physical(VK_X), Some(avk("AVK_SOFT2")));
    assert_eq!(physical(VK_RETURN), Some(avk("AVK_SELECT")));
    assert_ne!(physical(VK_Z), physical(VK_RETURN));
    assert_eq!(physical(VK_ESCAPE), None);
}

#[test]
fn both_digit_rows_reach_the_handset_codes() {
    assert_eq!(physical(VK_0), physical(VK_NUMPAD0));
    assert_eq!(physical(VK_0 + 5), physical(VK_NUMPAD0 + 5));
    assert_eq!(physical(VK_0 + 9), Some(avk("AVK_9")));
    // `AVK_0` is 0xE030 and `AVK_1` is 0xE021: the digits are not one run of arithmetic, which
    // is why zero is its own arm above.
    assert_ne!(avk("AVK_0"), avk("AVK_1") + 9);
}
