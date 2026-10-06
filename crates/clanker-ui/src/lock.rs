//! Canonical Native app lock and its declared-input manifest identity.
use crate::ports::LoadedPackage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};

pub const PROVIDER: &str = "clanker-ui.native";
const MAX_FILE: usize = 1_048_576;
const MAX_INPUTS: usize = 4096;
const MAX_TOTAL: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedInput {
    pub path: String,
    pub digest: String,
    pub bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub path: String,
    pub digest: String,
    pub inputs: Vec<LockedInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageLock {
    pub schema_version: u32,
    pub provider: String,
    pub package: LockedPackage,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Native's manifest hash: sorted path, NUL, decimal size, NUL, file digest, LF.
pub fn input_manifest_digest(inputs: &[LockedInput]) -> String {
    let mut hasher = Sha256::new();
    for input in inputs {
        hasher.update(input.path.as_bytes());
        hasher.update([0]);
        hasher.update(input.bytes.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(input.digest.as_bytes());
        hasher.update(b"\n");
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn safe_input_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.split('/').count() <= 8
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}

/// Package roots may be local children or one/two parent levels followed by children.
pub(crate) fn safe_package_relative(value: &str) -> bool {
    let parts = value.split('/').collect::<Vec<_>>();
    let parents = parts.iter().take_while(|part| **part == "..").count();
    parents <= 2
        && parents < parts.len()
        && parts[parents..].iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}

pub fn package_inputs(assets: &BTreeMap<String, Vec<u8>>) -> Result<Vec<LockedInput>, String> {
    if assets.len() > MAX_INPUTS || assets.values().map(Vec::len).sum::<usize>() > MAX_TOTAL {
        return Err("package declared input closure exceeds assembly limits".into());
    }
    let mut folded = BTreeSet::new();
    assets
        .iter()
        .map(|(path, bytes)| {
            if !safe_input_path(path) || bytes.len() > MAX_FILE {
                return Err(format!("unsafe or oversized package input: {path}"));
            }
            if !folded.insert(path.to_ascii_lowercase()) {
                return Err(format!("case-folded package input path collision: {path}"));
            }
            Ok(LockedInput {
                path: path.clone(),
                digest: digest(bytes),
                bytes: bytes.len(),
            })
        })
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

impl PackageLock {
    pub fn from_package(package: &LoadedPackage, path: &str) -> Result<Self, String> {
        let inputs = package_inputs(&package.assets)?;
        let lock = Self {
            schema_version: 1,
            provider: PROVIDER.into(),
            package: LockedPackage {
                name: package.catalog.package.name.clone(),
                version: package.catalog.package.version.clone(),
                path: path.into(),
                digest: input_manifest_digest(&inputs),
                inputs,
            },
        };
        lock.validate()?;
        Ok(lock)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_FILE {
            return Err("Native UI lock exceeds 1 MiB".into());
        }
        let lock: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        lock.validate()?;
        Ok(lock)
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_FILE as u64 {
            return Err(format!("invalid Native UI lock file: {}", path.display()));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(MAX_FILE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn validate(&self) -> Result<(), String> {
        let package = &self.package;
        if self.schema_version != 1
            || self.provider != PROVIDER
            || package.name.trim().is_empty()
            || package.version.trim().is_empty()
            || !safe_package_relative(&package.path)
            || !valid_digest(&package.digest)
            || package.inputs.is_empty()
            || package.inputs.len() > MAX_INPUTS
        {
            return Err("invalid Native UI lock schema, provider, identity, or path".into());
        }
        let mut previous: Option<&str> = None;
        let mut folded = BTreeSet::new();
        let mut total = 0usize;
        for input in &package.inputs {
            if !safe_input_path(&input.path)
                || !valid_digest(&input.digest)
                || input.bytes > MAX_FILE
                || previous.is_some_and(|path| path >= input.path.as_str())
                || !folded.insert(input.path.to_ascii_lowercase())
            {
                return Err("invalid, duplicate, or unsorted Native UI lock inputs".into());
            }
            total += input.bytes;
            previous = Some(&input.path);
        }
        if total > MAX_TOTAL || package.digest != input_manifest_digest(&package.inputs) {
            return Err("Native UI lock input manifest digest or byte budget differs".into());
        }
        Ok(())
    }
}
