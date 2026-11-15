//! Yamaha SMAF (`.mmf`) playback, now entirely in Rust.
//!
//! The binary format is parsed by the MIT `smaf` crate; [`sequencer`] maps its raw events to the
//! MA player's timeline, [`voice`] decodes the VM35/VMA patches, [`adpcm`] decodes the 4-bit
//! sample banks, [`fm`] is the FM DSP and [`player`] is the voice pool and mixer.

pub mod adpcm;
pub mod fm;
pub mod player;
pub mod sequencer;
pub mod voice;

pub const SAMPLE_RATE: u32 = 44_100;
pub const MAX_SECONDS: u32 = 300;

/// `MMMD` + the declared size the format parser itself accepts.
pub fn looks_like(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[..4] != b"MMMD" {
        return false;
    }
    let size = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    size >= 8 && size as usize <= bytes.len()
}

/// Length of the score without rendering it. The reference engine includes a one-second tail.
pub fn duration_ms(bytes: &[u8]) -> Option<u32> {
    let score = sequencer::Score::parse(bytes).ok()?;
    // Playback streams, so the length is the score's own end. `MAX_SECONDS` bounds only the
    // offline `decode` (which materialises one buffer) — not what `IMedia` reports or plays.
    (score.length_ms > 0).then_some(score.length_ms)
}

/// Render the whole file to mono PCM at [`SAMPLE_RATE`], downmixing the player's stereo output
/// the same way the mixer did for the C++ bridge. The render is bounded by [`MAX_SECONDS`].
pub fn decode(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    let score = sequencer::Score::parse(bytes)?;
    if score.events.is_empty() {
        return Err("SMAF file has no playable score track".to_string());
    }
    let mut player = player::Player::new(score, SAMPLE_RATE);
    let total = player
        .total_samples()
        .min(u64::from(SAMPLE_RATE) * u64::from(MAX_SECONDS));
    let mut mono = Vec::with_capacity(total as usize);
    let mut stereo = vec![0.0f32; 2048 * 2];
    while (mono.len() as u64) < total {
        let frames = ((total - mono.len() as u64) as usize).min(2048);
        let produced = player.render(&mut stereo[..frames * 2]);
        if produced == 0 {
            break;
        }
        for frame in 0..produced {
            mono.push((stereo[frame * 2] + stereo[frame * 2 + 1]) * 0.5);
        }
    }
    if mono.is_empty() {
        return Err("the SMAF player produced no samples".to_string());
    }
    Ok((SAMPLE_RATE, mono))
}

#[cfg(test)]
#[path = "../test/smaf.rs"]
mod tests;
