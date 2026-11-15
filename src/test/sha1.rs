use super::*;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

#[test]
fn sha1_matches_the_published_digests() {
    assert_eq!(
        hex(&digest(b"")),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    assert_eq!(
        hex(&digest(b"abc")),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    let bytes: Vec<u8> = (0..56u8).collect();
    assert_eq!(
        hex(&digest(&bytes)),
        "636e2ec698dac903498e648bd2f3af641d3c88cb",
        "the 56-byte case is the one that lands in the second block with a non-zero tail"
    );
}

#[test]
fn sha1_matches_openssl_at_every_padding_boundary() {
    let ramp = |n: usize| -> Vec<u8> { (0..n).map(|i| (i * 7 % 251) as u8).collect() };
    let table: [(usize, &str); 13] = [
        (0, "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
        (1, "5ba93c9db0cff93f52b521d7420e43f6eda2784f"),
        (55, "a83f94113f5292bb7ed9d7df07178ad7d931341a"),
        (56, "372e1b20329e0b2862472089ac00c55505116275"),
        (57, "f5ae9bfabee67dd2d35e5473a943ddb5b7fef1d2"),
        (63, "6944938dc131b6453b7d9637cfcbad8a3db28104"),
        (64, "aec4b7f13a2b75ec13bc0c3f13fa55caf97e621d"),
        (65, "29a20455c2f21fa85c66014ff3b75bfcce5aaba5"),
        (119, "43610a35bbf6783ba62fc82d421abc8b2f3e1abf"),
        (120, "482b555c1bda1cd10f9bdc34081249810af9fd77"),
        (127, "66ac6c2e5cd910ce898e7bd293ed6851b716b8c1"),
        (128, "ddde2e96d57b78eda0da1151110a11eaa1d21d7b"),
        (300, "31d9e1a93e23cdc259c2c40d94721e0e5dbe5c2e"),
    ];
    for (n, want) in table {
        assert_eq!(
            want.len(),
            40,
            "a digest that is not 40 hex digits is a typo"
        );
        let mut h = Sha1::new();
        for chunk in ramp(n).chunks(37) {
            h.update(chunk);
        }
        assert_eq!(hex(&h.finish()), want, "length {} mod 64", n);
    }
}

#[test]
fn feeding_a_stream_one_byte_at_a_time_gives_the_same_digest() {
    // The member buffers partial blocks at `[7]`, so the split points are its business, not the
    // caller's: a chunked run has to agree with the one-shot run at every chunk size tested here.
    let data: Vec<u8> = (0..300u32).map(|i| (i * 7 % 251) as u8).collect();
    let whole = digest(&data);
    for step in [1usize, 7, 63, 64, 65, 128] {
        let mut h = Sha1::new();
        for chunk in data.chunks(step) {
            h.update(chunk);
        }
        assert_eq!(h.finish(), whole, "chunk size {step}");
    }
}

#[test]
fn the_padded_length_is_counted_in_bits_over_the_whole_run() {
    // 64 bytes is exactly one block: the message ends on a boundary, so `Final` has to open a fresh
    // block for `0x80` + zeros + the 8-byte length. Getting this wrong is the classic off-by-one.
    let one_block = digest(&[0x61u8; 64]);
    assert_eq!(
        hex(&one_block),
        hex(&{
            let mut h = Sha1::new();
            h.update(&[0x61u8; 63]);
            h.update(&[0x61u8; 1]);
            h.finish()
        })
    );
    assert_ne!(one_block, digest(&[0x61u8; 63]));
}
