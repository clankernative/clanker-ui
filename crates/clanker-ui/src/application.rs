use crate::document;
pub use crate::lock::{LockedPackage, PackageLock};
use crate::native::NativeAdapter;
use crate::native_button::{self, ButtonInstances};
use crate::ports::{ArtifactSink, Bundle, LoadedPackage, PackageSource};
use catalog_core::Component;
use std::fs;
use std::path::{Path, PathBuf};

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
        if !crate::lock::safe_package_relative(package_path) {
            return Err("package path must be relative to the lock file".into());
        }
        let parent = lock_path.parent().ok_or("lock needs a parent directory")?;
        let loaded = self.source.load(&parent.join(package_path))?;
        let lock = PackageLock::from_package(&loaded, package_path)?;
        let json = serde_json::to_vec_pretty(&lock).map_err(|e| e.to_string())?;
        if json.len() > 1024 * 1024 {
            return Err("Native UI lock exceeds 1 MiB".into());
        }
        let bytes = json;
        if fs::symlink_metadata(lock_path).is_ok() {
            let old = PackageLock::read(lock_path)?;
            if old == lock {
                return Ok(lock);
            }
            if !update {
                return Err(format!(
                    "{} differs; use --update to explicitly refresh",
                    lock_path.display()
                ));
            }
            if old.package.name != lock.package.name {
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
        let lock = PackageLock::read(lock_path)?;
        let parent = lock_path.parent().ok_or("lock needs a parent directory")?;
        let package = self.source.load(&parent.join(&lock.package.path))?;
        let captured = PackageLock::from_package(&package, &lock.package.path)?;
        if lock != captured || package.digest != captured.package.digest {
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
