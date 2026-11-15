use crate::abi::generated as g;
use crate::mem::{Fault, Mem};
use crate::seed_tables::SS;

/// The block, the vector and the key are all sixteen bytes, and the member says so in both directions:
/// `Init` wants `cbIV == 16 && cbKey1 == 16`, and the walk moves `16` at a time.
pub const BLOCK: usize = 16;

/// `SEED_KeySchedule`'s sixteen constants: one word rotated left by one bit each step, starting at the
/// golden-ratio fraction. These are the immediates the member's folded arithmetic names
/// (`0x61c88647` = `-KC0`, `0x3c6ef373` = `KC1`, `0x78dde6e6` = `KC2`, …).
const KC: [u32; 16] = [
    0x9e3779b9, 0x3c6ef373, 0x78dde6e6, 0xf1bbcdcc, 0xe3779b99, 0xc6ef3733, 0x8dde6e67, 0x1bbcdccf,
    0x3779b99e, 0x6ef3733c, 0xdde6e678, 0xbbcdccf1, 0x779b99e3, 0xef3733c6, 0xde6e678d, 0xbcdccf1b,
];

/// The one substitution function the bodies use for all three lookups of a half-round. `SS[2]` and
/// `SS[3]` are the 16-bit rotations of `SS[0]`/`SS[1]`, which is why a fixed byte-to-table order here
/// still covers the rotations the published construction describes.
fn g_func(v: u32) -> u32 {
    SS[0][(v & 0xff) as usize]
        ^ SS[1][((v >> 8) & 0xff) as usize]
        ^ SS[2][((v >> 16) & 0xff) as usize]
        ^ SS[3][((v >> 24) & 0xff) as usize]
}

fn be_words(block: &[u8; BLOCK]) -> [u32; 4] {
    [
        u32::from_be_bytes(block[0..4].try_into().unwrap()),
        u32::from_be_bytes(block[4..8].try_into().unwrap()),
        u32::from_be_bytes(block[8..12].try_into().unwrap()),
        u32::from_be_bytes(block[12..16].try_into().unwrap()),
    ]
}

fn un_words(w: [u32; 4]) -> [u8; BLOCK] {
    let mut out = [0u8; BLOCK];
    for (i, v) in w.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

/// `SEED_KeySchedule(key, 16, roundKeys)`: two words per group, sixteen groups, with the group's own
/// rotation of the four key words. The member requires the length to be sixteen and answers `4098`
/// otherwise; through `Init` that value is unreachable, because `Init`'s own gate already demanded
/// sixteen and turns a failure of this function into `AEE_CRYPT_INVALID_KEY`.
pub fn schedule(key: &[u8; BLOCK]) -> [u32; 32] {
    let w = be_words(key);
    let (mut x1, mut x2, mut x3, mut x4) = (w[0], w[1], w[2], w[3]);
    let mut ks = [0u32; 32];
    // The group's rotation is a pair of plain shifts, not rotates: `x >> 8` feeds on the eight bits that
    // fall off the top, and `x << 24` keeps only the lowest byte. Written as rotates it is a different
    // function, and the difference is the whole stream.
    let mut t0 = x1.wrapping_add(x3).wrapping_sub(KC[0]);
    let mut t1 = x2.wrapping_sub(x4).wrapping_add(KC[0]);
    ks[0] = g_func(t0);
    ks[1] = g_func(t1);
    for i in 1..16 {
        if i % 2 == 1 {
            // rotate the pair this group reads, eight bits each, taking the other word's old half
            let (a, b) = (x1, x2);
            x1 = (x1 >> 8) ^ (b << 24);
            x2 = (x2 >> 8) ^ (a << 24);
        } else {
            let (a, b) = (x3, x4);
            x3 = (x3 << 8) ^ (b >> 24);
            x4 = (x4 << 8) ^ (a >> 24);
        }
        t0 = x1.wrapping_add(x3).wrapping_sub(KC[i]);
        t1 = x2.wrapping_add(KC[i]).wrapping_sub(x4);
        ks[i * 2] = g_func(t0);
        ks[i * 2 + 1] = g_func(t1);
    }
    ks
}

fn half_round(a: u32, b: u32, c: u32, d: u32, k0: u32, k1: u32) -> (u32, u32) {
    let mut t0 = c ^ k0;
    let mut t1 = (d ^ k1) ^ t0;
    t1 = g_func(t1);
    t0 = g_func(t0.wrapping_add(t1));
    t1 = g_func(t1.wrapping_add(t0));
    t0 = t0.wrapping_add(t1);
    (a ^ t0, b ^ t1)
}

fn run(w: [u32; 4], ks: &[u32; 32], encrypt: bool) -> [u8; BLOCK] {
    let (mut x1, mut x2, mut x3, mut x4) = (w[0], w[1], w[2], w[3]);
    for i in 0..16 {
        let r = if encrypt { 2 * i } else { 30 - 2 * i };
        let (k0, k1) = (ks[r], ks[r + 1]);
        if i % 2 == 0 {
            let (a, b) = half_round(x1, x2, x3, x4, k0, k1);
            x1 = a;
            x2 = b;
        } else {
            let (a, b) = half_round(x3, x4, x1, x2, k0, k1);
            x3 = a;
            x4 = b;
        }
    }
    // the two pairs come back swapped: the bodies store the pair written last into words 0 and 1
    un_words([x3, x4, x1, x2])
}

/// One block, the direction the class was created for.
pub fn step(block: &[u8; BLOCK], ks: &[u32; 32], encrypt: bool) -> [u8; BLOCK] {
    run(be_words(block), ks, encrypt)
}

/// One `ICipher` object of this family. The handset keeps all of it inside a 176-byte `aee_malloc`
/// (`+0` table, `+4` count, `+8` ready, `+9` direction, `+12` the 164-byte core: vector at `[+0]`,
/// holding area at `[+16]`, count at `[+32]`, thirty-two round keys at `[+36]`); the published type has
/// no fields, so this host holds the same five values opaque to the caller.
#[derive(Clone, Copy, Debug)]
pub struct Seed {
    /// `AEECLSID_SEED_ENCRYPT` versus `_DECRYPT`, the byte the constructor writes at `[+9]`.
    pub encrypt: bool,
    /// Set by `Init`'s success path and by nothing else; `Cipher` answers `~0` while it is clear.
    pub ready: bool,
    /// The running vector. `Init` overwrites it, and the walk advances it one block at a time.
    pub chain: [u8; BLOCK],
    /// The bytes a `Cipher` call was given but could not emit yet.
    pub partial: [u8; BLOCK],
    /// How many of `partial` are live — the member's `a1[8]`, and it survives an `Init`.
    pub pending: usize,
    pub ks: [u32; 32],
}

/// The two class ids `AEESEED_New` accepts. Anything else takes its `default` branch: the pointer slot
/// is written `NULL` and the answer is 3 — and, as in `AEERSA_New`, the failure to allocate answers the
/// same 3.
pub fn born(cls: u32) -> Option<Seed> {
    match cls {
        g::AEECLSID_SEED_ENCRYPT => Some(Seed::born(true)),
        g::AEECLSID_SEED_DECRYPT => Some(Seed::born(false)),
        _ => None,
    }
}

impl Seed {
    pub fn born(encrypt: bool) -> Self {
        Self {
            encrypt,
            ready: false,
            chain: [0; BLOCK],
            partial: [0; BLOCK],
            pending: 0,
            ks: [0; 32],
        }
    }

    /// The `Init` success path, in the order the body stores: the vector copy, then the schedule, then
    /// the flag as the last act.
    pub fn init(&mut self, chain: [u8; BLOCK], key: &[u8; BLOCK]) {
        self.chain = chain;
        self.ks = schedule(key);
        self.ready = true;
    }

    /// `SEED_CBC_EncUpdate`/`SEED_CBC_DecUpdate`: the buffered walk, statement for statement, including
    /// the reads and writes in the order the member touches the caller's memory. Returns the number of
    /// bytes emitted, which is also the value the member's `Cipher` slot answers with.
    pub fn chain_walk(
        &mut self,
        mem: &mut Mem,
        pin: u32,
        pout: u32,
        n: usize,
        pcbout: u32,
    ) -> Result<usize, Fault> {
        mem.write_u32(pcbout, 0)?;
        let mut total = self.pending + n;
        if total < BLOCK {
            let incoming = mem.read_bytes(pin, n)?;
            self.partial[self.pending..self.pending + n].copy_from_slice(&incoming);
            self.pending += n;
            return Ok(0);
        }
        // Complete the held block first: `16 - pending` bytes, and the cursor starts behind them.
        let need = BLOCK - self.pending;
        let head = mem.read_bytes(pin, need)?;
        self.partial[self.pending..BLOCK].copy_from_slice(&head);
        let mut cursor = pin + need as u32;
        let mut produced = 0usize;
        loop {
            if self.encrypt {
                let mut block = [0u8; BLOCK];
                for i in 0..BLOCK {
                    block[i] = self.chain[i] ^ self.partial[i];
                }
                self.chain = step(&block, &self.ks, true);
                total -= BLOCK;
                let take = if total < BLOCK { total } else { BLOCK };
                let next = mem.read_bytes(cursor, take)?;
                self.partial[..take].copy_from_slice(&next);
                self.partial[take..].fill(0);
                cursor += take as u32;
                mem.write_bytes(pout + produced as u32, &self.chain)?;
                produced += BLOCK;
            } else {
                let held = self.partial;
                let mut plain = step(&held, &self.ks, false);
                for i in 0..BLOCK {
                    plain[i] ^= self.chain[i];
                }
                mem.write_bytes(pout + produced as u32, &plain)?;
                produced += BLOCK;
                self.chain = held;
                total -= BLOCK;
                let take = if total < BLOCK { total } else { BLOCK };
                let next = mem.read_bytes(cursor, take)?;
                self.partial[..take].copy_from_slice(&next);
                self.partial[take..].fill(0);
                cursor += take as u32;
            }
            mem.write_u32(pcbout, produced as u32)?;
            if total < BLOCK {
                break;
            }
        }
        self.pending = total;
        Ok(produced)
    }
}
