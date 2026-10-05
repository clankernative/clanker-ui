use crate::document;
use crate::native::NativeAdapter;
use crate::native_button::{self, ButtonInstances};
use crate::ports::{ArtifactSink, Bundle, LoadedPackage, PackageSource};
use catalog_core::Component;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageLock {
    pub schema_version: u32,
    pub package: String,
    pub version: String,
    pub path: String,
    pub digest: String,
}

pub struct Application<S, O> {
    pub source: S,
    pub output: O,
}

impl<S: PackageSource, O: ArtifactSink> Application<S, O> {
    pub fn lock(
        &self,
        lock_path: &Path,
        package_path: &str,
        update: bool,
    ) -> Result<PackageLock, String> {
        if package_path.is_empty()
            || Path::new(package_path).is_absolute()
            || package_path.contains('\\')
        {
            return Err("package path must be relative to the lock file".into());
        }
        let parent = lock_path.parent().ok_or("lock needs a parent directory")?;
        let loaded = self.source.load(&parent.join(package_path))?;
        let lock = PackageLock {
            schema_version: 1,
            package: loaded.catalog.package.name,
            version: loaded.catalog.package.version,
            path: package_path.into(),
            digest: loaded.digest,
        };
        let json = serde_json::to_vec_pretty(&lock).map_err(|e| e.to_string())?;
        let bytes = if lock_path.extension().is_some_and(|ext| ext == "md") {
            document::encode("Clanker Native UI package lock", &json)?
        } else {
            json
        };
        if lock_path.exists() {
            let previous = fs::read(lock_path).map_err(|e| e.to_string())?;
            if previous == bytes {
                return Ok(lock);
            }
            if !update {
                return Err(format!(
                    "{} differs; use --update to explicitly refresh",
                    lock_path.display()
                ));
            }
            let old: PackageLock =
                serde_json::from_slice(document::decode(&previous)?).map_err(|e| e.to_string())?;
            if old.package != lock.package {
                return Err("cannot replace a locked package with a different identity".into());
            }
        }
        if !parent.is_dir() {
            return Err(format!("lock parent does not exist: {}", parent.display()));
        }
        let temp = tempfile::Builder::new()
            .prefix(".clanker-ui-lock-")
            .tempfile_in(parent)
            .map_err(|e| e.to_string())?;
        fs::write(temp.path(), bytes).map_err(|e| e.to_string())?;
        temp.persist(lock_path).map_err(|e| e.to_string())?;
        Ok(lock)
    }

    pub fn load(&self, lock_path: &Path) -> Result<LoadedPackage, String> {
        let bytes = fs::read(lock_path).map_err(|e| e.to_string())?;
        let lock: PackageLock = serde_json::from_slice(document::decode(&bytes)?)
            .map_err(|e| format!("{}: {e}", lock_path.display()))?;
        if lock.schema_version != 1
            || lock.path.is_empty()
            || Path::new(&lock.path).is_absolute()
            || lock.path.contains('\\')
        {
            return Err("invalid lock schema or path".into());
        }
        let parent = lock_path.parent().ok_or("lock needs a parent directory")?;
        let package = self.source.load(&parent.join(&lock.path))?;
        if lock.package != package.catalog.package.name
            || lock.version != package.catalog.package.version
            || lock.digest != package.digest
        {
            return Err(format!(
                "{}: package bytes differ from lock; explicitly refresh the lock",
                lock_path.display()
            ));
        }
        Ok(package)
    }

    pub fn find<'a>(
        &self,
        catalog: &'a LoadedPackage,
        query: &str,
        target: &str,
    ) -> Vec<&'a Component> {
        catalog.catalog.find(query, target)
    }

    pub fn compose(
        &self,
        lock_path: &Path,
        instances: &Path,
        base_css: &Path,
        theme: &Path,
        output: &Path,
        apply: bool,
    ) -> Result<Bundle, String> {
        let package = self.load(lock_path)?;
        NativeAdapter::verify(&self.source, &package)?;
        let parent = lock_path.parent().ok_or("lock needs a parent directory")?;
        let read = |path: &Path| -> Result<Vec<u8>, String> {
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                parent.join(path)
            };
            let bytes = fs::read(&resolved).map_err(|e| format!("{}: {e}", resolved.display()))?;
            if bytes.len() > 1024 * 1024 {
                return Err(format!("{} exceeds 1 MiB", resolved.display()));
            }
            Ok(bytes)
        };
        let instance_bytes = read(instances)?;
        let instances = ButtonInstances::parse(document::decode(&instance_bytes)?, &package)?;
        let bundle = native_button::compose(
            &self.source,
            &package,
            &instances,
            &read(base_css)?,
            &read(theme)?,
        )?;
        if self.source.load(&package.root)?.digest != package.digest {
            return Err("package changed while composing".into());
        }
        if apply {
            self.output.apply(output, &bundle)?;
        }
        Ok(bundle)
    }

    pub fn build(
        &self,
        lock_path: &Path,
        name: &str,
        variant: &str,
        theme: &Path,
        output: &Path,
        apply: bool,
    ) -> Result<Bundle, String> {
        let package = self.load(lock_path)?;
        NativeAdapter::verify(&self.source, &package)?;
        let theme_path = if theme.is_absolute() {
            PathBuf::from(theme)
        } else {
            lock_path
                .parent()
                .ok_or("lock needs a parent directory")?
                .join(theme)
        };
        let app_theme =
            fs::read(&theme_path).map_err(|e| format!("{}: {e}", theme_path.display()))?;
        if app_theme.len() > 1024 * 1024 {
            return Err("app theme exceeds 1 MiB".into());
        }
        let bundle = NativeAdapter::plan(&self.source, &package, name, variant, &app_theme)?;
        let reloaded = self.source.load(&package.root)?;
        if reloaded.digest != package.digest {
            return Err("package changed while building".into());
        }
        if apply {
            self.output.apply(output, &bundle)?;
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::directory_sink::DirectorySink;
    use crate::local::LocalPackage;

    fn copy_package_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let destination = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_package_tree(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }

    #[test]
    fn lock_detects_source_changes_and_needs_explicit_update() {
        let root = tempfile::tempdir().unwrap();
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
        let local = root.path().join("local");
        copy_package_tree(&package, &local);
        let app = Application {
            source: LocalPackage,
            output: DirectorySink,
        };
        let lock = root.path().join("ui.lock.json");
        app.lock(&lock, "local", false).unwrap();
        let snapshot = app.load(&lock).unwrap();
        let pinned_styles = app
            .source
            .asset(&snapshot, "components/button/styles.css")
            .unwrap();
        fs::write(local.join("components/button/styles.css"), "changed").unwrap();
        assert_eq!(
            app.source
                .asset(&snapshot, "components/button/styles.css")
                .unwrap(),
            pinned_styles
        );
        assert!(app.load(&lock).is_err());
        assert!(app.lock(&lock, "local", false).is_err());
        app.lock(&lock, "local", true).unwrap();
        assert!(app.load(&lock).is_ok());
    }
}
