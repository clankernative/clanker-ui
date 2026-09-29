//! Local app-owned artifact materialization. Only managed outputs may be replaced.

use crate::document;
use crate::ports::Bundle;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "ui/clanker-managed.md";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Managed {
    schema_version: u32,
    package_digest: String,
    files: BTreeMap<String, String>,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn check_path(app: &Path, path: &str) -> Result<PathBuf, String> {
    if !path.starts_with("ui/") || !catalog_core::safe_path(path) {
        return Err(format!("not a managed UI output path: {path}"));
    }
    let mut target = app.to_path_buf();
    for part in path.split('/') {
        target.push(part);
        if let Ok(meta) = fs::symlink_metadata(&target) {
            if meta.file_type().is_symlink() {
                return Err(format!(
                    "managed output crosses a symlink: {}",
                    target.display()
                ));
            }
        }
    }
    Ok(target)
}

pub fn install(app: &Path, bundle: &Bundle) -> Result<Vec<String>, String> {
    if !app.join("App.roc").is_file() || !app.join("ui/app.css").is_file() {
        return Err(format!(
            "{} is not a Clanker Native app with an authored stylesheet",
            app.display()
        ));
    }
    let owned = bundle
        .files
        .iter()
        .filter(|(path, _)| path.starts_with("ui/"))
        .map(|(path, bytes)| (path.clone(), digest(bytes)))
        .collect::<BTreeMap<_, _>>();
    let manifest_path = app.join(MANIFEST);
    let previous: Option<Managed> = if manifest_path.exists() {
        let bytes = fs::read(&manifest_path).map_err(|e| e.to_string())?;
        Some(
            serde_json::from_slice(document::decode(&bytes)?)
                .map_err(|e| format!("{}: {e}", manifest_path.display()))?,
        )
    } else {
        None
    };
    if let Some(previous) = &previous {
        if previous.schema_version != 1
            || previous.files.keys().collect::<Vec<_>>() != owned.keys().collect::<Vec<_>>()
        {
            return Err(
                "managed file set changed; review and clean previous output explicitly".into(),
            );
        }
    }
    // Validate every precondition before writing any output.
    for path in owned.keys() {
        let target = check_path(app, path)?;
        match fs::read(&target) {
            Ok(bytes) => {
                if previous.as_ref().and_then(|m| m.files.get(path)) != Some(&digest(&bytes)) {
                    return Err(format!(
                        "refusing to overwrite unowned or edited file: {}",
                        target.display()
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && previous.is_none() => {}
            Err(e) => return Err(format!("{}: {e}", target.display())),
        }
    }
    // Pre-stage all files. A later filesystem failure may still require manual recovery;
    // the managed manifest is written last, so an incomplete install is never certified.
    let staged = tempfile::tempdir_in(app).map_err(|e| e.to_string())?;
    for (path, bytes) in &bundle.files {
        if !path.starts_with("ui/") {
            continue;
        }
        let target = staged.path().join(path);
        fs::create_dir_all(target.parent().ok_or("invalid staged path")?)
            .map_err(|e| e.to_string())?;
        fs::write(&target, bytes).map_err(|e| e.to_string())?;
    }
    for path in owned.keys() {
        let destination = check_path(app, path)?;
        fs::create_dir_all(destination.parent().ok_or("invalid target")?)
            .map_err(|e| e.to_string())?;
        fs::rename(staged.path().join(path), &destination)
            .map_err(|e| format!("{}: {e}", destination.display()))?;
    }
    fs::create_dir_all(manifest_path.parent().ok_or("invalid manifest path")?)
        .map_err(|e| e.to_string())?;
    let manifest = Managed {
        schema_version: 1,
        package_digest: bundle.package_digest.clone(),
        files: owned.clone(),
    };
    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let bytes = document::encode("Clanker Native UI managed outputs", &json)?;
    let temp =
        tempfile::NamedTempFile::new_in(manifest_path.parent().ok_or("invalid manifest parent")?)
            .map_err(|e| e.to_string())?;
    fs::write(temp.path(), bytes).map_err(|e| e.to_string())?;
    temp.persist(&manifest_path).map_err(|e| e.to_string())?;
    Ok(owned.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_unowned_and_edited_outputs() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ui")).unwrap();
        fs::write(root.path().join("App.roc"), "app").unwrap();
        fs::write(root.path().join("ui/app.css"), "body{}").unwrap();
        let bundle = Bundle {
            files: BTreeMap::from([("ui/app.bundle.css".into(), b"generated".to_vec())]),
            package_digest: "sha256:test".into(),
            components: vec![],
        };
        fs::write(root.path().join("ui/app.bundle.css"), "user edit").unwrap();
        assert!(install(root.path(), &bundle).is_err());
        fs::remove_file(root.path().join("ui/app.bundle.css")).unwrap();
        install(root.path(), &bundle).unwrap();
        install(root.path(), &bundle).unwrap();
        fs::write(root.path().join("ui/app.bundle.css"), "user edit").unwrap();
        assert!(install(root.path(), &bundle).is_err());
    }
}
