use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn invoke(args: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn capabilities_are_offline_and_honest_about_host_ownership() {
    let (ok, result) = invoke(&["capabilities"]);
    assert!(ok, "{result}");
    assert_eq!(result["schemaVersion"], 1);
    assert_eq!(result["data"]["networkRequired"], false);
    assert!(result["data"]["unsupported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "host builds"));
}

#[test]
fn context_is_locked_bounded_and_keeps_remaining_checks_explicit() {
    let lock =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/button-app/ui.lock.json");
    let lock = lock.to_str().unwrap();
    let (ok, context) = invoke(&["context", "badge", "--lock", lock]);
    assert!(ok, "{context}");
    assert_eq!(context["data"]["id"], "@clanker/vanilla/badge");
    assert_eq!(context["data"]["dependencies"][0], "@clanker/vanilla/icon");
    assert!(context["data"]["example"]["input"]
        .get("expectedHtml")
        .is_none());
    assert_eq!(context["data"]["verification"]["package"][0], "verify");
    assert!(
        context["data"]["verification"]["remaining"]
            .as_array()
            .unwrap()
            .len()
            >= 3
    );
    let (ok, doctor) = invoke(&["doctor", "--lock", lock]);
    assert!(ok, "{doctor}");
    assert_eq!(doctor["data"]["digest"], context["data"]["packageDigest"]);
    let (ok, rejected) = invoke(&["context", "unregistered", "--lock", lock]);
    assert!(!ok);
    assert_eq!(rejected["diagnostics"][0]["code"], "CUI001");
}
