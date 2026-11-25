//! `IRamCache`: the shell's in-memory record store, keyed by an opaque byte string.
//!
//! The delivered object is a small cache with three limits a caller may move — a time-to-live in
//! seconds, a ceiling on stored bytes and a ceiling on entries — and the `IRecordStore` operations
//! `Add`, `Remove` and `Find` over byte-string keys. Nothing here is a device: the records live in
//! this process, which is what makes the class answerable without a handset.
//!
//! The virtual clock is the only clock, so a TTL is deterministic across runs: an entry stored at
//! `clock_ms` is gone on the first access at or after `stored + ttl * 1000`. A TTL of zero means
//! "no expiry", which is the state a fresh cache starts in.

use std::collections::HashMap;

use crate::abi::generated as g;

/// One stored record.
struct Entry {
    value: Vec<u8>,
    stored_ms: u32,
}

/// What one `IRamCache` object holds.
#[derive(Default)]
pub struct RamCache {
    entries: HashMap<Vec<u8>, Entry>,
    /// The payload bytes currently stored, so a replacement is one subtraction.
    bytes: u32,
    ttl_secs: u32,
    /// Zero means no ceiling, which is how a fresh cache starts.
    max_size: u32,
    max_entries: u32,
}

impl RamCache {
    /// Drop the entries whose TTL has run out at `now_ms`.
    fn expire(&mut self, now_ms: u32) {
        if self.ttl_secs == 0 {
            return;
        }
        let ttl = u64::from(self.ttl_secs) * 1000;
        let mut gone = Vec::new();
        for (key, entry) in &self.entries {
            if u64::from(now_ms.wrapping_sub(entry.stored_ms)) >= ttl {
                gone.push(key.clone());
            }
        }
        for key in gone {
            if let Some(entry) = self.entries.remove(&key) {
                self.bytes = self.bytes.saturating_sub(entry.value.len() as u32);
            }
        }
    }

    /// `IRamCache::Add`: replace a key's value, or store a new one under the three limits.
    pub fn add(&mut self, key: &[u8], value: &[u8], now_ms: u32) -> u32 {
        self.expire(now_ms);
        let value_bytes = value.len() as u32;
        let replaced = self.entries.get(key).map(|e| e.value.len() as u32);
        if replaced.is_none()
            && self.max_entries != 0
            && self.entries.len() as u32 >= self.max_entries
        {
            return g::AEE_ENOMEMORY;
        }
        let without = self.bytes.saturating_sub(replaced.unwrap_or(0));
        if self.max_size != 0 && without.saturating_add(value_bytes) > self.max_size {
            return g::AEE_ENOMEMORY;
        }
        self.bytes = without.saturating_add(value_bytes);
        self.entries.insert(
            key.to_vec(),
            Entry {
                value: value.to_vec(),
                stored_ms: now_ms,
            },
        );
        g::AEE_SUCCESS
    }

    /// `IRamCache::Remove`: the entry goes; an absent key is `AEE_ENOMORE`.
    pub fn remove(&mut self, key: &[u8], now_ms: u32) -> u32 {
        self.expire(now_ms);
        match self.entries.remove(key) {
            Some(entry) => {
                self.bytes = self.bytes.saturating_sub(entry.value.len() as u32);
                g::AEE_SUCCESS
            }
            None => g::AEE_ENOMORE,
        }
    }

    /// `IRamCache::Find`: a copy of the value, or `None` when the key is absent or expired.
    pub fn find(&mut self, key: &[u8], now_ms: u32) -> Option<Vec<u8>> {
        self.expire(now_ms);
        self.entries.get(key).map(|entry| entry.value.clone())
    }

    /// `IRamCache::SetTTL`: the new seconds-to-live, answering with the previous value.
    pub fn set_ttl(&mut self, ttl_secs: u32) -> u32 {
        std::mem::replace(&mut self.ttl_secs, ttl_secs)
    }

    /// `IRamCache::SetMaxSize`: the new byte ceiling, answering with the previous value.
    pub fn set_max_size(&mut self, max_size: u32) -> u32 {
        std::mem::replace(&mut self.max_size, max_size)
    }

    /// `IRamCache::SetMaxEntries`: the new entry ceiling, answering with the previous value.
    pub fn set_max_entries(&mut self, max_entries: u32) -> u32 {
        std::mem::replace(&mut self.max_entries, max_entries)
    }
}
