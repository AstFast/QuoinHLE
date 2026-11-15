/// The digest length, and the size of the block the member's `SHA1Transform` consumes.
pub const DIGEST: usize = 20;
pub const BLOCK: usize = 64;

/// The five initial values, in the order `SHA1Init` stores them at `[0]`..`[4]`.
const IV: [u32; 5] = [
    0x6745_2301,
    0xefcd_ab89,
    0x98ba_dcfe,
    0x1032_5476,
    0xc3d2_e1f0,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sha1 {
    state: [u32; 5],
    /// The bytes held so far — `[7]`..`[22]` of the member's context, 64 bytes at offset 28.
    buf: [u8; BLOCK],
    /// Bytes currently in `buf`.
    fill: usize,
    /// Total bytes fed, which is the member's `[5]`/`[6]` pair as one count.
    total: u64,
}

impl Sha1 {
    /// `SHA1Init`: the published initial values, an empty buffer and a zero count.
    pub fn new() -> Self {
        Self {
            state: IV,
            buf: [0; BLOCK],
            fill: 0,
            total: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if self.fill > 0 {
            let want = BLOCK - self.fill;
            let take = want.min(data.len());
            self.buf[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill == BLOCK {
                let block = self.buf;
                Self::transform(&mut self.state, &block);
                self.fill = 0;
                self.buf = [0; BLOCK];
            }
        }
        while data.len() >= BLOCK {
            Self::transform(&mut self.state, &data[..BLOCK]);
            data = &data[BLOCK..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.fill = data.len();
        }
    }

    /// `SHA1Final`: pad, emit the digest, and leave the context zeroed — `self` is consumed so a caller
    /// cannot read a second digest out of a finished run.
    pub fn finish(mut self) -> [u8; DIGEST] {
        let bits = self.total.wrapping_mul(8);
        // The member pads one byte at a time until its count is 56 mod 64, then adds the eight length
        // bytes. Here it is one go: `0x80`, `pad` zeros, the length — so the length sits directly after
        // the padding, not at a fixed offset, since how much padding there is depends on `fill`.
        let pad = 56usize.wrapping_sub(self.fill.wrapping_add(1) & 63) & 63;
        let mut tail = [0u8; 72];
        tail[0] = 0x80;
        let len = 1 + pad + 8;
        tail[1 + pad..len].copy_from_slice(&bits.to_be_bytes());
        self.update(&tail[..len]);
        let mut out = [0u8; DIGEST];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn transform(state: &mut [u32; 5], block: &[u8]) {
        let mut w = [0u32; 80];
        for (i, chunk) in block[..BLOCK].chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(chunk.try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = *state;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b & c) | (!b & d), 0x5a82_7999),
                1 => (b ^ c ^ d, 0x6ed9_eba1),
                2 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        let next = [
            state[0].wrapping_add(a),
            state[1].wrapping_add(b),
            state[2].wrapping_add(c),
            state[3].wrapping_add(d),
            state[4].wrapping_add(e),
        ];
        *state = next;
    }
}

/// One-shot digest, which is how the random source uses it: `Init`, one `Update`, `Final`.
pub fn digest(data: &[u8]) -> [u8; DIGEST] {
    let mut h = Sha1::new();
    h.update(data);
    h.finish()
}

#[cfg(test)]
#[path = "test/sha1.rs"]
mod tests;
