//! Portable golden corpus captured from the frozen, pre-migration Native renderer.
use clanker_ui::expand::expand;
use clanker_ui::{local::LocalPackage, ports::PackageSource};
use std::{fs, path::Path};

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        assert!(!kind.is_symlink());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()));
        } else {
            assert!(kind.is_file());
            fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
struct PrivatePackage(std::path::PathBuf);
impl Drop for PrivatePackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn seven_static_ports_match_reference_with_an_independent_frozen_package() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = std::env::temp_dir().join(format!("cui-seven-port-parity-{}", std::process::id()));
    fs::create_dir(&path).unwrap();
    let temporary = PrivatePackage(path);
    let package_root = temporary.0.join("package");
    copy_tree(
        &root.join("tests/fixtures/expansion-parity/package"),
        &package_root,
    );
    let names = [
        "activity-feed",
        "definition-list",
        "disclosure",
        "button-group",
        "tabs",
        "segmented-control",
        "progress-steps",
    ];
    for name in names {
        let path = package_root
            .join("components")
            .join(name)
            .join("component.json");
        let mut data: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        // This fixture grants selection only in a private copy. Neither output
        // parity nor fake scene rendering is production readiness evidence.
        data["status"] = "ready".into();
        fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    }
    let package = LocalPackage.load(&package_root).unwrap();
    let ui = temporary.0.join("app/ui");
    let corpus = root.join("tests/fixtures/expansion-parity/static-ports");
    copy_tree(&corpus.join("input/ui"), &ui);
    let lock = ui.join("clanker-ui.lock.json");
    fs::write(&lock,serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"package":package.catalog.package.name,"version":package.catalog.package.version,"path":"../../package","digest":package.digest})).unwrap()).unwrap();
    let bundle = expand(&lock, &ui, None).unwrap();
    assert_eq!(bundle.templates.len(), 1);
    assert_eq!(
        bundle.templates["pages/ports.html"].as_bytes(),
        fs::read(corpus.join("expected/pages/ports.html")).unwrap()
    );
    for name in names {
        assert!(bundle.templates["pages/ports.html"]
            .contains(&format!("data-cui-component=\"{name}\"")));
    }
    for resource in &bundle.resources {
        if let Some(content) = &resource.content {
            let path = resource.path.strip_prefix("ui/").unwrap();
            assert_eq!(
                content.as_bytes(),
                fs::read(corpus.join("expected").join(path)).unwrap(),
                "{path}"
            );
        }
    }
    assert_eq!(bundle.bindings.len(), 1);
    assert_eq!(bundle.bindings[0].field_path, "action_label");
    assert_eq!(bundle.bindings[0].expected_kind, "string");
}
#[test]
fn copied_consumers_preserve_every_expanded_template_and_managed_resource_byte() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = std::env::temp_dir().join(format!("cui-consumer-parity-{}", std::process::id()));
    fs::create_dir(&path).unwrap();
    let temporary = PrivatePackage(path);
    let package_root = temporary.0.join("package");
    copy_tree(
        &root.join("tests/fixtures/expansion-parity/package"),
        &package_root,
    );
    let package = LocalPackage.load(&package_root).unwrap();
    let mut checked = 0;
    for name in [
        "clanker-ui-gallery",
        "golinks-clanker-ui-button",
        "studio-gallery",
    ] {
        let corpus = root.join("tests/fixtures/expansion-parity").join(name);
        let ui = temporary.0.join(name).join("ui");
        copy_tree(&corpus.join("input/ui"), &ui);
        let lock = ui.join("clanker-ui.lock.json");
        fs::write(&lock, serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"package":package.catalog.package.name,"version":package.catalog.package.version,"path":"../../package","digest":package.digest})).unwrap()).unwrap();
        let bundle = expand(&lock, &ui, None).unwrap();
        for (path, html) in &bundle.templates {
            assert_eq!(
                html.as_bytes(),
                fs::read(corpus.join("expected").join(path)).unwrap(),
                "{name}: template {path}"
            );
            checked += 1;
        }
        for resource in &bundle.resources {
            let path = resource.path.strip_prefix("ui/").unwrap();
            if let Some(content) = &resource.content {
                assert_eq!(
                    content.as_bytes(),
                    fs::read(corpus.join("expected").join(path)).unwrap(),
                    "{name}: resource {path}"
                );
            } else if resource.kind == "font" {
                let expected = match resource.source.as_deref().unwrap() {
                    "fonts/geist-mono-variable.woff2" => {
                        "sha256:5f687a5dd4c87da13deaff9f6b9503d5e62249ff501265a96b134565f9aa8c87"
                    }
                    "fonts/geist-sans-variable.woff2" => {
                        "sha256:e24cec106619c03f0b3519e31b9bc55e0d5e926b6a95b8d798cd8cef215b1505"
                    }
                    other => panic!("unexpected reference font {other}"),
                };
                assert_eq!(resource.digest, expected, "{name}: font {path}");
            } else {
                panic!("unexpected new managed resource {name}: {path}");
            }
            checked += 1;
        }
        assert!(bundle.entrypoints.is_empty());
        assert_eq!(bundle.runtime_abi, 1);
        assert_eq!(bundle.template_engine, "minijinja-2.12.0");
    }
    assert_eq!(checked, 28, "Do not silently reduce parity corpus coverage");
}
