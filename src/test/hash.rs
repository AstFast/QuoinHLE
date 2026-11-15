// The algorithm's own arithmetic, checked against the seven vectors RFC 1321 publishes in its
// "Test suite" appendix. No guest, no runtime: a digest that is wrong here is wrong in every pack
// that asks for one, so these are worth pinning on their own — the `IHash` slots get their own test
// in `src/test/runtime.rs`, where the guest's buffers are.

use super::*;

fn hex(digest: [u8; DIGEST]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The appendix's seven inputs, in its order, with the digest it prints beside each one.
const RFC_1321: &[(&str, &str)] = &[
    ("", "d41d8cd98f00b204e9800998ecf8427e"),
    ("a", "0cc175b9c0f1b6a831c399e269772661"),
    ("abc", "900150983cd24fb0d6963f7d28e17f72"),
    ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
    (
        "abcdefghijklmnopqrstuvwxyz",
        "c3fcd3d76192e4007dfb496cca67e13b",
    ),
    (
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        "d174ab98d277d9f5a5611c2c9f419d9f",
    ),
    (
        "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        "57edf4a22be3c955ac49da2e2107b67a",
    ),
];

#[test]
fn the_seven_rfc_1321_vectors_come_out_as_the_appendix_prints_them() {
    for (input, want) in RFC_1321 {
        let mut h = Hash::born(g::AEECLSID_MD5).expect("MD5 is the class this host serves");
        h.update(input.as_bytes());
        assert_eq!(hex(h.result()), *want, "MD5({input:?})");
    }
}

/// The 56-byte case is the boundary of the padding rule: at `len % 64 == 56` the length word no longer
/// fits beside the message, so a whole second block is written that holds only the `0x80` marker and
/// the length. The appendix's vectors above do not stand in for it — its sixth is 62 bytes, which also
/// crosses a block, but by one byte of *message* rather than by the length word alone, and its seventh
/// is 80.
#[test]
fn a_message_that_pads_past_its_block_still_hashes() {
    let input = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert_eq!(input.len(), 56);
    let mut h = Hash::born(g::AEECLSID_MD5).unwrap();
    h.update(input.as_bytes());
    assert_eq!(hex(h.result()), "8215ef0796a20bcaaae116d3876c664a");
}

/// Feeding the same bytes in pieces must equal feeding them at once: a pack reads a file in whatever
/// sizes `IFile::Read` answers, so the split is not the pack's choice and must not be observable.
#[test]
fn how_the_input_is_cut_into_updates_does_not_show_in_the_digest() {
    let whole = "The quick brown fox jumps over the lazy dog";
    let mut once = Hash::born(g::AEECLSID_MD5).unwrap();
    once.update(whole.as_bytes());

    let mut pieces = Hash::born(g::AEECLSID_MD5).unwrap();
    for chunk in whole.as_bytes().chunks(3) {
        pieces.update(chunk);
    }
    assert_eq!(hex(pieces.result()), hex(once.result()));
    assert_eq!(hex(once.result()), "9e107d9d372bb6826bd81d3542a419d6");
}

/// `GetResult` twice with no update in between, and an `Update` after a `GetResult`: the state is not
/// consumed by asking for the answer. A host that ended the hash on the first `GetResult` would make
/// the second call read as a hash of nothing, which is the empty digest above.
#[test]
fn asking_for_the_result_does_not_end_the_hash() {
    let mut h = Hash::born(g::AEECLSID_MD5).unwrap();
    h.update(b"abc");
    let first = h.result();
    assert_eq!(first, h.result(), "the second ask answers the same bytes");

    h.update(b"def");
    assert_eq!(
        hex(h.result()),
        "e80b5017098950fc58aad83c8c14978e",
        "MD5(\"abcdef\")"
    );
    assert_ne!(hex(first), hex(h.result()));
}

/// `Restart` is the only thing that clears the state, and after it the object answers exactly what a
/// freshly created one would — including the empty digest.
#[test]
fn restart_puts_the_object_back_to_where_it_started() {
    let mut h = Hash::born(g::AEECLSID_MD5).unwrap();
    h.update(b"abc");
    assert_ne!(hex(h.result()), "d41d8cd98f00b204e9800998ecf8427e");

    h.restart();
    assert_eq!(
        hex(h.result()),
        "d41d8cd98f00b204e9800998ecf8427e",
        "MD5(\"\")"
    );
    h.update(b"abc");
    assert_eq!(hex(h.result()), "900150983cd24fb0d6963f7d28e17f72");
}

/// The gate that keeps the other algorithms honest: the same table serves SHA-1 and the HMAC ids, and
/// none of them is MD5. `Runtime` only asks about ids it found in its service table, so this is the
/// second half of that decision, not a duplicate of it.
#[test]
fn only_the_md5_class_builds_an_object() {
    assert!(Hash::born(g::AEECLSID_MD5).is_some());
    for cls in [
        g::AEECLSID_SHA1,
        g::AEECLSID_HMAC_MD5,
        g::AEECLSID_GL,
        0,
    ] {
        assert!(
            Hash::born(cls).is_none(),
            "{cls:#010x} is not an MD5 object, and answering it with one would be a wrong answer"
        );
    }
}
