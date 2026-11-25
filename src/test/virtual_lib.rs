use std::path::{Path, PathBuf};

use super::*;

#[test]
fn the_command_line_declares_one_library_per_name() {
    let mut plan = LibraryPlan::default();
    plan.add_virtual_class("kddi,0x01009999,IImage").unwrap();
    plan.add_virtual_class("kddi,0x0100999a,IImage").unwrap();
    assert_eq!(plan.virtual_libs.len(), 1, "one NAME is one library");
    let lib = &plan.virtual_libs[0];
    assert_eq!(lib.name, "kddi");
    assert_eq!(lib.iface_for(0x0100_9999), Some("IImage"));
    assert_eq!(lib.iface_for(0x0100_999a), Some("IImage"));
}

#[test]
fn a_class_served_as_two_interfaces_is_refused() {
    let mut plan = LibraryPlan::default();
    plan.add_virtual_class("lib,0x01009999,IImage").unwrap();
    let error = plan
        .add_virtual_class("lib,0x01009999,IWebUtil")
        .unwrap_err();
    assert!(error.contains("both"), "{error}");
}

#[test]
fn a_manifest_reads_paths_libs_and_virtual_classes() {
    let text = r#"{
        "lib_paths": ["libs"],
        "libs": ["ipremium.mod"],
        "virtual_libs": {
            "kddi": [{"class": "0x01009999", "iface": "IImage"}]
        }
    }"#;
    let plan = LibraryPlan::from_json(text, Path::new("run/manifest.json")).unwrap();
    assert!(plan.search_paths.contains(&PathBuf::from("run")));
    assert!(plan.search_paths.contains(&PathBuf::from("run/libs")));
    assert_eq!(plan.libs, vec!["ipremium.mod"]);
    assert_eq!(plan.virtual_libs[0].iface_for(0x0100_9999), Some("IImage"));
}

#[test]
fn a_manifest_refuses_an_unknown_interface_and_field() {
    let text = r#"{"virtual_libs": {"lib": [{"class": 1, "iface": "INoSuchThing"}]}}"#;
    let error = LibraryPlan::from_json(text, Path::new("m.json")).unwrap_err();
    assert!(error.contains("INoSuchThing"), "{error}");

    let text = r#"{"libs": [], "lib_path": ["x"]}"#;
    let error = LibraryPlan::from_json(text, Path::new("m.json")).unwrap_err();
    assert!(error.contains("lib_path"), "{error}");
}

#[test]
fn a_library_name_resolves_against_the_search_paths() {
    let dir = std::env::temp_dir().join(format!("quoin-vlib-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ipremium.mod");
    std::fs::write(&file, b"not an image").unwrap();

    let plan = LibraryPlan {
        search_paths: vec![dir.clone()],
        libs: vec!["ipremium.mod".into()],
        virtual_libs: Vec::new(),
    };
    assert_eq!(plan.resolve("ipremium.mod"), Some(file.clone()));
    assert_eq!(plan.resolve("missing.mod"), None);
    assert_eq!(plan.resolved_libs().unwrap().len(), 1);

    std::fs::remove_file(&file).ok();
    std::fs::remove_dir(&dir).ok();
}
