//! Operator-only release/install boundary. No downloaded executable is ever launched here.
use crate::native_bundle::{self, MAX_EXECUTABLE, MAX_FILE, MAX_INPUTS, MAX_TOTAL};
use flate2::{bufread::GzDecoder, write::GzEncoder, Compression, GzBuilder};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

const MAX_ARCHIVE: u64 = 100 * 1024 * 1024;
const MAX_EXPANDED: u64 =
    MAX_EXECUTABLE + MAX_TOTAL as u64 + (MAX_INPUTS as u64 + 3) * 1024 + 2 * MAX_FILE as u64 + 1024;

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn expected_hash(value: &str) -> Result<(), String> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(
            "expected SHA must be sha256: followed by 64 lowercase hexadecimal digits".into(),
        );
    }
    Ok(())
}

/// Replaceable hosted asset identity; not a registry or a provenance authority.
#[derive(Clone, Debug)]
pub struct GithubRelease {
    pub repository: String,
    pub version: String,
}
impl GithubRelease {
    pub fn url(&self) -> Result<String, String> {
        let parts: Vec<_> = self.repository.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || p.len() > 100
                    || *p == "."
                    || *p == ".."
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            })
            || !native_bundle::valid_tool_version(&self.version)
        {
            return Err(
                "release requires an explicit owner/repository and semantic tool version".into(),
            );
        }
        let target = release_target()?;
        Ok(format!(
            "https://github.com/{}/releases/download/v{}/clanker-ui-native-{}-{}.tar.gz",
            self.repository, self.version, self.version, target
        ))
    }
}

/// Acquisition is invoked only by explicit operator setup/restore commands.
/// Implementations must return bounded archive bytes; the installer rechecks the bound.
pub trait ArchiveSource {
    fn acquire(&self) -> Result<Vec<u8>, String>;
}
pub struct LocalArchive<'a>(pub &'a Path);
impl ArchiveSource for LocalArchive<'_> {
    fn acquire(&self) -> Result<Vec<u8>, String> {
        native_bundle::checked_read(self.0, MAX_ARCHIVE)
    }
}
/// Small HTTPS transport adapter. curl is an operator-installed prerequisite, not a downloaded script.
pub struct GithubArchive(pub GithubRelease);
impl ArchiveSource for GithubArchive {
    fn acquire(&self) -> Result<Vec<u8>, String> {
        let url = self.0.url()?;
        let mut child = Command::new("curl")
            .args([
                "--disable",
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-redirs",
                "5",
                "--connect-timeout",
                "15",
                "--max-time",
                "120",
                "--max-filesize",
            ])
            .arg(MAX_ARCHIVE.to_string())
            .arg("--output")
            .arg("-")
            .arg("--url")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("HTTPS acquisition requires operator-installed curl: {e}"))?;
        let mut bytes = Vec::new();
        // Enforce the streaming budget ourselves, including older curl versions or absent Content-Length.
        let read = child
            .stdout
            .take()
            .ok_or("missing HTTPS transport output")?
            .take(MAX_ARCHIVE + 1)
            .read_to_end(&mut bytes);
        if read.is_err() || bytes.len() as u64 > MAX_ARCHIVE {
            let _ = child.kill();
            let _ = child.wait();
            return Err("HTTPS transport failed or exceeded archive byte budget".into());
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("HTTPS archive acquisition failed: {status}"));
        }
        Ok(bytes)
    }
}

fn release_target() -> Result<&'static str, String> {
    match crate::expand::current_host_target()? {
        target @ ("linux-x86_64" | "macos-aarch64") => Ok(target),
        _ => Err("versioned release candidates are Linux x86_64 and macOS aarch64 only".into()),
    }
}

fn collect(
    root: &Path,
    relative: &str,
    files: &mut Vec<String>,
    count: &mut usize,
) -> Result<(), String> {
    for entry in fs::read_dir(root.join(relative)).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 release path")?;
        let path = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        *count += 1;
        if *count > MAX_INPUTS * 8 + 3
            || !native_bundle::safe_rel(path.strip_prefix("package/").unwrap_or(&path))
        {
            return Err("release source path/count exceeds bundle limits".into());
        }
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_dir() {
            collect(root, &path, files, count)?;
        } else if ty.is_file() {
            files.push(path);
            if files.len() > MAX_INPUTS + 3 {
                return Err("release source file count exceeds bundle limits".into());
            }
        } else {
            return Err("release source contains a link or special file".into());
        }
    }
    Ok(())
}
#[derive(Default)]
struct ByteBudget {
    total: u64,
    package: u64,
}
impl ByteBudget {
    fn admit(&mut self, path: &str, size: u64) -> Result<(), String> {
        let limit = if path == "bin/clanker-ui" {
            MAX_EXECUTABLE
        } else {
            MAX_FILE as u64
        };
        self.total = self
            .total
            .checked_add(size)
            .ok_or("archive byte overflow")?;
        if !matches!(
            path,
            "bin/clanker-ui" | "manifest.json" | "provider-pin.json"
        ) {
            self.package = self
                .package
                .checked_add(size)
                .ok_or("package byte overflow")?;
        }
        if size > limit
            || self.package > MAX_TOTAL as u64
            || self.total > MAX_EXECUTABLE + MAX_TOTAL as u64 + 2 * MAX_FILE as u64
        {
            return Err("archive file/package/total bytes exceed budget".into());
        }
        Ok(())
    }
}

fn archive(bundle: &Path) -> Result<Vec<u8>, String> {
    let encoder: GzEncoder<Vec<u8>> = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let mut files = Vec::new();
    collect(bundle, "", &mut files, &mut 0)?;
    files.sort();
    let mut budget = ByteBudget::default();
    for path in files {
        let limit = if path == "bin/clanker-ui" {
            MAX_EXECUTABLE
        } else {
            MAX_FILE as u64
        };
        let bytes = native_bundle::checked_read(&bundle.join(&path), limit)?;
        budget.admit(&path, bytes.len() as u64)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(if path == "bin/clanker-ui" {
            0o755
        } else {
            0o644
        });
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_path(&path).map_err(|e| e.to_string())?;
        header.set_cksum();
        tar.append(&header, bytes.as_slice())
            .map_err(|e| e.to_string())?;
    }
    let bytes = tar
        .into_inner()
        .map_err(|e| e.to_string())?
        .finish()
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("release archive exceeds byte budget".into());
    }
    Ok(bytes)
}

/// Deterministic versioned assets, atomically published as a fresh directory. Never publishes remotely.
pub fn prepare(bundle: &Path, output: &Path) -> Result<Value, String> {
    release_target()?;
    let identity = native_bundle::verify(bundle)?;
    let (parent, target) = native_bundle::output_destination(output)?;
    let source = fs::canonicalize(bundle).map_err(|e| e.to_string())?;
    if target.starts_with(&source) {
        return Err("release output must be outside bundle".into());
    }
    let bytes = archive(bundle)?;
    prepare_snapshot(&bytes, &identity, &parent, &target)
}

// Only captured bytes enter this boundary. Never re-open the original bundle for the bootstrap.
fn prepare_snapshot(
    bytes: &[u8],
    identity: &Value,
    parent: &Path,
    target: &Path,
) -> Result<Value, String> {
    // Check the snapshot encoded in the archive too; concurrent source changes cannot create an invalid release.
    let stage = tempfile::Builder::new()
        .prefix(".clanker-ui-release-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let validation = stage.path().join("validation");
    fs::create_dir(&validation).map_err(|e| e.to_string())?;
    extract(bytes, &validation)?;
    if &native_bundle::verify(&validation)? != identity {
        return Err("bundle changed during release capture".into());
    }
    let bootstrap =
        native_bundle::checked_read(&validation.join("bin/clanker-ui"), MAX_EXECUTABLE)?;
    let bootstrap_sha = hash(&bootstrap);
    if identity["executableDigest"].as_str() != Some(&bootstrap_sha) {
        return Err("validated snapshot executable digest mismatch".into());
    }
    let notices =
        native_bundle::checked_read(&validation.join("legal/NOTICES.txt"), MAX_FILE as u64)?;
    let notices_sha = hash(&notices);
    if identity["legal"][1]["digest"].as_str() != Some(&notices_sha) {
        return Err("validated snapshot notices digest mismatch".into());
    }
    fs::remove_dir_all(validation).map_err(|e| e.to_string())?;
    let version = identity["toolVersion"].as_str().ok_or("missing version")?;
    let host = identity["target"].as_str().ok_or("missing target")?;
    let name = format!("clanker-ui-native-{version}-{host}.tar.gz");
    let bootstrap_name = format!("clanker-ui-{version}-{host}");
    let metadata_name = format!("clanker-ui-release-{version}-{host}.json");
    let notices_name = format!("{bootstrap_name}.NOTICES.txt");
    let sha = hash(bytes);
    write_asset(stage.path(), &name, bytes, false)?;
    write_asset(
        stage.path(),
        &format!("{name}.sha256"),
        format!("{}  {name}\n", &sha[7..]).as_bytes(),
        false,
    )?;
    write_asset(stage.path(), &bootstrap_name, &bootstrap, true)?;
    write_asset(
        stage.path(),
        &format!("{bootstrap_name}.sha256"),
        format!("{}  {bootstrap_name}\n", &bootstrap_sha[7..]).as_bytes(),
        false,
    )?;
    write_asset(stage.path(), &notices_name, &notices, false)?;
    write_asset(
        stage.path(),
        &format!("{notices_name}.sha256"),
        format!("{}  {notices_name}\n", &notices_sha[7..]).as_bytes(),
        false,
    )?;
    let metadata = json!({
        "schemaVersion":1,"archive":name,"digest":sha,"bundle":identity,
        "notices":{"asset":notices_name,"digest":notices_sha,"bytes":notices.len(),"archivePath":"legal/NOTICES.txt","redistribution":"Must accompany the standalone executable; includes project MIT and separate third-party terms."},
        "bootstrap":{"asset":bootstrap_name,"digest":bootstrap_sha,"bytes":bootstrap.len(),"archivePath":"bin/clanker-ui"},
        "support":{"target":host,"scope":if host == "macos-aarch64" {"Primary Native qualification candidate; separate native host/consumer proof required."} else {"CLI-only candidate subject to producer tests; not full Linux Native builder qualification."}},
        "provenance":"Early-access candidate. Unsigned source revision/identity claims; hashes identify bytes, not producer provenance or execution approval. Publication and target qualification are separate."
    });
    write_asset(
        stage.path(),
        &metadata_name,
        &serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
        false,
    )?;
    native_bundle::publish_noclobber(stage.path(), target)?;
    Ok(json!({"output":target,"metadata":metadata_name,"release":metadata}))
}

fn write_asset(root: &Path, name: &str, bytes: &[u8], executable: bool) -> Result<(), String> {
    let path = root.join(name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(if executable {
            0o755
        } else {
            0o644
        }))
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn extract(bytes: &[u8], root: &Path) -> Result<(), String> {
    let mut decoded = Vec::new();
    let mut gzip = GzDecoder::new(bytes);
    gzip.by_ref()
        .take(MAX_EXPANDED + 1)
        .read_to_end(&mut decoded)
        .map_err(|e| format!("gzip archive: {e}"))?;
    if decoded.len() as u64 > MAX_EXPANDED {
        return Err("expanded archive exceeds byte budget".into());
    }
    if !gzip.into_inner().is_empty() {
        return Err("trailing compressed archive data or concatenated gzip members".into());
    }
    if decoded.len() < 1024
        || decoded.len() % 512 != 0
        || decoded[decoded.len() - 1024..].iter().any(|b| *b != 0)
    {
        return Err("tar archive is truncated or lacks its end markers".into());
    }
    let mut cursor = Cursor::new(decoded);
    let mut names: BTreeMap<String, (String, bool)> = BTreeMap::new();
    let mut count = 0;
    let mut budget = ByteBudget::default();
    {
        let mut tar = tar::Archive::new(&mut cursor);
        // Raw mode rejects GNU/PAX extensions instead of interpreting alternate path/link metadata.
        for entry in tar.entries().map_err(|e| e.to_string())?.raw(true) {
            let mut entry = entry.map_err(|e| format!("tar archive: {e}"))?;
            count += 1;
            if count > MAX_INPUTS + 3 {
                return Err("archive file count exceeds budget".into());
            }
            if !entry.header().entry_type().is_file() {
                return Err(
                    "archive admits regular files only; links, directories and extensions rejected"
                        .into(),
                );
            }
            let raw = entry.path_bytes();
            let path = std::str::from_utf8(&raw)
                .map_err(|_| "non-UTF8 archive path")?
                .to_owned();
            if !native_bundle::safe_rel(path.strip_prefix("package/").unwrap_or(&path)) {
                return Err(format!("unsafe archive path: {path}"));
            }
            let parts: Vec<_> = path.split('/').collect();
            for end in 1..=parts.len() {
                let prefix = parts[..end].join("/");
                let is_file = end == parts.len();
                let key = prefix.to_ascii_lowercase();
                if let Some((spelling, previous_file)) = names.get(&key) {
                    if spelling != &prefix || *previous_file || is_file {
                        return Err(format!("archive path collision: {path}"));
                    }
                } else {
                    names.insert(key, (prefix, is_file));
                }
            }
            let size = entry.size();
            budget.admit(&path, size)?;
            let dest = root.join(&path);
            fs::create_dir_all(dest.parent().ok_or("missing entry parent")?)
                .map_err(|e| e.to_string())?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)
                .map_err(|e| e.to_string())?;
            let copied = std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
            if copied != size {
                return Err("truncated archive entry".into());
            }
            file.flush().map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    dest,
                    fs::Permissions::from_mode(if path == "bin/clanker-ui" {
                        0o755
                    } else {
                        0o644
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    let position = cursor.position() as usize;
    if cursor.get_ref()[position..].iter().any(|b| *b != 0) {
        return Err("nonzero trailing archive data".into());
    }
    Ok(())
}

/// SHA is verified before decompression or writes. Compiler execution requires separate host/operator approval.
/// `version` is additionally checked for hosted restores; local archives remain usable offline.
pub fn install(
    source: &dyn ArchiveSource,
    sha: &str,
    output: &Path,
    version: Option<&str>,
) -> Result<Value, String> {
    expected_hash(sha)?;
    native_bundle::output_destination(output)?; // refuse an existing output before acquisition
    let bytes = source.acquire()?;
    install_bytes(&bytes, sha, output, version, None)
}

/// Existing local unsigned bundle setup, entirely offline. This is byte validation, not a trust grant.
pub fn install_local_bundle(bundle: &Path, output: &Path) -> Result<Value, String> {
    let identity = native_bundle::verify(bundle)?;
    let (_, target) = native_bundle::output_destination(output)?;
    let source = fs::canonicalize(bundle).map_err(|e| e.to_string())?;
    if target.starts_with(source) {
        return Err("install output must be outside the source bundle".into());
    }
    let bytes = archive(bundle)?;
    install_bytes(&bytes, &hash(&bytes), output, None, Some(&identity))
}

fn install_bytes(
    bytes: &[u8],
    sha: &str,
    output: &Path,
    version: Option<&str>,
    identity: Option<&Value>,
) -> Result<Value, String> {
    let (parent, target) = native_bundle::output_destination(output)?;
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("acquired archive exceeds byte budget".into());
    }
    if hash(bytes) != sha {
        return Err("archive SHA-256 mismatch; nothing extracted".into());
    }
    let stage = tempfile::Builder::new()
        .prefix(".clanker-ui-install-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    extract(bytes, stage.path())?;
    let verified = native_bundle::verify(stage.path())?;
    if identity.is_some_and(|expected| expected != &verified) {
        return Err("local bundle changed during capture".into());
    }
    if version.is_some_and(|v| verified["toolVersion"].as_str() != Some(v)) {
        return Err("bundle version differs from requested hosted release".into());
    }
    native_bundle::publish_noclobber(stage.path(), &target)?;
    Ok(
        json!({"output":target,"archiveDigest":sha,"bundle":verified,"compilerExecuted":false,"approval":"Installation verifies bytes only; operator/compiler execution approval is separate."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Bytes(Vec<u8>);
    impl ArchiveSource for Bytes {
        fn acquire(&self) -> Result<Vec<u8>, String> {
            Ok(self.0.clone())
        }
    }
    fn fixture(entries: &[(&str, tar::EntryType, u64)]) -> Vec<u8> {
        let mut tar = tar::Builder::new(Vec::new());
        for (name, kind, size) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(*size);
            h.set_entry_type(*kind);
            h.set_mode(0o777);
            // Deliberately write unsafe paths without set_path's own guard.
            h.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
            h.set_cksum();
            tar.append(&h, vec![0u8; *size as usize].as_slice())
                .unwrap();
        }
        let raw = tar.into_inner().unwrap();
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&raw).unwrap();
        gz.finish().unwrap()
    }
    #[test]
    fn hostile_paths_types_collisions_and_budgets_are_rejected() {
        for entries in [
            vec![("../escape", tar::EntryType::Regular, 1)],
            vec![("/escape", tar::EntryType::Regular, 1)],
            vec![("a\\b", tar::EntryType::Regular, 1)],
            vec![("link", tar::EntryType::Symlink, 0)],
            vec![("link", tar::EntryType::Link, 0)],
            vec![("fifo", tar::EntryType::Fifo, 0)],
            vec![("pax", tar::EntryType::XHeader, 0)],
            vec![
                ("a", tar::EntryType::Regular, 1),
                ("a", tar::EntryType::Regular, 1),
            ],
            vec![
                ("A/file", tar::EntryType::Regular, 1),
                ("a/other", tar::EntryType::Regular, 1),
            ],
            vec![
                ("a", tar::EntryType::Regular, 1),
                ("a/b", tar::EntryType::Regular, 1),
            ],
            vec![("big", tar::EntryType::Regular, MAX_FILE as u64 + 1)],
        ] {
            let root = tempfile::tempdir().unwrap();
            let bytes = fixture(&entries);
            assert!(extract(&bytes, root.path()).is_err(), "{entries:?}");
        }
        let names: Vec<_> = (0..MAX_INPUTS + 4).map(|i| format!("file-{i}")).collect();
        let entries: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), tar::EntryType::Regular, 0))
            .collect();
        let bytes = fixture(&entries);
        assert!(extract(&bytes, tempfile::tempdir().unwrap().path())
            .unwrap_err()
            .contains("file count"));
    }
    #[test]
    fn executable_package_and_total_byte_budgets_stay_bounded() {
        let mut budget = ByteBudget::default();
        budget.admit("bin/clanker-ui", MAX_EXECUTABLE).unwrap();
        budget.admit("manifest.json", MAX_FILE as u64).unwrap();
        budget.admit("provider-pin.json", MAX_FILE as u64).unwrap();
        for _ in 0..MAX_TOTAL / MAX_FILE {
            budget.admit("package/file", MAX_FILE as u64).unwrap();
        }
        assert!(budget.admit("package/extra", 1).is_err());
        assert!(ByteBudget::default()
            .admit("bin/clanker-ui", MAX_EXECUTABLE + 1)
            .is_err());
        assert!(ByteBudget {
            total: u64::MAX,
            package: 0
        }
        .admit("manifest.json", 1)
        .is_err());
        let names: Vec<_> = (0..MAX_TOTAL / MAX_FILE + 1)
            .map(|i| format!("package/file-{i}"))
            .collect();
        let entries: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), tar::EntryType::Regular, MAX_FILE as u64))
            .collect();
        let bytes = fixture(&entries);
        assert!(extract(&bytes, tempfile::tempdir().unwrap().path())
            .unwrap_err()
            .contains("bytes exceed budget"));
    }
    #[test]
    fn gzip_truncation_trailing_members_and_expansion_budget_are_rejected() {
        let original = fixture(&[("file", tar::EntryType::Regular, 1)]);
        for bytes in [
            original[..original.len() - 8].to_vec(),
            [original.as_slice(), b"trailing"].concat(),
            [original.as_slice(), original.as_slice()].concat(),
        ] {
            assert!(extract(&bytes, tempfile::tempdir().unwrap().path()).is_err());
        }
        let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
        let block = vec![0u8; 1024 * 1024];
        for _ in 0..=MAX_EXPANDED / block.len() as u64 {
            gzip.write_all(&block).unwrap();
        }
        let bytes = gzip.finish().unwrap();
        assert!(extract(&bytes, tempfile::tempdir().unwrap().path())
            .unwrap_err()
            .contains("expanded archive"));
    }
    #[test]
    fn injected_acquisition_and_atomic_publication_refuse_destination_races() {
        struct Race<'a>(&'a Path);
        impl ArchiveSource for Race<'_> {
            fn acquire(&self) -> Result<Vec<u8>, String> {
                fs::create_dir(self.0).unwrap();
                fs::write(self.0.join("sentinel"), b"keep").unwrap();
                Ok(b"not gzip".to_vec())
            }
        }
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("output");
        assert!(install(&Race(&out), &hash(b"not gzip"), &out, None)
            .unwrap_err()
            .contains("already exists"));
        assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"keep");
        let stage = tempfile::tempdir_in(parent.path()).unwrap();
        assert!(native_bundle::publish_noclobber(stage.path(), &out).is_err());
        assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"keep");
    }
    #[test]
    fn failures_cleanup_and_existing_destinations_survive() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("install");
        let bytes = fixture(&[("../escape", tar::EntryType::Regular, 1)]);
        assert!(install(&Bytes(bytes.clone()), &hash(b"wrong"), &out, None)
            .unwrap_err()
            .contains("SHA-256 mismatch"));
        assert!(install(&Bytes(bytes.clone()), &hash(&bytes), &out, None).is_err());
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
        fs::create_dir(&out).unwrap();
        fs::write(out.join("keep"), b"keep").unwrap();
        assert!(install(&Bytes(bytes.clone()), &hash(&bytes), &out, None).is_err());
        assert_eq!(fs::read(out.join("keep")).unwrap(), b"keep");
    }
    #[test]
    fn release_bootstrap_uses_only_captured_snapshot_and_rejects_changed_capture() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("bundle");
        native_bundle::prepare(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla"),
            &bundle,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .unwrap();
        let identity = native_bundle::verify(&bundle).unwrap();
        let captured = archive(&bundle).unwrap();
        let original = fs::read(bundle.join("bin/clanker-ui")).unwrap();
        // Same-length tamper after initial verification: a new capture must fail validation.
        let mut tampered = original.clone();
        tampered[0] ^= 1;
        fs::write(bundle.join("bin/clanker-ui"), &tampered).unwrap();
        let changed = archive(&bundle).unwrap();
        let out = root.path().join("changed");
        assert!(prepare_snapshot(&changed, &identity, root.path(), &out).is_err());
        assert!(!out.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);

        // Even a self-consistent but different source identity must be refused.
        fs::write(bundle.join("bin/clanker-ui"), &original).unwrap();
        let manifest_path = bundle.join("manifest.json");
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["sourceRevision"] = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        native_bundle::verify(&bundle).unwrap();
        assert!(
            prepare_snapshot(&archive(&bundle).unwrap(), &identity, root.path(), &out)
                .unwrap_err()
                .contains("changed during release capture")
        );
        assert!(!out.exists());
        fs::remove_dir_all(&bundle).unwrap();

        // Deleting the source after capture cannot change the bootstrap or metadata.
        let result = prepare_snapshot(&captured, &identity, root.path(), &out).unwrap();
        let bootstrap = result["release"]["bootstrap"]["asset"].as_str().unwrap();
        assert_eq!(fs::read(out.join(bootstrap)).unwrap(), original);
        assert_eq!(result["release"]["bundle"], identity);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 7);
        let notices = result["release"]["notices"]["asset"].as_str().unwrap();
        assert_eq!(
            fs::read(out.join(notices)).unwrap(),
            include_bytes!("../../../NOTICES.txt")
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);

        // Simulate a destination appearing after preflight/capture; publication remains no-replace.
        let race = root.path().join("race");
        fs::create_dir(&race).unwrap();
        fs::write(race.join("sentinel"), b"keep").unwrap();
        assert!(prepare_snapshot(&captured, &identity, root.path(), &race)
            .unwrap_err()
            .contains("without clobbering"));
        assert_eq!(fs::read(race.join("sentinel")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
    #[test]
    fn hosted_identity_is_explicit_and_has_no_latest_or_arbitrary_url() {
        let release = GithubRelease {
            repository: "owner/repo".into(),
            version: "0.1.0".into(),
        };
        assert!(release
            .url()
            .unwrap()
            .contains("/releases/download/v0.1.0/clanker-ui-native-0.1.0-"));
        for repository in [
            "../repo",
            "https://evil/repo",
            "owner/repo?x",
            "owner/repo/extra",
        ] {
            assert!(GithubRelease {
                repository: repository.into(),
                ..release.clone()
            }
            .url()
            .is_err());
        }
        assert!(GithubRelease {
            version: "latest".into(),
            ..release
        }
        .url()
        .is_err());
        assert!(expected_hash("sha256:bad").is_err());
    }
}
