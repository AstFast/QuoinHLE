// Moved out of `src/device.rs` so that file is implementation only. The `#[path]` attribute on
// the `mod tests;` declaration in it is what keeps this a child module of that file, and
// that is what lets these tests reach its private items with `use super::*`.

use super::*;

/// Every field name this module writes, and every bitfield it sets, has to be in the table.
#[test]
fn every_table_field_the_profile_writes_exists() {
    let st = info_struct();
    for name in [
        "cxScreen",
        "cyScreen",
        "cxAltScreen",
        "cyAltScreen",
        "cxScrollBar",
        "wEncoding",
        "wMenuTextScroll",
        "nColorDepth",
        "wMenuImageDelay",
        "dwRAM",
        "dwPromptProps",
        "wKeyCloseApp",
        "wKeyCloseAllApps",
        "dwLang",
        "wStructSize",
        "dwNetLinger",
        "dwSleepDefer",
        "wMaxPath",
        "dwPlatformID",
    ] {
        let _ = field(st, name);
    }
    for (name, _) in DeviceFlags::MEMBERS {
        set_bit(st, &mut vec![0u8; st.size as usize], name);
    }
    for name in [
        "KeySupportType",
        "AEEAutoRepeat",
        "AEEFSFreeSpace",
        "AEEBitmapInfo",
    ] {
        let _ = table(name);
    }
}

/// The byte layout is the one a compiler probe measured (a compiler probe), including
/// the four-byte storage unit the nine flags share.
#[test]
fn device_info_bytes_match_the_measured_layout() {
    let p = DeviceProfile {
        flags: DeviceFlags {
            vibrator: true,
            pen: true,
            ..DeviceFlags::default()
        },
        ..DeviceProfile::standard()
    };
    let b = p.device_info_bytes();
    assert_eq!(b.len(), 64);
    assert_eq!(u16::from_le_bytes([b[0], b[1]]), 240);
    assert_eq!(u16::from_le_bytes([b[2], b[3]]), 320);
    assert_eq!(
        u16::from_le_bytes([b[14], b[15]]),
        16,
        "nColorDepth is the depth of the frame this host actually hands out"
    );
    assert_eq!(u32::from_le_bytes([b[24], b[25], b[26], b[27]]), p.ram);
    // bVibrator is bit 2 and bPen bit 8 of the little-endian word at 28: one unit for all
    // nine flags, so bit 8 shows up in the next byte and the unit still ends at 32.
    assert_eq!(b[28], 0b0000_0100);
    assert_eq!(b[29], 0b0000_0001);
    assert_eq!(&b[30..32], &[0, 0]);
    assert_eq!(u16::from_le_bytes([b[36], b[37]]), g::AVK_END as u16);
    assert_eq!(u16::from_le_bytes([b[44], b[45]]), 64, "wStructSize");
    assert_eq!(u32::from_le_bytes([b[60], b[61], b[62], b[63]]), 0);
}

/// The diagnostic override moves the two advertised words and nothing else — in particular not the
/// depth of the frame, which is what makes a run under it a question about the pack rather than a
/// different handset.
#[test]
fn the_advertised_override_moves_only_the_advertised_words() {
    let base = DeviceProfile::standard();
    let p = base
        .clone()
        .with_advertised(Some(24), Some(g::AEE_ENC_UTF8 as u16));
    let b = p.device_info_bytes();
    assert_eq!(
        u16::from_le_bytes([b[14], b[15]]),
        24,
        "nColorDepth answered what was asked"
    );
    assert_eq!(
        u16::from_le_bytes([b[10], b[11]]),
        g::AEE_ENC_UTF8 as u16,
        "wEncoding answered what was asked"
    );
    // Everything else is the standard handset, byte for byte.
    let mut only_those_two = base.device_info_bytes();
    only_these(&mut only_those_two, 10, 2);
    only_these(&mut only_those_two, 14, 2);
    let mut same = b.clone();
    only_these(&mut same, 10, 2);
    only_these(&mut same, 14, 2);
    assert_eq!(same, only_those_two, "no other field moved");
    // And the frame itself still answers 16 through the bitmap the pack can composite into.
    let bi = p.bitmap_info();
    assert_eq!(
        u32::from_le_bytes([bi[8], bi[9], bi[10], bi[11]]),
        u32::from(crate::display::NATIVE_DEPTH),
        "AEEBitmapInfo::nDepth keeps telling the truth about the screen this host has"
    );
    // Half-answered overrides are the form a caller actually writes.
    let half = DeviceProfile::standard().with_advertised(None, Some(2));
    assert_eq!(half.color_depth, 16, "depth left alone");
    assert_eq!(half.encoding, 2, "encoding moved");
}

fn only_these(b: &mut [u8], at: usize, len: usize) {
    for x in &mut b[at..at + len] {
        *x = 0;
    }
}

/// `wStructSize` is a negotiation, and the slot returns void, so a small struct must be
/// filled and truncated rather than refused.
#[test]
fn a_short_guest_struct_is_filled_and_truncated() {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let p = DeviceProfile::standard();
    let full = mem.alloc(64, 4).unwrap();
    let small = mem.alloc(48, 4).unwrap();
    // Immediately after the 48-byte request, so an overlong copy lands here.
    let guard = mem.alloc(4, 4).unwrap();
    let zeroed = mem.alloc(64, 4).unwrap();
    mem.write_u16(small + 44, 48).unwrap();
    mem.write_u32(guard, 0xdead_beef).unwrap();
    mem.fill(zeroed, 64, 0xa5).unwrap();

    p.write_device_info(&mut mem, full).unwrap();
    p.write_device_info(&mut mem, small).unwrap();
    p.write_device_info(&mut mem, zeroed).unwrap();
    assert_eq!(mem.read_u16(full).unwrap(), 240);
    assert_eq!(mem.read_u16(small).unwrap(), 240);
    assert_eq!(mem.read_u16(small + 44).unwrap(), 64);
    assert_eq!(
        mem.read_u32(guard).unwrap(),
        0xdead_beef,
        "the copy stopped at the guest's declared size"
    );
    // A declared size of zero means "fill the whole struct".
    assert_eq!(mem.read_u16(zeroed + 44).unwrap(), 64);
    // A null pointer can not be answered, and must not fault either.
    p.write_device_info(&mut mem, 0).unwrap();
}

#[test]
fn device_items_follow_the_documented_size_negotiation() {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let p = DeviceProfile::standard();
    let size = mem.alloc(4, 4).unwrap();
    let buf = mem.alloc(64, 4).unwrap();

    // A null buffer is a size query, and still reports success.
    mem.write_u32(size, 0).unwrap();
    let esn = p
        .device_item(&mem, g::AEE_DEVICEITEM_ESN, 0)
        .unwrap()
        .unwrap();
    assert_eq!(esn.len(), 4);
    assert_eq!(
        put_sized_item(&mut mem, &esn, 0, size).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(mem.read_u32(size).unwrap(), 4);

    mem.write_u32(size, 2).unwrap();
    assert_eq!(
        put_sized_item(&mut mem, &esn, buf, size).unwrap(),
        g::AEE_EBUFFERTOOSMALL
    );
    assert_eq!(mem.read_u32(size).unwrap(), 4, "the need is reported back");
    mem.write_u32(size, 8).unwrap();
    assert_eq!(
        put_sized_item(&mut mem, &esn, buf, size).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(mem.read_u32(buf).unwrap(), p.esn);
    assert_eq!(
        put_sized_item(&mut mem, &esn, buf, 0).unwrap(),
        g::AEE_EBADPARM
    );

    // AEEBitmapInfo { cx, cy, nDepth }, three little-endian words.
    let info = p
        .device_item(&mem, g::AEE_DEVICEITEM_DISPINFO1, 0)
        .unwrap()
        .unwrap();
    assert_eq!(info, vec![240, 0, 0, 0, 64, 1, 0, 0, 16, 0, 0, 0]);
    let model = p
        .device_item(&mem, g::AEE_DEVICEITEM_MODEL_NAME, 0)
        .unwrap()
        .unwrap();
    assert_eq!(model.len(), (p.model_name.len() + 1) * 2);
    assert_eq!(&model[..4], b"Q\0u\0");
    let imei = p
        .device_item(&mem, g::AEE_DEVICEITEM_IMEI, 0)
        .unwrap()
        .unwrap();
    assert_eq!(imei.len(), 16);
    assert_eq!(imei[15], 0);
    assert!(p
        .device_item(&mem, g::AEE_DEVICEITEM_BKEY, 0)
        .unwrap()
        .is_none());

    // The carrier's subscriber number, which is not in the generated table and so has no `g::` path. The
    // pack that found it (`w62ca_new-ex-mahjong`) asks only the null-buffer question — "how many
    // bytes?" — while initialising its Carrier system, and treated `EUNSUPPORTED` as "no handset":
    // it printed `INITKDDISYSTEM` and returned from `AEEClsCreateInstance` without an applet.
    let number = p
        .device_item(
            &mem,
            crate::device::AEE_DEVICEITEM_CARRIER_KDDI_SUBSCRIBER_NO,
            0,
        )
        .unwrap()
        .unwrap();
    assert_eq!(number, b"09012345678 ".to_vec());

    // That null-buffer form, through the door the guest actually uses: the answer is a byte count and
    // `SUCCESS`, not a buffer.
    let size = mem.alloc(4, 4).unwrap();
    mem.write_u32(size, 0).unwrap();
    assert_eq!(
        put_sized_item(&mut mem, &number, 0, size).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(mem.read_u32(size).unwrap(), 12);
}

/// Key support is an in/out structure: the caller names the key, we answer for that key.
#[test]
fn key_support_answers_the_key_the_guest_asked_about() {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let p = DeviceProfile::standard();
    let st = table("KeySupportType");
    let supported = field(st, "supported");
    let buf = mem.alloc(st.size, 4).unwrap();
    let size = mem.alloc(4, 4).unwrap();
    mem.fill(buf, st.size, 0x5a).unwrap();

    let mut ask = |key: u16| -> (u16, u8) {
        mem.write_u16(buf, key as u32).unwrap();
        mem.write_u32(size, st.size).unwrap();
        let bytes = p
            .device_item(&mem, g::AEE_DEVICEITEM_KEY_SUPPORT, buf)
            .unwrap()
            .unwrap();
        assert_eq!(bytes.len(), st.size as usize);
        put_sized_item(&mut mem, &bytes, buf, size).unwrap();
        (
            mem.read_u16(buf).unwrap() as u16,
            mem.read_u8(buf + supported as u32).unwrap(),
        )
    };
    assert_eq!(ask(g::AVK_SELECT as u16), (g::AVK_SELECT as u16, 1));
    // An unsupported code: the same layout, false, and the caller's key echoed back.
    assert_eq!(ask(0x1234), (0x1234, 0));
}

#[test]
fn rgbval_is_not_the_usual_rgb() {
    assert_eq!(make_rgb(0x40, 0x30, 0x50), 0x5030_4000);
    assert_eq!(split_rgb(make_rgb(1, 2, 3)), (1, 2, 3));
    assert_eq!(RGB_WHITE, 0xffff_ff00);
    assert_eq!(RGB_BLACK, 0);
    assert_ne!(RGB_WHITE, RGB_NONE);
    assert_eq!(split_rgb(RGB_MASK_COLOR), (255, 0, 255));
}
