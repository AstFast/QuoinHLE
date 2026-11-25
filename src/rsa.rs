/// `SECURITY_ERROR_BASE` is 0x600, and the numbers this family publishes are `base + 3`, `+ 4`, `+ 5`,
/// `+ 6`, `+ 7`, `+ 9`. The literal pool of `i.AEERSA_Encrypt` and
/// `i.RemovePadding` holds them as 0x603/0x604/0x605/0x606/0x607/0x609, which is how each one below is
/// assigned to a branch.
pub const AEE_CRYPT_INVALID_KEY: u32 = 1539;
pub const AEE_CRYPT_INVALID_PADTYPE: u32 = 1540;
pub const AEE_CRYPT_PAD_ERROR: u32 = 1541;
pub const AEE_CRYPT_ROLLBACKATTACK: u32 = 1542;
pub const AEE_CRYPT_BUFFER_TOO_SMALL: u32 = 1543;
pub const AEE_CRYPT_INVALID_SIZE: u32 = 1545;
pub const AEE_RSA_NOT_VERIFIED: u32 = 0x401;

/// The status numbers `aeersa/rsa.o` writes into the word its callers hand it. They are OpenSSL-style
/// reason numbers, and only two of them are ever read by this family: the caller of `RemovePadding`
/// tests 115 (which it turns into `AEE_CRYPT_ROLLBACKATTACK`) and zero. The rest are carried because
/// the bodies write them, not because anything acts on them.
pub const PAD_TOO_LONG: u32 = 110;
pub const PAD_BLOCK_NOT_01: u32 = 106;
pub const PAD_BLOCK_NOT_02: u32 = 107;
pub const PAD_TOO_SHORT: u32 = 111;
pub const PAD_NO_ZERO: u32 = 113;
pub const PAD_FILL_TOO_SHORT: u32 = 102;
pub const PAD_ZERO_TOO_SHORT: u32 = 103;
pub const PAD_DATA_TOO_LONG: u32 = 109;
pub const PAD_ROLLBACK: u32 = 115;

/// A magnitude: base-2^32 limbs, least significant first, no leading zero limb. An empty vector is
/// zero — which is how the platform code represents it too, since `BIGINT_Create` starts with one zero
/// word and `BIGINT_RemoveLeadZeroes` trims after every set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big(Vec<u32>);

impl Big {
    /// `BIGINT_SetBuf`: the caller's bytes are read most significant first, four to a limb, a short
    /// final group filling the low bytes of its limb. `bytes.len() == 0` leaves the value at zero —
    /// and note the value is cleared first, so a zero-length set is a set to zero, not a no-op.
    pub fn from_be(bytes: &[u8]) -> Self {
        let mut limbs = Vec::new();
        let mut end = bytes.len();
        while end > 0 {
            let start = end.saturating_sub(4);
            let mut word = 0u32;
            for (k, b) in bytes[start..end].iter().rev().enumerate() {
                word |= (*b as u32) << (8 * k);
            }
            limbs.push(word);
            end = start;
        }
        let mut out = Self(limbs);
        out.trim();
        out
    }

    /// `BIGINT_OutToBuf`'s length rule: `(BIGINT_NumBits + 7) / 8` bytes, and zero is one `0x00` byte
    /// rather than no bytes. This is why a padded block — which begins with a `00` byte that is not
    /// part of the number — comes back as one byte shorter than the block.
    pub fn to_be(&self) -> Vec<u8> {
        // Zero has no bits, and the body answers that with **one `00` byte** rather than no bytes:
        // `if (v6 != 0) { … } else { *a3 = 1; *a2 = 0; return 0; }`.
        if self.is_zero() {
            return vec![0u8];
        }
        let n = (self.bits() + 7) / 8;
        let mut out = vec![0u8; n];
        for i in 0..n {
            let byte = n - 1 - i;
            let w = self.0.get(byte / 4).copied().unwrap_or(0);
            out[i] = (w >> (8 * (byte % 4))) as u8;
        }
        out
    }

    /// `BIGINT_NumBits`: `32 * used` minus the leading zero bits of the top word. Zero is 0 bits.
    pub fn bits(&self) -> usize {
        match self.0.last() {
            None => 0,
            Some(top) => 32 * self.0.len() - top.leading_zeros() as usize,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    pub fn one() -> Self {
        Self(vec![1])
    }

    /// `BIGINT_cmp`, whose answer is exactly `1`, `0`, `-1`. A shorter limb array loses without the
    /// words being looked at.
    pub fn cmp(&self, other: &Self) -> i32 {
        match self.0.len().cmp(&other.0.len()) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => {
                for i in (0..self.0.len()).rev() {
                    if self.0[i] != other.0[i] {
                        return if self.0[i] > other.0[i] { 1 } else { -1 };
                    }
                }
                0
            }
        }
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn bit_at(&self, index: usize) -> bool {
        self.0
            .get(index / 32)
            .is_some_and(|w| w & (1u32 << (index % 32)) != 0)
    }

    /// `BIGINT_Multiply`, sized at `used(a) + used(b)` words like the reference `BIGINT_Create` call.
    /// One limb of the multiplier is walked at a time, with the carry pushed along the row: two
    /// 32-bit words multiply into just under 2^64, so the products fit a `u64` — but *accumulating*
    /// them there would not, which is why each row carries as it goes instead of summing first.
    fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self(Vec::new());
        }
        let mut out = vec![0u32; self.0.len() + other.0.len()];
        for (i, a) in self.0.iter().enumerate() {
            let mut carry = 0u64;
            for (j, b) in other.0.iter().enumerate() {
                let cur = out[i + j] as u64 + (*a as u64) * (*b as u64) + carry;
                out[i + j] = cur as u32;
                carry = cur >> 32;
            }
            let mut k = i + other.0.len();
            while carry != 0 && k < out.len() {
                let cur = out[k] as u64 + carry;
                out[k] = cur as u32;
                carry = cur >> 32;
                k += 1;
            }
            debug_assert!(
                carry == 0,
                "the product of two n-limb numbers fits in 2n limbs"
            );
        }
        while out.last() == Some(&0) {
            out.pop();
        }
        Self(out)
    }

    /// `BIGINT_Divide`, as restoring long division one bit at a time. The reference version reaches the
    /// same quotient through Montgomery multiplication; the difference is speed, not answer, and this
    /// host never has to cancel halfway through. `None` is a division by zero, which the documented behaviour
    /// reaches only through `IBN::Div` — not through anything on this interface.
    pub fn divmod(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let mut quotient = vec![0u32; self.0.len().max(1)];
        let mut rem = Self(Vec::new());
        for bit in (0..self.bits()).rev() {
            rem.shl_one(self.bit_at(bit));
            if rem.cmp(divisor) >= 0 {
                rem.sub_in_place(divisor);
                quotient[bit / 32] |= 1u32 << (bit % 32);
            }
        }
        while quotient.last() == Some(&0) {
            quotient.pop();
        }
        Some((Self(quotient), rem))
    }

    /// One step of the shift-and-subtract: `self = 2*self + bit`.
    fn shl_one(&mut self, bit: bool) {
        let mut carry = bit as u32;
        for w in self.0.iter_mut() {
            let next = *w >> 31;
            *w = (*w << 1) | carry;
            carry = next;
        }
        if carry != 0 {
            self.0.push(carry);
        }
        self.trim();
    }

    fn sub_in_place(&mut self, other: &Self) {
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let a = self.0[i] as i64;
            let b = other.0.get(i).copied().unwrap_or(0) as i64;
            let mut d = a - b - borrow;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            self.0[i] = d as u32;
        }
        debug_assert!(borrow == 0);
        self.trim();
    }

    /// `BIGINT_ModExp` / `BIGINT_ModExpEx`: "square, and multiply by the base where the exponent has a
    /// bit", walked from the exponent's top bit down, every step reduced. Two reference details carry
    /// over: an exponent with **no bits at all** skips the loop and answers the accumulator, so
    /// `x^0 mod m` is `1 mod m` (which is `0` when `m` is `1`); and `BIGINT_ModExp` refuses an even
    /// modulus with `AEE_EBADPARM` before it starts, a rule the only caller on this path has already
    /// enforced at `Init`.
    pub fn modpow(&self, exponent: &Self, m: &Self) -> Self {
        let mut acc = Self::one().rem(m);
        let base = self.rem(m);
        for bit in (0..exponent.bits()).rev() {
            acc = acc.mul(&acc).rem(m);
            if exponent.bit_at(bit) {
                acc = acc.mul(&base).rem(m);
            }
        }
        acc
    }

    fn rem(&self, m: &Self) -> Self {
        match self.divmod(m) {
            Some((_, r)) => r,
            None => Self(Vec::new()),
        }
    }
}

/// `data - 11`, computed the way the body does: as a signed `int`. A block shorter than eleven bytes
/// makes this negative, and then even empty data fails the test — which `usize` subtraction would not.
fn room(block_len: usize, data_len: usize) -> bool {
    (data_len as i64) <= (block_len as i64 - 11)
}

/// `AEE_RSA_padding_add_PKCS1_type_1`: `00 01 <ff…> 00 <data>` across a block of `to.len()` bytes.
/// Answers `(succeeded, status)`; the reference return value is 1/0 and every caller here reads the
/// status word instead.
pub fn add_pkcs1_type_1(to: &mut [u8], from: &[u8]) -> (bool, u32) {
    if !room(to.len(), from.len()) {
        return (false, PAD_TOO_LONG);
    }
    let n = to.len();
    let fill = n - 3 - from.len();
    to[0] = 0;
    to[1] = 1;
    to[2..2 + fill].fill(0xff);
    to[2 + fill] = 0;
    to[3 + fill..].copy_from_slice(from);
    (true, 0)
}

/// `AEE_RSA_padding_add_PKCS1_type_2`: `00 02 <nonzero random…> 00 <data>`. The filler comes from
/// `AEE_RAND_bytes`, and the body re-reads it and asks for one replacement byte wherever it finds a
/// zero — so the rule this host must keep is "none of them zero", not which bytes they are.
pub fn add_pkcs1_type_2(
    to: &mut [u8],
    from: &[u8],
    random: &mut dyn FnMut(&mut [u8]),
) -> (bool, u32) {
    if !room(to.len(), from.len()) {
        return (false, PAD_TOO_LONG);
    }
    let n = to.len();
    let fill = n - 3 - from.len();
    to[0] = 0;
    to[1] = 2;
    random(&mut to[2..2 + fill]);
    for i in 2..2 + fill {
        while to[i] == 0 {
            random(&mut to[i..i + 1]);
        }
    }
    to[2 + fill] = 0;
    to[3 + fill..].copy_from_slice(from);
    (true, 0)
}

/// `AEE_RSA_padding_add_SSLv23`. Its size rule is the same `data <= block - 11` and its random run is
/// `block - 11 - data` bytes starting at index 2 — and then the body `memset`s **the same eight bytes
/// it just randomised** to `03`, puts the zero separator at index 10 and the data at index 11. So on
/// this runtime the random filler survives only where the run reaches past those eight bytes, which is
/// what the platform code does and not what OpenSSL's version does (it writes the `03` run at the end
/// of the padding instead).
pub fn add_sslv23(to: &mut [u8], from: &[u8], random: &mut dyn FnMut(&mut [u8])) -> (bool, u32) {
    if !room(to.len(), from.len()) {
        return (false, PAD_TOO_LONG);
    }
    let fill = to.len() - 11 - from.len();
    to[0] = 0;
    to[1] = 2;
    random(&mut to[2..2 + fill]);
    for i in 2..2 + fill {
        while to[i] == 0 {
            random(&mut to[i..i + 1]);
        }
    }
    to[2..10].fill(0x03);
    to[10] = 0;
    // `aee_memmove(to + 11, data, len)` — the rest of the block keeps whatever was there before, which
    // on a handset is heap the next operation may still read.
    to[11..11 + from.len()].copy_from_slice(from);
    (true, 0)
}

/// `AEE_RSA_padding_check_PKCS1_type_1`. `from` is the block **as a number** — the leading `00` of the
/// padded block is not part of it — so the reference first gate is `from.len() + 1 == capacity`, and
/// `from[0]` is the block-type byte. `to.len()` is the caller's output capacity, which is what the
/// body tests the data length against (`v10 <= a2`).
pub fn check_pkcs1_type_1(to: &mut [u8], from: &[u8], capacity: usize) -> (i32, u32) {
    let run = from.len() as i64 - 1;
    if run + 2 != capacity as i64 || from.first() != Some(&1) {
        return (-1, PAD_BLOCK_NOT_01);
    }
    let run = run as usize;
    let mut fill = 0usize;
    while fill < run && from[1 + fill] == 0xff {
        fill += 1;
    }
    if fill == run {
        return (-1, PAD_NO_ZERO);
    }
    if from[1 + fill] != 0 {
        return (-1, PAD_FILL_TOO_SHORT);
    }
    if fill < 8 {
        return (-1, PAD_ZERO_TOO_SHORT);
    }
    let out = run - (fill + 1);
    if out > to.len() {
        return (-1, PAD_DATA_TOO_LONG);
    }
    to[..out].copy_from_slice(&from[fill + 2..fill + 2 + out]);
    (out as i32, 0)
}

/// `AEE_RSA_padding_check_PKCS1_type_2`, same `len + 1 == capacity` rule, block byte `02`, and the
/// separator found by scanning for the first zero rather than by walking a fixed fill.
pub fn check_pkcs1_type_2(to: &mut [u8], from: &[u8], capacity: usize) -> (i32, u32) {
    let run = from.len() as i64 - 1;
    if run + 2 != capacity as i64 || from.first() != Some(&2) {
        return (-1, PAD_BLOCK_NOT_02);
    }
    let run = run as usize;
    let mut i = 0usize;
    while i < run && from[1 + i] != 0 {
        i += 1;
    }
    if i == run {
        return (-1, PAD_NO_ZERO);
    }
    if i < 8 {
        return (-1, PAD_ZERO_TOO_SHORT);
    }
    let out = run - (i + 1);
    if out > to.len() {
        return (-1, PAD_DATA_TOO_LONG);
    }
    // The body's cursor had already stepped past the zero byte when it broke out of the scan.
    to[..out].copy_from_slice(&from[i + 2..i + 2 + out]);
    (out as i32, 0)
}

/// `AEE_RSA_padding_check_SSLv23`. Two rules are this one's own: a block shorter than ten bytes is
/// refused with `PAD_TOO_SHORT` *before* the length gate, and the eight bytes ahead of the separator
/// being **all `03`** is what fails it — with status 115, which `RemovePadding` turns into
/// `AEE_CRYPT_ROLLBACKATTACK`. A block whose run does not look like that is accepted, so an ordinary
/// PKCS#1 type-2 block verifies here; that is the point of an SSLv23 check.
pub fn check_sslv23(to: &mut [u8], from: &[u8], capacity: usize) -> (i32, u32) {
    if (from.len() as i64) < 10 {
        return (-1, PAD_TOO_SHORT);
    }
    let run = from.len() as i64 - 1;
    if run + 2 != capacity as i64 || from.first() != Some(&2) {
        return (-1, PAD_BLOCK_NOT_02);
    }
    let run = run as usize;
    let mut i = 0usize;
    while i < run && from[1 + i] != 0 {
        i += 1;
    }
    if i == run || i < 8 {
        return (-1, PAD_NO_ZERO);
    }
    if (0..8).all(|k| from[i - 7 + k] == 3) {
        return (-1, PAD_ROLLBACK);
    }
    let out = run - (i + 1);
    if out > to.len() {
        return (-1, PAD_DATA_TOO_LONG);
    }
    to[..out].copy_from_slice(&from[i + 2..i + 2 + out]);
    (out as i32, 0)
}

/// `AEE_RSA_padding_check_none`: left-pad the shorter value to the full block with zeros and answer
/// the block length — the reference return is `cbTo`, not the length that went in.
pub fn check_none(to: &mut [u8], from: &[u8]) -> (i32, u32) {
    if from.len() > to.len() {
        return (-1, PAD_DATA_TOO_LONG);
    }
    let gap = to.len() - from.len();
    to[..gap].fill(0);
    to[gap..].copy_from_slice(from);
    (to.len() as i32, 0)
}

/// `RemovePadding`'s dispatch and its answer rewriting. The documented behaviour passes `*pcbOut` as the
/// output capacity and writes the plaintext length back through that same word, and its own return is
/// the status word after two rewrites: 115 becomes `AEE_CRYPT_ROLLBACKATTACK`, any other nonzero
/// becomes `AEE_CRYPT_PAD_ERROR`, and an unknown pad type answers `AEE_CRYPT_INVALID_PADTYPE` without
/// touching the caller's length word at all.
///
/// `from` is the block as a number (one byte shorter than `capacity`) and `to.len()` is the capacity
/// the caller named.
pub fn remove_padding(
    pad: u32,
    from: &[u8],
    capacity: usize,
    to: &mut [u8],
) -> (u32, Option<usize>) {
    let (ret, status) = match pad {
        1 => check_pkcs1_type_1(to, from, capacity),
        2 => check_pkcs1_type_2(to, from, capacity),
        3 => check_sslv23(to, from, capacity),
        4 => check_none(to, from),
        _ => return (AEE_CRYPT_INVALID_PADTYPE, None),
    };
    if status == PAD_ROLLBACK {
        return (AEE_CRYPT_ROLLBACKATTACK, None);
    }
    if status != 0 {
        return (AEE_CRYPT_PAD_ERROR, None);
    }
    (0, Some(ret.max(0) as usize))
}

/// `ASN1Thin_MapOID`: walk the packed prefix table, compare `prefix.len()` bytes (which is the entry's
/// length byte plus one — the tag and the body up to its last byte), and answer the family's base id
/// plus that last byte. A miss answers 0.
///
/// The documented behaviour reads `p[prefix.len()]` unconditionally once a prefix matches, so it takes the
/// *following* byte from wherever the block continues, not from inside the length the caller named.
/// This takes it the same way out of the slice it is handed — the caller passes the rest of the block,
/// which is what the handset was reading — and answers 0 for the one case where the block stops at the
/// prefix, where the handset would have read whatever sat after it.
pub fn map_oid(p: &[u8], claimed: usize) -> u32 {
    for (prefix, base) in OID_PREFIXES {
        if claimed >= prefix.len() && &p[..prefix.len()] == *prefix {
            return base + p.get(prefix.len()).copied().unwrap_or(0) as u32;
        }
    }
    0
}

static OID_PREFIXES: &[(&[u8], u32)] = &[
    (
        &[0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01],
        0x0200_0000,
    ),
    (&[0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02], 0x0300_0000),
    (&[0x06, 0x03, 0x55, 0x04], 0x0100_0000),
    (&[0x06, 0x03, 0x55, 0x1d], 0x0400_0000),
    (
        &[0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x86, 0xf8, 0x42, 0x01],
        0x0500_0000,
    ),
    (
        &[
            0x06, 0x0b, 0x60, 0x86, 0x48, 0x01, 0x86, 0xf8, 0x45, 0x01, 0x07, 0x01,
        ],
        0x0600_0000,
    ),
    (
        &[
            0x06, 0x0a, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x01, 0x03,
        ],
        0x0700_0000,
    ),
    (
        &[0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x86, 0xf8, 0x42, 0x04],
        0x0800_0000,
    ),
    (
        &[0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09],
        0x0900_0000,
    ),
    (
        &[0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01],
        0x0a00_0000,
    ),
    (
        &[
            0x06, 0x0a, 0x60, 0x86, 0x48, 0x01, 0x86, 0xf8, 0x45, 0x01, 0x06,
        ],
        0x0b00_0000,
    ),
    (
        &[0x06, 0x08, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x02],
        0x0c00_0000,
    ),
    (
        &[0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03],
        0x0d00_0000,
    ),
    (
        &[
            0x06, 0x0a, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x8b, 0x29, 0x09, 0x03,
        ],
        0x0e00_0000,
    ),
];

/// Which signature types accept which OID, as `AEERSA_Done`'s compare chain decides it. The chain is
/// ordered on the mapped value, and three families can reach it: `1.2.840.113549.1.1` (base `0x02`),
/// `1.2.840.113549.2` (base `0x0c`) and `1.3.14.3.2` (base `0x03`, whose `0x1a` = SHA-1 is tested first
/// of all). Type 1 takes `0x02000002`/`0x0c000002`, type 2 takes `0x02000004`/`0x0c000005`, type 3 takes
/// `0x02000005`/`0x0c000007` and `0x0300001a`. Anything else fails, including a mapping miss, which
/// arrives as `0` — and a `1.3.6.1.4.1.1449.9.3` OID maps under base `0x0e`, which no branch accepts.
pub fn sig_type_accepts(sig: u32, id: u32) -> bool {
    match id {
        0x0200_0002 | 0x0c00_0002 => sig == 1,
        0x0200_0004 | 0x0c00_0005 => sig == 2,
        0x0200_0005 | 0x0c00_0007 | 0x0300_001a => sig == 3,
        _ => false,
    }
}

/// The DigestInfo walk `AEERSA_Done` runs after the padding has been stripped into the same block. The
/// block is `30 <outer> 30 <inner> 06 <oidlen> <oid…> [05 00] 04 <diglen> <digest…>`, so the byte at
/// `[3]` is the **inner `AlgorithmIdentifier`'s own length**, and the reference arithmetic falls out of
/// that: the `04 <diglen>` pair sits at `inner + 5` and the digest at `inner + 6`, whether or not a
/// `NULL` parameter is present (the two shapes differ by exactly those two bytes). Then the caller's
/// digest has to match, length and bytes.
///
/// One hedge in the body is a bug it carries: if the length byte is nonzero **and** the first digest
/// byte is `00`, it drops one from the length and steps one forward — so a digest that legitimately
/// begins with a zero byte is compared shifted, and no digest of that length can match. That is what
/// the handset does, so this does it too.
pub fn digest_matches(block: &[u8], digest: &[u8]) -> bool {
    let Some(&oid_len) = block.get(3) else {
        return false;
    };
    let index_of_len_byte = oid_len as usize + 5;
    let mut at = oid_len as usize + 6;
    let Some(&len_byte) = block.get(index_of_len_byte) else {
        return false;
    };
    let mut dlen = len_byte;
    if dlen != 0 && block.get(at) == Some(&0) {
        dlen -= 1;
        at += 1;
    }
    let Some(end) = at.checked_add(dlen as usize) else {
        return false;
    };
    if end > block.len() {
        return false;
    }
    digest.len() == dlen as usize && block[at..end] == *digest
}

/// What one `IRSA` object holds between calls, in the reference object's words: `[+16]` the modulus,
/// `[+12]` the exponent, `[+92]` the block the operations write into, `[+96]` how big that block is.
/// The rest of the reference hundred bytes is either a scratch object that lives and dies inside one
/// call, or the plumbing of an operation the host does not defer.
#[derive(Clone, Default)]
pub struct Rsa {
    pub modulus: Option<Big>,
    pub exponent: Option<Big>,
    /// The guest address of the block, or 0 when there is none yet.
    pub block: u32,
    pub capacity: usize,
    /// The host has said once that an operation it was asked to defer came back synchronously.
    pub announced: bool,
}

/// The filler the two randomised paddings ask for. On a handset `AEE_RAND_bytes` asks the shell for
/// `AEECLSID_RNDSOURCE` (`0x0100103C`) and calls its third slot, falling back to the runtime's own
/// `GetRand`; **nothing on this interface reads those bytes**, because the only rule the check side
/// applies to them is "the run ends at the first zero, and the part before it is at least eight bytes".
/// So this host supplies its own, from a counter, and the property the paddings need is what the tests
/// hold it to. The counter advances on every call, which is also what keeps the reference
/// "ask again for this one zero byte" loop from ever spinning.
pub fn rand_bytes(out: &mut [u8]) {
    for b in out.iter_mut() {
        *b = next_byte();
    }
}

fn next_byte() -> u8 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0x2545_f491_4f6c_dd1d);
    let z = STATE
        .fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed)
        .wrapping_add(0xbf58_476d_1ce4_e5b9);
    let mut z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (z ^ (z >> 31)) as u8
}
