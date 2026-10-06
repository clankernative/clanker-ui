use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn run(args: &[&str]) -> (bool, Value) {
    let result = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    let value = serde_json::from_slice(&result.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&result.stderr)}));
    (result.status.success(), value)
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn bundle(root: &Path) -> PathBuf {
    let bundle = root.join("bundle");
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let (ok, result) = run(&[
        "native-bundle",
        "--package",
        package.to_str().unwrap(),
        "--output",
        bundle.to_str().unwrap(),
        "--source-revision",
        "0123456789abcdef0123456789abcdef01234567",
    ]);
    assert!(ok, "{result}");
    bundle
}
fn release(bundle: &Path, root: &Path, name: &str) -> (PathBuf, String, Value) {
    let output = root.join(name);
    let (ok, result) = run(&[
        "native-release",
        "--bundle",
        bundle.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(ok, "{result}");
    let metadata: Value =
        serde_json::from_slice(&fs::read(output.join("release.json")).unwrap()).unwrap();
    let archive = output.join(metadata["archive"].as_str().unwrap());
    let sha = metadata["digest"].as_str().unwrap().to_owned();
    assert_eq!(hash(&fs::read(&archive).unwrap()), sha);
    assert_eq!(
        fs::read_to_string(
            output.join(format!("{}.sha256", metadata["archive"].as_str().unwrap()))
        )
        .unwrap(),
        format!("{}  {}\n", &sha[7..], metadata["archive"].as_str().unwrap())
    );
    (archive, sha, metadata)
}
#[test]
fn deterministic_release_install_restore_and_relocation_offline() {
    let root = tempfile::tempdir().unwrap();
    let bundle = bundle(root.path());
    let (a, sha, ma) = release(&bundle, root.path(), "a");
    let (b, _, mb) = release(&bundle, root.path(), "b");
    assert_eq!(ma, mb);
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert!(a
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("clanker-ui-native-0.1.0-"));
    let wrong_version = root.path().join("wrong-version");
    let error = clanker_ui::native_release::install(
        &clanker_ui::native_release::LocalArchive(&a),
        &sha,
        &wrong_version,
        Some("99.0.0"),
    )
    .unwrap_err();
    assert!(error.contains("version differs"));
    assert!(!wrong_version.exists());
    for command in ["install-native-bundle", "restore-native-bundle"] {
        let output = root.path().join(command);
        let (ok, result) = run(&[
            command,
            "--archive",
            a.to_str().unwrap(),
            "--expected-sha256",
            &sha,
            "--output",
            output.to_str().unwrap(),
        ]);
        assert!(ok, "{result}");
        assert_eq!(result["data"]["compilerExecuted"], false);
        assert_eq!(
            fs::read(bundle.join("manifest.json")).unwrap(),
            fs::read(output.join("manifest.json")).unwrap()
        );
        let relocated = root.path().join(format!("relocated-{command}"));
        fs::rename(&output, &relocated).unwrap();
        let (ok, result) = run(&[
            "verify-native-bundle",
            "--bundle",
            relocated.to_str().unwrap(),
        ]);
        assert!(ok, "{result}");
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 5);
}
#[test]
fn existing_local_directory_install_remains_offline_and_relocatable() {
    let root = tempfile::tempdir().unwrap();
    let bundle = bundle(root.path());
    let out = root.path().join("installed");
    let (ok, result) = run(&[
        "install-native-bundle",
        "--bundle",
        bundle.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{result}");
    assert_eq!(result["data"]["compilerExecuted"], false);
    fs::remove_dir_all(&bundle).unwrap();
    let relocated = root.path().join("relocated");
    fs::rename(out, &relocated).unwrap();
    assert!(
        run(&[
            "verify-native-bundle",
            "--bundle",
            relocated.to_str().unwrap()
        ])
        .0
    );
    let (ok, _) = run(&[
        "install-native-bundle",
        "--bundle",
        relocated.to_str().unwrap(),
        "--output",
        relocated.join("nested").to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(!relocated.join("nested").exists());
}
#[test]
fn validation_never_executes_the_bundled_compiler() {
    let root = tempfile::tempdir().unwrap();
    let bundle = bundle(root.path());
    // Deliberately not an executable format. Byte validation succeeds; execution would fail.
    let bytes = b"inert invalid executable format";
    fs::write(bundle.join("bin/clanker-ui"), bytes).unwrap();
    let manifest = bundle.join("manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    m["executable"]["bytes"] = bytes.len().into();
    m["executable"]["digest"] = hash(bytes).into();
    fs::write(manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    let pin = bundle.join("provider-pin.json");
    let mut p: Value = serde_json::from_slice(&fs::read(&pin).unwrap()).unwrap();
    p["targets"][m["target"].as_str().unwrap()]["digest"] = hash(bytes).into();
    fs::write(pin, serde_json::to_vec(&p).unwrap()).unwrap();
    let (archive, sha, _) = release(&bundle, root.path(), "release");
    let out = root.path().join("installed");
    let (ok, result) = run(&[
        "install-native-bundle",
        "--archive",
        archive.to_str().unwrap(),
        "--expected-sha256",
        &sha,
        "--output",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{result}");
    assert_eq!(fs::read(out.join("bin/clanker-ui")).unwrap(), bytes);
}
#[test]
fn mismatch_malformed_archive_and_no_clobber_leave_no_partial_output() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("archive");
    fs::write(&file, b"not gzip").unwrap();
    let output = root.path().join("output");
    for sha in [hash(b"wrong"), hash(b"not gzip")] {
        let (ok, result) = run(&[
            "install-native-bundle",
            "--archive",
            file.to_str().unwrap(),
            "--expected-sha256",
            &sha,
            "--output",
            output.to_str().unwrap(),
        ]);
        assert!(!ok, "{result}");
        assert!(!output.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel"), b"keep").unwrap();
    let (ok, _) = run(&[
        "restore-native-bundle",
        "--archive",
        file.to_str().unwrap(),
        "--expected-sha256",
        &hash(b"not gzip"),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep");
}
#[test]
fn hosted_setup_requires_explicit_source_version_and_sha_without_network() {
    for args in [
        vec![
            "install-native-bundle",
            "--github-repository",
            "owner/repo",
            "--expected-sha256",
            "sha256:bad",
            "--output",
            "/unused",
        ],
        vec![
            "install-native-bundle",
            "--github-repository",
            "owner/repo",
            "--version",
            "0.1.0",
            "--output",
            "/unused",
        ],
        vec!["install-native-bundle", "--output", "/unused"],
    ] {
        assert!(!run(&args).0);
    }
}
