//! Golden coverage for the four provisional static component ports.
#[path = "../src/activity_feed.rs"]
mod activity_feed;
#[path = "../src/button.rs"]
mod button;
#[path = "../src/button_group.rs"]
mod button_group;
#[path = "../src/definition_list.rs"]
mod definition_list;
#[path = "../src/disclosure.rs"]
mod disclosure;
#[path = "../src/fragment.rs"]
mod fragment;
#[allow(dead_code)]
#[path = "../src/layout.rs"]
mod layout;
use serde_json::Value;
use std::{fs, path::PathBuf};
fn fixture(component: &str, name: &str) -> (PathBuf, Value, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let relative = format!("components/{component}/fixtures/{name}.json");
    let path = root.join(&relative);
    let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let expected = v
        .as_object_mut()
        .unwrap()
        .remove("expectedHtml")
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    (root, v, expected)
}
fn fragment(root: &std::path::Path, component: &str) -> String {
    fs::read_to_string(root.join(format!("components/{component}/fragment.html"))).unwrap()
}
#[test]
fn all_declared_fixtures_match_exact_fragment_output() {
    for name in ["typical", "compact-plain"] {
        let (root, v, e) = fixture("definition-list", name);
        let f = fragment(&root, "definition-list");
        let h = definition_list::render(&serde_json::from_value(v).unwrap(), &f).unwrap();
        assert_eq!(h, e, "definition-list/{name}");
    }
    for name in ["typical", "compact"] {
        let (root, v, e) = fixture("activity-feed", name);
        let f = fragment(&root, "activity-feed");
        let h = activity_feed::render(&serde_json::from_value(v).unwrap(), &f).unwrap();
        assert_eq!(h, e, "activity-feed/{name}");
    }
    for name in ["contained", "plain-open"] {
        let (root, mut v, e) = fixture("disclosure", name);
        let children = v
            .as_object_mut()
            .unwrap()
            .remove("children")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let i: disclosure::DisclosureInstance = serde_json::from_value(v).unwrap();
        let body = layout::AdmittedChildren::from_host_admitted(children);
        let h = disclosure::render_with_body(&i, &body, &fragment(&root, "disclosure")).unwrap();
        assert_eq!(h, e, "disclosure/{name}");
    }
    for name in ["typical", "spaced-full-width"] {
        let (root, mut v, e) = fixture("button-group", name);
        let children = v
            .as_object_mut()
            .unwrap()
            .remove("children")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| layout::AdmittedChildren::from_host_admitted(v.as_str().unwrap()))
            .collect::<Vec<_>>();
        let i: button_group::ButtonGroupInstance = serde_json::from_value(v).unwrap();
        let h = button_group::render(&i, &children, &fragment(&root, "button-group")).unwrap();
        assert_eq!(h, e, "button-group/{name}");
    }
}
