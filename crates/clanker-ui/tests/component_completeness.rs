use catalog_core::{integration::IntegrationStatus, Status};
use clanker_ui::{local::LocalPackage, native::NativeAdapter, ports::PackageSource};
use std::{
    fs,
    path::{Path, PathBuf},
};

const COMPLETE_WITHOUT_NATIVE: [&str; 9] = [
    "modal",
    "drawer",
    "popover",
    "command-menu",
    "confirm-dialog",
    "date-calendar",
    "date-picker",
    "file-upload",
    "data-viewport",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla")
}
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

#[test]
fn all_contracts_are_discoverable_without_claiming_native_support() {
    let package = LocalPackage.load(&root()).unwrap();
    assert_eq!(package.catalog.components().count(), 54);
    for name in COMPLETE_WITHOUT_NATIVE {
        let component = package.catalog.get(name).unwrap();
        assert_eq!(component.status, Status::Ready);
        assert_eq!(
            component.integration.as_ref().unwrap().native.status,
            IntegrationStatus::AdapterRequired
        );
        let fixture = component
            .fixtures
            .iter()
            .find(|path| path.contains("golden"))
            .unwrap();
        let html = NativeAdapter::fixture_html(&LocalPackage, &package, name, fixture).unwrap();
        assert!(!html.is_empty(), "{name}");
        for path in &component.assets.contracts {
            assert!(!LocalPackage.asset(&package, path).unwrap().is_empty());
        }
    }
    let expansion = catalog_core::expansion::Package::from_assets(&package.assets).unwrap();
    for name in COMPLETE_WITHOUT_NATIVE {
        assert!(
            catalog_core::expansion::expand(&format!("<cui-{name} />"), &expansion).is_err(),
            "component completeness must not silently admit {name}"
        );
    }
}

#[test]
fn declared_browser_port_types_are_digest_pinned_and_captured() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("package");
    copy_tree(&self::root(), &root);
    let package = LocalPackage.load(&root).unwrap();
    let path = "components/file-upload/browser.d.ts";
    let before = LocalPackage.asset(&package, path).unwrap();
    fs::write(
        root.join(path),
        b"export interface FileUploadAdapter { /* changed ABI documentation */ }",
    )
    .unwrap();
    let changed = LocalPackage.load(&root).unwrap();
    assert_ne!(package.digest, changed.digest);
    assert_eq!(LocalPackage.asset(&package, path).unwrap(), before);
    assert_ne!(LocalPackage.asset(&changed, path).unwrap(), before);
}

#[test]
fn date_picker_uses_the_same_captured_calendar_dependency_as_its_fixture() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("package");
    copy_tree(&self::root(), &root);
    let dependency = root.join("components/date-calendar/fragment.html");
    let captured = format!(
        "<span data-captured-calendar-proof=\"yes\"></span>{}",
        fs::read_to_string(&dependency).unwrap()
    );
    fs::write(&dependency, &captured).unwrap();
    // The fixture must attest the modified captured template, not the old output.
    let fixture_path = root.join("components/date-picker/fixtures/single-golden.json");
    let mut fixture: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture_path).unwrap()).unwrap();
    fixture["expectedHtml"] = serde_json::json!(fixture["expectedHtml"].as_str().unwrap().replace(
        "<section class=\"cui-date-calendar",
        "<span data-captured-calendar-proof=\"yes\"></span><section class=\"cui-date-calendar"
    ));
    fs::write(fixture_path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let package = LocalPackage.load(&root).unwrap();
    fs::write(&dependency, "uncaptured later bytes").unwrap();
    let component = package.catalog.get("date-picker").unwrap();
    let fixture = component
        .fixtures
        .iter()
        .find(|path| path.contains("golden"))
        .unwrap();
    let html =
        NativeAdapter::fixture_html(&LocalPackage, &package, "date-picker", fixture).unwrap();
    assert!(html.contains("captured-calendar-proof"));
    assert!(!html.contains("uncaptured later bytes"));
    assert!(NativeAdapter::fixture_html(
        &LocalPackage,
        &package,
        "date-picker",
        "components/modal/fixtures/standard-golden.json"
    )
    .is_err());
}
