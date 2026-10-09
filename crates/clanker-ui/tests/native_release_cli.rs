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
    let metadata_name = result["data"]["metadata"].as_str().unwrap();
    let metadata: Value =
        serde_json::from_slice(&fs::read(output.join(metadata_name)).unwrap()).unwrap();
    let version = metadata["bundle"]["toolVersion"].as_str().unwrap();
    let target = metadata["bundle"]["target"].as_str().unwrap();
    assert_eq!(
        metadata_name,
        format!("clanker-ui-release-{version}-{target}.json")
    );
    assert!(!output.join("release.json").exists());
    // Generic bundle metadata must not advertise vanilla-specific inventory counts.
    for key in ["componentComplete", "nativeSupported", "adapterRequired"] {
        assert!(metadata["support"].get(key).is_none());
    }
    let bootstrap_name = metadata["bootstrap"]["asset"].as_str().unwrap();
    assert_eq!(bootstrap_name, format!("clanker-ui-{version}-{target}"));
    let bootstrap = fs::read(output.join(bootstrap_name)).unwrap();
    assert!(bootstrap.len() <= 64 * 1024 * 1024);
    assert_eq!(metadata["bootstrap"]["bytes"], bootstrap.len());
    assert_eq!(metadata["bootstrap"]["digest"], hash(&bootstrap));
    assert_eq!(
        metadata["bootstrap"]["digest"],
        metadata["bundle"]["executableDigest"]
    );
    assert_eq!(metadata["bootstrap"]["archivePath"], "bin/clanker-ui");
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        metadata["bootstrap"]["digest"],
        manifest["executable"]["digest"]
    );
    assert_eq!(
        metadata["bootstrap"]["bytes"],
        manifest["executable"]["bytes"]
    );
    assert_eq!(
        fs::read_to_string(output.join(format!("{bootstrap_name}.sha256"))).unwrap(),
        format!("{}  {bootstrap_name}\n", &hash(&bootstrap)[7..])
    );
    let notices_name = metadata["notices"]["asset"].as_str().unwrap();
    assert_eq!(notices_name, format!("{bootstrap_name}.NOTICES.txt"));
    let notices = fs::read(output.join(notices_name)).unwrap();
    assert_eq!(notices, include_bytes!("../../../NOTICES.txt"));
    assert_eq!(metadata["notices"]["digest"], hash(&notices));
    assert_eq!(metadata["notices"]["bytes"], notices.len());
    assert_eq!(
        metadata["notices"]["digest"],
        manifest["legal"][1]["digest"]
    );
    assert_eq!(
        fs::read_to_string(output.join(format!("{notices_name}.sha256"))).unwrap(),
        format!("{}  {notices_name}\n", &hash(&notices)[7..])
    );
    let mut names = fs::read_dir(&output)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    names.sort();
    let mut expected = vec![
        metadata_name.to_owned(),
        bootstrap_name.to_owned(),
        format!("{bootstrap_name}.sha256"),
        metadata["archive"].as_str().unwrap().to_owned(),
        format!("{}.sha256", metadata["archive"].as_str().unwrap()),
        notices_name.to_owned(),
        format!("{notices_name}.sha256"),
    ];
    expected.sort();
    assert_eq!(names, expected);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in &names {
            assert_eq!(
                fs::metadata(output.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                if name == bootstrap_name { 0o755 } else { 0o644 }
            );
        }
    }
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
    for entry in fs::read_dir(a.parent().unwrap()).unwrap() {
        let name = entry.unwrap().file_name();
        assert_eq!(
            fs::read(a.parent().unwrap().join(&name)).unwrap(),
            fs::read(b.parent().unwrap().join(&name)).unwrap()
        );
    }
    let bootstrap = a
        .parent()
        .unwrap()
        .join(ma["bootstrap"]["asset"].as_str().unwrap());
    assert!(a
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with(&format!("clanker-ui-native-{}-", env!("CARGO_PKG_VERSION"))));
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
        // This is an explicit test approval to execute the captured bootstrap for first restore.
        let result = Command::new(&bootstrap)
            .args([
                command,
                "--archive",
                a.to_str().unwrap(),
                "--expected-sha256",
                &sha,
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let result: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(result["data"]["compilerExecuted"], false);
        assert_eq!(
            fs::read(&bootstrap).unwrap(),
            fs::read(output.join("bin/clanker-ui")).unwrap()
        );
        assert_eq!(
            fs::read(output.join("legal/LICENSE")).unwrap(),
            include_bytes!("../../../LICENSE")
        );
        assert_eq!(
            fs::read(output.join("legal/NOTICES.txt")).unwrap(),
            fs::read(
                a.parent()
                    .unwrap()
                    .join(ma["notices"]["asset"].as_str().unwrap())
            )
            .unwrap()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(output.join("bin/clanker-ui"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
        }
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
fn release_refuses_tamper_oversize_nested_and_existing_outputs_without_partial_assets() {
    let root = tempfile::tempdir().unwrap();
    let bundle = bundle(root.path());
    let (archive, _, _) = release(&bundle, root.path(), "release");
    let output = archive.parent().unwrap();
    let original = fs::read(&archive).unwrap();
    let (ok, _) = run(&[
        "native-release",
        "--bundle",
        bundle.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert_eq!(fs::read(&archive).unwrap(), original);
    assert_eq!(fs::read_dir(output).unwrap().count(), 7);
    let nested = bundle.join("nested");
    assert!(
        !run(&[
            "native-release",
            "--bundle",
            bundle.to_str().unwrap(),
            "--output",
            nested.to_str().unwrap()
        ])
        .0
    );
    assert!(!nested.exists());
    #[cfg(unix)]
    {
        let link = root.path().join("dangling");
        std::os::unix::fs::symlink("missing", &link).unwrap();
        assert!(
            !run(&[
                "native-release",
                "--bundle",
                bundle.to_str().unwrap(),
                "--output",
                link.to_str().unwrap()
            ])
            .0
        );
        assert_eq!(fs::read_link(&link).unwrap(), Path::new("missing"));
        fs::remove_file(link).unwrap();
    }
    let binary = bundle.join("bin/clanker-ui");
    let mut bytes = fs::read(&binary).unwrap();
    bytes[0] ^= 1;
    fs::write(&binary, bytes).unwrap();
    let bad = root.path().join("bad");
    assert!(
        !run(&[
            "native-release",
            "--bundle",
            bundle.to_str().unwrap(),
            "--output",
            bad.to_str().unwrap()
        ])
        .0
    );
    assert!(!bad.exists());
    fs::OpenOptions::new()
        .write(true)
        .open(binary)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert!(
        !run(&[
            "native-release",
            "--bundle",
            bundle.to_str().unwrap(),
            "--output",
            bad.to_str().unwrap()
        ])
        .0
    );
    assert!(!bad.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
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
