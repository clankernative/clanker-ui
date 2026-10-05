#[path = "../src/native_ports.rs"]
mod native_ports;

use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn six_draft_ports_render_their_locked_geometry_fixture_goldens() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla/components");
    let icons: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
    let icon_fragment = std::fs::read_to_string(root.join("icon/fragment.html")).unwrap();
    let fixtures = [
        ("modal", "small-golden.json"),
        ("modal", "standard-golden.json"),
        ("modal", "wide-golden.json"),
        ("drawer", "start-narrow-golden.json"),
        ("drawer", "end-standard-golden.json"),
        ("drawer", "start-wide-golden.json"),
        ("popover", "start-narrow-golden.json"),
        ("popover", "end-standard-golden.json"),
        ("popover", "start-standard-golden.json"),
        ("popover", "end-wide-golden.json"),
        ("popover", "start-wide-golden.json"),
        ("tooltip", "top-golden.json"),
        ("tooltip", "right-golden.json"),
        ("tooltip", "bottom-golden.json"),
        ("tooltip", "left-golden.json"),
        ("toast", "inline-golden.json"),
        ("toast", "top-center-golden.json"),
        ("toast", "top-end-golden.json"),
        ("toast", "bottom-center-golden.json"),
        ("toast", "bottom-end-golden.json"),
        ("toast", "preserve-golden.json"),
        ("toast", "remove-golden.json"),
        ("theme-switcher", "text-golden.json"),
        ("theme-switcher", "icons-golden.json"),
        ("date-calendar", "typical-golden.json"),
        ("date-picker", "single-golden.json"),
        ("date-picker", "range-golden.json"),
        ("command-menu", "typical-golden.json"),
        ("command-menu", "dense-golden.json"),
        ("confirm-dialog", "neutral-golden.json"),
        ("confirm-dialog", "danger-golden.json"),
        ("file-upload", "typical-golden.json"),
        ("file-upload", "validation-golden.json"),
        ("data-viewport", "paged-golden.json"),
        ("data-viewport", "windowed-golden.json"),
    ];

    for (name, fixture_name) in fixtures {
        let dir = root.join(name);
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(dir.join("fixtures").join(fixture_name)).unwrap(),
        )
        .unwrap();
        let template = std::fs::read_to_string(dir.join("fragment.html")).unwrap();
        native_ports::render_with_dependencies(
            name,
            &fixture,
            &template,
            &icons,
            &icon_fragment,
            Some(include_str!(
                "../../../packages/vanilla/components/date-calendar/fragment.html"
            )),
        )
        .unwrap_or_else(|error| panic!("{name}/{fixture_name}: {error}"));
    }
}

#[test]
fn viewport_fixture_navigation_is_a_single_admitted_child_not_a_property() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla/components");
    let dir = root.join("data-viewport");
    let mut fixture: Value =
        serde_json::from_slice(&std::fs::read(dir.join("fixtures/paged.json")).unwrap()).unwrap();
    fixture["children"] = serde_json::json!({"navigation":"<a href=\"/next\">Next page</a>"});
    let template = std::fs::read_to_string(dir.join("fragment.html")).unwrap();
    let icons: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
    let icon_fragment = std::fs::read_to_string(root.join("icon/fragment.html")).unwrap();
    let (html, _) =
        native_ports::render("data-viewport", &fixture, &template, &icons, &icon_fragment).unwrap();
    assert!(html.contains("href=\"/next\""));
    fixture["children"]["extra"] = serde_json::json!("<a href=\"/other\">Other</a>");
    assert!(
        native_ports::render("data-viewport", &fixture, &template, &icons, &icon_fragment).is_err()
    );
}
