//! The carrier extensions: the two `CreateInstance` classes, their slot tables, and the
//! answers this host can honestly give for a handset it is not.
//!
//! Every test here is sample-free. The slot order is the one the reference header declares, which is
//! what a pack compiled against that header calls; the reference module's own table is a revision
//! older and is deliberately not what `IFACES` publishes.

use super::*;
use crate::abi::jp;

fn call(rt: &mut Runtime, id: u32, this: u32, args: &[u32]) -> Result<u32, Error> {
    rt.cpu.r[0] = this;
    for i in 0..3 {
        rt.cpu.r[1 + i] = args.get(i).copied().unwrap_or(0);
    }
    rt.dispatch(id, 0)
}

fn create(rt: &mut Runtime, cls: u32) -> u32 {
    let id = abi::method_of("IShell", "CreateInstance").unwrap();
    let shell = rt.shell;
    let slot = rt.cpu.mem.alloc(4, 4).unwrap();
    assert_eq!(call(rt, id, shell, &[cls, slot]).unwrap(), g::AEE_SUCCESS);
    let object = rt.cpu.mem.read_u32(slot).unwrap();
    assert_ne!(object, 0);
    object
}

fn out_buffer(rt: &mut Runtime, len: u32) -> u32 {
    rt.cpu.mem.alloc(len, 4).unwrap()
}

#[test]
fn the_private_interfaces_publish_the_slots_the_header_declares() {
    let phone = abi::iface("IKDDIPHONESYSTEM").expect("IKDDIPHONESYSTEM is in the tables");
    assert_eq!(phone.base, "IQI");
    assert_eq!(phone.methods.len(), 16);
    assert_eq!(
        phone.methods.iter().map(|m| m.name).collect::<Vec<_>>(),
        vec![
            "AddRef",
            "Release",
            "QueryInterface",
            "GetCallInfo",
            "GetChargeInfo",
            "GetHandSetType",
            "GetHistory",
            "GetIntensity",
            "GetMannerMode",
            "GetMaxVolumeLevel",
            "GetMyProfile",
            "GetNumberOfHistories",
            "GetPowerSupplyLevel",
            "GetSubscriberID",
            "GetVibrationStatus",
            "GetVolumeLevel",
        ]
    );
    let ppp = abi::iface("IKDDIPPP").expect("IKDDIPPP is in the tables");
    assert_eq!(ppp.base, "IQI");
    assert_eq!(
        ppp.methods.iter().map(|m| m.name).collect::<Vec<_>>(),
        vec!["AddRef", "Release", "QueryInterface", "SetInfo"]
    );
    assert_eq!(abi::slot_of("IKDDIPPP", "SetInfo"), Some(3));
    assert_eq!(
        abi::method_of("IKDDIPHONESYSTEM", "GetVolumeLevel").map(abi::slot_of_method_id),
        Some(15)
    );
}

#[test]
fn create_instance_builds_a_fresh_counted_object_for_each_extension() {
    let mut rt = Runtime::new();
    let phone = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    let ppp = create(&mut rt, jp::AEECLSID_KDDIPPP);
    assert_ne!(phone, ppp);
    assert_eq!(rt.hle.object(phone).unwrap().iface, "IKDDIPHONESYSTEM");
    assert_eq!(
        rt.hle.object(phone).unwrap().clsid,
        jp::AEECLSID_KDDIPHONESYSTEM
    );
    assert_eq!(rt.hle.object(ppp).unwrap().iface, "IKDDIPPP");
    assert_eq!(rt.hle.object(ppp).unwrap().clsid, jp::AEECLSID_KDDIPPP);

    // The reference body allocates per call, so a second request is a second object.
    let again = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    assert_ne!(again, phone);

    let addref = abi::method_of("IKDDIPPP", "AddRef").unwrap();
    let release = abi::method_of("IKDDIPPP", "Release").unwrap();
    assert_eq!(call(&mut rt, addref, ppp, &[]).unwrap(), 2);
    assert_eq!(call(&mut rt, release, ppp, &[]).unwrap(), 1);
    assert_eq!(call(&mut rt, release, ppp, &[]).unwrap(), 0);
}

#[test]
fn query_interface_answers_the_class_the_base_iid_and_nothing_else() {
    let mut rt = Runtime::new();
    let phone = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    let qi = abi::method_of("IKDDIPHONESYSTEM", "QueryInterface").unwrap();
    let slot = rt.cpu.mem.alloc(4, 4).unwrap();

    assert_eq!(
        call(&mut rt, qi, phone, &[jp::AEECLSID_KDDIPHONESYSTEM, slot]).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(rt.cpu.mem.read_u32(slot).unwrap(), phone);
    // `answer_qi` hands back the same address with a reference, which is the delivery body's
    // `*ppobj = self; AddRef`.
    let held = *rt.hle.object(phone).unwrap();
    assert_eq!(rt.hle.refcount(&rt.cpu.mem, &held).unwrap(), 2);
    assert_eq!(
        call(&mut rt, qi, phone, &[g::AEEIID_IQI, slot]).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(rt.cpu.mem.read_u32(slot).unwrap(), phone);
    assert_eq!(
        call(&mut rt, qi, phone, &[0xdead_beef, slot]).unwrap(),
        g::AEE_ECLASSNOTSUPPORT
    );
    assert_eq!(rt.cpu.mem.read_u32(slot).unwrap(), 0);
    assert_eq!(
        call(&mut rt, qi, phone, &[jp::AEECLSID_KDDIPHONESYSTEM, 0]).unwrap(),
        g::AEE_EBADPARM
    );

    let ppp = create(&mut rt, jp::AEECLSID_KDDIPPP);
    let qi = abi::method_of("IKDDIPPP", "QueryInterface").unwrap();
    assert_eq!(
        call(&mut rt, qi, ppp, &[jp::AEECLSID_KDDIPPP, slot]).unwrap(),
        g::AEE_SUCCESS
    );
    assert_eq!(rt.cpu.mem.read_u32(slot).unwrap(), ppp);
    assert_eq!(
        call(&mut rt, qi, ppp, &[jp::AEECLSID_KDDIPHONESYSTEM, slot]).unwrap(),
        g::AEE_ECLASSNOTSUPPORT
    );
}

#[test]
fn the_phone_system_answers_only_what_a_host_can_know() {
    let mut rt = Runtime::new();
    let phone = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    let call_name = |rt: &mut Runtime, name: &str, args: &[u32]| {
        let id = abi::method_of("IKDDIPHONESYSTEM", name).unwrap();
        call(rt, id, phone, args).unwrap()
    };
    let buf = out_buffer(&mut rt, 64);

    // No line, so no minutes, no charge and no history. Empty history is a count of zero, not a
    // refusal; an out-of-range selector is the header's `KDDI_EBADPARAM`.
    assert_eq!(call_name(&mut rt, "GetCallInfo", &[0]), 0);
    assert_eq!(call_name(&mut rt, "GetCallInfo", &[2]), jp::KDDI_EBADPARAM);
    assert_eq!(call_name(&mut rt, "GetChargeInfo", &[1]), 0);
    assert_eq!(
        call_name(&mut rt, "GetChargeInfo", &[2]),
        jp::KDDI_EBADPARAM
    );
    assert_eq!(call_name(&mut rt, "GetHistory", &[0, buf, 1]), 0);
    assert_eq!(
        call_name(&mut rt, "GetHistory", &[2, buf, 1]),
        jp::KDDI_EBADPARAM
    );
    assert_eq!(
        call_name(&mut rt, "GetHistory", &[0, 0, 1]),
        jp::KDDI_EBADPARAM
    );
    assert_eq!(call_name(&mut rt, "GetNumberOfHistories", &[0]), 0);
    assert_eq!(
        call_name(&mut rt, "GetNumberOfHistories", &[2]),
        jp::KDDI_EBADPARAM
    );

    // No radio and no manner mode: both constants are valid readings of that state.
    assert_eq!(
        call_name(&mut rt, "GetIntensity", &[]),
        jp::KDDI_INTENSITY_NONE
    );
    assert_eq!(
        call_name(&mut rt, "GetMannerMode", &[]),
        jp::KDDI_MANNERMODE_OFF
    );

    // Identity, profile, battery, vibrator and volume are device records this host does not have;
    // the header's own failure code for each is a refusal a pack can tell from a value.
    assert_eq!(
        call_name(&mut rt, "GetHandSetType", &[buf]),
        jp::KDDI_EFAILED
    );
    assert_eq!(
        call_name(&mut rt, "GetSubscriberID", &[buf]),
        jp::KDDI_EFAILED
    );
    assert_eq!(
        call_name(&mut rt, "GetMyProfile", &[0, buf]),
        jp::KDDI_EUNSUPPORTED
    );
    assert_eq!(
        call_name(&mut rt, "GetMyProfile", &[jp::KDDI_ADDR_FIELD_MAX, buf]),
        jp::KDDI_EBADPARAM
    );
    assert_eq!(
        call_name(&mut rt, "GetMyProfile", &[0, 0]),
        jp::KDDI_EBADPARAM
    );
    assert_eq!(
        call_name(&mut rt, "GetMaxVolumeLevel", &[]),
        jp::KDDI_EUNSUPPORTED
    );
    assert_eq!(
        call_name(&mut rt, "GetPowerSupplyLevel", &[]),
        jp::KDDI_EUNSUPPORTED
    );
    // The default profile has a vibrator, so the support query is ON, not a refusal.
    assert_eq!(
        call_name(&mut rt, "GetVibrationStatus", &[jp::KDDI_VIBRATOR_ONOFF]),
        jp::KDDI_VIBRATION_ON
    );
    assert_eq!(
        call_name(&mut rt, "GetVolumeLevel", &[0]),
        jp::KDDI_EUNSUPPORTED
    );
    assert_eq!(
        call_name(&mut rt, "GetVolumeLevel", &[jp::KDDI_NOTIFICATION_TYPE_MAX]),
        jp::KDDI_EBADPARAM
    );
}

#[test]
fn the_phone_system_reads_the_device_profile_for_manner_mode_and_vibration() {
    let mode = |rt: &mut Runtime, phone: u32, name: &str, args: &[u32]| {
        let id = abi::method_of("IKDDIPHONESYSTEM", name).unwrap();
        call(rt, id, phone, args).unwrap()
    };

    // A profile without a vibrator and with a manner mode set: the two answers move together with
    // `AEEDeviceInfo`, which is the same machine's state seen through another door.
    let mut rt = Runtime::new();
    rt.device.flags.vibrator = false;
    rt.device.manner_mode = g::AEE_MANNER_MODE_SILENT;
    let phone = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    assert_eq!(
        mode(&mut rt, phone, "GetMannerMode", &[]),
        jp::KDDI_MANNERMODE_ON
    );
    assert_eq!(
        mode(
            &mut rt,
            phone,
            "GetVibrationStatus",
            &[jp::KDDI_VIBRATOR_ONOFF]
        ),
        jp::KDDI_EUNSUPPORTED
    );
    assert_eq!(
        mode(
            &mut rt,
            phone,
            "GetVibrationStatus",
            &[jp::KDDI_VIBRATOR_PATERN]
        ),
        jp::KDDI_EUNSUPPORTED
    );

    let mut rt = Runtime::new();
    rt.device.manner_mode = g::AEE_MANNER_MODE_VIBRATE;
    let phone = create(&mut rt, jp::AEECLSID_KDDIPHONESYSTEM);
    assert_eq!(
        mode(&mut rt, phone, "GetMannerMode", &[]),
        jp::KDDI_MANNERMODE_ON
    );
    assert_eq!(
        mode(
            &mut rt,
            phone,
            "GetVibrationStatus",
            &[jp::KDDI_VIBRATOR_ONOFF]
        ),
        jp::KDDI_VIBRATION_ON
    );
    assert_eq!(
        mode(
            &mut rt,
            phone,
            "GetVibrationStatus",
            &[jp::KDDI_VIBRATOR_PATERN]
        ),
        jp::KDDI_VIBRATION_DEFAULT_PATTERN
    );
    assert_eq!(
        mode(
            &mut rt,
            phone,
            "GetVibrationStatus",
            &[jp::KDDI_VIBRATOR_ATTRIBUTE_MAX]
        ),
        jp::KDDI_EUNSUPPORTED
    );
}

#[test]
fn the_ppp_extension_takes_a_pair_of_strings_and_refuses_missing_ones() {
    let mut rt = Runtime::new();
    let ppp = create(&mut rt, jp::AEECLSID_KDDIPPP);
    let set = abi::method_of("IKDDIPPP", "SetInfo").unwrap();
    let id = out_buffer(&mut rt, 16);
    let passwd = out_buffer(&mut rt, 16);
    rt.cpu.mem.write_bytes(id, b"kddi-id\0").unwrap();
    rt.cpu.mem.write_bytes(passwd, b"secret\0").unwrap();

    assert_eq!(
        call(&mut rt, set, ppp, &[id, passwd]).unwrap(),
        jp::KDDI_SUCCESS
    );
    assert_eq!(
        call(&mut rt, set, ppp, &[0, passwd]).unwrap(),
        jp::KDDI_EFAILED
    );
    assert_eq!(call(&mut rt, set, ppp, &[id, 0]).unwrap(), jp::KDDI_EFAILED);
}

#[test]
fn the_private_table_matches_the_compiled_layout_and_the_header_values() {
    assert_eq!(jp::AEECLSID_KDDIPHONESYSTEM, 0x0101_19f1);
    assert_eq!(jp::AEECLSID_KDDIPPP, 0x0101_0e5b);
    assert_eq!(jp::EVT_PPP_CHANGED, 500);
    assert_eq!(jp::KDDI_EFAILED, 0xffff_ffff);
    assert_eq!(jp::KDDI_EBADPARAM, 0xffff_fffe);
    assert_eq!(jp::KDDI_EUNSUPPORTED, 0xffff_fffd);
    assert_eq!(jp::KDDI_PHONENUMBER_MAX_LENGTH, 16);
    assert_eq!(jp::SHIM_ABK_ITEM_LOCATION, 0x0000_8000);

    let field = |name: &str, field: &str| {
        let st = jp::JP_STRUCTS
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} is not in JP_STRUCTS"));
        let f = st
            .fields
            .iter()
            .find(|f| f.name == field)
            .unwrap_or_else(|| panic!("{name}.{field} is not in JP_STRUCTS"));
        (f.off, f.size)
    };
    let size = |name: &str| jp::JP_STRUCTS.iter().find(|s| s.name == name).unwrap().size;

    // The record is `gcc -m32` reading the same headers: 32-bit pointers, `boolean` is a byte,
    // `JulianType` is seven uint16, and both structures round up to their own alignment.
    assert_eq!(size("KDDIHistory"), 40);
    assert_eq!(field("KDDIHistory", "pszPhoneNumber"), (0, 17));
    assert_eq!(field("KDDIHistory", "History"), (18, 14));
    assert_eq!(field("KDDIHistory", "IncallType"), (32, 4));
    assert_eq!(field("KDDIHistory", "bResponse"), (36, 1));

    assert_eq!(size("ShimAddrbookInfo"), 852);
    assert_eq!(field("ShimAddrbookInfo", "fields"), (16, 4));
    assert_eq!(field("ShimAddrbookInfo", "name"), (20, 25));
    assert_eq!(field("ShimAddrbookInfo", "group"), (274, 2));
    assert_eq!(field("ShimAddrbookInfo", "url"), (391, 257));
    assert_eq!(field("ShimAddrbookInfo", "location"), (782, 68));

    assert_eq!(size("ShimBrowserInfo"), 1036);
    assert_eq!(field("ShimBrowserInfo", "url"), (10, 1024));

    assert_eq!(size("ShimMUAInfo"), 32);
    assert_eq!(field("ShimMUAInfo", "to_list"), (12, 4));
    assert_eq!(field("ShimMUAInfo", "body"), (28, 4));
}

/// `AEECLSID_IUNICODE_SJIS` is the carrier class a J2B environment asks for at startup; a refusal is
/// a null store inside its own environment, not a quiet gap. The two conversions are the codec
/// [`crate::resource`] already runs for the `FD FE`-marked container strings, so the test pins the
/// bytes for two characters rather than only round-tripping the host against itself.
#[test]
fn the_unicode_sjis_extension_converts_both_ways_and_refuses_a_short_buffer() {
    let mut rt = Runtime::new();
    let obj = create(&mut rt, abi::AEECLSID_IUNICODE_SJIS);
    assert_eq!(rt.hle.object(obj).unwrap().iface, "IUnicodeSjis");
    let to_sjis = abi::method_id(abi::UNICODESJIS_IFACE, abi::UNICODESJIS_UCS2TOSJIS);
    let from_sjis = abi::method_id(abi::UNICODESJIS_IFACE, abi::UNICODESJIS_SJISTOUCS2);
    // "あア" (U+3042 U+30A2), NUL-terminated.
    let src = out_buffer(&mut rt, 8);
    rt.cpu.mem.write_u16(src, 0x3042).unwrap();
    rt.cpu.mem.write_u16(src + 2, 0x30a2).unwrap();
    rt.cpu.mem.write_u16(src + 4, 0).unwrap();
    let dst = out_buffer(&mut rt, 16);
    assert_eq!(
        call(&mut rt, to_sjis, obj, &[src, dst, 16]).unwrap(),
        g::TRUE
    );
    assert_eq!(
        rt.cpu.mem.read_cstr(dst, 16).unwrap(),
        vec![0x82, 0xa0, 0x83, 0x41],
        "あ is 82 A0 and ア is 83 41 in Shift-JIS"
    );
    // Four bytes plus the terminator do not fit in four.
    let short = out_buffer(&mut rt, 4);
    assert_eq!(
        call(&mut rt, to_sjis, obj, &[src, short, 4]).unwrap(),
        g::FALSE
    );
    // And back, including the terminator.
    let back = out_buffer(&mut rt, 8);
    assert_eq!(
        call(&mut rt, from_sjis, obj, &[dst, back, 8]).unwrap(),
        g::TRUE
    );
    assert_eq!(rt.cpu.mem.read_u16(back).unwrap(), 0x3042);
    assert_eq!(rt.cpu.mem.read_u16(back + 2).unwrap(), 0x30a2);
    assert_eq!(rt.cpu.mem.read_u16(back + 4).unwrap(), 0);
    // The i-mode slot has no mapping here; the header's failure answer is the input.
    let imode = abi::method_id(abi::UNICODESJIS_IFACE, abi::UNICODESJIS_IMODESJISTOSJIS);
    assert_eq!(call(&mut rt, imode, obj, &[0x1234]).unwrap(), 0x1234);
}
