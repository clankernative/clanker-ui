use crate::ports::{LoadedPackage, PackageSource};
use catalog_core::{safe_path, Catalog, Component, PackageManifest, Status};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct LocalPackage;

const MAX_FILE: u64 = 1024 * 1024;
const MAX_COMPONENTS: usize = 500;
const MAX_TOTAL: usize = 32 * 1024 * 1024;

fn read_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    if !safe_path(relative) {
        return Err(format!("unsafe package path: {relative}"));
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlink not allowed: {}", path.display()));
        }
    }
    let metadata = fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_FILE {
        return Err(format!(
            "expected file <= {MAX_FILE} bytes: {}",
            path.display()
        ));
    }
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(format!(
            "file grew beyond {MAX_FILE} bytes: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn capture(
    files: &mut BTreeMap<String, Vec<u8>>,
    root: &Path,
    relative: &str,
) -> Result<Vec<u8>, String> {
    let bytes = read_file(root, relative)?;
    if let Some(previous) = files.get(relative) {
        if previous != &bytes {
            return Err(format!("asset changed while loading: {relative}"));
        }
    } else {
        let total = files.values().map(Vec::len).sum::<usize>();
        if total + bytes.len() > MAX_TOTAL {
            return Err(format!("package inputs exceed {MAX_TOTAL} bytes"));
        }
        files.insert(relative.into(), bytes.clone());
    }
    Ok(bytes)
}

fn digest_entry(hasher: &mut Sha256, path: &str, bytes: &[u8]) {
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

impl LocalPackage {
    /// Capture every asset declared by component metadata, including non-executable contracts.
    pub fn complete_declared_inputs(&self, package: &mut LoadedPackage) -> Result<(), String> {
        for component in package.catalog.components() {
            let assets = component
                .assets
                .scripts
                .iter()
                .chain(component.assets.contracts.iter())
                .chain([&component.assets.template, &component.assets.styles])
                .chain(component.fixtures.iter());
            for asset in assets {
                capture(&mut package.assets, &package.root, asset)?;
            }
        }
        Ok(())
    }
}

impl PackageSource for LocalPackage {
    fn load(&self, root: &Path) -> Result<LoadedPackage, String> {
        let metadata =
            fs::symlink_metadata(root).map_err(|e| format!("{}: {e}", root.display()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "expected local package directory: {}",
                root.display()
            ));
        }
        let mut files = BTreeMap::new();
        let manifest_bytes = capture(&mut files, root, "ui-package.json")?;
        let manifest = PackageManifest::parse(&manifest_bytes)
            .map_err(|e| format!("{}/ui-package.json: {e}", root.display()))?;
        capture(&mut files, root, &manifest.theme)?;
        for resource in &manifest.resources {
            capture(&mut files, root, resource)?;
        }
        let component_root = root.join("components");
        let mut directories = Vec::new();
        for entry in fs::read_dir(&component_root)
            .map_err(|e| format!("{}: {e}", component_root.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let ty = entry.file_type().map_err(|e| e.to_string())?;
            if ty.is_symlink() {
                return Err(format!("symlink not allowed: {}", entry.path().display()));
            }
            if ty.is_dir() {
                directories.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        directories.sort();
        if directories.len() > MAX_COMPONENTS {
            return Err(format!(
                "package exceeds {MAX_COMPONENTS} component directories"
            ));
        }
        let mut components = Vec::new();
        for name in directories {
            let relative = format!("components/{name}/component.json");
            let path = root.join(&relative);
            match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A source directory without a manifest is not a catalog entry.
                    continue;
                }
                Err(error) => return Err(format!("{}: {error}", path.display())),
                Ok(_) => {}
            }
            let bytes = capture(&mut files, root, &relative)?;
            let component =
                Component::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            if component.name != name {
                return Err(format!(
                    "{}: component name must match its directory",
                    path.display()
                ));
            }
            if component.status == Status::Ready {
                let assets = component
                    .assets
                    .scripts
                    .iter()
                    .chain(component.assets.contracts.iter())
                    .chain([&component.assets.template, &component.assets.styles])
                    .chain(component.fixtures.iter());
                for asset in assets {
                    capture(&mut files, root, asset)?;
                }
            }
            components.push(component);
        }
        let catalog = Catalog::new(manifest, components)?;
        let mut hash = Sha256::new();
        for (path, bytes) in &files {
            digest_entry(&mut hash, path, bytes);
        }
        Ok(LoadedPackage {
            root: root.to_path_buf(),
            catalog,
            digest: format!("sha256:{:x}", hash.finalize()),
            assets: files,
        })
    }

    fn asset(&self, package: &LoadedPackage, relative: &str) -> Result<Vec<u8>, String> {
        package
            .assets
            .get(relative)
            .cloned()
            .ok_or_else(|| format!("asset not in locked input snapshot: {relative}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn unmarked_folders_are_ignored_but_malformed_metadata_fails() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("ui-package.json"), br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css"}"#).unwrap();
        fs::create_dir_all(root.path().join("theme")).unwrap();
        fs::write(
            root.path().join("theme/default.css"),
            ":root { --cui-accent: blue; }",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("components/unmarked")).unwrap();
        let loaded = LocalPackage.load(root.path()).unwrap();
        assert_eq!(loaded.catalog.components().count(), 0);
        fs::write(
            root.path().join("components/unmarked/component.json"),
            "not json",
        )
        .unwrap();
        let error = LocalPackage.load(root.path()).err().unwrap();
        assert!(error.contains("components/unmarked/component.json"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_manifest_is_not_silently_omitted() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("ui-package.json"), br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css"}"#).unwrap();
        fs::create_dir_all(root.path().join("theme")).unwrap();
        fs::write(root.path().join("theme/default.css"), ":root {}").unwrap();
        fs::create_dir_all(root.path().join("components/button")).unwrap();
        symlink(
            "missing.json",
            root.path().join("components/button/component.json"),
        )
        .unwrap();
        assert!(LocalPackage
            .load(root.path())
            .err()
            .unwrap()
            .contains("symlink not allowed"));
    }
}
