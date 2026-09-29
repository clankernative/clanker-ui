use catalog_core::Catalog;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct LoadedPackage {
    pub root: PathBuf,
    pub catalog: Catalog,
    pub digest: String,
    pub assets: BTreeMap<String, Vec<u8>>,
}

pub trait PackageSource {
    fn load(&self, root: &Path) -> Result<LoadedPackage, String>;
    fn asset(&self, package: &LoadedPackage, relative: &str) -> Result<Vec<u8>, String>;
}

pub struct Bundle {
    pub files: BTreeMap<String, Vec<u8>>,
    pub package_digest: String,
    pub components: Vec<String>,
}

pub trait ArtifactSink {
    fn apply(&self, output: &Path, bundle: &Bundle) -> Result<(), String>;
}
