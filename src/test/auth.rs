use std::path::Path;

use super::*;

#[test]
fn a_profile_defaults_to_deny_and_names_its_allowed_titles() {
    let text = r#"{
        "subscriber_number": "09012345678",
        "user_id": "kddi-test",
        "apps": [
            {"id": "tokumori", "mode": "authorized", "note": "owned"},
            {"id": "valkyrie4", "mode": "trial"}
        ]
    }"#;
    let profile = Profile::from_json(text, Path::new("auth.json")).unwrap();
    assert_eq!(profile.default, Decision::Deny);
    assert_eq!(profile.subscriber_number.as_deref(), Some("09012345678"));
    assert_eq!(profile.user_id.as_deref(), Some("kddi-test"));
    assert_eq!(profile.decision_for("tokumori"), Decision::Authorized);
    assert_eq!(profile.decision_for("valkyrie4"), Decision::Trial);
    assert_eq!(profile.decision_for("unlisted"), Decision::Deny);
    assert!(profile.decision_for("tokumori").allows());
    assert!(!profile.decision_for("valkyrie4").allows());
}

#[test]
fn an_unknown_field_or_mode_is_refused_with_the_file_name() {
    let text = r#"{"apps": [], "subscriber": {}}"#;
    let error = Profile::from_json(text, Path::new("m.json")).unwrap_err();
    assert!(error.contains("subscriber"), "{error}");

    let text = r#"{"apps": [{"id": "x", "mode": "always"}]}"#;
    let error = Profile::from_json(text, Path::new("m.json")).unwrap_err();
    assert!(error.contains("always"), "{error}");

    let text = r#"{"apps": [{"id": "x"}, {"id": "x"}]}"#;
    let error = Profile::from_json(text, Path::new("m.json")).unwrap_err();
    assert!(error.contains("twice"), "{error}");
}

#[test]
fn a_default_line_can_be_set_but_trial_still_does_not_allow() {
    let text = r#"{"default": "authorized"}"#;
    let profile = Profile::from_json(text, Path::new("m.json")).unwrap();
    assert_eq!(profile.default, Decision::Authorized);
    assert!(profile.decision_for("anything").allows());
}
