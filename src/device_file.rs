//! The handset description, versioned and grouped by the part of the device it describes.
//!
//! The document is a set of overrides over [`DeviceProfile::standard`]: an absent key keeps the
//! built-in value, so a profile only spells out what makes this handset different. Grouping keeps
//! future fields with the device area they belong to instead of growing one flat namespace.
//!
//! # Versioning
//!
//! `version` is `MAJOR.MINOR` and is required.
//!
//! - The major number names the layout: an unknown major is refused.
//! - A reader accepts every older minor of its major, filling absent fields from the built-in
//!   handset. That is the backward-compatibility contract.
//! - An older reader refuses a newer minor instead of silently dropping fields it does not know.
//! - A field whose meaning changes, or a section that moves, is a major bump.
//!
//! # Additions before the next release
//!
//! The backing fields added here — `screen.frame_bpp`, `abi.dib_pixel_offset`, `abi.dib_depth`,
//! `abi.graphics_canvas_offset` and `system.fs_case_insensitive` — are optional in layout 1.0:
//! absent keys keep the built-in defaults, so files written before they existed still read the
//! same handset. The minor number is
//! not bumped until the next release is declared; until then an older 1.0 reader may reject a key
//! it has never seen rather than silently ignoring it.

use std::path::Path;

use crate::abi;
use crate::device::{DeviceFlags, DeviceProfile};
use crate::json::{self, Map, Value};

/// The layout this build writes and the oldest minor it still understands.
pub const VERSION_MAJOR: u64 = 1;
pub const VERSION_MINOR: u64 = 0;
pub const VERSION: &str = "1.0";

fn object(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (key, value) in entries {
        m.insert(key, value);
    }
    Value::Object(m)
}

/// The document for one profile, in the section order this format declares.
pub fn to_json(profile: &DeviceProfile) -> Value {
    object(vec![
        ("version", Value::from(VERSION)),
        (
            "screen",
            object(vec![
                ("width", Value::from(profile.width)),
                ("height", Value::from(profile.height)),
                ("color_depth", Value::from(profile.color_depth)),
                ("frame_bpp", Value::from(profile.frame_bpp)),
                ("alt_width", Value::from(profile.alt_width)),
                ("alt_height", Value::from(profile.alt_height)),
                ("scroll_bar", Value::from(profile.scroll_bar)),
            ]),
        ),
        (
            "text",
            object(vec![
                ("encoding", Value::from(profile.encoding)),
                ("text_bytes", Value::from(profile.text_bytes)),
                ("font_normal", Value::from(profile.font_normal)),
                ("font_large", Value::from(profile.font_large)),
            ]),
        ),
        (
            "loader",
            object(vec![
                ("stack_reserve", Value::from(profile.stack_reserve)),
                ("module_base", Value::from(profile.module_base)),
            ]),
        ),
        (
            "abi",
            object(vec![
                ("device_info_size", Value::from(profile.device_info_size)),
                ("platform_id", Value::from(profile.platform_id)),
                ("ktf", Value::from(profile.ktf)),
                ("dib_pixel_offset", Value::from(profile.dib_pixel_offset)),
                ("dib_depth", Value::from(profile.dib_depth)),
                (
                    "graphics_canvas_offset",
                    Value::from(profile.graphics_canvas_offset),
                ),
            ]),
        ),
        (
            "input",
            object(vec![
                (
                    "softkey_count",
                    Value::from(u32::from(profile.softkey_count)),
                ),
                ("key_close_app", Value::from(profile.key_close_app)),
                (
                    "key_close_all_apps",
                    Value::from(profile.key_close_all_apps),
                ),
                ("repeat_start", Value::from(profile.repeat_start)),
                ("repeat_rate", Value::from(profile.repeat_rate)),
                ("key_repeat_start", Value::from(profile.key_repeat_start)),
                ("key_repeat_rate", Value::from(profile.key_repeat_rate)),
                ("keys", keys_json(&profile.keys)),
            ]),
        ),
        (
            "identity",
            object(vec![
                ("chip_id", Value::from(profile.chip_id.as_str())),
                ("model_name", Value::from(profile.model_name.as_str())),
                ("manufacturer", Value::from(profile.manufacturer.as_str())),
                ("friendly_name", Value::from(profile.friendly_name.as_str())),
                (
                    "firmware_version",
                    Value::from(profile.firmware_version.as_str()),
                ),
                ("carrier_id", Value::from(profile.carrier_id)),
                ("esn", Value::from(profile.esn)),
                ("imei", Value::from(profile.imei.as_str())),
                (
                    "subscriber_number",
                    Value::from(profile.subscriber_number.as_str()),
                ),
            ]),
        ),
        (
            "system",
            object(vec![
                ("menu_text_scroll", Value::from(profile.menu_text_scroll)),
                ("menu_image_delay", Value::from(profile.menu_image_delay)),
                ("ram", Value::from(profile.ram)),
                ("heap_premap", Value::from(profile.heap_premap)),
                ("object_premap", Value::from(profile.object_premap)),
                ("prompt_props", Value::from(profile.prompt_props)),
                ("lang", Value::from(profile.lang)),
                ("net_linger", Value::from(profile.net_linger)),
                ("sleep_defer", Value::from(profile.sleep_defer)),
                ("max_path", Value::from(profile.max_path)),
                ("manner_mode", Value::from(profile.manner_mode)),
                ("storage_total", Value::from(profile.storage_total)),
                ("storage_free", Value::from(profile.storage_free)),
                (
                    "fs_case_insensitive",
                    Value::from(profile.fs_case_insensitive),
                ),
            ]),
        ),
        (
            "memory",
            object(vec![(
                "windows",
                Value::Array(
                    profile
                        .ram_windows
                        .iter()
                        .map(|w| {
                            object(vec![
                                ("base", Value::from(w.base)),
                                ("len", Value::from(w.len)),
                            ])
                        })
                        .collect(),
                ),
            )]),
        ),
        (
            "capabilities",
            object(vec![
                ("flags", flags_json(&profile.flags)),
                (
                    "overlay",
                    object(vec![
                        ("present", Value::from(profile.overlay.present)),
                        (
                            "max_overlay",
                            Value::from(u32::from(profile.overlay.max_overlay)),
                        ),
                        (
                            "max_underlay",
                            Value::from(u32::from(profile.overlay.max_underlay)),
                        ),
                        (
                            "max_surface_per_layer",
                            Value::from(profile.overlay.max_surface_per_layer),
                        ),
                        (
                            "max_total_surfaces",
                            Value::from(profile.overlay.max_total_surfaces),
                        ),
                        ("pbuffer", Value::from(profile.overlay.pbuffer)),
                        ("pixmap", Value::from(profile.overlay.pixmap)),
                    ]),
                ),
                (
                    "gl",
                    object(vec![
                        ("igl", Value::from(profile.gl.igl)),
                        ("qegl", Value::from(profile.gl.qegl)),
                        ("gles10", Value::from(profile.gl.gles10)),
                        ("gles11", Value::from(profile.gl.gles11)),
                        ("gles10_ext", Value::from(profile.gl.gles10_ext)),
                        ("gles11_ext", Value::from(profile.gl.gles11_ext)),
                        ("gles11_ext_pak", Value::from(profile.gl.gles11_ext_pak)),
                        ("imageon_ext", Value::from(profile.gl.imageon_ext)),
                        ("imageon_ext_v1", Value::from(profile.gl.imageon_ext_v1)),
                        ("vendor", Value::from(profile.gl.vendor.as_str())),
                        ("renderer", Value::from(profile.gl.renderer.as_str())),
                        ("version", Value::from(profile.gl.version.as_str())),
                        ("extensions", Value::from(profile.gl.extensions.as_str())),
                        ("max_texture_size", Value::from(profile.gl.max_texture_size)),
                        (
                            "max_texture_max_anisotropy",
                            Value::from(profile.gl.max_texture_max_anisotropy),
                        ),
                        ("max_lights", Value::from(profile.gl.max_lights)),
                        (
                            "max_texture_units",
                            Value::from(profile.gl.max_texture_units),
                        ),
                        (
                            "max_palette_matrices",
                            Value::from(profile.gl.max_palette_matrices),
                        ),
                        ("max_vertex_units", Value::from(profile.gl.max_vertex_units)),
                        (
                            "max_modelview_stack_depth",
                            Value::from(profile.gl.max_modelview_stack_depth),
                        ),
                        (
                            "max_projection_stack_depth",
                            Value::from(profile.gl.max_projection_stack_depth),
                        ),
                        (
                            "max_texture_stack_depth",
                            Value::from(profile.gl.max_texture_stack_depth),
                        ),
                        ("point_size_min", Value::from(profile.gl.point_size_min)),
                        ("point_size_max", Value::from(profile.gl.point_size_max)),
                        ("line_width_min", Value::from(profile.gl.line_width_min)),
                        ("line_width_max", Value::from(profile.gl.line_width_max)),
                        ("subpixel_bits", Value::from(profile.gl.subpixel_bits)),
                        ("red_bits", Value::from(profile.gl.red_bits)),
                        ("green_bits", Value::from(profile.gl.green_bits)),
                        ("blue_bits", Value::from(profile.gl.blue_bits)),
                        ("alpha_bits", Value::from(profile.gl.alpha_bits)),
                        ("depth_bits", Value::from(profile.gl.depth_bits)),
                        ("stencil_bits", Value::from(profile.gl.stencil_bits)),
                        (
                            "max_elements_vertices",
                            Value::from(profile.gl.max_elements_vertices),
                        ),
                        (
                            "max_elements_indices",
                            Value::from(profile.gl.max_elements_indices),
                        ),
                        ("read_format", Value::from(profile.gl.read_format)),
                        ("read_type", Value::from(profile.gl.read_type)),
                        (
                            "compressed_texture_formats",
                            Value::Array(
                                profile
                                    .gl
                                    .compressed_texture_formats
                                    .iter()
                                    .map(|v| Value::from(*v))
                                    .collect(),
                            ),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

/// The document as text, ending in a newline.
pub fn to_text(profile: &DeviceProfile) -> String {
    json::write(&to_json(profile))
}

fn keys_json(keys: &[u16]) -> Value {
    Value::Array(
        keys.iter()
            .map(|code| match key_name(*code) {
                Some(name) => Value::from(name),
                None => Value::from(*code),
            })
            .collect(),
    )
}

fn flags_json(flags: &DeviceFlags) -> Value {
    object(vec![
        ("alt_display", Value::from(flags.alt_display)),
        ("flip", Value::from(flags.flip)),
        ("vibrator", Value::from(flags.vibrator)),
        ("ext_speaker", Value::from(flags.ext_speaker)),
        ("voice_recognition", Value::from(flags.voice_recognition)),
        ("position_location", Value::from(flags.position_location)),
        ("midi", Value::from(flags.midi)),
        ("cmx", Value::from(flags.cmx)),
        ("pen", Value::from(flags.pen)),
    ])
}

fn check_version(version: &str) -> Result<(), String> {
    let mut parts = version.split('.');
    let (Some(major), Some(minor), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!(
            "\"version\" is {version:?}; it has to be MAJOR.MINOR, such as {VERSION}"
        ));
    };
    let major: u64 = major
        .parse()
        .map_err(|_| format!("\"version\" major is not a number: {major:?}"))?;
    let minor: u64 = minor
        .parse()
        .map_err(|_| format!("\"version\" minor is not a number: {minor:?}"))?;
    if major != VERSION_MAJOR {
        return Err(format!(
            "\"version\" is {version}; this build reads layout {VERSION_MAJOR}.x"
        ));
    }
    if minor > VERSION_MINOR {
        return Err(format!("\"version\" is {version}; this build reads up to {VERSION_MAJOR}.{VERSION_MINOR} and will not drop fields it does not know"));
    }
    Ok(())
}

/// Read a versioned document into a profile, starting from the built-in handset.
pub fn from_json(value: &Value) -> Result<DeviceProfile, String> {
    let Some(object) = value.as_object() else {
        return Err(format!(
            "the document has to be an object, and this is {}",
            value.kind()
        ));
    };
    let mut root = Reader::new(object, "document");
    let version = root.text("version")?.ok_or_else(|| "\"version\" is required: this is a versioned layout, and a file without one is not one of its versions".to_string())?;
    check_version(&version)?;
    let mut p = DeviceProfile::standard();
    if let Some(mut sec) = root.section("screen")? {
        if let Some(v) = sec.u16("width")? {
            p.width = v;
        }
        if let Some(v) = sec.u16("height")? {
            p.height = v;
        }
        if let Some(v) = sec.u16("color_depth")? {
            p.color_depth = v;
        }
        if let Some(v) = sec.u16("frame_bpp")? {
            p.frame_bpp = v;
        }
        if let Some(v) = sec.u16("alt_width")? {
            p.alt_width = v;
        }
        if let Some(v) = sec.u16("alt_height")? {
            p.alt_height = v;
        }
        if let Some(v) = sec.u16("scroll_bar")? {
            p.scroll_bar = v;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("text")? {
        if let Some(v) = sec.u16("encoding")? {
            p.encoding = v;
            p.declared.encoding = true;
        }
        if let Some(v) = sec.bool("text_bytes")? {
            p.text_bytes = v;
            p.declared.text_bytes = true;
        }
        if let Some(v) = sec.u16("font_normal")? {
            p.font_normal = v;
            p.declared.font_normal = true;
        }
        if let Some(v) = sec.u16("font_large")? {
            p.font_large = v;
            p.declared.font_large = true;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("loader")? {
        if let Some(v) = sec.u32("stack_reserve")? {
            p.stack_reserve = v;
        }
        if let Some(v) = sec.u32("module_base")? {
            p.module_base = v;
            p.declared.module_base = true;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("abi")? {
        if let Some(v) = sec.u16("device_info_size")? {
            p.device_info_size = v;
            p.declared.device_info_size = true;
        }
        if let Some(v) = sec.u32("platform_id")? {
            p.platform_id = v;
        }
        if let Some(v) = sec.bool("ktf")? {
            p.ktf = v;
        }
        if let Some(v) = sec.u32("dib_pixel_offset")? {
            p.dib_pixel_offset = v;
        }
        if let Some(v) = sec.u16("dib_depth")? {
            p.dib_depth = v;
        }
        if let Some(v) = sec.u32("graphics_canvas_offset")? {
            p.graphics_canvas_offset = v;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("input")? {
        if let Some(v) = sec.u8("softkey_count")? {
            p.softkey_count = v;
        }
        if let Some(v) = sec.u16("key_close_app")? {
            p.key_close_app = v;
        }
        if let Some(v) = sec.u16("key_close_all_apps")? {
            p.key_close_all_apps = v;
        }
        if let Some(v) = sec.u32("repeat_start")? {
            p.repeat_start = v;
        }
        if let Some(v) = sec.u32("repeat_rate")? {
            p.repeat_rate = v;
        }
        if let Some(v) = sec.u32("key_repeat_start")? {
            p.key_repeat_start = v;
        }
        if let Some(v) = sec.u32("key_repeat_rate")? {
            p.key_repeat_rate = v;
        }
        if let Some(v) = sec.keys("keys")? {
            p.keys = v;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("identity")? {
        if let Some(v) = sec.text("chip_id")? {
            p.chip_id = v;
        }
        if let Some(v) = sec.text("model_name")? {
            p.model_name = v;
        }
        if let Some(v) = sec.text("manufacturer")? {
            p.manufacturer = v;
        }
        if let Some(v) = sec.text("friendly_name")? {
            p.friendly_name = v;
        }
        if let Some(v) = sec.text("firmware_version")? {
            p.firmware_version = v;
        }
        if let Some(v) = sec.u32("carrier_id")? {
            p.carrier_id = v;
        }
        if let Some(v) = sec.u32("esn")? {
            p.esn = v;
        }
        if let Some(v) = sec.text("imei")? {
            p.imei = v;
        }
        if let Some(v) = sec.text("subscriber_number")? {
            p.subscriber_number = v;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("system")? {
        if let Some(v) = sec.u16("menu_text_scroll")? {
            p.menu_text_scroll = v;
        }
        if let Some(v) = sec.u32("menu_image_delay")? {
            p.menu_image_delay = v;
        }
        if let Some(v) = sec.u32("ram")? {
            p.ram = v;
        }
        if let Some(v) = sec.u32("heap_premap")? {
            p.heap_premap = v;
            p.declared.heap_premap = true;
        }
        if let Some(v) = sec.u32("object_premap")? {
            p.object_premap = v;
            p.declared.object_premap = true;
        }
        if let Some(v) = sec.u32("prompt_props")? {
            p.prompt_props = v;
        }
        if let Some(v) = sec.u32("lang")? {
            p.lang = v;
        }
        if let Some(v) = sec.u32("net_linger")? {
            p.net_linger = v;
        }
        if let Some(v) = sec.u32("sleep_defer")? {
            p.sleep_defer = v;
        }
        if let Some(v) = sec.u16("max_path")? {
            p.max_path = v;
        }
        if let Some(v) = sec.u32("manner_mode")? {
            p.manner_mode = v;
        }
        if let Some(v) = sec.u64("storage_total")? {
            p.storage_total = v;
        }
        if let Some(v) = sec.u64("storage_free")? {
            p.storage_free = v;
        }
        if let Some(v) = sec.bool("fs_case_insensitive")? {
            p.fs_case_insensitive = v;
        }
        sec.finish()?;
    }
    if let Some(mut sec) = root.section("memory")? {
        if let Some(windows) = sec.ram_windows("windows")? {
            p.ram_windows = windows;
            p.declared.ram_windows = true;
        }
        sec.finish()?;
    }
    if let Some(mut cap) = root.section("capabilities")? {
        if let Some(v) = cap.flags("flags", p.flags)? {
            p.flags = v;
        }
        if let Some(mut sec) = cap.section("overlay")? {
            if let Some(v) = sec.bool("present")? {
                p.overlay.present = v;
            }
            if let Some(v) = sec.u8("max_overlay")? {
                p.overlay.max_overlay = v;
            }
            if let Some(v) = sec.u8("max_underlay")? {
                p.overlay.max_underlay = v;
            }
            if let Some(v) = sec.u16("max_surface_per_layer")? {
                p.overlay.max_surface_per_layer = v;
            }
            if let Some(v) = sec.u16("max_total_surfaces")? {
                p.overlay.max_total_surfaces = v;
            }
            if let Some(v) = sec.bool("pbuffer")? {
                p.overlay.pbuffer = v;
            }
            if let Some(v) = sec.bool("pixmap")? {
                p.overlay.pixmap = v;
            }
            sec.finish()?;
        }
        if let Some(mut sec) = cap.section("gl")? {
            if let Some(v) = sec.bool("igl")? {
                p.gl.igl = v;
            }
            if let Some(v) = sec.bool("qegl")? {
                p.gl.qegl = v;
            }
            if let Some(v) = sec.bool("gles10")? {
                p.gl.gles10 = v;
            }
            if let Some(v) = sec.bool("gles11")? {
                p.gl.gles11 = v;
            }
            if let Some(v) = sec.bool("gles10_ext")? {
                p.gl.gles10_ext = v;
            }
            if let Some(v) = sec.bool("gles11_ext")? {
                p.gl.gles11_ext = v;
            }
            if let Some(v) = sec.bool("gles11_ext_pak")? {
                p.gl.gles11_ext_pak = v;
            }
            if let Some(v) = sec.bool("imageon_ext")? {
                p.gl.imageon_ext = v;
            }
            if let Some(v) = sec.bool("imageon_ext_v1")? {
                p.gl.imageon_ext_v1 = v;
            }
            if let Some(v) = sec.text("vendor")? {
                p.gl.vendor = v;
            }
            if let Some(v) = sec.text("renderer")? {
                p.gl.renderer = v;
            }
            if let Some(v) = sec.text("version")? {
                p.gl.version = v;
            }
            if let Some(v) = sec.text("extensions")? {
                p.gl.extensions = v;
            }
            if let Some(v) = sec.u32("max_texture_size")? {
                p.gl.max_texture_size = v;
            }
            if let Some(v) = sec.u32("max_texture_max_anisotropy")? {
                p.gl.max_texture_max_anisotropy = v;
            }
            if let Some(v) = sec.u32("max_lights")? {
                p.gl.max_lights = v;
            }
            if let Some(v) = sec.u32("max_texture_units")? {
                p.gl.max_texture_units = v;
            }
            if let Some(v) = sec.u32("max_palette_matrices")? {
                p.gl.max_palette_matrices = v;
            }
            if let Some(v) = sec.u32("max_vertex_units")? {
                p.gl.max_vertex_units = v;
            }
            if let Some(v) = sec.u32("max_modelview_stack_depth")? {
                p.gl.max_modelview_stack_depth = v;
            }
            if let Some(v) = sec.u32("max_projection_stack_depth")? {
                p.gl.max_projection_stack_depth = v;
            }
            if let Some(v) = sec.u32("max_texture_stack_depth")? {
                p.gl.max_texture_stack_depth = v;
            }
            if let Some(v) = sec.u32("point_size_min")? {
                p.gl.point_size_min = v;
            }
            if let Some(v) = sec.u32("point_size_max")? {
                p.gl.point_size_max = v;
            }
            if let Some(v) = sec.u32("line_width_min")? {
                p.gl.line_width_min = v;
            }
            if let Some(v) = sec.u32("line_width_max")? {
                p.gl.line_width_max = v;
            }
            if let Some(v) = sec.u32("subpixel_bits")? {
                p.gl.subpixel_bits = v;
            }
            if let Some(v) = sec.u32("red_bits")? {
                p.gl.red_bits = v;
            }
            if let Some(v) = sec.u32("green_bits")? {
                p.gl.green_bits = v;
            }
            if let Some(v) = sec.u32("blue_bits")? {
                p.gl.blue_bits = v;
            }
            if let Some(v) = sec.u32("alpha_bits")? {
                p.gl.alpha_bits = v;
            }
            if let Some(v) = sec.u32("depth_bits")? {
                p.gl.depth_bits = v;
            }
            if let Some(v) = sec.u32("stencil_bits")? {
                p.gl.stencil_bits = v;
            }
            if let Some(v) = sec.u32("max_elements_vertices")? {
                p.gl.max_elements_vertices = v;
            }
            if let Some(v) = sec.u32("max_elements_indices")? {
                p.gl.max_elements_indices = v;
            }
            if let Some(v) = sec.u32("read_format")? {
                p.gl.read_format = v;
            }
            if let Some(v) = sec.u32("read_type")? {
                p.gl.read_type = v;
            }
            if let Some(v) = sec.u32s("compressed_texture_formats")? {
                p.gl.compressed_texture_formats = v;
            }
            sec.finish()?;
        }
        cap.finish()?;
    }
    root.finish()?;
    check(&p)?;
    Ok(p)
}

/// Refuse a device that is not a device: arithmetic the host would have to answer for.
fn check(p: &DeviceProfile) -> Result<(), String> {
    if p.stack_reserve == 0 || p.stack_reserve > crate::layout::STACK_SIZE {
        return Err(format!(
            "\"loader.stack_reserve\" has to be between 1 and {}, and this document gives {}",
            crate::layout::STACK_SIZE,
            p.stack_reserve
        ));
    }
    if p.width == 0 || p.height == 0 {
        return Err(format!("\"screen.width\" and \"screen.height\" have to be at least 1, and this document gives {}x{}", p.width, p.height));
    }
    if p.color_depth == 0 {
        return Err("\"screen.color_depth\" is 0; it is the depth of the screen, so it cannot be no bits at all".to_string());
    }
    if !matches!(p.frame_bpp, 16 | 32) {
        return Err(format!(
            "\"screen.frame_bpp\" is {}; this host backs a 16- or 32-bpp frame",
            p.frame_bpp
        ));
    }
    if !matches!(p.dib_depth, 16 | 32) {
        return Err(format!(
            "\"abi.dib_depth\" is {}; off-screen DIBs are backed at 16 or 32 bpp",
            p.dib_depth
        ));
    }
    if p.dib_depth < p.frame_bpp {
        return Err(format!(
            "\"abi.dib_depth\" is {} but \"screen.frame_bpp\" is {}: a 32-bpp screen whose own compatible bitmap is refused is not one device",
            p.dib_depth, p.frame_bpp
        ));
    }
    if p.dib_pixel_offset < crate::display::MIN_DIB_PIXEL_OFFSET || p.dib_pixel_offset > 0x10000 {
        return Err(format!(
            "\"abi.dib_pixel_offset\" is {}; it is the offset of pixel (0,0) in the DIB object, so it has to be {}..=65536",
            p.dib_pixel_offset, crate::display::MIN_DIB_PIXEL_OFFSET
        ));
    }
    if p.graphics_canvas_offset != 0
        && (p.graphics_canvas_offset < 8
            || p.graphics_canvas_offset % 4 != 0
            || p.graphics_canvas_offset > 0x1000)
    {
        return Err(format!(
            "\"abi.graphics_canvas_offset\" is {}; zero turns the mirror off, otherwise it is a 4-byte-aligned offset of at least 8 (0 and 4 are the vptr and refcount)",
            p.graphics_canvas_offset
        ));
    }
    if p.storage_free > p.storage_total {
        return Err(format!("\"system.storage_free\" is {} and \"system.storage_total\" is {}: more free space than the filesystem has is an answer a pack divides by", p.storage_free, p.storage_total));
    }
    for (i, w) in p.ram_windows.iter().enumerate() {
        if w.len == 0 {
            return Err(format!(
                "\"memory.windows\"[{i}] has no length; a window of nothing is not a window"
            ));
        }
        if w.base.checked_add(w.len).is_none() {
            return Err(format!(
                "\"memory.windows\"[{i}] runs from 0x{:08x} for 0x{:x} bytes, which is past the top of the address space",
                w.base, w.len
            ));
        }
        // A window the guest heap already owns would be handed out by the allocator as well, and
        // the two would write over one another with nothing to say which won. That is a file
        // mistake, not a handset.
        let heap_lo = crate::layout::HEAP_BASE;
        let heap_hi = crate::layout::HEAP_LIMIT;
        if w.base < heap_hi && heap_lo < w.base.saturating_add(w.len) {
            return Err(format!(
                "\"memory.windows\"[{i}] (0x{:08x}+0x{:x}) overlaps the guest heap 0x{heap_lo:08x}..0x{heap_hi:08x}",
                w.base, w.len
            ));
        }
    }
    if p.imei.len() > 15 || !p.imei.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "\"identity.imei\" has to be at most fifteen ASCII digits, and this is {:?}",
            p.imei
        ));
    }
    if p.subscriber_number.len() > 15 || !p.subscriber_number.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("\"identity.subscriber_number\" has to be at most fifteen ASCII digits, and this is {:?}", p.subscriber_number));
    }
    for (key, text) in [
        ("identity.chip_id", &p.chip_id),
        ("identity.model_name", &p.model_name),
        ("identity.manufacturer", &p.manufacturer),
        ("identity.friendly_name", &p.friendly_name),
        ("identity.firmware_version", &p.firmware_version),
        ("identity.subscriber_number", &p.subscriber_number),
    ] {
        if text.contains('\0') {
            return Err(format!("\"{key}\" contains U+0000; it is answered as a terminated string, so the guest would stop reading there and never see the rest of {text:?}"));
        }
    }
    Ok(())
}

/// What [`load`] found.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub profile: DeviceProfile,
    /// Whether the file was absent and has just been written with the standard handset. The caller
    /// says so out loud: a run that silently invented a file its operator did not ask for is a
    /// surprise waiting for the first person who edits the wrong one.
    pub created: bool,
}

/// Read `path`, or write the standard profile there and use it.
///
/// Creating the file is the whole discovery story: there is no search path, no environment variable
/// and no default location, because a device description that a run picked up from somewhere is
/// exactly the kind of thing that makes two runs of the same command disagree. Ask for the file and
/// you get it; do not ask and the built-in handset is used.
pub fn load(path: &Path) -> Result<Loaded, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let value = json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile = from_json(&value).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Loaded {
                profile,
                created: false,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let profile = DeviceProfile::standard();
            save(path, &profile).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Loaded {
                profile,
                created: true,
            })
        }
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Write `profile` to `path`, replacing whatever was there.
///
/// The `_`-prefixed notes already in the file are carried over. They are the one part of the document
/// this module does not own — a person wrote them — and a re-save that dropped them would delete an
/// explanation at the moment someone was rewriting the values it explained.
///
/// That is also why a file that cannot be *read* is refused instead of overwritten: "no notes found"
/// and "the file did not parse" are the same thing to a writer that shrugs, and only one of them is
/// the truth. A file that is not there is the one case with nothing to lose.
///
/// What comes out is the notes first, then `version` and the sections in the order this format
/// declares them — so a file that has notes does not start with `version`, and that is on purpose:
/// the note is the reason the file exists.
pub fn save(path: &Path, profile: &DeviceProfile) -> Result<(), String> {
    let mut document = to_json(profile);
    let notes = match std::fs::read_to_string(path) {
        Ok(text) => {
            let value = json::parse(&text).map_err(|e| {
                format!(
                    "{}: {e} — not overwriting it, because the `_` notes in it would be lost",
                    path.display()
                )
            })?;
            match value {
                Value::Object(object) => object
                    .iter()
                    .filter(|(k, _)| k.starts_with('_'))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if !notes.is_empty() {
        let mut merged = Map::new();
        for (k, v) in notes {
            merged.insert(k, v);
        }
        if let Some(object) = document.as_object() {
            for (k, v) in object.iter() {
                merged.insert(k.clone(), v.clone());
            }
        }
        document = Value::Object(merged);
    }
    // Through a temporary file: `fs::write` truncates first, so a run that dies mid-write leaves a
    // half document that the *next* run then refuses to parse — the file is the answer to "what did
    // I describe", and an answer cut in half is worse than the previous one left alone.
    let text = json::write(&document);
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text).map_err(|e| format!("{}: {e}", temp.display()))?;
    std::fs::rename(&temp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The name the ABI and the keypad table give a key code, if either knows one.
///
/// The keypad table first: it is this host's own list of the keys a player can press, so its names
/// are the ones the rest of the tree already prints. `const_name` is the fallback for the keys a
/// handset advertises but no key draws (`AVK_SEND`, `AVK_MENU`, `AVK_END` are in [`DeviceProfile`]'s
/// default list and not on the drawn phone).
fn key_name(code: u16) -> Option<&'static str> {
    if let Some(key) = crate::keymap::KEYS.iter().find(|k| k.avk() as u16 == code) {
        return Some(key.name);
    }
    abi::const_name("AVK_", u32::from(code))
}

/// A key code from an `AVK_*` name. Only names under that prefix: `ABI` has nine thousand constants
/// and a keypad entry that happens to be `AEE_EBADPARM` is a file that means something else.
///
/// Any name under the prefix is taken, including the five the platform uses as markers rather than keys
/// (`AVK_FIRST` `0x0000e020`, `AVK_UNDEFINED` `0x0000e010`, `AVK_OEM_RANGE_START`/`_END`
/// `0x0000f800`/`0x0000f8ff`, `AVK_MAX_OVERLOAD` `0x00000005`). Telling those apart would take a
/// hardcoded list that goes stale with the next platform, and the cost of getting it wrong is a handset
/// that claims support for a code no pack will ever press — cheaper than refusing a key this host
/// has not heard of, which would be the file saying something true and the host calling it a typo.
fn key_code(name: &str) -> Option<u16> {
    if !name.starts_with("AVK_") {
        return None;
    }
    let value = abi::const_value(name)?;
    if value > u32::from(u16::MAX) {
        return None;
    }
    Some(value as u16)
}

/// One object being read, remembering which keys were asked for so the rest can be named as errors.
struct Reader<'a> {
    object: &'a Map,
    seen: Vec<&'static str>,
    label: &'static str,
}

impl<'a> Reader<'a> {
    fn new(object: &'a Map, label: &'static str) -> Self {
        Reader {
            object,
            seen: Vec::new(),
            label,
        }
    }

    fn section(&mut self, key: &'static str) -> Result<Option<Reader<'a>>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(object) = value.as_object() else {
            return Err(mismatch(key, "an object", value));
        };
        Ok(Some(Reader::new(object, key)))
    }

    fn finish(&self) -> Result<(), String> {
        let unknown = self.leftovers();
        if unknown.is_empty() {
            return Ok(());
        }
        let names: Vec<String> = unknown.iter().map(|k| format!("\"{k}\"")).collect();
        Err(format!(
            "unknown key(s) in \"{}\": {}. A key with no field behind it would be a run that              reports this host's default while the file says otherwise; start a key with `_` to              keep a note in the file",
            self.label,
            names.join(", ")
        ))
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        let value = self.object.get(key);
        if value.is_some() {
            self.seen.push(key);
        }
        value
    }

    fn u64(&mut self, key: &'static str) -> Result<Option<u64>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_u64() {
            Some(n) => Ok(Some(n)),
            None => Err(mismatch(key, "a non-negative whole number", value)),
        }
    }

    /// An array of `{"base", "len"}` objects: the spans of the handset's memory a pack's saves may
    /// point into. Written as objects rather than two parallel arrays because the two numbers only
    /// mean anything together — a length without its base is not half a window, it is a typo.
    fn ram_windows(
        &mut self,
        key: &'static str,
    ) -> Result<Option<Vec<crate::device::RamWindow>>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(items) = value.as_array() else {
            return Err(mismatch(
                key,
                "an array of {\"base\", \"len\"} objects",
                value,
            ));
        };
        let mut out = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let Some(object) = item.as_object() else {
                return Err(format!(
                    "\"{key}\"[{i}]: expected an object with \"base\" and \"len\", found {}",
                    item.kind()
                ));
            };
            let mut r = Reader::new(object, "memory.windows");
            let at = |what: &str| format!("\"{key}\"[{i}] has no \"{what}\"");
            let base = r.u32("base")?.ok_or_else(|| at("base"))?;
            let len = r.u32("len")?.ok_or_else(|| at("len"))?;
            r.finish().map_err(|e| format!("\"{key}\"[{i}]: {e}"))?;
            out.push(crate::device::RamWindow { base, len });
        }
        Ok(Some(out))
    }

    fn u32(&mut self, key: &'static str) -> Result<Option<u32>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u32::MAX) => Ok(Some(n as u32)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 32 bits")),
        }
    }

    fn u16(&mut self, key: &'static str) -> Result<Option<u16>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u16::MAX) => Ok(Some(n as u16)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 16 bits")),
        }
    }

    fn u8(&mut self, key: &'static str) -> Result<Option<u8>, String> {
        match self.u64(key)? {
            None => Ok(None),
            Some(n) if n <= u64::from(u8::MAX) => Ok(Some(n as u8)),
            Some(n) => Err(format!("\"{key}\" is {n}, which does not fit 8 bits")),
        }
    }

    fn text(&mut self, key: &'static str) -> Result<Option<String>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_str() {
            Some(s) => Ok(Some(s.to_string())),
            None => Err(mismatch(key, "a string", value)),
        }
    }

    fn bool(&mut self, key: &'static str) -> Result<Option<bool>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        match value.as_bool() {
            Some(b) => Ok(Some(b)),
            None => Err(mismatch(key, "true or false", value)),
        }
    }

    /// The flags object, each bit a key, applied over `base` so that naming one bit leaves the rest.
    fn flags(
        &mut self,
        key: &'static str,
        base: DeviceFlags,
    ) -> Result<Option<DeviceFlags>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(object) = value.as_object() else {
            return Err(mismatch(key, "an object of flags", value));
        };
        let mut inner = Reader::new(object, key);
        let mut flags = base;
        if let Some(b) = inner.bool("alt_display")? {
            flags.alt_display = b;
        }
        if let Some(b) = inner.bool("flip")? {
            flags.flip = b;
        }
        if let Some(b) = inner.bool("vibrator")? {
            flags.vibrator = b;
        }
        if let Some(b) = inner.bool("ext_speaker")? {
            flags.ext_speaker = b;
        }
        if let Some(b) = inner.bool("voice_recognition")? {
            flags.voice_recognition = b;
        }
        if let Some(b) = inner.bool("position_location")? {
            flags.position_location = b;
        }
        if let Some(b) = inner.bool("midi")? {
            flags.midi = b;
        }
        if let Some(b) = inner.bool("cmx")? {
            flags.cmx = b;
        }
        if let Some(b) = inner.bool("pen")? {
            flags.pen = b;
        }
        let unknown = inner.leftovers();
        if !unknown.is_empty() {
            let names: Vec<String> = unknown.iter().map(|k| format!("\"{k}\"")).collect();
            return Err(format!("\"{key}\": unknown flag {}", names.join(", ")));
        }
        Ok(Some(flags))
    }

    /// An array of 32-bit numbers, used for the compressed-texture-format enum list.
    fn u32s(&mut self, key: &'static str) -> Result<Option<Vec<u32>>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(items) = value.as_array() else {
            return Err(mismatch(key, "an array of whole numbers", value));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item.as_u64() {
                Some(n) if n <= u64::from(u32::MAX) => out.push(n as u32),
                Some(n) => return Err(format!("\"{key}\": {n} does not fit 32 bits")),
                None => {
                    return Err(format!(
                        "\"{key}\": expected a whole number, found {}",
                        item.kind()
                    ))
                }
            }
        }
        Ok(Some(out))
    }

    /// The keypad: `AVK_*` names, or raw 16-bit codes for the ones no name covers.
    fn keys(&mut self, key: &'static str) -> Result<Option<Vec<u16>>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let Some(items) = value.as_array() else {
            return Err(mismatch(key, "an array of key names or codes", value));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Value::String(name) => match key_code(name) {
                    Some(code) => out.push(code),
                    None => {
                        return Err(format!(
                            "\"{key}\": \"{name}\" is not an `AVK_*` name in the ABI tables"
                        ))
                    }
                },
                other => match other.as_u64() {
                    Some(n) if n <= u64::from(u16::MAX) => out.push(n as u16),
                    Some(n) => {
                        return Err(format!("\"{key}\": {n} does not fit a 16-bit key code"))
                    }
                    None => {
                        return Err(format!(
                            "\"{key}\": a key is a name or a 16-bit code, and this is {}",
                            other.kind()
                        ))
                    }
                },
            }
        }
        Ok(Some(out))
    }

    /// Every key nobody asked for. An underscore is how a person writes a note in a format that has
    /// no comments, so those are skipped.
    fn leftovers(&self) -> Vec<&'a str> {
        self.object
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !k.starts_with('_'))
            .filter(|k| !self.seen.iter().any(|seen| seen == k))
            .collect()
    }
}

fn mismatch(key: &str, wanted: &str, found: &Value) -> String {
    format!("\"{key}\": expected {wanted}, found {}", found.kind())
}

#[cfg(test)]
#[path = "test/device_file.rs"]
mod tests;
