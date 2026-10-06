use clanker_ui::expand;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    copy_tree(&package, &temp.path().join("package"));
    let ui = temp.path().join("ui");
    fs::create_dir_all(ui.join("pages")).unwrap();
    fs::write(ui.join("app.css"), b"/* app-owned */").unwrap();
    fs::write(
        ui.join("pages/index.html"),
        b"<main><cui-button label=\"Save\"/></main>",
    )
    .unwrap();
    let lock = ui.join("ui.lock.json");
    let app = clanker_ui::application::Application {
        source: clanker_ui::local::LocalPackage,
        output: clanker_ui::directory_sink::DirectorySink,
    };
    app.lock(&lock, "../package", false).unwrap();
    (temp, lock, ui)
}

#[test]
fn manifest_is_deterministic_and_includes_complete_locked_inputs() {
    let (_temp, lock, ui) = fixture();
    let first = expand::expand(&lock, &ui, None).unwrap();
    let second = expand::expand(&lock, &ui, None).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema_version, 1);
    assert!(first.templates["pages/index.html"].contains("<button"));
    assert!(first
        .inputs
        .iter()
        .any(|i| i.path == "package/ui-package.json"));
    assert!(first
        .inputs
        .iter()
        .any(|i| i.path == "package/components/button/styles.css"));
    assert!(first
        .resources
        .iter()
        .any(|r| r.path == "ui/app.css" && r.content.is_some()));
    assert_eq!(
        first
            .resources
            .iter()
            .filter(|r| r.path == "ui/fonts/clanker-geist-sans-variable.woff2")
            .count(),
        1
    );
}

#[test]
fn static_templates_do_not_pull_any_unused_browser_modules() {
    let (_temp, lock, ui) = fixture();
    let bundle = expand::expand(&lock, &ui, None).unwrap();
    assert!(!bundle.resources.iter().any(|r| r.kind == "module"));
    assert!(!bundle
        .resources
        .iter()
        .any(|r| r.path.ends_with("interaction.js") || r.path.ends_with("install.js")));
}

#[test]
fn malformed_template_paths_and_generated_resource_collisions_fail_closed() {
    let (_temp, lock, ui) = fixture();
    fs::write(ui.join("pages/bad name.html"), "plain template").unwrap();
    assert!(expand::expand(&lock, &ui, None)
        .unwrap_err()
        .contains("unsafe UI path"));
    fs::remove_file(ui.join("pages/bad name.html")).unwrap();
    fs::write(ui.join("clanker-properties.js"), "author-owned").unwrap();
    assert!(expand::expand(&lock, &ui, None)
        .unwrap_err()
        .contains("collides"));
    assert_eq!(
        fs::read_to_string(ui.join("clanker-properties.js")).unwrap(),
        "author-owned"
    );
}

#[test]
fn lock_tampering_and_unsafe_templates_fail_closed() {
    let (_temp, lock, ui) = fixture();
    fs::write(ui.join("pages/unsafe.html"), "<cui-button label=\"x\"/>").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("/etc/passwd", ui.join("pages/linked.html")).unwrap();
        assert!(expand::expand(&lock, &ui, None)
            .unwrap_err()
            .contains("symlink"));
        fs::remove_file(ui.join("pages/linked.html")).unwrap();
    }
    let text = fs::read_to_string(&lock)
        .unwrap()
        .replacen("sha256:", "sha256:tampered", 1);
    fs::write(&lock, text).unwrap();
    assert!(expand::expand(&lock, &ui, None).is_err());
}

#[test]
fn native_lock_and_assembly_request_are_closed_deterministic_and_abi2() {
    let (temp, _lock, ui) = fixture();
    let native_lock_path = ui.join("ui.lock.json");
    let generated = expand::native_lock(&native_lock_path, "../package").unwrap();
    let nested_ui = temp.path().join("app/ui");
    fs::create_dir_all(&nested_ui).unwrap();
    let nested_lock =
        expand::native_lock(&nested_ui.join("ui.lock.json"), "../../package").unwrap();
    assert_eq!(
        nested_lock["package"]["digest"],
        generated["package"]["digest"]
    );
    for path in [
        "./package",
        "../../../package",
        "/package",
        "../package/../package",
    ] {
        assert!(expand::native_lock(&nested_ui.join("invalid.lock.json"), path).is_err());
    }
    let package_lock = generated["package"].clone();
    assert_eq!(generated["provider"], "clanker-ui.native");
    let inputs = package_lock["inputs"].as_array().unwrap();
    assert!(inputs
        .iter()
        .any(|input| input["path"] == "components/button/component.json"));
    assert!(inputs
        .iter()
        .any(|input| input["path"] == "components/date-picker/browser.d.ts"));
    let mut manifest = Sha256::new();
    for input in inputs {
        manifest.update(input["path"].as_str().unwrap().as_bytes());
        manifest.update([0]);
        manifest.update(input["bytes"].to_string().as_bytes());
        manifest.update([0]);
        manifest.update(input["digest"].as_str().unwrap().as_bytes());
        manifest.update(b"\n");
    }
    assert_eq!(
        package_lock["digest"],
        format!("sha256:{:x}", manifest.finalize())
    );
    fs::write(
        ui.join("pages/index.html"),
        b"<form><cui-form-field id=\"edit-url\" name=\"url\" label=\"Destination\" value=\"{{ link.url }}\" /><cui-button kind=\"submit\" label=\"Save\" /></form>",
    )
    .unwrap();
    let request = serde_json::json!({
        "schemaVersion": 1,
        "assemblyProtocol": 2,
        "provider": "clanker-ui.native",
        "target": {"bindingAbi": 2, "templateEngine": "minijinja-2.12.0"},
        "package": {
            "name": package_lock["name"],
            "version": package_lock["version"],
            "path": temp.path().join("package"),
            "digest": package_lock["digest"],
            "inputs": package_lock["inputs"]
        },
        "ui": ui
    });
    let request_path = temp.path().join("request.json");
    fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let ui_lock_path = ui.join("ui.lock.json");
    let valid_ui_lock = fs::read(&ui_lock_path).unwrap();
    fs::write(&ui_lock_path, b"{}").unwrap();
    assert!(expand::assemble_request(&request_path).is_err());
    fs::write(&ui_lock_path, valid_ui_lock).unwrap();
    let first = expand::assemble_request(&request_path).unwrap();
    let second = expand::assemble_request(&request_path).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.runtime_abi, 2);
    assert_eq!(first.package_digest, package_lock["digest"]);
    let form = &first.templates["pages/index.html"];
    assert!(form.contains("value=\"{{ link.url }}\""));
    assert!(form.contains("type=\"submit\""));
    assert!(!form.contains("<cui-") && !form.contains("ui_text("));
    let serialized = serde_json::to_value(&first).unwrap();
    assert!(serialized.get("bindings").is_none());
    assert!(serialized.get("entrypoints").is_none());
    assert!(!serialized["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["path"] == "ui/clanker-ui.js" || resource["path"] == "ui/ui-package.js"
        }));
    let cli = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(["assemble", "--request", request_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stdout)
    );
    let cli_result: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert!(cli_result["data"].get("bindings").is_none());
    assert!(cli_result["data"].get("entrypoints").is_none());
    fs::write(
        ui.join("pages/index.html"),
        b"<cui-copy-field name=\"code\" label=\"Reference\" value=\"TASK-42\" />",
    )
    .unwrap();
    let helper_result = expand::assemble_request(&request_path).unwrap();
    let bootstrap = helper_result
        .resources
        .iter()
        .find(|resource| resource.path == "ui/clanker-ui.js")
        .and_then(|resource| resource.content.as_deref())
        .expect("selected helper requires generated bootstrap module");
    assert!(bootstrap.contains("./clanker-ui/browser/lifecycle.js"));
    assert!(bootstrap.contains("./clanker-ui/components/copy-field/interaction.js"));
    let loader = helper_result
        .resources
        .iter()
        .find(|resource| resource.path == "ui/ui-package.js")
        .unwrap();
    assert_eq!(loader.kind, "module");
    assert_eq!(
        loader.content.as_deref(),
        Some("import './clanker-ui.js';\n")
    );
    // An app-owned module cannot be silently overwritten, even though it is conventional.
    fs::write(ui.join("ui-package.js"), b"export const appOwned = true;").unwrap();
    assert!(expand::assemble_request(&request_path).is_err());
    fs::write(ui.join("ui-package.js"), b"import './clanker-ui.js';\n").unwrap();
    let idempotent = expand::assemble_request(&request_path).unwrap();
    assert!(idempotent
        .inputs
        .iter()
        .any(|input| input.path == "ui/ui-package.js"));
    fs::remove_file(ui.join("ui-package.js")).unwrap();
    assert!(helper_result.resources.iter().any(|resource| {
        resource.path == "ui/clanker-ui/components/copy-field/interaction.js"
            && resource.kind == "module"
    }));
    assert!(first
        .inputs
        .iter()
        .any(|input| input.path == "ui/ui.lock.json"));
    assert!(!first.consumed_inputs.contains(&"ui/ui.lock.json".into()));

    let mutations: [fn(&mut serde_json::Value); 7] = [
        |value| value["schemaVersion"] = 2.into(),
        |value| value["assemblyProtocol"] = 1.into(),
        |value| value["target"]["bindingAbi"] = 1.into(),
        |value| value["target"]["templateEngine"] = "jinja".into(),
        |value| value["package"]["digest"] = "sha256:wrong".into(),
        |value| {
            value["package"]["inputs"].as_array_mut().unwrap().pop();
        },
        |value| value["unexpected"] = true.into(),
    ];
    for mutate in mutations {
        let mut invalid = request.clone();
        mutate(&mut invalid);
        fs::write(&request_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(expand::assemble_request(&request_path).is_err());
    }
}

#[test]
fn native_pin_is_explicit_local_unsigned_output_for_the_current_executable() {
    let temp = tempfile::tempdir().unwrap();
    let app_lock = temp.path().join("ui.lock.json");
    fs::write(&app_lock, b"app lock stays intact").unwrap();
    assert!(expand::native_pin(&app_lock).is_err());
    assert_eq!(fs::read(&app_lock).unwrap(), b"app lock stays intact");
    let occupied = temp.path().join("occupied-pin.json");
    fs::write(&occupied, b"keep").unwrap();
    assert!(expand::native_pin(&occupied).is_err());
    assert_eq!(fs::read(&occupied).unwrap(), b"keep");

    let output = temp.path().join("provider-pin.json");
    let pin = expand::native_pin(&output).unwrap();
    assert_eq!(pin["schemaVersion"], 1);
    assert_eq!(pin["provider"], "clanker-ui.native");
    assert_eq!(pin["assemblyProtocol"], 2);
    assert_eq!(pin["bindingAbi"], 2);
    let host_target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        target => panic!("unsupported test host target: {target:?}"),
    };
    assert_eq!(pin["targets"].as_object().unwrap().len(), 1);
    let target = &pin["targets"][host_target];
    let executable = PathBuf::from(target["executable"].as_str().unwrap());
    let executable_bytes = fs::read(executable).unwrap();
    assert!(executable_bytes.len() <= 64 * 1024 * 1024);
    assert_eq!(
        target["digest"],
        format!("sha256:{:x}", Sha256::digest(executable_bytes))
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(output).unwrap()).unwrap(),
        pin
    );
}

#[test]
fn output_is_atomic_private_and_does_not_overwrite_author_files() {
    let (temp, lock, ui) = fixture();
    let output = temp.path().join("private-out");
    let bundle = expand::expand(&lock, &ui, Some(&output)).unwrap();
    assert_eq!(
        fs::read(output.join("ui/pages/index.html")).unwrap(),
        bundle.templates["pages/index.html"].as_bytes()
    );
    assert!(output
        .join("ui/fonts/clanker-geist-mono-variable.woff2")
        .is_file());
    let occupied = temp.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("author.txt"), "keep").unwrap();
    assert!(expand::expand(&lock, &ui, Some(&occupied)).is_err());
    assert_eq!(
        fs::read_to_string(occupied.join("author.txt")).unwrap(),
        "keep"
    );
    assert_eq!(fs::read(ui.join("app.css")).unwrap(), b"/* app-owned */");
}
