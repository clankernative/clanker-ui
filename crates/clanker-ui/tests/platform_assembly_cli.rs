use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn producer_cli_matches_recorded_platform_assembly_contract() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/platform-assembly");
    let ui = fixture.join("app/ui");
    let package_path = fixture.join("package");
    let lock_path = ui.join("ui.lock.json");
    let lock_bytes = fs::read(&lock_path).expect("read fixture UI lock");
    let page_path = ui.join("pages/proof.html");
    let page_bytes = fs::read(&page_path).expect("read fixture page");
    let css_path = ui.join("app.css");
    let css_bytes = fs::read(&css_path).expect("read fixture CSS");

    let lock: Value = serde_json::from_slice(&lock_bytes).expect("parse fixture UI lock");
    let package_sources = lock["package"]["inputs"]
        .as_array()
        .expect("locked package inputs")
        .iter()
        .map(|input| {
            let path = package_path.join(input["path"].as_str().expect("locked input path"));
            let bytes = fs::read(&path).expect("read locked package input");
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let mut package = lock["package"].clone();
    package["path"] = json!(package_path);
    let request = json!({
        "schemaVersion": 1,
        "assemblyProtocol": 2,
        "provider": lock["provider"],
        "target": {
            "bindingAbi": 2,
            "templateEngine": "minijinja-2.12.0"
        },
        "package": package,
        "ui": ui
    });

    let temp = tempfile::tempdir().expect("create temporary request directory");
    let request_path = temp.path().join("request.json");
    fs::write(&request_path, serde_json::to_vec(&request).unwrap())
        .expect("write temporary assembly request");

    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(["assemble", "--request"])
        .arg(&request_path)
        .output()
        .expect("run producer CLI");
    assert!(
        output.status.success(),
        "clanker-ui assemble failed ({}):\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "parse producer CLI JSON ({error}):\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let expected: Value = serde_json::from_slice(
        &fs::read(fixture.join("response.json")).expect("read recorded CLI response"),
    )
    .expect("parse recorded CLI response");
    assert_eq!(
        actual, expected,
        "producer CLI output drifted from the golden response"
    );

    assert_eq!(
        fs::read(lock_path).unwrap(),
        lock_bytes,
        "UI lock was modified"
    );
    assert_eq!(
        fs::read(page_path).unwrap(),
        page_bytes,
        "fixture page was modified"
    );
    assert_eq!(
        fs::read(css_path).unwrap(),
        css_bytes,
        "fixture CSS was modified"
    );
    for (path, bytes) in package_sources {
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "locked package input was modified: {}",
            path.display()
        );
    }
}
