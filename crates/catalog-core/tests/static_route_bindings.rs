use catalog_core::expansion::{expand, Package};
use std::{collections::BTreeMap, fs, path::Path};

fn context() -> Package {
    fn collect(base: &Path, dir: &Path, assets: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            assert!(!entry.file_type().unwrap().is_symlink());
            if entry.file_type().unwrap().is_dir() {
                collect(base, &entry.path(), assets);
            } else {
                assets.insert(
                    entry
                        .path()
                        .strip_prefix(base)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let mut assets = BTreeMap::new();
    collect(&root, &root, &mut assets);
    for name in [
        "activity-feed",
        "tabs",
        "segmented-control",
        "progress-steps",
    ] {
        let key = format!("components/{name}/component.json");
        let mut value: serde_json::Value = serde_json::from_slice(&assets[&key]).unwrap();
        value["status"] = "ready".into();
        assets.insert(key, serde_json::to_vec(&value).unwrap());
    }
    Package::from_assets(&assets).unwrap()
}
fn sources(href: &str) -> Vec<String> {
    let href = serde_json::to_string(href).unwrap();
    vec![
        format!(
            r#"<cui-activity-feed label="History" entries='[{{"title":"Done","occurredAt":"2026-01-01T00:00:00Z","timeLabel":"Today","href":{href}}}]'/>"#
        ),
        format!(
            r#"<cui-tabs label="Pages" items='[{{"label":"Tasks","active":true,"href":{href}}},{{"label":"Other","href":"https://example.org"}}]'/>"#
        ),
        format!(
            r#"<cui-segmented-control label="Scope" items='[{{"kind":"current","label":"Here"}},{{"kind":"link","label":"Tasks","href":{href}}}]'/>"#
        ),
        format!(
            r#"<cui-progress-steps label="Review" items='[{{"state":"complete","label":"Prepared","href":{href}}},{{"state":"current","label":"Review"}}]'/>"#
        ),
    ]
}
#[test]
fn native_json_destinations_use_the_same_closed_named_route_grammar() {
    let package = context();
    for source in sources("{{ routes.tasks() }}") {
        let result = expand(&source, &package).unwrap();
        assert!(result.html.contains("href=\"{{ routes.tasks() }}\""));
    }
    for source in sources("{{ routes.task(task_id=task.id) }}") {
        assert!(expand(&source, &package)
            .unwrap()
            .html
            .contains("routes.task(task_id=task.id)"));
    }
    // Names and Roc argument types are intentionally host-owned, not inferred here.
    for source in sources("{{ routes.not_registered() }}") {
        assert!(expand(&source, &package).is_ok());
    }
}
#[test]
fn json_destinations_reject_unchecked_fields_arbitrary_expressions_and_unsafe_urls() {
    let package = context();
    for bad in [
        "{{ item.url }}",
        "{{ routes.tasks() | safe }}",
        "{{ routes.tasks(item.id) }}",
        "{{ routes.task(id=a.id,id=b.id) }}",
        "{{ routes.task(id=item.id,) }}",
        "javascript:alert(1)",
        "//evil.example",
        "/tasks/{{ item.id }}",
    ] {
        for source in sources(bad) {
            assert!(
                expand(&source, &package).is_err(),
                "accepted {bad}: {source}"
            );
        }
    }
    for source in sources("https://example.org/?x=1&y=2") {
        assert!(expand(&source, &package)
            .unwrap()
            .html
            .contains("href=\"https://example.org/?x=1&amp;y=2\""));
    }
}
