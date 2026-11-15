use super::*;

fn unhex(text: &str) -> Vec<u8> {
    assert_eq!(
        text.len() % 2,
        0,
        "a hex fixture with an odd digit count is a typo"
    );
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

const SEED: &str = "202122232425262728292a2b2c2d2e2f30313233";
const SEEDED_KEY: &str = "8b08d580b0fcfdb44343b8db2d24fc4ae7171c43";
const POOL53: &str = "0726456483a2c1e004234261809fbedd01203f5e7d9cbbdaf91d3c5b7a99b8d7f61a39587796b5d4f31736557493b2d1f014335271";
const POOL_AGAIN: &str = "96b0dec426086ee004234261809fbedd01203f5e7d9cbbdaf91d3c5b7a99b8d7f61a39587796b5d4f31736557493b2d1f014335271";
const LONG_POOL: &str = "a1a7a1a3a1bfa116191c1f2225282b2e3134373a3d404346494c4f5255585b5e6164676a6d707376797c7f8285888b8e9194979a9d";
const READ13: &str = "5b7ee73df9bb81bb358ad7480a";
const READ25: &str = "5b7ee73df9bb81bb358ad7480ae1a3a1c25394b5091d199b7a";
const READ40: &str =
    "5b7ee73df9bb81bb358ad7480ae1a3a1c25394b5091d199b7ad49b24b4a5a85afb87bf0b99030c9f";
const READ45: &str =
    "5b7ee73df9bb81bb358ad7480ae1a3a1c25394b5091d199b7ad49b24b4a5a85afb87bf0b99030c9f3b2b35e702";
const KEY_AFTER_1: &str = "d429d1c0200399b3ff0e1bd98f645218bd858e01";
const KEY_AFTER_2: &str = "de46ea5b9bd734d8b3b4c3338bec112456899aa0";
const KEY_AFTER_3: &str = "1a721f439e7c4c23df7239fcb972fac71b0e8c5d";

/// The per-refill entropy the fixtures drew: `REFILL[i][j] = (0xa0 + 17*i + j) % 256`.
fn refill(i: usize) -> [u8; KEY] {
    let mut block = [0u8; KEY];
    for (j, slot) in block.iter_mut().enumerate() {
        *slot = (0xa0 + 17 * i + j) as u8;
    }
    block
}

fn seeded() -> RandomSource {
    let mut source = RandomSource::new();
    source.seed(unhex(SEED).try_into().unwrap());
    source.accumulate(&unhex(POOL53));
    source
}

/// A stream drawn with the refills in order, so `drawn` counts the times the pool was emptied —
/// which is the shape of the shipped code: the twenty OEM bytes are asked for inside the refill.
/// A stream drawn with the mixing block's own entropy order — `REFILL[i]` for the `i`th draw — so
/// `drawn` says how many times the body reached for the OEM's twenty bytes.
fn draw_counted(source: &mut RandomSource, n: usize) -> Option<(Vec<u8>, usize)> {
    let mut drawn = 0usize;
    let out = source
        .next(n, 0x1234, &mut || {
            let block = refill(drawn);
            drawn += 1;
            block
        })
        .map(|bytes| (bytes, drawn))?;
    Some(out)
}

fn draw(source: &mut RandomSource, n: usize) -> Option<Vec<u8>> {
    draw_counted(source, n).map(|(bytes, _)| bytes)
}

#[test]
fn the_fixtures_are_the_widths_the_body_works_in() {
    // Checked before anything uses them: a copied hex string one digit long is the way a table of
    // "expected bytes" quietly stops meaning anything.
    for (name, text) in [
        ("SEED", SEED),
        ("SEEDED_KEY", SEEDED_KEY),
        ("KEY_AFTER_1", KEY_AFTER_1),
        ("KEY_AFTER_2", KEY_AFTER_2),
        ("READ13", READ13),
        ("READ25", READ25),
        ("READ40", READ40),
        ("READ45", READ45),
        ("KEY_AFTER_3", KEY_AFTER_3),
    ] {
        assert_eq!(text.len() % 2, 0, "{name}");
    }
    assert_eq!(SEED.len(), KEY * 2);
    assert_eq!(SEEDED_KEY.len(), KEY * 2);
    assert_eq!(KEY_AFTER_1.len(), KEY * 2);
    assert_eq!(KEY_AFTER_2.len(), KEY * 2);
    assert_eq!(KEY_AFTER_3.len(), KEY * 2);
    assert_eq!(READ13.len(), 13 * 2);
    assert_eq!(READ25.len(), 25 * 2);
    assert_eq!(READ40.len(), 40 * 2);
    assert_eq!(READ45.len(), 45 * 2);
    assert_eq!(POOL_HASHED, 53, "the width the body hashes out of the pool");
    assert_eq!(POOL_FIRST + POOL_HASHED, POOL_END);
    assert_eq!(
        POOL_CURSOR + 4,
        POOL,
        "the cursor word is the last four bytes of the pool"
    );
    assert_eq!(
        OFF_SEEDED + 4,
        STATE,
        "and the seeded byte is the last word of the 204-byte state"
    );
}

#[test]
fn one_call_draws_at_most_one_entropy_block_however_many_refills_it_needs() {
    // The host must know **whether** a call will mix before it can hand the closure over, because it
    // draws the block first. The arithmetic predicate and the real walk have to agree — and the walk
    // must never take a second block, which is the `v25` rule.
    for (n, held) in [
        (0usize, 0u32),
        (1, 0),
        (1, 20),
        (1, 1),
        (20, 0),
        (21, 0),
        (7, 13),
        (60, 5),
        (120, 0),
        (200, 0),
    ] {
        let mut source = seeded();
        source.pool_count = held;
        let (out, drawn) = draw_counted(&mut source, n).unwrap();
        assert_eq!(out.len(), n, "{n} bytes asked, {held} held");
        assert_eq!(
            drawn,
            usize::from(entropy_needed(n, held)),
            "{n} bytes from a pool holding {held}: draws"
        );
        assert!(
            drawn <= 1,
            "one call mixes at most once, {n}/{held} drew {drawn}"
        );
    }
}

#[test]
fn the_constructor_seeds_the_key_from_a_digest_and_leaves_the_pool_empty() {
    let mut source = RandomSource::new();
    assert!(!source.seeded);
    assert_eq!(source.pool_count, 0);
    // `AEERandomSource_RandBytes` answers zero for a generator nobody seeded, and the code that
    // would say why (14) is swallowed on the way — so the refusal has to be visible in the state.
    assert_eq!(draw(&mut source, 4), None);
    assert!(
        source.xkey.iter().all(|b| *b == 0),
        "a refused read touches nothing"
    );

    source.seed(unhex(SEED).try_into().unwrap());
    let block = source.state_bytes();
    assert_eq!(block.len(), STATE);
    assert_eq!(hex(&source.xout), SEEDED_KEY, "XOUT = SHA1(gathered bytes)");
    assert_eq!(
        hex(&source.xkey),
        SEEDED_KEY,
        "and the copy makes XKEY the same twenty bytes"
    );
    assert_eq!(
        &block[OFF_XOUT..OFF_XOUT + KEY],
        source.xout.as_slice(),
        "XOUT lives at the offset the body indexes, +156"
    );
    assert_eq!(
        &block[OFF_XKEY..OFF_XKEY + KEY],
        source.xkey.as_slice(),
        "XKEY lives at +176, twenty bytes past it"
    );
    assert_eq!(block[OFF_SEEDED], 1, "the byte `New` sets at +200");
    assert_eq!(
        u32::from_le_bytes(
            block[OFF_POOL_COUNT..OFF_POOL_COUNT + 4]
                .try_into()
                .unwrap()
        ),
        0,
        "and it clears the digest count at +196, so the first read regenerates"
    );
    assert!(
        source.pool.iter().all(|b| *b == 0),
        "`New` never reads or writes the pool: entropy arrives through the shell's accumulate calls"
    );
}

#[test]
fn a_refill_mixes_the_pool_the_clock_and_twenty_entropy_bytes_in_that_order() {
    let mut source = seeded();
    let (out, drawn) = draw_counted(&mut source, 13).unwrap();
    assert_eq!(hex(&out), READ13);
    assert_eq!(
        out.len(),
        13,
        "the answer is exactly what was asked for, in one call"
    );
    assert_eq!(
        source.pool_count, 7,
        "and the rest of the block stays in the object"
    );
    assert_eq!(hex(&source.xkey), KEY_AFTER_1);
    assert_eq!(drawn, 1, "one refill, one draw");

    // The same starting state, asked for a whole block plus five: two refills, but the second one
    // walks on the entropy the first already spent — that is what `v25` does, and it is the reason
    // the bytes below changed when this file stopped mixing on every refill.
    let mut again = seeded();
    let (more, more_draws) = draw_counted(&mut again, 25).unwrap();
    assert_eq!(more.len(), 25);
    assert_eq!(hex(&more), READ25);
    assert!(
        more[..13] == out[..],
        "the first thirteen bytes do not depend on how much was asked"
    );
    assert_eq!(again.pool_count, 15);
    assert_eq!(
        hex(&again.xkey),
        KEY_AFTER_2,
        "the key still advanced twice: once with the seed, once with the digest"
    );
    assert_eq!(more_draws, 1, "two refills, still one draw");

    // Forty bytes is two full blocks: the count lands back on zero, and no second draw happens.
    let mut third = seeded();
    let (whole, third_draws) = draw_counted(&mut third, 40).unwrap();
    assert_eq!(whole.len(), 40);
    assert_eq!(hex(&whole), READ40);
    assert_eq!(third.pool_count, 0);
    assert_eq!(
        hex(&third.xkey),
        KEY_AFTER_2,
        "two refills, the same key as the 25-byte ask"
    );
    assert_eq!(&whole[..25], more.as_slice());
    assert_eq!(third_draws, 1);

    // Forty-five asks for a third block, and the key moves again with no new entropy in it.
    let mut fourth = seeded();
    let (long, fourth_draws) = draw_counted(&mut fourth, 45).unwrap();
    assert_eq!(long.len(), 45);
    assert_eq!(hex(&long), READ45);
    assert_eq!(
        &long[..40],
        whole.as_slice(),
        "the first forty are the whole-block answer"
    );
    assert_eq!(fourth.pool_count, 15);
    assert_eq!(
        hex(&fourth.xkey),
        KEY_AFTER_3,
        "three refills, a key no draw touched"
    );
    assert_eq!(fourth_draws, 1, "three refills, one draw");
}

#[test]
fn an_empty_ask_answers_empty_without_drawing_anything() {
    // `FIPS186Next` guards the whole loop with `cbOut > 0`, and `RandBytes` still answers with the
    // count it was handed — so zero bytes asked is zero bytes produced and no entropy is spent.
    let mut source = seeded();
    let before = source.clone();
    let out = source.next(0, 0x1234, &mut || [0u8; KEY]).unwrap();
    assert!(out.is_empty());
    assert_eq!(source.pool_count, before.pool_count);
    assert_eq!(source.xkey, before.xkey);
    assert_eq!(source.xout, before.xout);
}

#[test]
fn the_pool_is_an_xor_ring_from_one_to_fifty_four_and_keeps_its_cursor_inside_itself() {
    // Each fixture string is pinned to the width its region has, so a hand-copied constant that
    // grew or shrank a digit fails here rather than in the comparison below.
    assert_eq!(POOL53.len(), POOL_HASHED * 2);
    assert_eq!(POOL_AGAIN.len(), POOL_HASHED * 2);
    assert_eq!(LONG_POOL.len(), POOL_HASHED * 2);

    let mut source = RandomSource::new();
    source.accumulate(&unhex(POOL53));
    assert_eq!(
        source.pool[0], 1,
        "the first call marks the pool started at [0]"
    );
    assert_eq!(
        hex(&source.pool[POOL_FIRST..POOL_END]),
        POOL53,
        "a fresh pool XORed with the ramp is the ramp"
    );
    assert_eq!(
        u32::from_le_bytes(
            source.pool[POOL_CURSOR..POOL_CURSOR + 4]
                .try_into()
                .unwrap()
        ),
        POOL_FIRST as u32,
        "fifty-three writes land the cursor back at [1], and it is stored at [56]"
    );

    // Seven more bytes only touch [1..8], because the ring wraps before [54].
    let extra = unhex("91969ba0a5aaaf");
    assert_eq!(extra.len(), 7);
    source.accumulate(&extra);
    assert_eq!(hex(&source.pool[POOL_FIRST..POOL_END]), POOL_AGAIN);
    assert_eq!(
        u32::from_le_bytes(
            source.pool[POOL_CURSOR..POOL_CURSOR + 4]
                .try_into()
                .unwrap()
        ),
        8
    );

    // Longer than the ring: sixty bytes wrap once, so [1..8] take two bytes of it.
    let long: Vec<u8> = (0..60u32).map(|i| ((i * 3 + 1) % 251) as u8).collect();
    let mut wrapped = RandomSource::new();
    wrapped.accumulate(&long);
    assert_eq!(hex(&wrapped.pool[POOL_FIRST..POOL_END]), LONG_POOL);
    assert_eq!(
        u32::from_le_bytes(
            wrapped.pool[POOL_CURSOR..POOL_CURSOR + 4]
                .try_into()
                .unwrap()
        ),
        8
    );
}

#[test]
fn the_two_byte_walks_add_from_index_zero_and_drop_the_carry_out_of_the_key() {
    let left = unhex("010e1b2835424f5c697683909daab7c4d1deebf8");
    let right = unhex("fff4e9ded3c8bdb2a79c91867b70655a4f44392e");
    assert_eq!(left.len(), KEY);
    assert_eq!(right.len(), KEY);
    let mut sum = [0u8; KEY];
    add_bytes(&mut sum, &left, &right);
    assert_eq!(hex(&sum), "00030507090b0d0f11131517191b1d1f21232527");

    // The `+1` goes on index zero — the low end of this walk — and only that end.
    let mut stepped = [0u8; KEY];
    add_bytes_plus_one(&mut stepped, &left, &right);
    assert_eq!(hex(&stepped), "01030507090b0d0f11131517191b1d1f21232527");

    // `FF..FF + 01` at index zero: every byte carries, so the whole key goes to zeros and the
    // carry that leaves the top byte is gone. Adding the `1` on top brings it back at index zero.
    let all_ones = [0xffu8; KEY];
    let mut low_one = [0u8; KEY];
    low_one[0] = 1;
    add_bytes(&mut sum, &all_ones, &low_one);
    assert_eq!(hex(&sum), "0000000000000000000000000000000000000000");
    add_bytes_plus_one(&mut stepped, &all_ones, &low_one);
    assert_eq!(hex(&stepped), "0100000000000000000000000000000000000000");

    // A short operand contributes nothing past its end, and "past its end" means the high end:
    // `[00,00,ff]` here is the number 0xFF0000 in this walk, not 0x0000FF.
    let mut part = [0u8; 4];
    add_bytes(&mut part, &[0x00, 0x00, 0xff], &[0x01, 0x02]);
    assert_eq!(part, [0x01, 0x02, 0xff, 0x00]);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
