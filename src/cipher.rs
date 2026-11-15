//! `ICipher`: the DES and triple-DES classes.
//!
//! Four classes — `AEECLSID_DES_ENCRYPT`/`DES_DECRYPT`/`3DES_ENCRYPT`/`3DES_DECRYPT` — share one
//! five-slot `ICipher`, and the only thing that separates a class from its sibling is the direction
//! the `Cipher` slot runs its block primitive in. The interface is five slots — three of its own over
//! `IBase` — with **no `QueryInterface`**, so `Init` sits in the slot an `IQI` caller would use;
//! nothing here
//! answers a `QueryInterface` because the object has no such slot to answer with.
//!
//! What the object keeps is one eight-byte initialization vector, one or three eight-byte keys and a
//! byte that says an `Init` has run. The handset stores all of it inside the allocation — 212 bytes
//! for a DES object, 600 for a 3DES one, with the vector at `[+4]`, the schedules at
//! `[+12]`/`[+204]`/`[+396]`, the count at `[+204]`/`[+592]` and the flag at `[+208]`/`[+596]` — but
//! `ICipher` is an opaque type with no published fields, so that layout is a private detail and none
//! of it is a caller's to read. This host therefore keeps an
//! opaque shell like the other services ([`crate::runtime`] hands out eight bytes with the count at
//! `[+4]`) and holds the same five values host-side in [`Cipher`]. The two differ in one observable:
//! a handset's `aee_malloc` leaves the flag byte uninitialized, so a `Cipher` call made before any
//! `Init` answers `~garbage`, while this host's objects start with the flag clear and answer the
//! `-1` that a clear byte computes to.
//!
//! The block primitive comes from `des`; everything else here is this host's own reading of what the
//! object does. That split is worth stating plainly, because the interesting half is not
//! the DES round function — it is the parts the object wraps around it:
//!
//! * `Init` reads an `AEECipherInit`, a struct of four
//!   pointer/length pairs. It copies **eight** bytes of vector whatever `cbIV` claimed, refuses a
//!   null vector only when the caller said eight, and then demands a key: exactly one eight-byte key
//!   for DES, and at least one of the three for 3DES. Having passed that gate it schedules all three
//!   keys unconditionally, so a 3DES `Init` that offered only `pbKey2` walks into address zero on its
//!   way to `pbKey1`.
//! * The key schedule's self-checks are off on a handset and always have been, so the parity and
//!   weak-key tests never run. An odd-parity key, an all-`0x01` key: the schedule is built from the
//!   56 bits as given and `Init` says success. `Init` also throws away the scheduler's answer, so
//!   even if the check were on it could not be reported.
//! * `Cipher` clamps the caller's length word to `min(cbIn, *pcbOut)` before it looks at anything
//!   else, and then runs the CBC walk over that many bytes — which, for a length that is not a
//!   multiple of eight, is not a whole number of blocks. The tail rules are asymmetric: encrypting a
//!   short tail fills a
//!   block with the remaining bytes and leaves the rest zero, XORs **all eight** of those against the
//!   vector — so the missing positions come out as the vector's own bytes — encrypts that **full**
//!   block and writes all eight bytes out, while `*pcbOut` and the
//!   return still say the shorter number; decrypting a short tail reads a full block — up to seven
//!   bytes past what the caller declared — and writes only the remaining few. The vector is updated
//!   in place from whatever block was last touched, so a second call continues the chain, and a
//!   caller that trusts `*pcbOut` about the size of its own output buffer is wrong by up to seven
//!   bytes.
//!
//! [`Cipher::chain`] is that walk, and it is the one piece of this interface that reaches into the
//! caller's memory a block at a time rather than in one span — which is what makes an output buffer
//! overlapping its input come out the way it does on a handset.
//!
//! Two shapes are deliberately not reproduced, both because the handset's version is a memory
//! scribble rather than an answer: a negative `*pcbOut` reaches the tail branch with every byte of the
//! block missing, so the handset encrypts the vector itself (zeros XORed against it) into the caller's
//! buffer and, when decrypting, writes below the output pointer it was handed; and `n == 0` is the one
//! case the walk handles by doing nothing at all, which this host also does. Lengths of zero are
//! therefore exact; negative ones get the count the handset reports without its out-of-bounds writes.

use des::cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use des::{Des, TdesEde3};

use crate::abi::generated as g;
use crate::mem::{Fault, Mem};

/// The four class ids `AEEDES3DES_New` accepts, and what the object each one builds is born doing.
///
/// Every other id takes the handset's default branch: the pointer slot is written `NULL` and the
/// answer is 3 — which is the shell's refusal to a `CreateInstance`, not something an object of this
/// type can be asked.
pub fn born(cls: u32) -> Option<Cipher> {
    match cls {
        g::AEECLSID_DES_ENCRYPT => Some(Cipher::born(false, true)),
        g::AEECLSID_DES_DECRYPT => Some(Cipher::born(false, false)),
        g::AEECLSID_3DES_ENCRYPT => Some(Cipher::born(true, true)),
        g::AEECLSID_3DES_DECRYPT => Some(Cipher::born(true, false)),
        _ => None,
    }
}

/// `AEE_CRYPT_INVALID_KEY` — `SECURITY_ERROR_BASE + 3`. `Init` answers this
/// for a missing parameter struct as well as for a missing key, because the struct is where the key is.
pub const AEE_CRYPT_INVALID_KEY: u32 = 0x603;
/// `AEE_CRYPT_INVALID_IV` — `SECURITY_ERROR_BASE + 8`.
pub const AEE_CRYPT_INVALID_IV: u32 = 0x608;
/// The size of the vector slot and of every key: the block, always eight, whatever `cbIV` says.
pub const BLOCK: usize = 8;

/// One `ICipher` object's whole state.
#[derive(Clone, Copy, Debug)]
pub struct Cipher {
    /// Which of the two key schedules this class runs: three for the `3DES_*` classes, one for `DES_*`.
    pub triple: bool,
    /// `AEECLSID_*_ENCRYPT` versus `_DECRYPT` — the direction the block primitive runs in.
    pub encrypt: bool,
    /// The running vector. `Init` overwrites it, `Cipher` advances it, and it is the only piece of
    /// state that survives a call, which is what makes two calls one stream.
    pub iv: [u8; BLOCK],
    /// The three keys, in `pbKey1`/`pbKey2`/`pbKey3` order. A single-DES object uses `keys[0]` and
    /// never looks at the other two — a handset's DES `Init` does not even read those fields.
    pub keys: [[u8; BLOCK]; 3],
    /// The flag the `Init` success path sets. `Flush` never clears it.
    pub ready: bool,
}

impl Cipher {
    /// A brand-new object: no vector, no key, and not yet initialized.
    pub fn born(triple: bool, encrypt: bool) -> Self {
        Self {
            triple,
            encrypt,
            iv: [0; BLOCK],
            keys: [[0; BLOCK]; 3],
            ready: false,
        }
    }

    /// The `Init` success path. The caller has already applied the handset's gates, so the parts are
    /// here in the order the body stores them: the vector copy first, the key schedules second.
    pub fn init(&mut self, iv: [u8; BLOCK], keys: [[u8; BLOCK]; 3]) {
        self.iv = iv;
        self.keys = keys;
        self.ready = true;
    }

    /// Only the flag is `Init`'s last act, so a run that reached the schedule step and then failed is
    /// still left holding the vector it copied. This is that write, on its own, for the refusal path.
    pub fn set_iv(&mut self, iv: [u8; BLOCK]) {
        self.iv = iv;
    }

    /// Run the handset's walk over `n` bytes — `pin` in, `pout` out — block by block through the guest's
    /// own memory and in the order the handset touches it. That order is the whole reason the walk is not
    /// "read the span, then write the span": a caller whose buffers overlap (`pbOut` a byte past `pbIn`,
    /// say) has its later input bytes already overwritten by the earlier output blocks, and a snapshot
    /// would hand it bytes nobody had written yet. It is also why a fault partway through leaves the
    /// blocks before it in the caller's buffer, as it does on a handset. Returns how many bytes it
    /// wrote — `n`, or a block more for an encrypting tail.
    pub fn chain(&mut self, mem: &mut Mem, pin: u32, pout: u32, n: usize) -> Result<usize, Fault> {
        let engine = Engine::build(self);
        let mut iv = self.iv;
        let mut written = 0usize;
        let mut at = 0usize;
        while at + BLOCK <= n {
            let mut block = take(mem, pin + at as u32, BLOCK)?;
            if self.encrypt {
                for i in 0..BLOCK {
                    block[i] ^= iv[i];
                }
                engine.step(&mut block, self.encrypt);
                iv = block;
            } else {
                let cipher = block;
                let mut plain = engine.stepped(&cipher, self.encrypt);
                for i in 0..BLOCK {
                    plain[i] ^= iv[i];
                }
                block = plain;
                iv = cipher;
            }
            mem.write_bytes(pout + at as u32, &block)?;
            written = at + BLOCK;
            at += BLOCK;
        }
        // The tail: one more block for a direction that pads or over-reads, and nothing at all when
        // `n` was a whole number of blocks — the test a handset's tail branch makes.
        let rest = n - at;
        if rest != 0 {
            if self.encrypt {
                // The remaining bytes, then zeros, and the *whole* eight XORed against the vector: the
                // empty positions come out as the vector's own bytes, not as nothing.
                let mut block = take(mem, pin + at as u32, rest)?;
                for i in 0..BLOCK {
                    block[i] ^= iv[i];
                }
                engine.step(&mut block, self.encrypt);
                mem.write_bytes(pout + at as u32, &block)?;
                iv = block;
                written = at + BLOCK;
            } else {
                // Eight bytes are taken from the caller's buffer no matter how few are left in it — the
                // over-read a handset's tail branch performs — and only the remainder goes back.
                let block = take(mem, pin + at as u32, BLOCK)?;
                let mut plain = engine.stepped(&block, self.encrypt);
                for i in 0..rest {
                    plain[i] ^= iv[i];
                }
                mem.write_bytes(pout + at as u32, &plain[..rest])?;
                iv = block;
                written = at + rest;
            }
        }
        self.iv = iv;
        Ok(written)
    }
}

/// The `len` bytes at `at`, in a block, with anything past them left zero — which is how a handset's
/// tail assembly reads, and why `len` is `BLOCK` for a whole block and the remainder for a short one.
fn take(mem: &Mem, at: u32, len: usize) -> Result<[u8; BLOCK], Fault> {
    let mut block = [0u8; BLOCK];
    block[..len].copy_from_slice(&mem.read_bytes(at, len)?);
    Ok(block)
}

/// The block primitive, in whichever of the two shapes this class was created to run.
///
/// `TdesEde3` is the composition a handset's 3DES path performs: encrypt `pbKey1`, decrypt `pbKey2`,
/// encrypt `pbKey3`, and the exact reverse on the way back. The 24-byte key this type takes is those
/// three halves in that order, which is also the order the handset keeps its three schedules in.
enum Engine {
    Des(Des),
    Des3(TdesEde3),
}

impl Engine {
    fn build(cipher: &Cipher) -> Self {
        if cipher.triple {
            let mut key = [0u8; 3 * BLOCK];
            for (i, half) in cipher.keys.iter().enumerate() {
                key[i * BLOCK..(i + 1) * BLOCK].copy_from_slice(half);
            }
            Engine::Des3(TdesEde3::new(&key.into()))
        } else {
            Engine::Des(Des::new(&cipher.keys[0].into()))
        }
    }

    /// Apply the primitive to eight bytes in place, in the direction this object was created for.
    fn step(&self, block: &mut [u8; BLOCK], encrypt: bool) {
        match self {
            Engine::Des(c) => {
                let mut b = Block::<Des>::from(*block);
                if encrypt {
                    c.encrypt_block(&mut b);
                } else {
                    c.decrypt_block(&mut b);
                }
                *block = b.into();
            }
            Engine::Des3(c) => {
                let mut b = Block::<TdesEde3>::from(*block);
                if encrypt {
                    c.encrypt_block(&mut b);
                } else {
                    c.decrypt_block(&mut b);
                }
                *block = b.into();
            }
        }
    }

    /// The same, over a copy, for a byte span that has to stay where it is.
    fn stepped(&self, block: &[u8; BLOCK], encrypt: bool) -> [u8; BLOCK] {
        let mut b = *block;
        self.step(&mut b, encrypt);
        b
    }
}
