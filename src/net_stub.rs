//! No network proxy on Web.
//!
//! `ureq`/`socket2` do not build for `wasm32-unknown-unknown`. The runtime still has the
//! `Option<net::Proxy>` field, so this module keeps the same small API and refuses the flag rather
//! than pretending a request went anywhere.

use std::path::Path;

use crate::socket::ScriptRoutes;

pub struct Ask {
    pub id: u64,
    pub url: String,
}

pub struct Answer {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub truncated: bool,
}

pub struct Reply {
    pub id: u64,
    pub url: String,
    pub result: Result<Answer, String>,
}

pub struct Proxy {
    pub at: String,
    routes: ScriptRoutes,
}

impl Proxy {
    pub fn new(_addr: &str) -> Result<Proxy, String> {
        Err("--net-proxy is not available in this build".into())
    }

    pub fn from_script(_path: &Path) -> Result<Proxy, String> {
        Err("--net-script is not available in this build".into())
    }

    pub fn sockets(&self) -> &ScriptRoutes {
        &self.routes
    }

    pub fn ask(&self, _id: u64, _url: String) -> Result<(), String> {
        Err("networking is not available in this build".into())
    }

    pub fn asked(&self) -> u64 {
        0
    }

    pub fn take_replies(&self) -> Vec<Reply> {
        Vec::new()
    }
}
