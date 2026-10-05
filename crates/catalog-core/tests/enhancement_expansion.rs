use catalog_core::expansion::{self, Package};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn package() -> Package {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let mut assets = BTreeMap::new();
    fn collect(
        root: &std::path::Path,
        directory: &std::path::Path,
        assets: &mut BTreeMap<String, Vec<u8>>,
    ) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, assets);
            } else {
                assets.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    collect(&root, &root, &mut assets);
    for name in ["copy-field", "theme-switcher", "tooltip", "toast"] {
        let key = format!("components/{name}/component.json");
        let mut manifest: Value = serde_json::from_slice(&assets[&key]).unwrap();
        manifest["status"] = "ready".into();
        assets.insert(key, serde_json::to_vec(&manifest).unwrap());
    }
    Package::from_assets(&assets).unwrap()
}

#[test]
fn enhancements_expand_typed_literal_configuration_and_no_js_fallbacks() {
    let p = package();
    for (source, component, expected) in [
        ("<cui-copy-field id=\"copy-code\" name=\"code\" label=\"Reference\" value=\"TASK-<42>\" />", "copy-field", "readonly spellcheck=\"false\""),
        ("<cui-tooltip id=\"help-code\" trigger-label=\"About the reference\" title=\"App-owned reference\" arrow=\"true\" />", "tooltip", "role=\"tooltip\""),
        ("<cui-theme-switcher label=\"Theme\" choices='[{\"value\":\"light\",\"label\":\"Light\",\"icon\":\"sun\"},{\"value\":\"dark\",\"label\":\"Dark\",\"icon\":\"moon\"}]' default-choice=\"dark\" system-light-theme=\"light\" system-dark-theme=\"dark\" storage-key=\"null\" />", "theme-switcher", "interactive choices require JavaScript"),
        ("<cui-toast label=\"Illustrative notices\" position=\"inline\" messages='[{\"title\":\"Illustration\",\"body\":\"Not a command result\",\"tone\":\"info\"}]' />", "toast", "Not a command result"),
    ] {
        let result = expansion::expand(source, &p).unwrap();
        assert!(result.used_components.contains(component));
        assert!(result.html.contains(expected), "{}", result.html);
        assert!(!result.html.contains("<script"));
        assert!(result.bindings.is_empty());
    }
    let result = expansion::expand(
        "<cui-copy-field name=\"code\" label=\"Reference\" value=\"TASK-<42>\" />",
        &p,
    )
    .unwrap();
    assert!(result.html.contains("value=\"TASK-&lt;42&gt;\""));
}

#[test]
fn enhancements_reject_unknown_options_children_malformed_data_and_template_expressions() {
    let p = package();
    for source in [
        "<cui-copy-field name=\"x\" label=\"X\" value=\"v\" command=\"save\" />",
        "<cui-copy-field name=\"x\" label=\"X\" value=\"{{secret}}\" />",
        "<cui-copy-field name=\"x\" label=\"X\" value=\"{% include 'secret' %}\" />",
        "<cui-tooltip id=\"help\" trigger-label=\"Help\" title=\"Help\" arrow=\"yes\" />",
        "<cui-tooltip id=\"help\" trigger-label=\"Help\" title=\"Help\">Unsupported child</cui-tooltip>",
        "<cui-toast label=\"Notices\" messages=\"invalid JSON\" />",
        "<cui-toast label=\"Notices\" messages='[{\"title\":\"{{secret}}\",\"body\":\"No\",\"tone\":\"info\"}]' />",
        "<cui-toast label=\"Notices\" messages='[{\"title\":\"No\",\"body\":\"No\",\"tone\":\"info\",\"command\":\"save\"}]' />",
    ] {
        assert!(expansion::expand(source, &p).is_err(), "accepted {source}");
    }
}

#[test]
fn enhancements_emit_stable_accessibility_ids_for_host_admission() {
    let p = package();
    let html = expansion::expand("<cui-copy-field name=\"copy\" label=\"Copy\" value=\"value\" hint=\"Select the reference\" /><cui-tooltip id=\"help\" trigger-label=\"Help\" title=\"Help\" />", &p).unwrap().html;
    for id in ["copy", "copy-hint", "copy-status", "help"] {
        assert_eq!(html.matches(&format!("id=\"{id}\"")).count(), 1);
    }
    assert!(html.contains("aria-describedby=\"copy-hint\""));
    assert!(html.contains("aria-describedby=\"help\""));
    // Cross-component/authored ID collisions remain authoritative Native admission.
}
