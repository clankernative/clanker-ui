use clanker_ui::directory_sink::DirectorySink;
use clanker_ui::local::LocalPackage;
use clanker_ui::native::NativeAdapter;
use clanker_ui::ports::{ArtifactSink, Bundle, PackageSource};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[test]
fn button_output_is_deterministic_and_refuses_conflicting_files() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let loaded = LocalPackage.load(&package).unwrap();
    NativeAdapter::verify(&LocalPackage, &loaded).unwrap();
    let bundle = NativeAdapter::plan(
        &LocalPackage,
        &loaded,
        "button",
        "primary",
        b":root { --cui-accent: purple; }",
    )
    .unwrap();
    assert!(bundle.files.contains_key("ui/components/button.html"));
    assert!(bundle.files["ui/app.css"]
        .windows(6)
        .any(|w| w == b"purple"));
    let output_root = tempfile::tempdir().unwrap();
    let output = output_root.path().join("staged");
    DirectorySink.apply(&output, &bundle).unwrap();
    DirectorySink.apply(&output, &bundle).unwrap();
    fs::write(output.join("ui/app.css"), "tampered").unwrap();
    assert!(DirectorySink.apply(&output, &bundle).is_err());
}

#[cfg(unix)]
#[test]
fn stage_refuses_symlinked_output() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    fs::create_dir(&target).unwrap();
    let output = root.path().join("output");
    symlink(&target, &output).unwrap();
    let bundle = Bundle {
        files: BTreeMap::new(),
        package_digest: "sha256:test".into(),
        components: vec![],
    };
    assert!(DirectorySink
        .apply(&output, &bundle)
        .err()
        .unwrap()
        .contains("symlink"));
}
