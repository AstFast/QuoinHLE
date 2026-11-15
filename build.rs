use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;

/// The source SVG carries a PNG data URI. Decoding it here keeps the repository to one icon file
/// while still giving eframe the raster icon it needs.
fn decode_base64(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &byte in input {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => continue,
        };
        buf = (buf << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    out
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let svg_path = manifest.join("assets/app-icon.svg");
    println!("cargo:rerun-if-changed={}", svg_path.display());

    let svg = fs::read_to_string(&svg_path).expect("read assets/app-icon.svg");
    let marker = "data:image/png;base64,";
    let start = svg.find(marker).expect("app-icon.svg has an embedded PNG") + marker.len();
    let end = start
        + svg[start..]
            .find('"')
            .expect("the PNG data URI ends with a quote");
    let png = decode_base64(&svg.as_bytes()[start..end]);

    let image = image::load_from_memory(&png).expect("decode the embedded icon PNG");
    let icon = image.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("out dir"));
    let png_path = out.join("app-icon-256.png");
    icon.save(&png_path).expect("write the window icon PNG");

    let mut png_bytes = Vec::new();
    icon.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .expect("encode the 256px icon");
    let mut ico = Vec::with_capacity(22 + png_bytes.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // ICONDIR: reserved, type=icon, count=1
    ico.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0]); // 256x256, 32bpp
    ico.extend_from_slice(&(png_bytes.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png_bytes);
    let ico_path = out.join("app-icon.ico");
    fs::write(&ico_path, ico).expect("write the Windows icon");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let rc = out.join("quoin-icon.rc");
    let escaped = ico_path
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    if fs::write(&rc, format!("IDI_ICON ICON \"{escaped}\"\n")).is_err() {
        return;
    }

    let object = out.join("quoin-icon.o");
    match Command::new("windres")
        .args(["-O", "coff", "-i"])
        .arg(&rc)
        .arg("-o")
        .arg(&object)
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("cargo:rustc-link-arg-bins={}", object.display());
        }
        Ok(output) => println!(
            "cargo:warning=windres failed; executable icon not embedded: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => {
            println!("cargo:warning=windres not found; executable icon not embedded ({error})")
        }
    }
}
