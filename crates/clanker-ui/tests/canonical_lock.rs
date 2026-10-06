use clanker_ui::{
    application::{Application, PackageLock},
    directory_sink::DirectorySink,
    local::LocalPackage,
    lock::{input_manifest_digest, LockedInput},
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn app() -> Application<LocalPackage, DirectorySink> {
    Application {
        source: LocalPackage,
        output: DirectorySink,
    }
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PackageLock) {
    let temp = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    copy_tree(&root.join("packages/vanilla"), &temp.path().join("package"));
    let ui = temp.path().join("ui");
    fs::create_dir_all(ui.join("pages")).unwrap();
    fs::write(ui.join("app.css"), "/* app */").unwrap();
    fs::write(
        ui.join("pages/index.html"),
        "<main><cui-button label=\"Save\" /></main>",
    )
    .unwrap();
    let lock_path = ui.join("ui.lock.json");
    let lock = app().lock(&lock_path, "../package", false).unwrap();
    (temp, ui, lock_path, lock)
}

#[test]
fn manifest_hash_has_a_frozen_native_encoding_vector() {
    let inputs = [LockedInput {
        path: "ui-package.json".into(),
        digest: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        bytes: 0,
    }];
    assert_eq!(
        input_manifest_digest(&inputs),
        "sha256:69c4497b4e1cbf83dc4e8b48b1141a1aac40c775ddba84f82e23be8e4c8b7374"
    );
}

#[test]
fn authoring_commands_are_identical_and_updates_are_explicit() {
    let (_temp, ui, path, lock) = fixture();
    let other = ui.join("other.lock.json");
    let native = clanker_ui::expand::native_lock(&other, "../package").unwrap();
    assert_eq!(native, serde_json::to_value(&lock).unwrap());
    assert_eq!(fs::read(&path).unwrap(), fs::read(&other).unwrap());
    assert_eq!(app().load(&path).unwrap().digest, lock.package.digest);
    assert!(lock
        .package
        .inputs
        .iter()
        .any(|input| input.path == "components/date-picker/browser.d.ts"));
    fs::write(
        ui.parent()
            .unwrap()
            .join("package/components/date-picker/browser.d.ts"),
        "changed contract",
    )
    .unwrap();
    let before = fs::read(&other).unwrap();
    assert!(app().load(&path).is_err());
    assert!(clanker_ui::expand::native_lock(&other, "../package").is_err());
    assert_eq!(fs::read(&other).unwrap(), before);
    let refreshed =
        clanker_ui::expand::native_lock_with_update(&other, "../package", true).unwrap();
    assert_ne!(refreshed["package"]["digest"], native["package"]["digest"]);
    assert!(app().load(&other).is_ok());
}

#[test]
fn draft_component_assets_remain_in_the_canonical_declared_closure() {
    let (temp, _ui, path, _lock) = fixture();
    let manifest_path = temp
        .path()
        .join("package/components/date-picker/component.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["status"] = json!("draft");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let lock = app().lock(&path, "../package", true).unwrap();
    let snapshot = app().load(&path).unwrap();
    assert!(snapshot.catalog.get("date-picker").is_none());
    assert!(lock
        .package
        .inputs
        .iter()
        .any(|input| input.path == "components/date-picker/browser.d.ts"));
    fs::write(
        temp.path()
            .join("package/components/date-picker/browser.d.ts"),
        "draft contract changed",
    )
    .unwrap();
    assert!(app().load(&path).is_err());
}

#[test]
fn rejects_legacy_and_noncanonical_schema_and_manifests() {
    let (_temp, _ui, path, lock) = fixture();
    let canonical = serde_json::to_value(&lock).unwrap();
    let edits: Vec<Box<dyn Fn(&mut Value)>> = vec![
        Box::new(|v| v["schemaVersion"] = json!(2)),
        Box::new(|v| v["provider"] = json!("other")),
        Box::new(|v| {
            v.as_object_mut().unwrap().remove("provider");
        }),
        Box::new(|v| v["extra"] = json!(true)),
        Box::new(|v| v["package"]["extra"] = json!(true)),
        Box::new(|v| v["package"]["inputs"][0]["extra"] = json!(true)),
        Box::new(|v| v["package"]["path"] = json!("/package")),
        Box::new(|v| v["package"]["path"] = json!("../package/../package")),
        Box::new(|v| v["package"]["path"] = json!("..\\package")),
        Box::new(|v| v["package"]["digest"] = json!("sha256:tampered")),
        Box::new(|v| v["package"]["inputs"][0]["bytes"] = json!(1)),
        Box::new(|v| v["package"]["inputs"][0]["bytes"] = json!(-1)),
        Box::new(|v| v["package"]["inputs"][0]["bytes"] = json!(1_048_577)),
        Box::new(|v| v["package"]["inputs"][0]["digest"] = json!("sha256:ABCDEF")),
        Box::new(|v| {
            v["package"].as_object_mut().unwrap().remove("inputs");
        }),
        Box::new(|v| v["package"]["inputs"][0]["path"] = json!("../escape")),
        Box::new(|v| {
            v["package"]["inputs"].as_array_mut().unwrap().reverse();
        }),
        Box::new(|v| {
            let first = v["package"]["inputs"][0].clone();
            v["package"]["inputs"]
                .as_array_mut()
                .unwrap()
                .insert(0, first);
        }),
        Box::new(|v| v["package"]["inputs"] = json!([])),
    ];
    for edit in edits {
        let mut value = canonical.clone();
        edit(&mut value);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(PackageLock::parse(&bytes).is_err(), "accepted {value}");
        fs::write(&path, &bytes).unwrap();
        assert!(app().load(&path).is_err());
    }
    let legacy = json!({"schemaVersion":1,"package":lock.package.name,"version":lock.package.version,"path":"../package","digest":lock.package.digest});
    assert!(PackageLock::parse(&serde_json::to_vec(&legacy).unwrap()).is_err());
    assert!(PackageLock::parse(b"# Lock\n```json\n{}\n```\n").is_err());
    assert!(PackageLock::parse(&vec![b' '; 1_048_577]).is_err());
}

#[test]
fn a_self_consistent_but_incomplete_input_manifest_is_not_a_valid_package_lock() {
    let (_temp, _ui, path, mut lock) = fixture();
    lock.package.inputs.remove(0);
    lock.package.digest = input_manifest_digest(&lock.package.inputs);
    lock.validate().unwrap();
    fs::write(&path, serde_json::to_vec(&lock).unwrap()).unwrap();
    let error = app().load(&path).err().unwrap();
    assert!(error.contains("package bytes differ from lock"), "{error}");
}

fn cli(args: &[&str]) -> (bool, Value) {
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
fn every_discovery_validation_expansion_and_render_command_uses_one_lock() {
    let (temp, ui, path, lock) = fixture();
    let scene = temp.path().join("scene.json");
    fs::write(&scene, r#"{"page":"pages/index.html","data":{}}"#).unwrap();
    let ui_arg = ui.to_str().unwrap();
    let scene_arg = scene.to_str().unwrap();
    let path_arg = path.to_str().unwrap();
    let commands = vec![
        vec!["list"],
        vec!["find", "button"],
        vec!["find-icon", "arrow"],
        vec!["describe", "button"],
        vec!["properties"],
        vec!["graph", "button"],
        vec!["context", "button"],
        vec!["tokens"],
        vec!["doctor"],
        vec!["verify"],
        vec!["check-css", "--ui", ui_arg],
        vec!["expand", "--ui", ui_arg],
        vec!["render", "--ui", ui_arg, "--scene", scene_arg],
    ];
    for args in &commands {
        let mut args = args.clone();
        args.extend(["--lock", path_arg]);
        let (ok, response) = cli(&args);
        assert!(ok, "{args:?}: {response}");
        if let Some(digest) = response["data"]
            .get("packageDigest")
            .or_else(|| response["data"].get("digest"))
        {
            assert_eq!(digest, &json!(lock.package.digest), "{args:?}");
        }
    }
    // Even a recomputed, internally valid manifest cannot lie about captured bytes.
    let mut tampered = lock;
    tampered.package.inputs[0].bytes += 1;
    tampered.package.digest = input_manifest_digest(&tampered.package.inputs);
    tampered.validate().unwrap();
    fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    for mut args in commands {
        args.extend(["--lock", path_arg]);
        let (ok, response) = cli(&args);
        assert!(!ok, "{args:?}: {response}");
        assert_eq!(response["ok"], false);
        assert!(response["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("package bytes differ from lock"));
    }
}

#[cfg(unix)]
#[test]
fn symlinked_locks_are_rejected_for_reading_and_authoring() {
    use std::os::unix::fs::symlink;
    let (_temp, ui, path, _lock) = fixture();
    let link = ui.join("linked.lock.json");
    symlink(&path, &link).unwrap();
    assert!(app().load(&link).is_err());
    assert!(app().lock(&link, "../package", true).is_err());
}
