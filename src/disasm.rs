//! Disassembly: what the guest's own bytes mean, written the way an assembler would write them.
//!
//! [`crate::cpu::arm`] decodes and executes in one pass — it matches on an encoding and does the
//! work — so it cannot be asked "what is this word?" without executing it. This is the other half,
//! and it is what makes a fault report actionable: a fault says where the program counter was, and
//! the listing says what it was about to do.
//!
//! [`yaxpeax_arm`] does the decoding. It is the one dependency in this tree that is not a part of the
//! machine the emulator talks to, and it earns its place the same way `cpal` and `eframe` do: the
//! alternative is a second, quieter copy of [`crate::cpu::arm`]'s decoder, and two decoders that
//! disagree are worse than one that says it does not know.

use yaxpeax_arch::{Decoder, Reader};
use yaxpeax_arm::armv7::InstDecoder;

/// One listing line: where the bytes are, what they are, and what they mean.
#[derive(Debug, Clone)]
pub struct Line {
    /// The guest address of these bytes.
    pub addr: u32,
    /// The bytes themselves — four of them in ARM state, two or four in Thumb.
    pub bytes: Vec<u8>,
    /// The decoder's text, empty when it refused the bytes.
    pub text: String,
    /// Why the bytes were refused, when they were. Refusing is a normal outcome here: a listing that
    /// runs off the end of a function walks into data, and a decoder that invented an instruction for
    /// it would be inventing the answer to the question the listing was asked.
    pub error: Option<String>,
}

impl Line {
    /// The shape a listing is read in, which is also how [`crate::cpu::Cpu`]'s own trace prints: the
    /// address, the encoding, then the text, all in fixed columns so a diff of two listings lines up.
    pub fn render(&self) -> String {
        let mut bytes = String::new();
        for b in &self.bytes {
            bytes.push_str(&format!("{b:02x}"));
        }
        match &self.error {
            Some(why) => format!("{:08x}  {:<8}  ; {why}", self.addr, bytes),
            None => format!("{:08x}  {:<8}  {}", self.addr, bytes, self.text),
        }
    }
}

/// Disassemble bytes that live at `base`, in ARM or Thumb state, up to `limit` instructions.
///
/// A word the decoder refuses does not end the listing: it is reported as a line and the walk
/// resumes one instruction (or two bytes in Thumb) later. That is the same choice `--trace` makes
/// when it meets a group the interpreter has not built — say so and keep going, because the reason
/// the listing was asked for is usually a few instructions further on.
pub fn disassemble(bytes: &[u8], base: u32, thumb: bool, limit: usize) -> Vec<Line> {
    // Held for the whole walk rather than one decode: taking the hook back out while a panic is still
    // unwinding is a second panic in the making.
    let _quiet = Quiet::new();
    let mut decoder = InstDecoder::default();
    decoder.set_thumb_mode(thumb);
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() && out.len() < limit {
        // A reader per instruction, so how far the previous one was advanced cannot matter: this is
        // also how the instruction's own length is measured, since the decoder is never asked for
        // one. The trait is spelled out in full because `U8Reader` implements it for every address
        // and word width the crate knows, and nothing here would tell the compiler which.
        let mut reader = yaxpeax_arch::U8Reader::new(&bytes[at..]);
        // `yaxpeax-arm` panics on some encodings instead of refusing them — a "register number out of
        // range" inside its own `armv7.rs`, reached by walking off the end of a function into the data
        // that follows it, which is exactly what a listing asked for at a fault address does. The
        // listing is the tool that answers "what was the guest doing", and one word the decoder
        // cannot express must not take the run down with it, so the panic is caught and reported as
        // the refusal it should have returned. The hook is silenced for the duration because the
        // message it would print says the process panicked, which is no longer true.
        let decoded =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decoder.decode(&mut reader)));
        let why = match decoded {
            Ok(Ok(inst)) => {
                let len = <yaxpeax_arch::U8Reader<'_> as Reader<u32, u8>>::total_offset(&mut reader)
                    as usize;
                let end = (at + len.max(1)).min(bytes.len());
                out.push(Line {
                    addr: base.wrapping_add(at as u32),
                    bytes: bytes[at..end].to_vec(),
                    text: inst.to_string(),
                    error: None,
                });
                at = end;
                continue;
            }
            Ok(Err(e)) => e.to_string(),
            Err(_) => "the decoder panicked on these bytes".to_string(),
        };
        // A word the decoder refuses — or falls over on — still moves the walk on by one instruction,
        // because the reason someone asked for a listing is usually a few instructions further on and
        // a listing that stops at the first byte it does not know answers nothing.
        let step = if thumb { 2 } else { 4 };
        let end = (at + step).min(bytes.len());
        out.push(Line {
            addr: base.wrapping_add(at as u32),
            bytes: bytes[at..end].to_vec(),
            text: String::new(),
            error: Some(why),
        });
        at = end;
    }
    out
}

/// Silence the panic hook for as long as one decode is in flight.
///
/// A caught panic would still announce itself, and the announcement is wrong twice over: it names
/// the thread that was about to survive, and it prints to the same stderr the run's own report is
/// written to. Taking the hook back out on the way drop restores whatever was there before, which
/// for everything outside this file is the default one.
struct Quiet(());

impl Quiet {
    fn new() -> Self {
        std::panic::set_hook(Box::new(|_| {}));
        Quiet(())
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        // The returned hook is the one being removed; dropping it restores the default printer, which
        // is the whole point.
        drop(std::panic::take_hook());
    }
}

#[cfg(test)]
#[path = "test/disasm.rs"]
mod tests;
