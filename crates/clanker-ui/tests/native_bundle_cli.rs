use serde_json::Value;
use sha2::Digest;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clanker-ui"))
}
fn run(args: &[&str]) -> (bool, Value) {
    let output = Command::new(cli()).args(args).output().unwrap();
    let value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stdout)));
    (output.status.success(), value)
}
fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla")
}
fn revision() -> &'static str {
    "0123456789abcdef0123456789abcdef01234567"
}
fn prepare(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    let (ok, result) = run(&[
        "native-bundle",
        "--package",
        package().to_str().unwrap(),
        "--output",
        path.to_str().unwrap(),
        "--source-revision",
        revision(),
    ]);
    assert!(ok, "{result}");
    path
}
fn verify(bundle: &Path) -> (bool, Value) {
    run(&["verify-native-bundle", "--bundle", bundle.to_str().unwrap()])
}
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[test]
fn deterministic_relocatable_bundle_contains_complete_contracts_and_relative_pin() {
    let temp = tempfile::tempdir().unwrap();
    let a = prepare(temp.path(), "a");
    let b = prepare(temp.path(), "b");
    let (ok, result) = verify(&a);
    assert!(ok, "{result}");
    let ma: Value = serde_json::from_slice(&fs::read(a.join("manifest.json")).unwrap()).unwrap();
    let mb: Value = serde_json::from_slice(&fs::read(b.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(ma, mb);
    assert_eq!(
        fs::read(a.join("provider-pin.json")).unwrap(),
        fs::read(b.join("provider-pin.json")).unwrap()
    );
    assert_eq!(
        fs::read(a.join("bin/clanker-ui")).unwrap(),
        fs::read(b.join("bin/clanker-ui")).unwrap()
    );
    assert_eq!(ma["sourceRevision"], revision());
    assert_eq!(ma["assemblyProtocol"], 2);
    assert_eq!(ma["bindingAbi"], 2);
    assert_eq!(ma["templateEngine"], "minijinja-2.12.0");
    let legal = ma["legal"].as_array().unwrap();
    assert_eq!(legal.len(), 2);
    for (entry, (path, expected)) in legal.iter().zip([
        (
            "legal/LICENSE",
            include_bytes!("../../../LICENSE").as_slice(),
        ),
        (
            "legal/NOTICES.txt",
            include_bytes!("../../../NOTICES.txt").as_slice(),
        ),
    ]) {
        assert_eq!(entry["path"], path);
        let bytes = fs::read(a.join(path)).unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(entry["bytes"], bytes.len());
        assert_eq!(
            entry["digest"],
            format!("sha256:{:x}", sha2::Sha256::digest(&bytes))
        );
        assert_eq!(fs::read(b.join(path)).unwrap(), bytes);
    }
    let lock: Value = serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/button-app/ui.lock.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(ma["package"]["digest"], lock["package"]["digest"]);
    assert_eq!(ma["entries"], lock["package"]["inputs"]);
    let entries = ma["entries"].as_array().unwrap();
    assert!(entries
        .iter()
        .any(|e| e["path"].as_str().unwrap().ends_with(".d.ts")));
    let pin: Value =
        serde_json::from_slice(&fs::read(a.join("provider-pin.json")).unwrap()).unwrap();
    let target = ma["target"].as_str().unwrap();
    assert_eq!(pin["targets"][target]["executable"], "bin/clanker-ui");
    assert_eq!(pin["targets"][target]["digest"], ma["executable"]["digest"]);
    assert!(!a.join("app.html").exists());
    let relocated = temp.path().join("relocated");
    fs::rename(&a, &relocated).unwrap();
    let (ok, result) = verify(&relocated);
    assert!(ok, "{result}");
    let bytes = fs::read(relocated.join("bin/clanker-ui")).unwrap();
    assert_eq!(
        format!("sha256:{:x}", sha2::Sha256::digest(bytes)),
        ma["executable"]["digest"]
    );
}

#[test]
fn verifier_rejects_tampering_extra_missing_wrong_target_and_pin_mismatch() {
    let temp = tempfile::tempdir().unwrap();
    let base = prepare(temp.path(), "base");
    for (name, mutation) in [
        ("target", 0),
        ("pin", 1),
        ("package-digest", 2),
        ("package-bytes", 3),
        ("executable-bytes", 4),
        ("traversal", 5),
        ("missing", 6),
        ("extra", 7),
        ("symlink", 8),
        ("legal-tamper", 9),
        ("legal-missing", 10),
        ("legal-legacy-shape", 11),
        ("legal-extra", 12),
        ("legal-path", 13),
        ("legal-oversize", 14),
        ("legal-reordered", 15),
    ] {
        let copy = temp.path().join(name);
        copy_tree(&base, &copy);
        match mutation {
            0 => {
                let path = copy.join("manifest.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                v["target"] = "wrong-platform".into();
                fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            1 => {
                let path = copy.join("provider-pin.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                let target = v["targets"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .clone();
                v["targets"][target]["digest"] = "sha256:bad".into();
                fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            2 => {
                let path = copy.join("manifest.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                v["package"]["digest"] = "sha256:bad".into();
                fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            3 => {
                let manifest: Value =
                    serde_json::from_slice(&fs::read(copy.join("manifest.json")).unwrap()).unwrap();
                let path = copy
                    .join("package")
                    .join(manifest["entries"][0]["path"].as_str().unwrap());
                fs::write(path, b"tampered").unwrap();
            }
            4 => {
                let path = copy.join("bin/clanker-ui");
                let mut bytes = fs::read(&path).unwrap();
                bytes[0] ^= 1;
                fs::write(path, bytes).unwrap();
            }
            5 => {
                let path = copy.join("manifest.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                v["entries"][0]["path"] = "../../outside".into();
                fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            6 => {
                let manifest: Value =
                    serde_json::from_slice(&fs::read(copy.join("manifest.json")).unwrap()).unwrap();
                let path = copy
                    .join("package")
                    .join(manifest["entries"][0]["path"].as_str().unwrap());
                fs::remove_file(path).unwrap();
            }
            7 => fs::write(copy.join("extra.txt"), b"extra").unwrap(),
            8 => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(copy.join("manifest.json"), copy.join("intruder"))
                    .unwrap();
            }
            9 => fs::write(copy.join("legal/NOTICES.txt"), b"tampered").unwrap(),
            10 => fs::remove_file(copy.join("legal/LICENSE")).unwrap(),
            11..=15 => {
                let path = copy.join("manifest.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                match mutation {
                    11 => {
                        v.as_object_mut().unwrap().remove("legal");
                    }
                    12 => {
                        let extra = v["legal"][0].clone();
                        v["legal"].as_array_mut().unwrap().push(extra);
                    }
                    13 => v["legal"][0]["path"] = "package/LICENSE".into(),
                    14 => v["legal"][0]["bytes"] = (1024 * 1024 + 1).into(),
                    15 => v["legal"].as_array_mut().unwrap().swap(0, 1),
                    _ => unreachable!(),
                }
                fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        let (ok, error) = verify(&copy);
        assert!(!ok, "mutation {name} unexpectedly passed: {error}");
    }
}

#[test]
fn preparation_rejects_bad_revision_clobber_source_nesting_and_source_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let source = package();
    let output = temp.path().join("bad-revision");
    let (ok, _) = run(&[
        "native-bundle",
        "--package",
        source.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--source-revision",
        "short",
    ]);
    assert!(!ok);
    let clobber = temp.path().join("exists");
    fs::create_dir(&clobber).unwrap();
    let (ok, _) = run(&[
        "native-bundle",
        "--package",
        source.to_str().unwrap(),
        "--output",
        clobber.to_str().unwrap(),
        "--source-revision",
        revision(),
    ]);
    assert!(!ok);
    #[cfg(unix)]
    {
        let dangling = temp.path().join("dangling-output");
        std::os::unix::fs::symlink(temp.path().join("absent"), &dangling).unwrap();
        let (ok, _) = run(&[
            "native-bundle",
            "--package",
            source.to_str().unwrap(),
            "--output",
            dangling.to_str().unwrap(),
            "--source-revision",
            revision(),
        ]);
        assert!(!ok);
    }
    let nested = source.join("bundle-must-not-be-created");
    let (ok, _) = run(&[
        "native-bundle",
        "--package",
        source.to_str().unwrap(),
        "--output",
        nested.to_str().unwrap(),
        "--source-revision",
        revision(),
    ]);
    assert!(!ok);
    let src = temp.path().join("source");
    copy_tree(&source, &src);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(src.join("ui-package.json"), src.join("linked.json")).unwrap();
    }
    // A declared-path symlink is rejected by LocalPackage; replacing a declared theme catches it directly.
    #[cfg(unix)]
    {
        let theme = src.join("theme/default.css");
        let real = temp.path().join("real.css");
        fs::rename(&theme, &real).unwrap();
        std::os::unix::fs::symlink(&real, &theme).unwrap();
    }
    let destination = temp.path().join("symlink-source-bundle");
    let (ok, error) = run(&[
        "native-bundle",
        "--package",
        src.to_str().unwrap(),
        "--output",
        destination.to_str().unwrap(),
        "--source-revision",
        revision(),
    ]);
    assert!(!ok, "{error}");
}

#[test]
fn capabilities_describe_only_the_supported_local_bundle_protocol() {
    let (ok, result) = run(&["capabilities"]);
    assert!(ok, "{result}");
    assert_eq!(result["data"]["adapterProtocol"], 2);
    assert_eq!(result["data"]["bindingAbi"], 2);
    assert_eq!(
        result["data"]["nativeBundle"]["ciArtifactTargets"][0],
        "linux-x86_64"
    );
    assert!(!result["data"]["unsupported"].as_array().unwrap().is_empty());
}
