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
    for name in ["badge", "divider", "status-indicator"] {
        let expected = format!("@clanker/vanilla/{name}");
        assert!(ids.contains(&expected.as_str()));
        let (ok, described) = run(&["describe", name, "--lock", lock]);
        assert!(ok, "{described}");
        assert_eq!(described["data"]["component"]["status"], "ready");
    }
    let (ok, graph) = run(&["graph", "badge", "--lock", lock]);
    assert!(ok, "{graph}");
    assert_eq!(graph["data"]["components"][0], "@clanker/vanilla/icon");
    assert_eq!(graph["data"]["components"][1], "@clanker/vanilla/badge");
    let (ok, verified) = run(&["verify", "--lock", lock]);
    assert!(ok, "{verified}");
    assert_eq!(
        verified["data"]["verifiedComponents"]
            .as_array()
            .unwrap()
            .len(),
        5
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
