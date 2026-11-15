use crate::sha1::Sha1;

/// `XKEY` and `XOUT` are both one SHA-1 digest wide.
pub const KEY: usize = 20;
/// `gFIPS186` — the whole state block the member keeps in `.bss`.
pub const STATE: usize = 204;
/// `gFIPS186Pool`, and the three roles its 60 bytes are split between.
pub const POOL: usize = 60;
/// Where entropy starts (`unk_499` = pool+1) and where the cursor stops (`unk_4CE` = pool+54).
pub const POOL_FIRST: usize = 1;
pub const POOL_END: usize = 54;
/// The write cursor lives in the pool's own tail: `dword_4D0` = pool+56.
pub const POOL_CURSOR: usize = 56;
/// How many pool bytes the generator hashes at each refill — `POOL_END - POOL_FIRST`.
pub const POOL_HASHED: usize = POOL_END - POOL_FIRST;
/// The state offsets the body indexes, as the numbers it uses.
pub const OFF_XOUT: usize = 156;
pub const OFF_XKEY: usize = 176;
pub const OFF_POOL_COUNT: usize = 196;
pub const OFF_SEEDED: usize = 200;

/// `AEERandomSourceAddBytes` — the body's add: it walks the destination from index 0 upward, summing
/// `left[i] + right[i] + carry` and pushing the carry **into the next index**. So index 0 is the least
/// significant byte (which is also why the `+1` of X9.17 is written at `v24[0]`), and a short operand
/// is read as zeros past its end — padding at the high end. The carry out of the last byte is dropped,
/// which is the `mod 2^160` the standard says and this code does.
pub fn add_bytes(dst: &mut [u8], left: &[u8], right: &[u8]) {
    let mut carry = 0u16;
    for (i, slot) in dst.iter_mut().enumerate() {
        let a = u16::from(*left.get(i).unwrap_or(&0));
        let b = u16::from(*right.get(i).unwrap_or(&0));
        let sum = a + b + carry;
        *slot = sum as u8;
        carry = sum >> 8;
    }
}

pub fn add_bytes_plus_one(dst: &mut [u8], left: &[u8], right: &[u8]) {
    let mut carry = 0u16;
    for (i, slot) in dst.iter_mut().enumerate() {
        let a = u16::from(*left.get(i).unwrap_or(&0));
        let b = u16::from(*right.get(i).unwrap_or(&0));
        let sum = a + b + u16::from(i == 0) + carry;
        *slot = sum as u8;
        carry = sum >> 8;
    }
}

pub fn entropy_needed(n: usize, pool_count: u32) -> bool {
    // The held bytes cover `pool_count` of the ask; anything past them can only come from a refill,
    // and the first refill of a call is the one that mixes.
    n > pool_count as usize
}

/// The generator, with the same fields in the same roles as `gFIPS186` and `gFIPS186Pool`.
#[derive(Clone, Debug)]
pub struct RandomSource {
    /// `XKEY` at state+176 — the value the next digest comes from.
    pub xkey: [u8; KEY],
    /// `XOUT` at state+156 — the digest currently being handed out.
    pub xout: [u8; KEY],
    /// State+196: how many bytes of `xout` nobody has taken yet.
    pub pool_count: u32,
    /// State+200: the byte `AEERandomSource_New` sets, and `FIPS186Next` refuses without.
    pub seeded: bool,
    /// `gFIPS186Pool`: `[0]` the "started" flag, `[1..54]` the entropy, `[56..60]` the write cursor.
    pub pool: [u8; POOL],
}

impl Default for RandomSource {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomSource {
    /// The `.bss` image before anyone has run: zeros everywhere, so `seeded` is false and a `Read`
    /// refuses. The shipped `New` is the only thing that changes this.
    pub fn new() -> Self {
        Self {
            xkey: [0; KEY],
            xout: [0; KEY],
            pool_count: 0,
            seeded: false,
            pool: [0; POOL],
        }
    }

    /// `AEERandomSource_New`, seeding half: it clears the digest count and the seeded byte, hashes the
    /// twenty bytes it gathered into `XOUT` (state+156) and copies that into `XKEY` (state+176), then
    /// sets the seeded byte. Note it never reads or writes the pool — the pool is filled by
    /// [`RandomSource::accumulate`], which the shell calls on its own events.
    ///
    /// Marked `#[inline(never)]`: the `Sha1` context is 104 bytes and this runs inside the dispatcher's
    /// frame, which every nested downcall pays for.
    #[inline(never)]
    pub fn seed(&mut self, entropy: [u8; KEY]) {
        self.pool_count = 0;
        self.seeded = false;
        self.xout = crate::sha1::digest(&entropy);
        self.xkey = self.xout;
        self.seeded = true;
    }

    /// `AEEAccumulateRandomnessForFIPS186`: XOR each byte into the pool at the cursor, wrapping before
    /// `pool[54]`. The first call of the run initialises the cursor to `pool[1]` and marks the pool
    /// started at `pool[0]`. The cursor itself is kept inside the pool, at `pool[56..60]`, which is why
    /// the shipped loop reads and writes it as an address.
    pub fn accumulate(&mut self, data: &[u8]) {
        if self.pool[0] == 0 {
            self.set_cursor(POOL_FIRST as u32);
            self.pool[0] = 1;
        }
        for byte in data {
            let at = self.cursor() as usize;
            self.pool[at] ^= byte;
            let next = at + 1;
            self.set_cursor(if next >= POOL_END {
                POOL_FIRST as u32
            } else {
                next as u32
            });
        }
    }

    fn cursor(&self) -> u32 {
        u32::from_le_bytes(self.pool[POOL_CURSOR..POOL_CURSOR + 4].try_into().unwrap())
    }

    fn set_cursor(&mut self, value: u32) {
        self.pool[POOL_CURSOR..POOL_CURSOR + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[inline(never)]
    pub fn next(
        &mut self,
        n: usize,
        time_ms: u16,
        entropy: &mut dyn FnMut() -> [u8; KEY],
    ) -> Option<Vec<u8>> {
        if !self.seeded {
            return None;
        }
        let mut out = Vec::with_capacity(n);
        let mut remaining = n;
        // `v25`, the body's own local: it is cleared before the loop and set inside the seed block, so
        // one call mixes at most once — a 45-byte ask that refills three times pays for one digest of
        // entropy and two plain `XKEY → XOUT` steps.
        let mut mixed = false;
        while remaining > 0 {
            if self.pool_count == 0 {
                if !mixed {
                    let mut h = Sha1::new();
                    h.update(&self.pool[POOL_FIRST..POOL_END]);
                    h.update(&time_ms.to_le_bytes());
                    h.update(&entropy());
                    let seed = h.finish();
                    let mut stepped = [0u8; KEY];
                    add_bytes(&mut stepped, &self.xkey, &seed);
                    self.xkey = stepped;
                    mixed = true;
                }
                self.xout = crate::sha1::digest(&self.xkey);
                self.pool_count = KEY as u32;
                let mut stepped = [0u8; KEY];
                add_bytes_plus_one(&mut stepped, &self.xkey, &self.xout);
                self.xkey = stepped;
            }
            let take = remaining.min(self.pool_count as usize);
            let from = KEY - self.pool_count as usize;
            out.extend_from_slice(&self.xout[from..from + take]);
            self.pool_count -= take as u32;
            remaining -= take;
        }
        Some(out)
    }

    /// The state as the member's own offsets would show it, for a test or a dump to compare against.
    pub fn state_bytes(&self) -> [u8; STATE] {
        let mut block = [0u8; STATE];
        block[OFF_XOUT..OFF_XOUT + KEY].copy_from_slice(&self.xout);
        block[OFF_XKEY..OFF_XKEY + KEY].copy_from_slice(&self.xkey);
        block[OFF_POOL_COUNT..OFF_POOL_COUNT + 4].copy_from_slice(&self.pool_count.to_le_bytes());
        block[OFF_SEEDED] = u8::from(self.seeded);
        block
    }
}

#[cfg(test)]
#[path = "test/randomsource.rs"]
mod tests;
