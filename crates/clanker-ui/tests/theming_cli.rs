use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn lock() -> PathBuf {
    root().join("examples/button-app/ui.lock.json")
}
fn invoke(args: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    let response = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stdout)));
    (output.status.success(), response)
}
fn fixture(css: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("pages")).unwrap();
    fs::write(temp.path().join("pages/index.html"), r#"<main class="page"><cui-page-header title="Customers" /><cui-data-table caption="Customers" columns='["Name"]'><cui-table-row id="one"><cui-table-cell>Ada</cui-table-cell></cui-table-row></cui-data-table><table class="plain"><tr><th>Ordinary</th></tr></table></main>"#).unwrap();
    fs::write(temp.path().join("app.css"), css).unwrap();
    temp
}
#[test]
fn described_token_defaults_roles_readers_and_capabilities_are_generic() {
    let path = lock();
    let path = path.to_str().unwrap();
    let (ok, all) = invoke(&["tokens", "--lock", path]);
    assert!(ok, "{all}");
    let (ok, filtered) = invoke(&["tokens", "--lock", path, "--component", "data-table"]);
    assert!(ok, "{filtered}");
    let tokens = filtered["data"]["tokens"].as_array().unwrap();
    let padding = tokens
        .iter()
        .find(|token| token["name"] == "--cui-data-table-cell-padding")
        .unwrap();
    assert_eq!(padding["owner"], "data-table");
    assert_eq!(padding["role"], "spacing");
    assert_eq!(padding["defaultValue"], "0.75rem 1rem");
    assert!(!padding["purpose"].as_str().unwrap().is_empty());
    let shared = tokens
        .iter()
        .find(|token| token["name"] == "--cui-heading-text")
        .unwrap();
    assert_eq!(shared["owner"], "theme");
    assert_eq!(shared["role"], "text");
    assert_eq!(shared["semantic"], "heading");
    assert!(shared["readers"]
        .as_array()
        .unwrap()
        .contains(&json!("data-table")));
    assert!(shared["readers"]
        .as_array()
        .unwrap()
        .contains(&json!("page-header")));
    for command in ["describe", "context"] {
        let (ok, response) = invoke(&[command, "data-table", "--lock", path]);
        assert!(ok, "{response}");
        assert_eq!(
            response["data"]["component"]["tokenDetails"],
            filtered["data"]["tokens"]
        );
        assert!(response["data"]["component"]["tokens"][0].is_string());
    }
    let (ok, capabilities) = invoke(&["capabilities"]);
    assert!(ok);
    assert!(capabilities["data"]["validation"]
        .as_array()
        .unwrap()
        .contains(&json!("check-css")));
    assert_eq!(capabilities["data"]["theming"]["readOnly"], true);
    let (ok, again) = invoke(&["tokens", "--lock", path]);
    assert!(ok);
    assert_eq!(again, all);
}
#[test]
fn css_errors_warnings_and_important_have_render_compatible_read_only_envelopes() {
    let temp = fixture("/* var(--cui-comment-only) */\n:root{--cui-heading-text:green;--cui-typo:red}\nth{color:green!important}\n.cui-data-table__head{background:red}\n[data-cui-component='data-table'] th{border-color:red}\n.plain th{padding:var(--cui-data-table-cell-padding)}\n.plain::before{content:'var(--cui-string-only)'}");
    let before = fs::read(temp.path().join("app.css")).unwrap();
    let page_before = fs::read(temp.path().join("pages/index.html")).unwrap();
    let lock_before = fs::read(lock()).unwrap();
    let (ok, response) = invoke(&[
        "check-css",
        "--ui",
        temp.path().to_str().unwrap(),
        "--lock",
        lock().to_str().unwrap(),
    ]);
    assert!(!ok, "{response}");
    assert_eq!(response["ok"], false);
    assert_eq!(response["command"], "check-css");
    let diagnostics = response["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().all(|d| d["code"] == "CUI001"));
    assert!(diagnostics
        .iter()
        .any(|d| d["severity"] == "error" && d["token"] == "--cui-typo"));
    assert!(diagnostics.iter().any(|d| d["severity"] == "warning"));
    assert!(diagnostics
        .iter()
        .all(|d| d["location"]["line"].as_u64().unwrap() >= 1));
    assert!(!diagnostics
        .iter()
        .any(|d| d["token"] == "--cui-comment-only" || d["token"] == "--cui-string-only"));
    assert_eq!(fs::read(temp.path().join("app.css")).unwrap(), before);
    assert_eq!(
        fs::read(temp.path().join("pages/index.html")).unwrap(),
        page_before
    );
    assert_eq!(fs::read(lock()).unwrap(), lock_before);
    let (_, again) = invoke(&[
        "check-css",
        "--ui",
        temp.path().to_str().unwrap(),
        "--lock",
        lock().to_str().unwrap(),
    ]);
    assert_eq!(again, response);
}
#[test]
fn ordinary_inherited_overrides_are_clean_and_warnings_do_not_fail() {
    let temp = fixture(":root{--cui-heading-text:green;--cui-data-table-cell-padding:1rem}\n.page{--cui-heading-text:rebeccapurple}");
    let (ok, clean) = invoke(&[
        "check-css",
        "--ui",
        temp.path().to_str().unwrap(),
        "--lock",
        lock().to_str().unwrap(),
    ]);
    assert!(ok, "{clean}");
    assert_eq!(clean["diagnostics"], json!([]));
    fs::write(temp.path().join("app.css"), "h1 { color: green }").unwrap();
    let (ok, warning) = invoke(&[
        "check-css",
        "--ui",
        temp.path().to_str().unwrap(),
        "--lock",
        lock().to_str().unwrap(),
    ]);
    assert!(ok, "{warning}");
    assert_eq!(warning["ok"], true);
    assert!(warning["data"]["warnings"].as_u64().unwrap() > 0);
    assert_eq!(warning["data"]["errors"], 0);
}
