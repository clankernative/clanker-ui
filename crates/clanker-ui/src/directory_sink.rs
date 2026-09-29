use crate::ports::{ArtifactSink, Bundle};
use catalog_core::safe_path;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub struct DirectorySink;

fn verify_existing(output: &Path, bundle: &Bundle) -> Result<(), String> {
    fn visit(dir: &Path, root: &Path, actual: &mut BTreeSet<String>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let ty = entry.file_type().map_err(|e| e.to_string())?;
            if ty.is_symlink() {
                return Err(format!(
                    "symlink in staged output: {}",
                    entry.path().display()
                ));
            }
            if ty.is_dir() {
                visit(&entry.path(), root, actual)?;
            } else if ty.is_file() {
                actual.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            } else {
                return Err(format!(
                    "unsupported staged entry: {}",
                    entry.path().display()
                ));
            }
        }
        Ok(())
    }
    let mut actual = BTreeSet::new();
    visit(output, output, &mut actual)?;
    let expected = bundle.files.keys().cloned().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "staged output differs at {}: file set changed",
            output.display()
        ));
    }
    for (path, bytes) in &bundle.files {
        let existing = fs::read(output.join(path)).map_err(|e| e.to_string())?;
        if existing != *bytes {
            return Err(format!(
                "staged output differs at {}",
                output.join(path).display()
            ));
        }
    }
    Ok(())
}

impl ArtifactSink for DirectorySink {
    fn apply(&self, output: &Path, bundle: &Bundle) -> Result<(), String> {
        if bundle.files.keys().any(|path| !safe_path(path)) {
            return Err("unsafe bundle path".into());
        }
        match fs::symlink_metadata(output) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("staged output is a symlink: {}", output.display()));
            }
            Ok(_) => return verify_existing(output, bundle),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", output.display())),
        }
        let parent = output
            .parent()
            .ok_or("output must have a parent directory")?;
        if !parent.is_dir() {
            return Err(format!(
                "output parent does not exist: {}",
                parent.display()
            ));
        }
        let temp = tempfile::Builder::new()
            .prefix(".clanker-ui-")
            .tempdir_in(parent)
            .map_err(|e| e.to_string())?;
        for (path, bytes) in &bundle.files {
            let dest = temp.path().join(path);
            fs::create_dir_all(dest.parent().ok_or("invalid bundle path")?)
                .map_err(|e| e.to_string())?;
            fs::write(dest, bytes).map_err(|e| e.to_string())?;
        }
        fs::rename(temp.path(), output).map_err(|e| format!("{}: {e}", output.display()))?;
        Ok(())
    }
}
