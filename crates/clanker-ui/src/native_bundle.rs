//! Strict relocatable directory bundles for explicitly operator-approved Native tools.
use crate::{
    expand::{self, LockedInput},
    local::LocalPackage,
    ports::PackageSource,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) const MAX_EXECUTABLE: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_INPUTS: usize = 4096;
pub(crate) const MAX_TOTAL: usize = 32 * 1024 * 1024;
pub(crate) const MAX_FILE: usize = 1024 * 1024;

// Closed, source-reviewed legal closure, outside the unchanged catalog inputs.
const LEGAL: [(&str, &[u8]); 2] = [
    ("legal/LICENSE", include_bytes!("../../../LICENSE")),
    ("legal/NOTICES.txt", include_bytes!("../../../NOTICES.txt")),
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: usize,
    digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundleManifest {
    schema_version: u32,
    target: String,
    tool_version: String,
    source_revision: String,
    provider: String,
    assembly_protocol: u32,
    binding_abi: u32,
    template_engine: String,
    executable: Entry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    presentation_worker: Option<PresentationWorker>,
    package: BundlePackage,
    legal: Vec<Entry>,
    entries: Vec<Entry>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundlePackage {
    name: String,
    version: String,
    digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationWorker {
    abi: u32,
    executable: Entry,
    contracts: serde_json::Value,
}

pub(crate) fn is_executable(path: &str) -> bool {
    matches!(path, "bin/clanker-ui" | "bin/clanker-chart-worker")
}

fn chart_contract(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if value["schemaVersion"] != 1
        || value.as_object().is_none_or(|v| v.len() != 2)
        || value["renderers"]
            .as_object()
            .is_none_or(|v| v.len() != 1 || !v.contains_key("echarts_chart_v1"))
    {
        return Err("invalid locked chart renderer contract".into());
    }
    Ok(value["renderers"].clone())
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pin {
    schema_version: u32,
    provider: String,
    assembly_protocol: u32,
    binding_abi: u32,
    targets: std::collections::BTreeMap<String, PinTarget>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PinTarget {
    executable: String,
    digest: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn valid_tool_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let identifiers = |text: &str, prerelease: bool| {
        text.split('.').all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !(prerelease
                    && part.len() > 1
                    && part.starts_with('0')
                    && part.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    let (release, build) = value
        .split_once('+')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    let (core, prerelease) = release
        .split_once('-')
        .map_or((release, None), |(a, b)| (a, Some(b)));
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        })
        && prerelease.is_none_or(|part| identifiers(part, true))
        && build.is_none_or(|part| identifiers(part, false))
}
pub(crate) fn safe_rel(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.contains('\\')
        && !Path::new(value).is_absolute()
        && value.split('/').count() <= 8
        && value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                && !p.ends_with(' ')
        })
}
fn check_source_tree(directory: &Path, depth: usize, count: &mut usize) -> Result<(), String> {
    if depth > 8 {
        return Err("package source path depth exceeds bundle limits".into());
    }
    for entry in fs::read_dir(directory).map_err(|e| format!("{}: {e}", directory.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *count += 1;
        if *count > MAX_INPUTS * 8 {
            return Err("package source entry count exceeds bundle limits".into());
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "symlink not allowed in package source: {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            check_source_tree(&entry.path(), depth + 1, count)?;
        } else if !kind.is_file() {
            return Err(format!(
                "special file not allowed in package source: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn checked_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit {
        return Err(format!(
            "not a regular file within {limit} bytes: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 != meta.len() || bytes.len() as u64 > limit {
        return Err(format!(
            "file changed or exceeds budget: {}",
            path.display()
        ));
    }
    Ok(bytes)
}
fn input_entries(inputs: &[LockedInput]) -> Vec<Entry> {
    inputs
        .iter()
        .map(|i| Entry {
            path: i.path.clone(),
            bytes: i.bytes,
            digest: i.digest.clone(),
        })
        .collect()
}
fn input_digest(inputs: &[LockedInput]) -> String {
    expand::input_manifest_digest(inputs)
}
pub(crate) fn publish_noclobber(staged: &Path, output: &Path) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let staged =
            CString::new(staged.as_os_str().as_bytes()).map_err(|_| "staging path contains NUL")?;
        let output =
            CString::new(output.as_os_str().as_bytes()).map_err(|_| "output path contains NUL")?;
        #[cfg(target_os = "linux")]
        let result = {
            use std::os::raw::{c_char, c_int};
            extern "C" {
                fn renameat2(
                    oldfd: c_int,
                    old: *const c_char,
                    newfd: c_int,
                    new: *const c_char,
                    flags: c_int,
                ) -> c_int;
            }
            // RENAME_NOREPLACE makes publication atomic and refuses a destination created after preflight.
            unsafe { renameat2(-100, staged.as_ptr(), -100, output.as_ptr(), 1) }
        };
        #[cfg(target_os = "macos")]
        let result = {
            use std::os::raw::{c_char, c_int};
            extern "C" {
                fn renamex_np(old: *const c_char, new: *const c_char, flags: c_int) -> c_int;
            }
            // RENAME_EXCL is the macOS equivalent of no-replace atomic rename.
            unsafe { renamex_np(staged.as_ptr(), output.as_ptr(), 0x0000_0004) }
        };
        if result != 0 {
            return Err(format!(
                "cannot atomically publish bundle without clobbering: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (staged, output);
        Err("atomic no-clobber bundle publication is unsupported on this host".into())
    }
}

pub(crate) fn output_destination(output: &Path) -> Result<(PathBuf, PathBuf), String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let leaf = output
        .file_name()
        .ok_or("bundle output needs a directory name")?;
    let target = parent.join(leaf);
    if fs::symlink_metadata(&target).is_ok() {
        return Err("bundle output already exists; choose a fresh directory".into());
    }
    Ok((parent, target))
}

/// Prepare an unsigned, relocatable artifact. It is not a trust grant; callers must approve its source and bytes.
pub fn prepare(
    package_path: &Path,
    output: &Path,
    source_revision: &str,
) -> Result<serde_json::Value, String> {
    prepare_with_worker(package_path, output, source_revision, None)
}

/// Capture an explicitly reviewed worker without executing it. Installation is not execution approval.
pub fn prepare_with_worker(
    package_path: &Path,
    output: &Path,
    source_revision: &str,
    chart_worker: Option<&Path>,
) -> Result<serde_json::Value, String> {
    if !valid_revision(source_revision) {
        return Err("source revision must be exactly 40 hexadecimal characters".into());
    }
    let source_meta = fs::symlink_metadata(package_path)
        .map_err(|e| format!("{}: {e}", package_path.display()))?;
    if !source_meta.is_dir() || source_meta.file_type().is_symlink() {
        return Err("package source must be a real directory, not a symlink".into());
    }
    let source =
        fs::canonicalize(package_path).map_err(|e| format!("{}: {e}", package_path.display()))?;
    check_source_tree(&source, 0, &mut 0)?;
    let mut package = LocalPackage.load(&source)?;
    LocalPackage.complete_declared_inputs(&mut package)?;
    let inputs = expand::package_inputs(&package.assets)?;
    let legal: Vec<_> = LEGAL
        .iter()
        .map(|(path, bytes)| Entry {
            path: (*path).into(),
            bytes: bytes.len(),
            digest: digest(bytes),
        })
        .collect();
    if LEGAL
        .iter()
        .any(|(_, bytes)| bytes.is_empty() || bytes.len() > MAX_FILE)
        || inputs.len() + legal.len() > MAX_INPUTS
        || package.assets.values().map(Vec::len).sum::<usize>()
            + legal.iter().map(|entry| entry.bytes).sum::<usize>()
            > MAX_TOTAL
    {
        return Err("package declared input closure exceeds bundle limits".into());
    }
    let (parent, target) = output_destination(output)?;
    if target.starts_with(&source) {
        return Err("bundle output must not be inside the package source tree".into());
    }
    let executable_path = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let executable = checked_read(&executable_path, MAX_EXECUTABLE)?;
    let worker_bytes = chart_worker
        .map(|path| checked_read(path, MAX_EXECUTABLE))
        .transpose()?;
    if executable.is_empty()
        || worker_bytes.as_ref().is_some_and(Vec::is_empty)
        || executable.len() as u64 + worker_bytes.as_ref().map_or(0, |v| v.len() as u64)
            > MAX_EXECUTABLE
        || inputs.len() + legal.len() + usize::from(worker_bytes.is_some()) > MAX_INPUTS
    {
        return Err("bundle executable closure exceeds existing limits".into());
    }
    let presentation_worker = worker_bytes
        .as_ref()
        .map(|bytes| {
            let contract = package
                .assets
                .get("components/chart/renderer-contract.json")
                .ok_or("chart worker requires the locked chart contract")?;
            Ok::<_, String>(PresentationWorker {
                abi: 1,
                executable: Entry {
                    path: "bin/clanker-chart-worker".into(),
                    bytes: bytes.len(),
                    digest: digest(bytes),
                },
                contracts: chart_contract(contract)?,
            })
        })
        .transpose()?;
    let target_name = expand::current_host_target()?;
    let manifest = BundleManifest {
        schema_version: 1,
        target: target_name.into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        source_revision: source_revision.to_ascii_lowercase(),
        provider: "clanker-ui.native".into(),
        assembly_protocol: 2,
        binding_abi: 2,
        template_engine: "minijinja-2.12.0".into(),
        executable: Entry {
            path: "bin/clanker-ui".into(),
            bytes: executable.len(),
            digest: digest(&executable),
        },
        presentation_worker,
        package: BundlePackage {
            name: package.catalog.package.name.clone(),
            version: package.catalog.package.version.clone(),
            digest: input_digest(&inputs),
        },
        legal,
        entries: input_entries(&inputs),
    };
    let pin = Pin {
        schema_version: 1,
        provider: "clanker-ui.native".into(),
        assembly_protocol: 2,
        binding_abi: 2,
        targets: [(
            target_name.into(),
            PinTarget {
                executable: "bin/clanker-ui".into(),
                digest: digest(&executable),
            },
        )]
        .into_iter()
        .collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let pin_bytes = serde_json::to_vec_pretty(&pin).map_err(|e| e.to_string())?;
    if manifest_bytes.len() > MAX_FILE || pin_bytes.len() > MAX_FILE {
        return Err("bundle metadata exceeds the 1 MiB file limit".into());
    }
    let stage = tempfile::Builder::new()
        .prefix(".clanker-ui-bundle-")
        .tempdir_in(&parent)
        .map_err(|e| e.to_string())?;
    fs::create_dir(stage.path().join("bin")).map_err(|e| e.to_string())?;
    fs::create_dir(stage.path().join("package")).map_err(|e| e.to_string())?;
    fs::create_dir(stage.path().join("legal")).map_err(|e| e.to_string())?;
    for (path, bytes) in LEGAL {
        fs::write(stage.path().join(path), bytes).map_err(|e| e.to_string())?;
    }
    fs::write(stage.path().join("manifest.json"), manifest_bytes).map_err(|e| e.to_string())?;
    fs::write(stage.path().join("provider-pin.json"), pin_bytes).map_err(|e| e.to_string())?;
    let binary = stage.path().join("bin/clanker-ui");
    fs::write(&binary, &executable).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    if let Some(bytes) = &worker_bytes {
        let binary = stage.path().join("bin/clanker-chart-worker");
        fs::write(&binary, bytes).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
    }
    for (relative, bytes) in &package.assets {
        if !safe_rel(relative) || bytes.len() > MAX_FILE {
            return Err(format!("unsafe or oversized package input: {relative}"));
        }
        let path = stage.path().join("package").join(relative);
        let parent = path.parent().ok_or("invalid package entry")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    // Re-read the captured source closure before publication; any concurrent edit aborts.
    let mut after = LocalPackage.load(&source)?;
    LocalPackage.complete_declared_inputs(&mut after)?;
    if after.assets != package.assets {
        return Err("package source changed while bundling".into());
    }
    publish_noclobber(stage.path(), &target)?;
    Ok(
        serde_json::json!({"target": target_name, "output": target, "package": manifest.package, "entries": manifest.entries.len(), "executableDigest": manifest.executable.digest, "presentationWorker":manifest.presentation_worker, "unsigned": true}),
    )
}

fn enumerate(
    root: &Path,
    relative: &str,
    files: &mut BTreeSet<String>,
    dirs: &mut BTreeSet<String>,
) -> Result<(), String> {
    let path = if relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    };
    for item in fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))? {
        let item = item.map_err(|e| e.to_string())?;
        let ty = item.file_type().map_err(|e| e.to_string())?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 path in bundle")?;
        let rel = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        // The package prefix is a bundle container, not a declared package path segment.
        let admitted_path = rel.strip_prefix("package/").unwrap_or(&rel);
        if !safe_rel(admitted_path) || ty.is_symlink() {
            return Err(format!("unsafe path or symlink in bundle: {rel}"));
        }
        if files.len() + dirs.len() >= MAX_INPUTS * 8 + 3 {
            return Err("bundle entry count exceeds limits".into());
        }
        if ty.is_dir() {
            dirs.insert(rel.clone());
            enumerate(root, &rel, files, dirs)?;
        } else if ty.is_file() {
            files.insert(rel);
        } else {
            return Err(format!("special file in bundle: {rel}"));
        }
    }
    Ok(())
}

/// Verify a bundle without launching its executable or consulting any external package source.
pub fn verify(bundle: &Path) -> Result<serde_json::Value, String> {
    let meta = fs::symlink_metadata(bundle).map_err(|e| format!("{}: {e}", bundle.display()))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("bundle must be a real directory".into());
    }
    let manifest: BundleManifest = serde_json::from_slice(&checked_read(
        &bundle.join("manifest.json"),
        MAX_FILE as u64,
    )?)
    .map_err(|e| format!("manifest.json: {e}"))?;
    let pin: Pin = serde_json::from_slice(&checked_read(
        &bundle.join("provider-pin.json"),
        MAX_FILE as u64,
    )?)
    .map_err(|e| format!("provider-pin.json: {e}"))?;
    let target = expand::current_host_target()?;
    if manifest.schema_version != 1
        || manifest.target != target
        || manifest.provider != "clanker-ui.native"
        || manifest.assembly_protocol != 2
        || manifest.binding_abi != 2
        || manifest.template_engine != "minijinja-2.12.0"
        || !valid_revision(&manifest.source_revision)
        || !valid_tool_version(&manifest.tool_version)
    {
        return Err(
            "bundle identity, target, revision, or supported protocol does not match this verifier"
                .into(),
        );
    }
    if manifest.executable.path != "bin/clanker-ui"
        || manifest.executable.bytes == 0
        || manifest.executable.bytes as u64 > MAX_EXECUTABLE
    {
        return Err("invalid bundle executable declaration".into());
    }
    // Inspect the entire bounded tree before opening nested files, rejecting ancestor symlinks.
    let mut actual_files = BTreeSet::new();
    let mut dirs = BTreeSet::new();
    enumerate(bundle, "", &mut actual_files, &mut dirs)?;
    let executable_path = bundle.join(&manifest.executable.path);
    let _executable_meta = fs::symlink_metadata(&executable_path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if _executable_meta.permissions().mode() & 0o111 == 0 {
            return Err("bundled CLI is not executable".into());
        }
    }
    let executable = checked_read(&executable_path, MAX_EXECUTABLE)?;
    if executable.len() != manifest.executable.bytes
        || digest(&executable) != manifest.executable.digest
    {
        return Err("bundle executable bytes or digest mismatch".into());
    }
    if pin.schema_version != 1
        || pin.provider != manifest.provider
        || pin.assembly_protocol != manifest.assembly_protocol
        || pin.binding_abi != manifest.binding_abi
        || pin.targets.len() != 1
        || pin
            .targets
            .get(target)
            .map(|p| (p.executable.as_str(), p.digest.as_str()))
            != Some(("bin/clanker-ui", manifest.executable.digest.as_str()))
    {
        return Err("provider pin does not exactly match bundle executable and target".into());
    }
    let mut folded = BTreeSet::new();
    let mut expected_files = BTreeSet::from([
        "manifest.json".to_string(),
        "provider-pin.json".to_string(),
        "bin/clanker-ui".to_string(),
    ]);
    if manifest.legal.len() != LEGAL.len()
        || manifest.legal.iter().zip(LEGAL).any(|(entry, (path, _))| {
            entry.path != path || entry.bytes == 0 || entry.bytes > MAX_FILE
        })
        || manifest.entries.len() + manifest.legal.len() > MAX_INPUTS
    {
        return Err(
            "bundle requires exactly legal/LICENSE and legal/NOTICES.txt within existing budgets"
                .into(),
        );
    }
    if let Some(worker) = &manifest.presentation_worker {
        if worker.abi != 1
            || worker.executable.path != "bin/clanker-chart-worker"
            || worker.executable.bytes == 0
            || worker.executable.bytes as u64 > MAX_EXECUTABLE
            || manifest.executable.bytes as u64 + worker.executable.bytes as u64 > MAX_EXECUTABLE
            || manifest.entries.len() + manifest.legal.len() + 1 > MAX_INPUTS
        {
            return Err("invalid presentation worker declaration or executable budget".into());
        }
        let path = bundle.join(&worker.executable.path);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o111
                == 0
            {
                return Err("bundled presentation worker is not executable".into());
            }
        }
        let bytes = checked_read(&path, MAX_EXECUTABLE)?;
        if bytes.len() != worker.executable.bytes || digest(&bytes) != worker.executable.digest {
            return Err("presentation worker bytes or digest mismatch".into());
        }
        let contract = checked_read(
            &bundle.join("package/components/chart/renderer-contract.json"),
            MAX_FILE as u64,
        )?;
        if chart_contract(&contract)? != worker.contracts {
            return Err("presentation worker contract differs from locked package".into());
        }
        expected_files.insert(worker.executable.path.clone());
    }
    let mut total = 0usize;
    for entry in &manifest.legal {
        let bytes = checked_read(&bundle.join(&entry.path), MAX_FILE as u64)?;
        if bytes.len() != entry.bytes || digest(&bytes) != entry.digest {
            return Err(format!("legal input digest mismatch: {}", entry.path));
        }
        total += entry.bytes;
        expected_files.insert(entry.path.clone());
    }
    let mut locked = Vec::new();
    let mut previous: Option<&str> = None;
    for entry in &manifest.entries {
        if !safe_rel(&entry.path)
            || entry.path.starts_with("bin/")
            || entry.bytes > MAX_FILE
            || previous.is_some_and(|p| p >= entry.path.as_str())
            || !folded.insert(entry.path.to_ascii_lowercase())
        {
            return Err(format!(
                "invalid, unsorted, duplicate, or unsafe package entry: {}",
                entry.path
            ));
        }
        previous = Some(&entry.path);
        total = total
            .checked_add(entry.bytes)
            .ok_or("package byte count overflow")?;
        if total > MAX_TOTAL || manifest.entries.len() > MAX_INPUTS {
            return Err("package closure exceeds bundle limits".into());
        }
        let bytes = checked_read(&bundle.join("package").join(&entry.path), MAX_FILE as u64)?;
        if bytes.len() != entry.bytes || digest(&bytes) != entry.digest {
            return Err(format!("package input digest mismatch: {}", entry.path));
        }
        expected_files.insert(format!("package/{}", entry.path));
        locked.push(LockedInput {
            path: entry.path.clone(),
            bytes: entry.bytes,
            digest: entry.digest.clone(),
        });
    }
    if input_digest(&locked) != manifest.package.digest {
        return Err("package digest does not match declared inputs".into());
    }
    if actual_files != expected_files {
        return Err("bundle has missing or unexpected files".into());
    }
    let mut expected_dirs = BTreeSet::from([
        "bin".to_string(),
        "package".to_string(),
        "legal".to_string(),
    ]);
    for name in &expected_files {
        if let Some(rest) = name.strip_prefix("package/") {
            let mut parent = Path::new(rest).parent();
            while let Some(p) = parent {
                if p.as_os_str().is_empty() {
                    break;
                }
                expected_dirs.insert(format!(
                    "package/{}",
                    p.to_string_lossy().replace('\\', "/")
                ));
                parent = p.parent();
            }
        }
    }
    if dirs != expected_dirs {
        return Err("bundle has unexpected or missing directories".into());
    }
    let mut package = LocalPackage.load(&bundle.join("package"))?;
    LocalPackage.complete_declared_inputs(&mut package)?;
    let rebuilt = expand::package_inputs(&package.assets)?;
    if input_entries(&rebuilt) != manifest.entries
        || input_digest(&rebuilt) != manifest.package.digest
        || package.catalog.package.name != manifest.package.name
        || package.catalog.package.version != manifest.package.version
    {
        return Err(
            "package metadata or complete declared input closure does not match bundle manifest"
                .into(),
        );
    }
    Ok(
        serde_json::json!({"target":target,"toolVersion":manifest.tool_version,"sourceRevision":manifest.source_revision,"package":manifest.package,"legal":manifest.legal,"entries":manifest.entries.len(),"executableDigest":manifest.executable.digest,"presentationWorker":manifest.presentation_worker,"verified":true,"trust":"unsigned; operator approval required"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_versions_have_bounded_semantic_version_identity() {
        for value in ["0.1.0", "1.2.3-rc.1+build.001"] {
            assert!(valid_tool_version(value));
        }
        for value in [
            "latest",
            "1.2",
            "01.2.3",
            "1.2.3-",
            "1.2.3+",
            "1.2.3-01",
            "1.2.3+a+b",
        ] {
            assert!(!valid_tool_version(value));
        }
    }

    #[test]
    fn source_walk_rejects_excessive_depth_and_entry_count() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("a/b/c/d/e/f/g/h/i")).unwrap();
        assert!(check_source_tree(root.path(), 0, &mut 0).is_err());
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), b"data").unwrap();
        assert!(check_source_tree(root.path(), 0, &mut (MAX_INPUTS * 8)).is_err());
    }

    #[test]
    fn bundle_container_does_not_reduce_declared_path_depth_budget() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("package/a/b/c/d/e/f/g")).unwrap();
        fs::write(root.path().join("package/a/b/c/d/e/f/g/file.css"), b"data").unwrap();
        let mut files = BTreeSet::new();
        let mut dirs = BTreeSet::new();
        enumerate(root.path(), "", &mut files, &mut dirs).unwrap();
        assert!(files.contains("package/a/b/c/d/e/f/g/file.css"));
        fs::create_dir_all(root.path().join("package/a/b/c/d/e/f/g/h")).unwrap();
        fs::write(
            root.path().join("package/a/b/c/d/e/f/g/h/file.css"),
            b"data",
        )
        .unwrap();
        assert!(enumerate(root.path(), "", &mut files, &mut dirs).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn tree_inspection_rejects_ancestor_symlinks_before_nested_reads() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("clanker-ui"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("bin")).unwrap();
        assert!(enumerate(root.path(), "", &mut BTreeSet::new(), &mut BTreeSet::new()).is_err());
    }
}
