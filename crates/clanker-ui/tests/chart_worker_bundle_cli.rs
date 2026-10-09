use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn run(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn bundle(root: &Path) -> std::path::PathBuf {
    let worker = root.join("reviewed-worker");
    // Deliberately not runnable: packaging, verification and restore must never execute it.
    fs::write(&worker, b"opaque reviewed worker bytes").unwrap();
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let output = root.join("bundle");
    run(&[
        "native-bundle",
        "--package",
        package.to_str().unwrap(),
        "--chart-worker",
        worker.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--source-revision",
        "0123456789abcdef0123456789abcdef01234567",
    ]);
    output
}

#[test]
fn worker_survives_archive_install_and_relocation_without_execution_or_source_lookup() {
    let root = tempfile::tempdir().unwrap();
    let source = bundle(root.path());
    let verified = run(&["verify-native-bundle", "--bundle", source.to_str().unwrap()]);
    let worker = &verified["data"]["presentationWorker"];
    assert_eq!(worker["abi"], 1);
    assert_eq!(worker["executable"]["path"], "bin/clanker-chart-worker");
    assert_eq!(
        worker["executable"]["digest"],
        format!(
            "sha256:{:x}",
            Sha256::digest(b"opaque reviewed worker bytes")
        )
    );
    let contract: Value = serde_json::from_slice(
        &fs::read(source.join("package/components/chart/renderer-contract.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(worker["contracts"], contract["renderers"]);
    assert!(!source.join("presentation-pin.json").exists());

    let assets = root.path().join("release");
    let released = run(&[
        "native-release",
        "--bundle",
        source.to_str().unwrap(),
        "--output",
        assets.to_str().unwrap(),
    ]);
    let release = &released["data"]["release"];
    assert_eq!(release["presentationWorker"], *worker);
    let archive = assets.join(release["archive"].as_str().unwrap());
    let bootstrap = assets.join(release["bootstrap"]["asset"].as_str().unwrap());
    let sha = release["digest"].as_str().unwrap();
    fs::remove_dir_all(&source).unwrap();
    fs::remove_file(root.path().join("reviewed-worker")).unwrap();
    let install = root.path().join("installed");
    let result = Command::new(&bootstrap)
        .args([
            "restore-native-bundle",
            "--archive",
            archive.to_str().unwrap(),
            "--expected-sha256",
            sha,
            "--output",
            install.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["data"]["compilerExecuted"], false);
    assert_eq!(result["data"]["presentationWorkerExecuted"], false);
    assert_eq!(
        fs::read(install.join("bin/clanker-chart-worker")).unwrap(),
        b"opaque reviewed worker bytes"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(install.join("bin/clanker-chart-worker"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    let relocated = root.path().join("relocated");
    fs::rename(&install, &relocated).unwrap();
    let verified = Command::new(relocated.join("bin/clanker-ui"))
        .args([
            "verify-native-bundle",
            "--bundle",
            relocated.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(verified.status.success());
    let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verified["data"]["presentationWorker"], *worker);
}

#[test]
fn worker_identity_contract_paths_modes_and_bytes_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let source = bundle(root.path());
    let manifest_path = source.join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    for (field, value) in [
        ("abi", json!(2)),
        ("contracts", json!({})),
        (
            "executable",
            json!({"path":"../worker", "bytes":27, "digest":"sha256:wrong"}),
        ),
        (
            "executable",
            json!({"path":"bin/clanker-chart-worker", "bytes":0, "digest":"sha256:wrong"}),
        ),
        (
            "executable",
            json!({"path":"bin/clanker-chart-worker", "bytes":67108865u64, "digest":"sha256:wrong"}),
        ),
    ] {
        let mut manifest: Value = serde_json::from_slice(&original).unwrap();
        manifest["presentationWorker"][field] = value;
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(
            clanker_ui::native_bundle::verify(&source).is_err(),
            "{field}"
        );
    }
    fs::write(&manifest_path, original).unwrap();
    let worker = source.join("bin/clanker-chart-worker");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(clanker_ui::native_bundle::verify(&source).is_err());
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(&worker, b"tampered").unwrap();
    assert!(clanker_ui::native_bundle::verify(&source).is_err());
    fs::remove_file(&worker).unwrap();
    assert!(clanker_ui::native_bundle::verify(&source).is_err());
}
