//! `IHash`: the MD5 class (`AEECLSID_MD5`, the delivered body).
//!
//! The header (`AddRef`, `Release`) and then `Update`/`GetResult`/`Restart`/`SetKey` — and says the class id
//! handed to `ISHELL_CreateInstance` is what picks the algorithm. This host serves exactly one of
//! those ids, MD5, because it is the one the sample set asks for: the S7 platform layer behind Sprint
//! Mail and Mobile Email creates an `AEECLSID_MD5` while it is building its context and treats the
//! refusal as fatal, so the pack stops on its own "Not enough memory" page with no memory missing.
//!
//! The other algorithms are **not** here on purpose. `AEECLSID_SHA1`/`SHA256`/`SHA384`/`SHA512` and
//! the twelve `HMAC_*` ids share this table, and building an MD5 object under any of them would be a
//! wrong answer where a missing one is honest. A pack that asks for one of them gets the shell's own
//! `ECLASSNOTSUPPORT`, which is what a handset without that class would give it too.
//!
//! What this module keeps is the algorithm's running state and nothing else; the guest's buffers are
//! read and written by [`crate::runtime`], one level up, where the memory and the fault rules live.

use crate::abi::generated as g;
use md5::Context;

/// Bytes in an MD5 digest, and the figure `GetResult` publishes back through `pcbData`.
pub const DIGEST: usize = 16;

/// `AEE_HASH_MORE_DATA`: `SECURITY_ERROR_BASE + 2`, the answer `GetResult` gives when the caller's
/// buffer cannot hold the result (with naming the `0x600` base).
///
/// Written here rather than taken from [`crate::abi::generated::consts`] because that table carries no
/// `SECURITY_ERROR_BASE` family at all: `AEE_HASH_MORE_DATA`, `AEE_HASH_INVALID_CTX` and the
/// `AEE_CRYPT_*` codes all have zero hits there, while `AEE_EBADCLASS` and `AEE_ENOMEMORY` do. The spec the table is built from simply does not reach the security layer.
pub const AEE_HASH_MORE_DATA: u32 = 0x602;

/// One hash object's state: everything fed since the last `Restart`.
#[derive(Clone)]
pub struct Hash {
    ctx: Context,
}

impl Hash {
    /// The object a class id builds, or `None` for a hash this host does not implement. The caller
    /// ([`crate::runtime`]) only reaches this for ids it found in its own service table, so the
    /// `None` here is a second gate rather than the decision.
    pub fn born(cls: u32) -> Option<Hash> {
        if cls == g::AEECLSID_MD5 {
            Some(Hash {
                ctx: Context::new(),
            })
        } else {
            None
        }
    }

    /// `void Update(IHash* po, byte* pbData, int cbData)` — append. The header gives it no return
    /// value and no length limit, so a run of empty updates is a no-op rather than an error.
    pub fn update(&mut self, data: &[u8]) {
        self.ctx.consume(data);
    }

    /// The digest of everything fed since the last `Restart` — **without** ending the hash: the
    /// state is cloned and finalised, so a second `GetResult` returns the same sixteen bytes and a
    /// further `Update` after it still counts. `Restart` is the header's way to begin a new hash
    /// ("call this to reuse an instance of IHash for a hash over some new data"), which is only a
    /// distinct operation if `GetResult` leaves the state alone.
    pub fn result(&self) -> [u8; DIGEST] {
        self.ctx.clone().compute().0
    }

    /// `void Restart(IHash* po)`: back to the empty state, exactly as a freshly created object.
    pub fn restart(&mut self) {
        self.ctx = Context::new();
    }
}

#[cfg(test)]
#[path = "test/hash.rs"]
mod tests;
