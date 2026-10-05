use clanker_ui::expand;
use std::{
    fs,
    path::{Path, PathBuf},
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
    let lock = temp.path().join("clanker-ui.lock.json");
    let app = clanker_ui::application::Application {
        source: clanker_ui::local::LocalPackage,
        output: clanker_ui::directory_sink::DirectorySink,
    };
    app.lock(&lock, "package", false).unwrap();
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
