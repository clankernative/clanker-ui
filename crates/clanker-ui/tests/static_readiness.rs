use clanker_ui::local::LocalPackage;
use clanker_ui::native::NativeAdapter;
use clanker_ui::ports::PackageSource;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const READY: &[&str] = &[
    "activity-feed",
    "alert",
    "avatar",
    "badge",
    "breadcrumbs",
    "button-group",
    "button",
    "card",
    "cluster",
    "container",
    "cover",
    "data-table",
    "definition-list",
    "disclosure",
    "divider",
    "empty-state",
    "filter-bar",
    "form-field",
    "grid",
    "icon",
    "layer",
    "metric",
    "page-header",
    "pagination",
    "pane",
    "progress-steps",
    "progress",
    "reel",
    "segmented-control",
    "select-field",
    "sidebar",
    "skeleton",
    "split",
    "stack",
    "status-indicator",
    "switch",
    "tabs",
    "tag",
];

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn package_copy() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    copy_tree(&source, &package);
    for entry in fs::read_dir(package.join("components")).unwrap() {
        let manifest_path = entry.unwrap().path().join("component.json");
        if !manifest_path.exists() {
            continue;
        }
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let name = manifest["name"].as_str().unwrap();
        manifest["status"] = json!(if READY.contains(&name) {
            "ready"
        } else {
            "draft"
        });
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }
    (temp, package)
}

fn write_fresh_property_catalog(package_path: &Path) {
    let source = LocalPackage;
    let package = source.load(package_path).unwrap();
    let icons: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&source.asset(&package, "icons.json").unwrap()).unwrap();
    let catalog = NativeAdapter::property_catalog_with_icons(&package, &icons).unwrap();
    fs::write(
        package_path.join("property-catalog.json"),
        serde_json::to_vec_pretty(&catalog).unwrap(),
    )
    .unwrap();
}

fn write_lock(package_path: &Path, lock_path: &Path) {
    let package = LocalPackage.load(package_path).unwrap();
    fs::write(
        lock_path,
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "package": package.catalog.package.name,
            "version": package.catalog.package.version,
            "path": "package",
            "digest": package.digest,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn cli_verify(lock: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(["verify", "--lock", lock.to_str().unwrap()])
        .output()
        .unwrap()
}

#[test]
fn enhancement_verification_accepts_only_closed_declared_browser_entrypoints() {
    let (_temp, package_path) = package_copy();
    for name in ["copy-field", "theme-switcher", "tooltip", "toast"] {
        let path = package_path.join(format!("components/{name}/component.json"));
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["status"] = json!("ready");
        fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }
    write_fresh_property_catalog(&package_path);
    NativeAdapter::verify(&LocalPackage, &LocalPackage.load(&package_path).unwrap()).unwrap();

    // A declared, readable package file is not permission to assign arbitrary JS
    // to a component. Full emitted import-graph admission remains host-owned.
    let path = package_path.join("components/copy-field/component.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["assets"]["scripts"] = json!(["components/toast/install.js"]);
    fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    write_fresh_property_catalog(&package_path);
    let error = NativeAdapter::verify(&LocalPackage, &LocalPackage.load(&package_path).unwrap())
        .unwrap_err();
    assert!(
        error.contains("copy-field has unsupported browser resource declarations"),
        "{error}"
    );
}

#[test]
fn native_verify_accepts_the_exact_38_component_ready_set() {
    let (_temp, package_path) = package_copy();
    write_fresh_property_catalog(&package_path);
    let source = LocalPackage;
    let package = source.load(&package_path).unwrap();
    let mut names: Vec<_> = package
        .catalog
        .components()
        .map(|component| component.name.as_str())
        .collect();
    names.sort_unstable();
    let mut expected = READY.to_vec();
    expected.sort_unstable();
    assert_eq!(names, expected);
    NativeAdapter::verify(&source, &package).unwrap();
}

#[test]
fn cli_verify_rejects_stale_properties_and_modified_golden_html() {
    // Keep separate locked copies so each failure isolates the changed input.
    let (_stale_temp, stale_package) = package_copy();
    write_fresh_property_catalog(&stale_package);
    fs::write(stale_package.join("property-catalog.json"), b"{}\n").unwrap();
    let stale_lock = stale_package.parent().unwrap().join("clanker-ui.lock.json");
    write_lock(&stale_package, &stale_lock);
    let stale = cli_verify(&stale_lock);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stdout).contains("property-catalog.json is stale"));

    let (_golden_temp, golden_package) = package_copy();
    write_fresh_property_catalog(&golden_package);
    let fixture_path = golden_package.join("components/activity-feed/fixtures/warning.json");
    let mut fixture: Value = serde_json::from_slice(&fs::read(&fixture_path).unwrap()).unwrap();
    fixture["expectedHtml"] = json!("<p>intentionally incorrect</p>");
    fs::write(&fixture_path, serde_json::to_vec_pretty(&fixture).unwrap()).unwrap();
    let golden_lock = golden_package
        .parent()
        .unwrap()
        .join("clanker-ui.lock.json");
    write_lock(&golden_package, &golden_lock);
    let golden = cli_verify(&golden_lock);
    assert!(!golden.status.success());
    assert!(String::from_utf8_lossy(&golden.stdout)
        .contains("rendered HTML differs from expected fixture"));
}
