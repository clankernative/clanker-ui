use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn run(args: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.success(), result)
}

#[test]
fn an_agent_can_discover_and_verify_static_components() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/button-app");
    let lock = root.join("clanker-ui.lock.json");
    let lock = lock.to_str().unwrap();
    let (ok, listed) = run(&["list", "--lock", lock]);
    assert!(ok, "{listed}");
    let ids = listed["data"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|component| component["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    let (ok, properties) = run(&["properties", "--lock", lock]);
    assert!(ok, "{properties}");
    assert_eq!(
        properties["data"]["components"].as_array().unwrap().len(),
        ids.len()
    );
    let declared: Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../packages/vanilla/property-catalog.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(properties["data"], declared);
    for component in properties["data"]["components"].as_array().unwrap() {
        assert!(component["sample"].get("id").is_none());
        assert!(component["sample"].get("component").is_none());
    }
    for name in [
        "badge",
        "divider",
        "status-indicator",
        "tag",
        "alert",
        "progress",
        "form-field",
        "avatar",
        "empty-state",
        "metric",
        "skeleton",
        "page-header",
        "card",
        "cluster",
        "container",
        "grid",
        "split",
        "stack",
        "cover",
        "layer",
        "pane",
        "reel",
        "sidebar",
        "switch",
        "activity-feed",
        "button-group",
        "definition-list",
        "disclosure",
        "progress-steps",
        "segmented-control",
        "tabs",
        "checkbox-group",
        "radio-group",
        "toggle",
        "copy-field",
        "theme-switcher",
        "tooltip",
        "toast",
    ] {
        let expected = format!("@clanker/vanilla/{name}");
        assert!(ids.contains(&expected.as_str()));
        let (ok, described) = run(&["describe", name, "--lock", lock]);
        assert!(ok, "{described}");
        assert_eq!(described["data"]["component"]["status"], "ready");
    }
    let (ok, layouts) = run(&["find", "layout", "--lock", lock]);
    assert!(ok, "{layouts}");
    for name in ["cover", "layer", "pane", "reel", "sidebar", "switch"] {
        let expected = format!("@clanker/vanilla/{name}");
        assert!(layouts["data"]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|component| component["id"] == expected));
        let (ok, graph) = run(&["graph", name, "--lock", lock]);
        assert!(ok, "{graph}");
        assert_eq!(graph["data"]["components"].as_array().unwrap().len(), 1);
        assert_eq!(graph["data"]["components"][0], expected);
    }
    let (ok, graph) = run(&["graph", "badge", "--lock", lock]);
    assert!(ok, "{graph}");
    assert_eq!(graph["data"]["components"][0], "@clanker/vanilla/icon");
    assert_eq!(graph["data"]["components"][1], "@clanker/vanilla/badge");
    let (ok, metric_graph) = run(&["graph", "metric", "--lock", lock]);
    assert!(ok, "{metric_graph}");
    assert_eq!(
        metric_graph["data"]["components"][0],
        "@clanker/vanilla/icon"
    );
    assert_eq!(
        metric_graph["data"]["components"][1],
        "@clanker/vanilla/metric"
    );
    let (ok, verified) = run(&["verify", "--lock", lock]);
    assert!(ok, "{verified}");
    assert_eq!(
        verified["data"]["verifiedComponents"]
            .as_array()
            .unwrap()
            .len(),
        ids.len()
    );
}

#[test]
fn an_agent_can_find_verify_preview_and_stage_a_button() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/button-app");
    let lock = root.join("clanker-ui.lock.json");
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("staged");
    let lock = lock.to_str().unwrap();
    let out = out.to_str().unwrap();

    let (ok, found) = run(&["find", "button", "--lock", lock]);
    assert!(ok);
    assert_eq!(found["data"]["matches"][0]["id"], "@clanker/vanilla/button");
    let (ok, icon_found) = run(&["find", "icon", "--lock", lock]);
    assert!(ok);
    assert!(icon_found["data"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|component| component["id"] == "@clanker/vanilla/icon"));
    let (ok, icon) = run(&["describe", "icon", "--lock", lock]);
    assert!(ok);
    assert_eq!(icon["data"]["component"]["contract"]["role"], "img");
    assert_eq!(
        icon["data"]["component"]["assets"]["template"],
        "components/icon/fragment.html"
    );
    assert_eq!(
        icon["data"]["iconCatalog"]["icons"]
            .as_array()
            .unwrap()
            .len(),
        100
    );
    assert!(icon["data"]["iconCatalog"]["categories"]
        .as_array()
        .unwrap()
        .iter()
        .all(|category| category["label"].is_string()));
    let (ok, icons) = run(&["find-icon", "PROCESSOR", "--lock", lock]);
    assert!(ok, "{icons}");
    assert_eq!(icons["data"]["matches"][0]["name"], "cpu");
    assert_eq!(icons["data"]["matches"][0]["label"], "Processor");
    assert!(icons["data"]["matches"][0]["category"].is_string());
    let (ok, by_category) = run(&["find-icon", "COMMUNICATION", "--lock", lock]);
    assert!(ok, "{by_category}");
    assert!(!by_category["data"]["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    let (ok, by_name) = run(&["find-icon", "arrow", "--lock", lock]);
    assert!(ok, "{by_name}");
    assert!(!by_name["data"]["matches"].as_array().unwrap().is_empty());
    let (ok, verified) = run(&["verify", "--lock", lock]);
    assert!(ok, "{verified}");
    assert!(verified["data"]["verifiedComponents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "icon"));
    let args = [
        "build",
        "button",
        "--lock",
        lock,
        "--theme",
        "theme.css",
        "--out",
        out,
    ];
    let (ok, preview) = run(&args);
    assert!(ok, "{preview}");
    assert_eq!(preview["data"]["applied"], false);
    assert!(!temp.path().join("staged").exists());
    let (ok, applied) = run(&[&args[..], &["--apply"]].concat());
    assert!(ok, "{applied}");
    assert!(temp
        .path()
        .join("staged/ui/components/button.html")
        .exists());
    assert!(temp.path().join("staged/catalog/actions.md").exists());
    let (ok, failure) = run(&[
        "build",
        "button",
        "--lock",
        lock,
        "--theme",
        "theme.css",
        "--out",
        out,
        "--variant",
        "imaginary",
        "--apply",
    ]);
    assert!(!ok);
    assert_eq!(failure["ok"], false);
    assert_eq!(failure["diagnostics"][0]["code"], "CUI001");
}
