//! `IRSA`, the ciphers, the hash faces and `IPeek`.
//!
//! Moved out of `src/runtime/mod.rs` without editing a body: this is where these
//! behaviours are defined, and `mod.rs` keeps the seam that reaches them.

use super::*;

impl Runtime {
    /// `ICipher` as the SEED member implements it: the same five slots and much of
    /// the same gate order, and one difference a caller cannot miss. DES pads or over-reads a length
    /// that is not a whole block; **SEED holds those bytes in the object** and emits nothing for them,
    /// and because this table has no `Finish` and its `Flush` is empty, a held tail is never produced.
    /// The other difference is the answer itself: the shipped `Cipher` returns `*pcbOut`, so a call that
    /// produced thirty-two bytes answers `32` where the DES family answered `0`.
    ///
    /// Kept out of the dispatch frame on purpose: `the_host_stack_one_downcall_level_costs_is_measured`
    /// bounds the cost of one nested down-call, and this handler's state is a 176-byte object copy, so
    /// inlining it would charge those bytes to *every* level of the chain rather than to the one call
    /// that asks for SEED.
    #[inline(never)]
    pub(crate) fn icipher_seed(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `int Init(ICipher* po, const byte* pbKey, int cbKey)`, with an `AEECipherInit` behind that
            // second word: `pbIV`/`cbIV` then `pbKey1`/`cbKey1`. The third
            // argument is not read by any body here.
            "Init" => {
                let [pinit, _cbkey] = self.args()?;
                let Some(mut record) = self.seeds.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                if pinit == 0 {
                    return Ok(cipher::AEE_CRYPT_INVALID_KEY);
                }
                let pbiv = self.cpu.mem.read_u32(pinit)?;
                let cbiv = self.cpu.mem.read_u32(pinit + 4)?;
                // Exactly one null-vector case is named, and it is the one whose caller claimed a
                // block's worth; any other null pointer reaches the copy below, which is where the
                // handset faults as well.
                if pbiv == 0 && cbiv == 16 {
                    return Ok(cipher::AEE_CRYPT_INVALID_IV);
                }
                // Sixteen bytes of vector, whatever `cbIV` said, and this copy runs *before* the key is
                // looked at — so a refusal on the key still leaves the object holding this vector.
                let mut chain = [0u8; seed::BLOCK];
                chain.copy_from_slice(&self.cpu.mem.read_bytes(pbiv, seed::BLOCK)?);
                let pbkey = self.cpu.mem.read_u32(pinit + 8)?;
                let cbkey = self.cpu.mem.read_u32(pinit + 12)?;
                if pbkey == 0 || cbkey != 16 {
                    // The key problem is reported with the key error even when it is the *length* that
                    // was wrong, because the body's third gate is a single `||` chain into `0x603`.
                    record.chain = chain;
                    self.seeds.insert(this, record);
                    return Ok(cipher::AEE_CRYPT_INVALID_KEY);
                }
                let mut key = [0u8; seed::BLOCK];
                key.copy_from_slice(&self.cpu.mem.read_bytes(pbkey, seed::BLOCK)?);
                record.init(chain, &key);
                self.seeds.insert(this, record);
                Ok(0)
            }
            // `int Cipher(ICipher* po, const byte* pbIn, int cbIn, byte* pbOut, int* pcbOut)`. No
            // capacity clamp: the fifth word is a *pointer* to the count, and the walk is what decides
            // how much comes out.
            "Cipher" => {
                let [pin, cbin, pout, pcbout] = self.args()?;
                let Some(mut record) = self.seeds.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                if !record.ready {
                    // `if (!bReady) return ~bReady;` — a clear flag answers `-1` and touches nothing,
                    // the count word included.
                    return Ok(u32::MAX);
                }
                let produced =
                    record.chain_walk(&mut self.cpu.mem, pin, pout, cbin as usize, pcbout)?;
                self.seeds.insert(this, record);
                // `return *pcbOut`: the byte count, not a code.
                Ok(produced as u32)
            }
            // `void Flush(ICipher* po)` again — `BX LR`, and on this member it is *all* it is: the
            // buffered tail it would take to finish a stream is in the delivered body's `*_Final`, which
            // the delivered body does not import.
            "Flush" => Ok(this),
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// `IHash`'s four slots over `IBase`: `Update`, `GetResult`, `Restart`, `SetKey` — the whole
    /// table is six words. The algorithm lives in [`crate::hash`]; what is
    /// decided here is only what the guest's memory sees.
    ///
    /// The fault rules are the caller's, not this host's: a null `pbData` beside a non-zero count
    /// faults at address 0 through `read_bytes`, exactly where the shipped body's copy would; a zero
    /// count beside a null pointer is the no-op that body performs without touching it.
    pub(crate) fn ihash(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        match name {
            // `void Update(IHash* po, byte* pbData, int cbData)`. `cbData` is signed in the
            // prototype and the shipped body walks it as a count, so a negative figure feeds
            // nothing rather than being widened into four gigabytes of reads.
            "Update" => {
                let [pdata, cbdata] = self.args()?;
                let Some(mut record) = self.hashes.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                let len = (cbdata as i32).max(0) as usize;
                if len > 0 {
                    let data = self.cpu.mem.read_bytes(pdata, len)?;
                    record.update(&data);
                    self.hashes.insert(this, record);
                }
                Ok(0)
            }
            // `int GetResult(IHash* po, byte* pbData, int* pcbData)`. `pcbData` is the size of the
            // caller's buffer on the way in and the size of the result on the way out, so a null
            // there is read (and faults) before anything is written anywhere.
            "GetResult" => {
                let [pbdata, pcbdata] = self.args()?;
                let Some(record) = self.hashes.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // The header declares this word `int*`, so it is read as a *signed* size: a negative
                // capacity is a caller error, and reading it as `u32` would turn it into 4 GiB and send
                // the write down the "it fits" branch with the smallest buffer of all.
                let cap = self.cpu.mem.read_u32(pcbdata)? as i32;
                let digest = record.result();
                // `AEE_HASH_MORE_DATA`: the buffer is left exactly as it was, and the figure the
                // header says this word holds is written anyway — "on return, this contains the size
                // of the hashed result" is stated without limiting it to the success path, and it is
                // the only way a caller can learn what to allocate before asking again. What the
                // header does **not** say is whether the buffer is left alone on that path; leaving
                // it alone is this host's reading, and the safe one: a caller that ignored the code
                // would otherwise read a truncated digest as a whole one.
                if cap < 0 || (cap as usize) < digest.len() {
                    self.cpu.mem.write_u32(pcbdata, digest.len() as u32)?;
                    return Ok(hash::AEE_HASH_MORE_DATA);
                }
                self.cpu.mem.write_bytes(pbdata, &digest)?;
                self.cpu.mem.write_u32(pcbdata, digest.len() as u32)?;
                Ok(g::AEE_SUCCESS)
            }
            // `void Restart(IHash* po)`: back to an object that has hashed nothing.
            "Restart" => {
                let Some(mut record) = self.hashes.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                record.restart();
                self.hashes.insert(this, record);
                Ok(0)
            }
            // `int SetKey(IHash* po, const byte* pbKey, int cbKey)`. The header is explicit that it
            // "is only used with HMAC hashes" and "has no affect on non-HMAC hashes", and the class
            // id decides which kind an object is — every id in this host's table is the plain kind,
            // so the key is neither read nor kept. The object is still looked up, because a slot
            // called through an address this host never made (or made and freed) is the same
            // `Unimplemented` the other three answer with; answering success there would be the one
            // place a stale call went unnoticed.
            "SetKey" => {
                let [_pkey, _cbkey] = self.args()?;
                if !self.hashes.contains_key(&this) {
                    return Err(Error::Hle(unimplemented()));
                }
                Ok(g::AEE_SUCCESS)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// Four shapes a caller can see and a host could easily smooth over: the vector is copied **eight
    /// bytes deep** whatever `cbIV` claimed, so the only null vector refused is the one whose length
    /// says eight; that copy happens before any key test, so a refused `Init` still leaves the object
    /// holding the vector it was given; the length word is clamped to `min(cbIn, *pcbOut)` *before* the
    /// initialized flag is looked at, and is echoed back after a walk that wrote a whole block past it;
    /// and `Flush` is two bytes of `BX LR` that clear nothing at all.
    pub(crate) fn icipher(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        // Two shipped members publish this same five-slot table and disagree about what a short length
        // means, so the record map decides which body answers — the class decided at creation.
        if self.seeds.contains_key(&this) {
            return self.icipher_seed(id, this, label, name);
        }
        match name {
            // `int Init(ICipher* po, const byte* pbKey, int cbKey)`, and the published second argument
            // is where an `AEECipherInit` goes — the header's own macro puts it there:
            // `ICIPHER_InitCipher(p, pi)` is `Init(p, (const byte*)pi, 0)`.
            // No body in this member reads the third word, so this host does not read it either.
            "Init" => {
                let [pinit, _cbkey] = self.args()?;
                let Some(mut record) = self.ciphers.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // The first gate is the struct pointer alone, and it answers with the key error: no
                // struct is no key. Nothing else is read, so a caller's vector stays where it was.
                if pinit == 0 {
                    return Ok(cipher::AEE_CRYPT_INVALID_KEY);
                }
                // The struct's eight words, in the order the body reaches them: `pbIV@0` `cbIV@4`,
                // `pbKey1@8` `cbKey1@12`, `pbKey2@16` `cbKey2@20`, `pbKey3@24` `cbKey3@28`
                // (`AEECipherInit`, the header, 32 bytes in the ABI table too). Read
                // lazily rather than as one 32-byte block, because the body is lazy: a DES body never
                // gets past word 3, and a 3DES gate stops reading *lengths* the moment one of its three
                // pairs has passed. A caller whose struct is shorter than published therefore faults in
                // the same place on both machines, and not one word earlier here.
                let pbiv = self.cpu.mem.read_u32(pinit)?;
                let cbiv = self.cpu.mem.read_u32(pinit + 4)?;
                // The vector gate names exactly one case: a null pointer whose caller claimed eight
                // bytes. Any other length beside a null pointer gets past it and into the copy.
                if pbiv == 0 && cbiv == 8 {
                    return Ok(cipher::AEE_CRYPT_INVALID_IV);
                }
                // `aee_memmove(obj + 4, params[0], 8)` — eight bytes out of wherever the caller
                // pointed, with `cbIV` never consulted again. Taking them through the guest's own
                // memory is what makes a null vector with a non-eight length fault here exactly where
                // it faults on a handset. The store comes first because the copy did: a run that
                // refuses, or that faults on the way to a key, has already moved the vector.
                let mut iv = [0u8; cipher::BLOCK];
                iv.copy_from_slice(&self.cpu.mem.read_bytes(pbiv, cipher::BLOCK)?);
                record.set_iv(iv);
                self.ciphers.insert(this, record);
                // DES wants its one key and says so; 3DES wants *at least one* of its three and then
                // schedules all three — which is why its gate and its use disagree. An object handed
                // only `pbKey2` passes the test and then reads `pbKey1`, and `pbKey1` is zero.
                let mut keys = [[0u8; cipher::BLOCK]; 3];
                if record.triple {
                    let mut offered = false;
                    let mut key_at = [0u32; 3];
                    for slot in 0..3 {
                        key_at[slot] = self.cpu.mem.read_u32(pinit + 8 + 8 * slot as u32)?;
                        if offered {
                            continue;
                        }
                        let cb = self.cpu.mem.read_u32(pinit + 12 + 8 * slot as u32)?;
                        offered = key_at[slot] != 0 && cb == 8;
                    }
                    if !offered {
                        return Ok(cipher::AEE_CRYPT_INVALID_KEY);
                    }
                    for slot in 0..3 {
                        keys[slot].copy_from_slice(
                            &self.cpu.mem.read_bytes(key_at[slot], cipher::BLOCK)?,
                        );
                    }
                } else {
                    let pbkey1 = self.cpu.mem.read_u32(pinit + 8)?;
                    if pbkey1 == 0 || self.cpu.mem.read_u32(pinit + 12)? != 8 {
                        return Ok(cipher::AEE_CRYPT_INVALID_KEY);
                    }
                    keys[0].copy_from_slice(&self.cpu.mem.read_bytes(pbkey1, cipher::BLOCK)?);
                }
                // What a schedule is, on a handset, is `des_key_sched` — and that consults
                // `des_check_key`, a word this member defines in its `.bss` and no other member names,
                // so it is zero and the parity and weak-key tests written into this member never run.
                // The value `des_key_sched` returns is thrown away anyway. The key bytes are therefore
                // taken exactly as handed over, which is what [`Cipher::init`] does with them.
                record.init(iv, keys);
                self.ciphers.insert(this, record);
                Ok(g::AEE_SUCCESS)
            }
            "Cipher" => {
                let [pin, cbin, pout, ptotal] = self.args()?;
                let Some(mut record) = self.ciphers.get(&this).cloned() else {
                    return Err(Error::Hle(unimplemented()));
                };
                // `if (*pcbOut <= cbIn) cbIn = *pcbOut;` — signed, as the `cmp` is, and before anything
                // else, so a caller's own size word is what decides how much of its buffer gets
                // touched even on a call that then refuses. The length word is the caller's: a null one
                // is dereferenced by the body, and by this host.
                let capacity = self.cpu.mem.read_u32(ptotal)?;
                let n = if (capacity as i32) <= (cbin as i32) {
                    capacity
                } else {
                    cbin
                };
                if !record.ready {
                    // `return ~this[+208]` — the flag byte complemented. This host starts every object
                    // with the flag clear, so a `Cipher` before any `Init` answers `-1`, writes nothing
                    // and leaves the caller's length word holding what the caller put in it.
                    return Ok(u32::MAX);
                }
                if (n as i32) <= 0 {
                    // A length of nothing is the walk's own no-op — the member's `v6 != -8` test — and
                    // is answered as one, count included. A *negative* count is not: the tail branch
                    // would encrypt a block of zeros into the caller's buffer and, decrypting, write
                    // below the pointer it was handed. This host keeps the number the handset reports
                    // and drops the scribble.
                    self.cpu.mem.write_u32(ptotal, n)?;
                    return Ok(n);
                }
                let n = n as usize;
                // The walk itself touches the caller's buffers block by block, in the order the member
                // does, and updates the object's vector from the last block it went through.
                record.chain(&mut self.cpu.mem, pin, pout, n)?;
                // The echo is `n`, not the number of bytes written: an encrypting tail wrote a full
                // eight and this word still says the shorter number it was asked for.
                self.cpu.mem.write_u32(ptotal, n as u32)?;
                self.ciphers.insert(this, record);
                Ok(n as u32)
            }
            // `void Flush(ICipher* po)`, in all four classes: `BX LR` and nothing else. It clears no
            // key, no vector and no flag, so the object goes on encrypting after it; the only thing a
            // caller could read out of the call is `r0`, which the body leaves holding `po`.
            "Flush" => Ok(this),
            // The shipped table is five words with **no `QueryInterface`** among them, so there is no
            // such slot to answer — a caller that reached for one was calling `Init`.
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The six own slots of `IRSA` (whose eight-word table is the published one:
    /// `AddRef Release Init RSA ModExp Encrypt Decrypt VerifySig`, base `IBase` so no
    /// `QueryInterface`). Five of them are one machine: store what the caller named in a byte and four
    /// words, run one modular exponentiation, and let `AEERSA_Done` decide what the answer *is* — an
    /// `Encrypt` hands the ciphertext to the caller's buffer, a `Decrypt` strips padding off the block,
    /// a `VerifySig` strips it in place and then walks a DER `DigestInfo`.
    ///
    /// Nothing here defers. The shipped deferral is a chunked loop inside `BIGINT_ModExpEx` that
    /// reaches eighteen bits before it yields (`if (a6 == 0 || v53 < 18)`), and this host has no
    /// chunking to yield from, so it publishes the result and resumes the callback — the state a
    /// handset ends in, one dispatch step early, and without the `pfnCancel` hookup the shipped body
    /// writes into the caller's callback (`AEERSA_Cancel` is a function address, and the host's stub
    /// space encodes "interface and slot", not a free closure). The first operation that asks for a
    /// callback says so once.
    pub(crate) fn irsa(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        if !self.rsa_objects.contains_key(&this) {
            return Err(Error::Hle(unimplemented()));
        }
        match name {
            // `int Init(IRSA* po, byte* pbMod, int cbMod, byte* pbExp, int cbExp)`.
            "Init" => {
                let [pbmod, cbmod, pbexp, cbexp] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                // The gate is one `if` with five tests, and the fifth reads a byte of the caller's
                // modulus: `(pbMod[cbMod - 1] & 1) == 0` refuses an **even modulus**, because the
                // number is big-endian and that byte is its least significant one. Every test is
                // signed, so a negative length refuses rather than reaching for `2^32` bytes.
                if pbmod == 0 || pbexp == 0 || (cbexp as i32) <= 0 || (cbmod as i32) <= 0 {
                    return Ok(rsa::AEE_CRYPT_INVALID_KEY);
                }
                if self.cpu.mem.read_bytes(pbmod + cbmod - 1, 1)?[0] & 1 == 0 {
                    return Ok(rsa::AEE_CRYPT_INVALID_KEY);
                }
                // The old key pair is given back before the new one is taken (`aee_releaseobj` on both
                // `[+12]` and `[+16]` at the top of every turn of its loop), and the block is freed on
                // the same turn it stops being big enough — but a *smaller* `cbMod` leaves the old
                // block and the old capacity standing, because the body only ever allocates when
                // `[+92]` is zero. Capacity therefore ratchets up and never down, and the block a
                // caller is handed after a short key is still the long one.
                let modulus = rsa::Big::from_be(&self.cpu.mem.read_bytes(pbmod, cbmod as usize)?);
                let exponent = rsa::Big::from_be(&self.cpu.mem.read_bytes(pbexp, cbexp as usize)?);
                if (record.capacity as u32) < cbmod {
                    if record.block != 0 {
                        // `aee_free(this[+92])`: the host allocated these bytes itself, so it gives
                        // them back itself rather than through the table of objects it owns.
                        self.cpu.mem.free(record.block);
                    }
                    record.block = self.cpu.mem.alloc(cbmod, 4)?;
                    record.capacity = cbmod as usize;
                }
                record.modulus = Some(modulus);
                record.exponent = Some(exponent);
                self.rsa_objects.insert(this, record);
                // `AEE_ENOMEMORY` is this body's only other answer — the second pass of its loop,
                // reached when the block cannot be had — and allocation here does not fail.
                Ok(0)
            }
            // `byte** IRSA_RSA(po, pbIn, cbIn, ppbOut, pcbOut, reserved)` — declared `void` in the
            // header, and the body returns the code it wrote into a local of its own. The sixth word is
            // never read. The answer goes **into the object's block**: `[+92]`, with `[+96]` as the
            // count, so what the caller is handed is a pointer into the object and a capacity rather
            // than a length (the real length lands in a local `AEERSA_Done` throws away).
            "RSA" => {
                let [pbin, cbin, ppbout, pcbout, _reserved] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                let mut code = 0u32;
                // Both words are written before anything is computed, and with the value the object
                // holds — so an operation that was never given a key hands back `0` and a capacity of
                // `0`, which is what the shipped body hands back out of its uninitialized record.
                self.cpu.mem.write_u32(ppbout, record.block)?;
                self.cpu.mem.write_u32(pcbout, record.capacity as u32)?;
                if record.modulus.is_none() || record.exponent.is_none() {
                    code = rsa::AEE_CRYPT_INVALID_KEY;
                } else if let Some(out) = self.irsa_transform(&mut record, pbin, cbin)? {
                    self.cpu.mem.write_bytes(record.block, &out)?;
                } else {
                    code = g::AEE_EBADPARM;
                }
                self.rsa_objects.insert(this, record);
                Ok(code)
            }
            // `void ModExp(po, pbIn, cbIn, ppbOut, pcbOut, pdwResult, pCB)` — the same raw operation
            // with the two things `RSA` was obsoleted for: a result word the caller owns, and a
            // callback the caller can cancel.
            "ModExp" => {
                let [pbin, cbin, ppbout, pcbout, pres, pcb] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                let mut code = 0u32;
                // Both words are written before anything is computed, and with the value the object
                // holds — so an operation that was never given a key hands back `0` and a capacity of
                // `0`, which is what the shipped body hands back out of its uninitialized record.
                self.cpu.mem.write_u32(ppbout, record.block)?;
                self.cpu.mem.write_u32(pcbout, record.capacity as u32)?;
                if record.modulus.is_none() || record.exponent.is_none() {
                    code = rsa::AEE_CRYPT_INVALID_KEY;
                } else if let Some(out) = self.irsa_transform(&mut record, pbin, cbin)? {
                    self.cpu.mem.write_bytes(record.block, &out)?;
                } else {
                    code = g::AEE_EBADPARM;
                }
                self.rsa_publish(this, &mut record, pres, code, pcb)?;
                Ok(0)
            }
            // `void Encrypt(po, pbIn, cbIn, pbOut, pcbOut, ePadType, pdwResult, pCB)`. Its own gates
            // run before any arithmetic, in this order: the caller's offered capacity against the
            // block size, then the pad type.
            "Encrypt" => {
                let [pbin, cbin, pbout, pcbout, pad, pres, pcb] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                let offered = self.cpu.mem.read_u32(pcbout)?;
                if (offered as usize) < record.capacity {
                    // `AEE_CRYPT_BUFFER_TOO_SMALL`, and nothing else is read or written.
                    self.rsa_publish(
                        this,
                        &mut record,
                        pres,
                        rsa::AEE_CRYPT_BUFFER_TOO_SMALL,
                        pcb,
                    )?;
                    return Ok(0);
                }
                // The switch is what reads the caller's message, and only the four known types have a
                // branch that reaches it: an unknown `ePadType`, and a type 4 whose length is not a whole
                // block, are both refused before a byte of `pbIn` is looked at.
                if pad == 0 || pad > 4 {
                    self.rsa_publish(this, &mut record, pres, rsa::AEE_CRYPT_INVALID_PADTYPE, pcb)?;
                    return Ok(0);
                }
                // Type 4 has no `AEE_RSA_padding_add_none` call in this body at all: it tests the length
                // and moves the bytes itself. A message that is not a whole block is
                // `AEE_CRYPT_INVALID_SIZE`; the offered capacity having *more* room than a block is not
                // tested again here, because the body's only capacity test ran before the switch.
                if pad == 4 && cbin as usize != record.capacity {
                    self.rsa_publish(this, &mut record, pres, rsa::AEE_CRYPT_INVALID_SIZE, pcb)?;
                    return Ok(0);
                }
                let data = self.cpu.mem.read_bytes(pbin, cbin as usize)?;
                let mut block = vec![0u8; record.capacity];
                let status = match pad {
                    1 => rsa::add_pkcs1_type_1(&mut block, &data).1,
                    2 => {
                        let mut rng = rsa::rand_bytes;
                        rsa::add_pkcs1_type_2(&mut block, &data, &mut rng).1
                    }
                    3 => {
                        let mut rng = rsa::rand_bytes;
                        rsa::add_sslv23(&mut block, &data, &mut rng).1
                    }
                    _ => {
                        block.copy_from_slice(&data);
                        0
                    }
                };
                if status != 0 {
                    self.rsa_publish(this, &mut record, pres, rsa::AEE_CRYPT_PAD_ERROR, pcb)?;
                    return Ok(0);
                }
                // The padded block is what the modular exponentiation reads, and it stays in the
                // object's block afterwards: `AEERSA_Done` sends the ciphertext to the caller's buffer
                // and never rewrites the block, so a pack that kept the pointer from an earlier
                // `IRSA_RSA` can read this plaintext back out of it.
                if record.capacity > 0 {
                    self.cpu.mem.write_bytes(record.block, &block)?;
                }
                let code = if record.modulus.is_none() || record.exponent.is_none() {
                    rsa::AEE_CRYPT_INVALID_KEY
                } else {
                    let (block_at, block_len) = (record.block, record.capacity as u32);
                    match self.irsa_transform(&mut record, block_at, block_len)? {
                        Some(out) if out.len() <= offered as usize => {
                            self.cpu.mem.write_bytes(pbout, &out)?;
                            self.cpu.mem.write_u32(pcbout, out.len() as u32)?;
                            0
                        }
                        // `BIGINT_OutToBuf` answers "too small" with a zero written into the count.
                        Some(_) => {
                            self.cpu.mem.write_u32(pcbout, 0)?;
                            1
                        }
                        None => g::AEE_EBADPARM,
                    }
                };
                self.rsa_publish(this, &mut record, pres, code, pcb)?;
                Ok(0)
            }
            // `void Decrypt(po, pbIn, cbIn, pbOut, pcbOut, ePadType, pdwResult, pCB)` — the ciphertext
            // in, the padded block out of the modular exponentiation, and the padding taken off with
            // the same type number that went in. Its capacity gate is on the *offered* buffer, and it
            // wants a full block: `*pcbOut >= [+96]`.
            "Decrypt" => {
                let [pbin, cbin, pbout, pcbout, pad, pres, pcb] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                let offered = self.cpu.mem.read_u32(pcbout)?;
                if (offered as usize) < record.capacity {
                    self.rsa_publish(
                        this,
                        &mut record,
                        pres,
                        rsa::AEE_CRYPT_BUFFER_TOO_SMALL,
                        pcb,
                    )?;
                    return Ok(0);
                }
                let code = if record.modulus.is_none() || record.exponent.is_none() {
                    rsa::AEE_CRYPT_INVALID_KEY
                } else {
                    match self.irsa_finished(&mut record, pbin, cbin)? {
                        Some(block) => {
                            let capacity = record.capacity;
                            let mut out = vec![0u8; offered as usize];
                            let (code, len) = rsa::remove_padding(pad, &block, capacity, &mut out);
                            if code == 0 {
                                let len = len.unwrap_or(0);
                                self.cpu.mem.write_bytes(pbout, &out[..len])?;
                                self.cpu.mem.write_u32(pcbout, len as u32)?;
                            }
                            code
                        }
                        // `AEERSA_ModExp2`'s own refuse: the ciphertext came back bigger than the
                        // modulus, so nothing reaches `RemovePadding` and no length word is touched.
                        None => g::AEE_EBADPARM,
                    }
                };
                self.rsa_publish(this, &mut record, pres, code, pcb)?;
                Ok(0)
            }
            // `void VerifySig(po, pbIn, cbIn, pbDig, cbDig, eSigType, pdwResult, pCB)` — the signature
            // in, and the answer is whether it carries the digest the caller named. There is no
            // capacity gate: this slot's fourth and fifth words are the digest, not an output buffer.
            "VerifySig" => {
                let [pbin, cbin, pbdig, cbdig, sig, pres, pcb] = self.args()?;
                let mut record = self.rsa_objects[&this].clone();
                let code = if record.modulus.is_none() || record.exponent.is_none() {
                    rsa::AEE_CRYPT_INVALID_KEY
                } else {
                    match self.irsa_finished(&mut record, pbin, cbin)? {
                        Some(block) => {
                            let capacity = record.capacity;
                            // `AEERSA_Done`'s verify branch strips **in place**: `RemovePadding(1,
                            // from=[+92], cbFrom, cbTo=[+96], to=[+92], &cb)` names the block on both
                            // sides, so what the walk then reads is the block's own front and the only
                            // bytes the copy touches are the message it moved there.
                            let mut stripped = vec![0u8; capacity];
                            let held = block.len().min(capacity);
                            stripped[..held].copy_from_slice(&block[..held]);
                            let (code, len) =
                                rsa::remove_padding(1, &block, capacity, &mut stripped);
                            if code != 0 {
                                code
                            } else {
                                let len = len.unwrap_or(0);
                                if record.block != 0 && len > 0 {
                                    self.cpu.mem.write_bytes(record.block, &stripped[..len])?;
                                }
                                // `1025` is this branch's own default, set before the tests: an
                                // oversized type, a stripped block outside ten to one hundred and
                                // twenty-seven bytes, an algorithm that does not match, or a digest
                                // that is not the one named all answer that number, and only a match
                                // answers 0.
                                if sig > 3 || len > 127 || len < 10 {
                                    rsa::AEE_RSA_NOT_VERIFIED
                                } else {
                                    let id = rsa::map_oid(&stripped[4..len], len - 4);
                                    let digest = self.cpu.mem.read_bytes(pbdig, cbdig as usize)?;
                                    if rsa::sig_type_accepts(sig, id)
                                        && rsa::digest_matches(&stripped[..len], &digest)
                                    {
                                        0
                                    } else {
                                        rsa::AEE_RSA_NOT_VERIFIED
                                    }
                                }
                            }
                        }
                        None => g::AEE_EBADPARM,
                    }
                };
                self.rsa_publish(this, &mut record, pres, code, pcb)?;
                Ok(0)
            }
            // Eight words, `IBase` under them: `AddRef` and `Release` are the only ones before `Init`,
            // and there is no `QueryInterface` in the table to answer.
            _ => Err(Error::Hle(unimplemented())),
        }
    }

    /// The arithmetic half of `AEERSA_ModExp2`: read the caller's `cbIn` bytes as a number — which is
    /// `BIGINT_SetBuf`, and it clears the value before it starts, so a zero length is a zero and not
    /// "leave it alone" — compare it against the modulus the way the body's `Cmp` call does, and
    /// exponentiate. `None` is that comparison refusing: the body tests `> 0`, so a value **equal** to
    /// the modulus gets through and comes back `0`, and only a bigger one answers `AEE_EBADPARM`. That
    /// off-by-one is shipped, and the header even describes the gate with the same inequality.
    pub(crate) fn irsa_transform(
        &mut self,
        record: &mut rsa::Rsa,
        pb_in: u32,
        cb_in: u32,
    ) -> HResult<Option<Vec<u8>>> {
        let bytes = self.cpu.mem.read_bytes(pb_in, cb_in as usize)?;
        let (Some(modulus), Some(exponent)) = (record.modulus.as_ref(), record.exponent.as_ref())
        else {
            return Ok(None);
        };
        let base = rsa::Big::from_be(&bytes);
        if base.cmp(modulus) > 0 {
            return Ok(None);
        }
        Ok(Some(base.modpow(exponent, modulus).to_be()))
    }

    /// The same, plus the first thing `AEERSA_Done` does with it for the three operations that are not
    /// `Encrypt`: `Get` the answer **into the object's block**, which is where `Decrypt` and
    /// `VerifySig` then read their padded block from and where a caller of `IRSA_RSA` or
    /// `IRSA_ModExp` was pointed. The length `Get` reports goes into a local the body drops — the
    /// caller's `pcbOut` keeps the capacity the slot wrote into it, not the number of bytes that came
    /// back — and the bytes past the answer are the previous operation's, exactly as the shipped
    /// hundred-byte record leaves them.
    pub(crate) fn irsa_finished(
        &mut self,
        record: &mut rsa::Rsa,
        pb_in: u32,
        cb_in: u32,
    ) -> HResult<Option<Vec<u8>>> {
        let Some(out) = self.irsa_transform(record, pb_in, cb_in)? else {
            return Ok(None);
        };
        if record.block != 0 {
            self.cpu.mem.write_bytes(record.block, &out)?;
        }
        Ok(Some(out))
    }

    /// The end of `AEERSA_Done`, in order: write the answer through the caller's result word — with no
    /// test on it, so a null `pdwResult` faults here the way the shipped body faults writing through
    /// zero — and then, if a callback was named, resume it the way the body calls `IShell::Resume`
    /// (slot 36, byte 144) rather than calling the pack's function itself.
    pub(crate) fn rsa_publish(
        &mut self,
        this: u32,
        record: &mut rsa::Rsa,
        p_result: u32,
        code: u32,
        pcb: u32,
    ) -> Result<(), Error> {
        self.cpu.mem.write_u32(p_result, code)?;
        if pcb != 0 {
            if !record.announced {
                record.announced = true;
                self.display.note(
                    "IRSA: this host has no chunked modular exponentiation to yield from, so an \
                     operation that was handed a callback answered it in the same call and resumed the \
                     callback on the next dispatch."
                        .to_string(),
                );
            }
            if self.dispatcher_prepare(pcb)? {
                let owner = self.applet;
                self.dispatcher.resume_from(pcb, owner);
            }
        }
        self.rsa_objects.insert(this, record.clone());
        Ok(())
    }

    /// The memory-backed source `ISourceUtil` hands out. `gvtMemPeek_ISource` is the eight-slot
    /// `IPeek` table — the base three, then `Read`/`Readable`, then `Peek`/`Peekable`/`Advance` — over
    /// bytes the caller owns, with the shell that made the object copied into `+4`.
    pub(crate) fn ipeek(&mut self, id: u32, this: u32, label: &str, name: &str) -> Result<u32, Error> {
        let unimplemented = || HleError::Unimplemented {
            id,
            label: label.to_string(),
            this,
        };
        let Some(record) = self.mem_sources.get(&this).cloned() else {
            return Err(Error::Hle(unimplemented()));
        };
        match name {
            // Three ids: the base's, `AEECLSID_SOURCE` and `AEECLSID_PEEK`. No `AEE_CheckPtr`.
            "QueryInterface" => {
                let [iid, ppv] = self.args()?;
                self.answer_qi(this, iid, ppv, &[g::AEECLSID_PEEK, g::AEECLSID_SOURCE])
                    .map_err(Error::Hle)
            }
            // `Read` answers the **number of bytes copied**, clamped to what is left, and 0 at the
            // end — a short read is an answer, not an error.
            "Read" => {
                let [buf, cb] = self.args()?;
                let left = record.len.saturating_sub(record.pos);
                let n = (cb as u32).min(left);
                if n > 0 {
                    let bytes = self
                        .cpu
                        .mem
                        .read_bytes(record.buf + record.pos, n as usize)?;
                    self.cpu.mem.write_bytes(buf, &bytes)?;
                    if let Some(record) = self.mem_sources.get_mut(&this) {
                        record.pos += n;
                    }
                }
                Ok(n)
            }
            // Both slots are the same one line in the shipped object: `IShell::Resume` on the shell the
            // factory copied into `+4` (vtable byte 144 is slot 36, which is `Resume` in the vendored
            // table), i.e. "tell me through this callback when the applet is running again". Bytes
            // that are already in memory have nothing to wait for, so the callback is queued for the
            // next tick — the way this host delivers every other `AEECallback` — instead of being run
            // inside the call the pack is still in the middle of.
            "Readable" | "Peekable" => {
                let [pcb] = self.args()?;
                if pcb != 0 {
                    self.timers.push(Timer {
                        due: self.clock_ms,
                        pfn: pcb,
                        user: pcb,
                        owner: self.applet,
                    });
                }
                Ok(g::SUCCESS)
            }
            // `Peek` answers a pointer into the caller's own bytes. The special length is the one
            // question that does not ask for data: the shipped body writes `0x7fffffff` as the room
            // and `IPEEK_UNK` as the code, which is this object's way of saying "never full".
            "Peek" => {
                let [wanted, avail_at, code_at] = self.args()?;
                if wanted == IPEEK_GETBUFSIZE {
                    self.cpu.mem.write_u32(avail_at, 0x7fff_ffff)?;
                    self.cpu.mem.write_u32(code_at, g::IPEEK_UNK)?;
                    return Ok(0);
                }
                let left = record.len - record.pos;
                self.cpu.mem.write_u32(avail_at, left)?;
                self.cpu.mem.write_u32(code_at, g::ISOURCE_END)?;
                Ok(record.buf + record.pos)
            }
            "Advance" => {
                let [cb] = self.args()?;
                let moved = (record.pos as i32 + cb as i32).clamp(0, record.len as i32) as u32;
                if let Some(record) = self.mem_sources.get_mut(&this) {
                    record.pos = moved;
                }
                Ok(0)
            }
            _ => Err(Error::Hle(unimplemented())),
        }
    }

}
