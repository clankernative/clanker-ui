//! Read-only CLI expansion boundary for locked Native UI packages.
use crate::{
    application::Application, directory_sink::DirectorySink, local::LocalPackage,
    ports::PackageSource,
};
use catalog_core::expansion::{self, Binding, Package};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path},
};

const MAX_FILE: usize = 1_048_576;
const MAX_FILES: usize = 512;
const MAX_INPUTS: usize = 4096;
const MAX_TOTAL: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpansionBundle {
    pub schema_version: u32,
    pub runtime_abi: u32,
    pub template_engine: String,
    pub package_digest: String,
    pub templates: BTreeMap<String, String>,
    pub bindings: Vec<BindingManifest>,
    pub resources: Vec<Resource>,
    pub inputs: Vec<LockedInput>,
    /// Authored inputs folded into managed outputs; remove only in a private host snapshot.
    pub consumed_inputs: Vec<String>,
    /// Admitted module resources the standalone diagnostic can identify.
    pub entrypoints: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssemblyBundle {
    pub schema_version: u32,
    pub runtime_abi: u32,
    pub template_engine: String,
    pub package_digest: String,
    pub templates: BTreeMap<String, String>,
    pub resources: Vec<Resource>,
    pub inputs: Vec<LockedInput>,
    pub consumed_inputs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingManifest {
    pub template: String,
    pub field_path: String,
    pub expected_kind: String,
    pub component: String,
    pub attribute: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    pub path: String,
    pub source: Option<String>,
    pub content: Option<String>,
    pub digest: String,
    pub bytes: usize,
    pub kind: String,
}

pub use crate::lock::LockedInput;
pub(crate) use crate::lock::{input_manifest_digest, package_inputs};
use crate::lock::{safe_package_relative, PackageLock};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeAssemblyRequest {
    schema_version: u32,
    assembly_protocol: u32,
    provider: String,
    target: NativeTarget,
    package: NativePackage,
    ui: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeTarget {
    binding_abi: u32,
    template_engine: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativePackage {
    name: String,
    version: String,
    path: String,
    digest: String,
    inputs: Vec<LockedInput>,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn safe_manifest_path(value: &str) -> bool {
    safe_relative(value) && value.len() <= 512 && value.split('/').count() <= 8
}

/// Author the same canonical lock as `lock`; refreshes require explicit consent.
pub fn native_lock(lock_path: &Path, package_path: &str) -> Result<serde_json::Value, String> {
    native_lock_with_update(lock_path, package_path, false)
}

pub fn native_lock_with_update(
    lock_path: &Path,
    package_path: &str,
    update: bool,
) -> Result<serde_json::Value, String> {
    let app = Application {
        source: LocalPackage,
        output: DirectorySink,
    };
    serde_json::to_value(app.lock(lock_path, package_path, update)?)
        .map_err(|error| error.to_string())
}

pub(crate) fn current_host_target() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("macos-aarch64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        (os, arch) => Err(format!("unsupported native pin host target: {os}-{arch}")),
    }
}

/// Write an explicit, local-only unsigned override pin for this executable.
pub fn native_pin(output: &Path) -> Result<serde_json::Value, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable = fs::canonicalize(&executable).map_err(|error| error.to_string())?;
    let metadata = fs::symlink_metadata(&executable).map_err(|error| error.to_string())?;
    const MAX_EXECUTABLE: u64 = 64 * 1024 * 1024;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_EXECUTABLE {
        return Err("current executable is special or exceeds Native's 64 MiB pin budget".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&executable)
        .map_err(|error| error.to_string())?
        .take(MAX_EXECUTABLE + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_EXECUTABLE {
        return Err("current executable changed or exceeds the pin budget".into());
    }
    let parent = output
        .parent()
        .ok_or("pin output needs a parent directory")?;
    if !parent.is_dir() {
        return Err(format!(
            "pin output parent does not exist: {}",
            parent.display()
        ));
    }
    if matches!(
        output.file_name().and_then(|name| name.to_str()),
        Some("ui.lock.json" | "clanker-ui.lock.json")
    ) {
        return Err("pin output must remain separate from an app UI lock".into());
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err("pin output already exists; choose an unused path".into());
    }
    let target = current_host_target()?;
    let mut targets = serde_json::Map::new();
    targets.insert(
        target.into(),
        serde_json::json!({ "executable": executable, "digest": digest(&bytes) }),
    );
    let pin = serde_json::json!({
        "schemaVersion": 1,
        "provider": "clanker-ui.native",
        "assemblyProtocol": 2,
        "bindingAbi": 2,
        "targets": targets
    });
    let encoded = serde_json::to_vec_pretty(&pin).map_err(|error| error.to_string())?;
    let temp = tempfile::Builder::new()
        .prefix(".clanker-ui-native-pin-")
        .tempfile_in(parent)
        .map_err(|error| error.to_string())?;
    fs::write(temp.path(), encoded).map_err(|error| error.to_string())?;
    temp.persist_noclobber(output)
        .map_err(|error| error.to_string())?;
    Ok(pin)
}

/// Build an ABI-2 assembly envelope from Native's private captured request.
pub fn assemble_request(request_path: &Path) -> Result<AssemblyBundle, String> {
    let bytes = checked_file(request_path)?;
    let request: NativeAssemblyRequest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: {error}", request_path.display()))?;
    if request.schema_version != 1
        || request.assembly_protocol != 2
        || request.provider != "clanker-ui.native"
        || request.target.binding_abi != 2
        || request.target.template_engine != "minijinja-2.12.0"
    {
        return Err(
            "unsupported native assembly protocol, provider, target, or binding ABI".into(),
        );
    }
    let package_path = Path::new(&request.package.path);
    let ui_path = Path::new(&request.ui);
    if !package_path.is_absolute() || !ui_path.is_absolute() {
        return Err("captured package and UI paths must be absolute".into());
    }
    let mut loaded = crate::local::LocalPackage.load(package_path)?;
    crate::local::LocalPackage.complete_declared_inputs(&mut loaded)?;
    let inputs = package_inputs(&loaded.assets)?;
    if request.package.name != loaded.catalog.package.name
        || request.package.version != loaded.catalog.package.version
        || request.package.inputs != inputs
        || request.package.digest != input_manifest_digest(&inputs)
    {
        return Err(
            "captured package identity or complete input closure differs from request".into(),
        );
    }
    validate_captured_ui_lock(ui_path, &request.package)?;
    loaded.digest = request.package.digest.clone();
    let mut expanded = expand_package(loaded, ui_path, None, &[request_path, package_path])?;
    expanded.runtime_abi = 2;
    expanded.package_digest = request.package.digest;
    let bundle = AssemblyBundle {
        schema_version: expanded.schema_version,
        runtime_abi: expanded.runtime_abi,
        template_engine: expanded.template_engine,
        package_digest: expanded.package_digest,
        templates: expanded.templates,
        resources: expanded.resources,
        inputs: expanded.inputs,
        consumed_inputs: expanded.consumed_inputs,
    };
    // Expansion already validated the complete output; this projection drops diagnostics only.
    Ok(bundle)
}

fn validate_captured_ui_lock(ui: &Path, package: &NativePackage) -> Result<(), String> {
    let path = ui.join("ui.lock.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
        Ok(_) => {
            let bytes = checked_ui_file(ui, "ui.lock.json")?;
            let lock = PackageLock::parse(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let locked = lock.package;
            if lock.schema_version != 1
                || lock.provider != "clanker-ui.native"
                || !safe_package_relative(&locked.path)
                || locked.name != package.name
                || locked.version != package.version
                || locked.digest != package.digest
                || locked.inputs != package.inputs
            {
                return Err(format!(
                    "{}: app UI lock differs from the captured package closure",
                    path.display()
                ));
            }
            Ok(())
        }
    }
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}

fn checked_file(path: &Path) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_FILE as u64 {
        return Err(format!(
            "invalid, special, or oversized input: {}",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > MAX_FILE {
        return Err(format!("input grew beyond 1 MiB: {}", path.display()));
    }
    Ok(bytes)
}

fn checked_ui_file(ui: &Path, relative: &str) -> Result<Vec<u8>, String> {
    if !safe_relative(relative) {
        return Err(format!("unsafe UI path: {relative}"));
    }
    let mut path = ui.to_path_buf();
    let parts = relative.split('/').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!(
                "symlink not allowed in UI input: {}",
                path.display()
            ));
        }
        if index + 1 < parts.len() && !meta.is_dir() {
            return Err(format!("invalid UI resource parent: {}", path.display()));
        }
    }
    checked_file(&path)
}

fn collect_templates(
    root: &Path,
    rel: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let directory = root.join(rel);
    let meta =
        fs::symlink_metadata(&directory).map_err(|e| format!("{}: {e}", directory.display()))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(format!(
            "invalid template directory: {}",
            directory.display()
        ));
    }
    for entry in fs::read_dir(&directory).map_err(|e| format!("{}: {e}", directory.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 UI path".to_owned())?;
        let child_rel = format!("{rel}/{name}");
        if !safe_relative(&child_rel) {
            return Err(format!("unsafe UI path: {child_rel}"));
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "symlink not allowed in UI input: {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            collect_templates(root, &child_rel, files)?;
        } else if kind.is_file() {
            if !name.ends_with(".html") {
                return Err(format!(
                    "only HTML templates are allowed under pages/components: {child_rel}"
                ));
            }
            if files.len() >= MAX_FILES {
                return Err("UI template count exceeds 512 files".into());
            }
            let bytes = checked_file(&entry.path())?;
            std::str::from_utf8(&bytes).map_err(|e| format!("{child_rel}: {e}"))?;
            let total = files.values().map(Vec::len).sum::<usize>() + bytes.len();
            if total > MAX_TOTAL {
                return Err("UI template inputs exceed 32 MiB".into());
            }
            files.insert(child_rel, bytes);
        } else {
            return Err(format!(
                "special file not allowed in UI input: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn resource(
    path: &str,
    source: Option<&str>,
    content: Option<Vec<u8>>,
    kind: &str,
    source_bytes: &[u8],
) -> Result<Resource, String> {
    if source.is_some() == content.is_some() {
        return Err(format!(
            "resource must have exactly one source/content: {path}"
        ));
    }
    if !["stylesheet", "module", "font", "metadata"].contains(&kind) {
        return Err(format!("invalid resource kind: {kind}"));
    }
    let owned_content = content
        .map(|b| {
            String::from_utf8(b).map_err(|_| format!("binary resource cannot be inline: {path}"))
        })
        .transpose()?;
    Ok(Resource {
        path: path.into(),
        source: source.map(str::to_owned),
        content: owned_content,
        digest: digest(source_bytes),
        bytes: source_bytes.len(),
        kind: kind.into(),
    })
}

fn font_css(assets: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let Some(source) = assets.get("theme/fonts.css") else {
        if [
            "fonts/geist-sans-variable.woff2",
            "fonts/geist-mono-variable.woff2",
        ]
        .iter()
        .any(|p| assets.contains_key(*p))
        {
            return Err("locked font faces require theme/fonts.css".into());
        }
        return Ok(Vec::new());
    };
    if source.len() > 16_384 {
        return Err("font CSS exceeds 16 KiB".into());
    }
    let mut css =
        String::from_utf8(source.clone()).map_err(|e| format!("font CSS is not UTF-8: {e}"))?;
    let license = std::str::from_utf8(
        assets
            .get("fonts/GEIST-LICENSE.txt")
            .ok_or("missing locked font license")?,
    )
    .map_err(|e| format!("font license is not UTF-8: {e}"))?;
    if license.trim().is_empty() || license.contains("*/") || license.len() > 16_384 {
        return Err("unsafe/oversized font license".into());
    }
    css = format!("/* Locked reference-font license:\n{license}\n*/\n{css}");
    for (input, output) in [
        (
            "fonts/geist-sans-variable.woff2",
            "clanker-geist-sans-variable.woff2",
        ),
        (
            "fonts/geist-mono-variable.woff2",
            "clanker-geist-mono-variable.woff2",
        ),
    ] {
        if !assets.contains_key(input) {
            return Err(format!("missing locked font: {input}"));
        }
        let old = format!("../{input}");
        if css.matches(&old).count() != 1 {
            return Err("font CSS must reference each fixed font exactly once".into());
        }
        css = css.replace(&old, &format!("./fonts/{output}"));
    }
    Ok(css.into_bytes())
}

// Keep this generated entry point byte-for-byte aligned with the host adapter runtime.
fn runtime_entry(entries: &BTreeSet<String>) -> String {
    let mut program = String::from(
        "import {install as installLifecycle} from './clanker-ui/browser/lifecycle.js';\nexport {captureScrollPosition,restoreScrollPosition,preserveScroll} from './clanker-ui/browser/lifecycle.js';\n",
    );
    for (index, entry) in entries.iter().enumerate() {
        program.push_str(&format!(
            "import {{install as install{index}}} from './clanker-ui/{entry}';\n"
        ));
    }
    let installers = (0..entries.len())
        .map(|index| format!("install{index}"))
        .collect::<Vec<_>>()
        .join(",");
    program.push_str(&format!("export function install(root=document){{return installLifecycle(root,[{installers}]);}}\nif(typeof document!=='undefined'){{if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',()=>install(document),{{once:true}});else install(document);}}\n"));
    program
}

/// Expand all HTML templates under `ui/pages` and `ui/components` from a digest-verified lock snapshot.
pub fn expand(lock: &Path, ui: &Path, out: Option<&Path>) -> Result<ExpansionBundle, String> {
    let lock_meta = fs::symlink_metadata(lock).map_err(|e| format!("{}: {e}", lock.display()))?;
    if !lock_meta.is_file()
        || lock_meta.file_type().is_symlink()
        || lock_meta.len() > MAX_FILE as u64
    {
        return Err(format!("invalid Native UI lock file: {}", lock.display()));
    }
    let app = Application {
        source: LocalPackage,
        output: DirectorySink,
    };
    let package = app.load(lock)?;
    expand_package(package, ui, out, &[lock])
}

fn expand_package(
    package: crate::ports::LoadedPackage,
    ui: &Path,
    out: Option<&Path>,
    extra_excluded_paths: &[&Path],
) -> Result<ExpansionBundle, String> {
    let ui_meta = fs::symlink_metadata(ui).map_err(|e| format!("{}: {e}", ui.display()))?;
    if !ui_meta.is_dir() || ui_meta.file_type().is_symlink() {
        return Err(format!(
            "UI root must be a real directory: {}",
            ui.display()
        ));
    }
    let core = Package::from_assets(&package.assets)
        .map_err(|e| format!("locked package expansion context: {e:#}"))?;
    let mut html_inputs = BTreeMap::new();
    for name in ["pages", "components"] {
        let path = ui.join(name);
        if path.exists() {
            collect_templates(ui, name, &mut html_inputs)?;
        } else if let Ok(meta) = fs::symlink_metadata(&path) {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(format!("invalid UI template directory: {}", path.display()));
            }
        }
    }
    if html_inputs.is_empty() {
        return Err("ui/pages and ui/components contain no HTML templates".into());
    }
    let app_css = checked_ui_file(ui, "app.css")?;
    let original_app_css = app_css.clone();
    let app_theme_path = ui.join("clanker-theme.css");
    let app_theme = match fs::symlink_metadata(&app_theme_path) {
        Ok(_) => Some(checked_ui_file(ui, "clanker-theme.css")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", app_theme_path.display())),
    };
    let ui_lock_path = ui.join("ui.lock.json");
    let ui_lock = match fs::symlink_metadata(&ui_lock_path) {
        Ok(_) => Some(checked_ui_file(ui, "ui.lock.json")?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("{}: {error}", ui_lock_path.display())),
    };
    let total_ui = html_inputs.values().map(Vec::len).sum::<usize>()
        + app_css.len()
        + app_theme.as_ref().map_or(0, Vec::len)
        + ui_lock.as_ref().map_or(0, Vec::len);
    let total_inputs = total_ui + package.assets.values().map(Vec::len).sum::<usize>();
    let ui_file_count =
        html_inputs.len() + 1 + usize::from(app_theme.is_some()) + usize::from(ui_lock.is_some());
    if ui_file_count + package.assets.len() > MAX_FILES || total_inputs > MAX_TOTAL {
        return Err("captured input file/count/byte budget exceeded".into());
    }

    let mut templates = BTreeMap::new();
    let mut bindings = Vec::new();
    let mut used = BTreeSet::new();
    for (path, bytes) in &html_inputs {
        let input = std::str::from_utf8(bytes).map_err(|e| format!("{path}: {e}"))?;
        let result = expansion::expand(input, &core).map_err(|e| format!("{path}: {e:#}"))?;
        used.extend(result.used_components);
        bindings.extend(
            result
                .bindings
                .into_iter()
                .map(|b: Binding| BindingManifest {
                    template: path.clone(),
                    field_path: b.field_path,
                    expected_kind: b.expected_kind,
                    component: b.component,
                    attribute: b.attribute,
                }),
        );
        templates.insert(path.clone(), result.html);
    }

    let mut resources = Vec::new();
    let mut css = app_css;
    css.push(b'\n');
    css.extend_from_slice(core.theme());
    css.push(b'\n');
    let fonts = font_css(&package.assets)?;
    css.extend_from_slice(&fonts);
    css.push(b'\n');
    if let Some(reference) = package.assets.get("theme/reference.css") {
        css.extend_from_slice(reference);
    }
    css.push(b'\n');
    css.extend_from_slice(core.styles());
    if let Some(theme) = &app_theme {
        css.push(b'\n');
        css.extend_from_slice(theme);
    }
    if css.len() > MAX_FILE {
        return Err("expanded app.css exceeds 1 MiB".into());
    }
    resources.push(resource(
        "ui/app.css",
        None,
        Some(css.clone()),
        "stylesheet",
        &css,
    )?);

    if let Some(catalog) = core.property_catalog() {
        let mut js = format!(
            "export default {};\n",
            serde_json::to_string(catalog).map_err(|e| e.to_string())?
        );
        js = js
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        if js.len() > MAX_FILE {
            return Err("generated clanker-properties.js exceeds 1 MiB".into());
        }
        resources.push(resource(
            "ui/clanker-properties.js",
            None,
            Some(js.as_bytes().to_vec()),
            "metadata",
            js.as_bytes(),
        )?);
    }

    let names: BTreeMap<String, String> = [
        ("copy-field", "components/copy-field/interaction.js"),
        ("theme-switcher", "components/theme-switcher/install.js"),
        ("tooltip", "components/tooltip/install.js"),
        ("toast", "components/toast/install.js"),
        ("popover", "components/popover/interaction.js"),
        ("modal", "components/modal/interaction.js"),
        ("drawer", "components/drawer/interaction.js"),
        ("date-calendar", "components/date-calendar/interaction.js"),
        ("date-picker", "components/date-picker/interaction.js"),
        ("command-menu", "components/command-menu/interaction.js"),
        ("confirm-dialog", "components/confirm-dialog/interaction.js"),
    ]
    .into_iter()
    .map(|(n, p)| (n.into(), p.into()))
    .collect();
    let mut entries = BTreeSet::new();
    for name in used {
        let Some(component) = package.catalog.components().find(|c| c.name == name) else {
            continue;
        };
        for script in &component.assets.scripts {
            if names.get(&name) != Some(script) {
                return Err(format!(
                    "unsupported compiler-owned script for {name}: {script}"
                ));
            }
            entries.insert(script.clone());
        }
    }
    let mut module_paths = BTreeSet::new();
    if !entries.is_empty() {
        module_paths.insert("browser/lifecycle.js".to_owned());
        module_paths.extend(entries.iter().cloned());
        if entries.contains("components/date-picker/interaction.js") {
            module_paths.insert("components/date-calendar/interaction.js".into());
        }
    }
    for path in &module_paths {
        let bytes = package
            .assets
            .get(path)
            .ok_or_else(|| format!("missing locked module {path}"))?;
        std::str::from_utf8(bytes).map_err(|e| format!("module {path} is not UTF-8: {e}"))?;
        resources.push(resource(
            &format!("ui/clanker-ui/{path}"),
            Some(path),
            None,
            "module",
            bytes,
        )?);
    }
    if !entries.is_empty() {
        let js = runtime_entry(&entries);
        if js.len() > MAX_FILE {
            return Err("generated Native UI entry module exceeds 1 MiB".into());
        }
        resources.push(resource(
            "ui/clanker-ui.js",
            None,
            Some(js.as_bytes().to_vec()),
            "module",
            js.as_bytes(),
        )?);
        // Native discovers this conventional module like any ordinary app resource.
        // The producer, not the host, owns which library entrypoints it imports.
        let bootstrap = b"import './clanker-ui.js';\n";
        resources.push(resource(
            "ui/ui-package.js",
            None,
            Some(bootstrap.to_vec()),
            "module",
            bootstrap,
        )?);
    }
    for (source, target) in [
        (
            "fonts/geist-sans-variable.woff2",
            "ui/fonts/clanker-geist-sans-variable.woff2",
        ),
        (
            "fonts/geist-mono-variable.woff2",
            "ui/fonts/clanker-geist-mono-variable.woff2",
        ),
    ] {
        if let Some(bytes) = package.assets.get(source) {
            resources.push(resource(target, Some(source), None, "font", bytes)?);
        }
    }
    let mut managed_inputs = BTreeMap::<String, Vec<u8>>::new();
    for resource in &resources {
        if resource.path == "ui/app.css" {
            continue;
        }
        let relative = resource
            .path
            .strip_prefix("ui/")
            .ok_or_else(|| format!("managed resource outside ui/: {}", resource.path))?;
        let target = ui.join(relative);
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                let existing = checked_ui_file(ui, relative)?;
                let expected = if let Some(content) = &resource.content {
                    content.as_bytes().to_vec()
                } else {
                    let source = resource
                        .source
                        .as_deref()
                        .ok_or("resource has no source/content")?;
                    package
                        .assets
                        .get(source)
                        .ok_or_else(|| {
                            format!("resource source is outside locked snapshot: {source}")
                        })?
                        .clone()
                };
                if existing != expected {
                    return Err(format!(
                        "captured managed resource collides with generated output: {}",
                        target.display()
                    ));
                }
                managed_inputs.insert(format!("ui/{relative}"), existing);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", target.display())),
        }
    }
    resources.sort_by(|a, b| a.path.cmp(&b.path));

    if package.assets.len() + ui_file_count + managed_inputs.len() > MAX_INPUTS
        || total_inputs + managed_inputs.values().map(Vec::len).sum::<usize>() > MAX_TOTAL
    {
        return Err("captured input file/count/byte budget exceeded".into());
    }
    let mut inputs = package
        .assets
        .iter()
        .map(|(path, bytes)| LockedInput {
            path: format!("package/{path}"),
            digest: digest(bytes),
            bytes: bytes.len(),
        })
        .collect::<Vec<_>>();
    inputs.extend(html_inputs.iter().map(|(path, bytes)| LockedInput {
        path: format!("ui/{path}"),
        digest: digest(bytes),
        bytes: bytes.len(),
    }));
    inputs.extend(managed_inputs.iter().map(|(path, bytes)| LockedInput {
        path: path.clone(),
        digest: digest(bytes),
        bytes: bytes.len(),
    }));
    inputs.push(LockedInput {
        path: "ui/app.css".into(),
        digest: digest(&original_app_css),
        bytes: original_app_css.len(),
    });
    if let Some(theme) = &app_theme {
        inputs.push(LockedInput {
            path: "ui/clanker-theme.css".into(),
            digest: digest(theme),
            bytes: theme.len(),
        });
    }
    if let Some(lock) = &ui_lock {
        inputs.push(LockedInput {
            path: "ui/ui.lock.json".into(),
            digest: digest(lock),
            bytes: lock.len(),
        });
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let bundle = ExpansionBundle {
        schema_version: 1,
        runtime_abi: 2,
        template_engine: "minijinja-2.12.0".into(),
        package_digest: package.digest,
        templates,
        bindings,
        resources,
        inputs,
        consumed_inputs: if app_theme.is_some() {
            vec!["ui/clanker-theme.css".into()]
        } else {
            Vec::new()
        },
        entrypoints: if entries.is_empty() {
            Vec::new()
        } else {
            vec!["ui/clanker-ui.js".into()]
        },
    };
    validate_bundle(&bundle)?;
    if let Some(out) = out {
        let mut excluded = vec![ui, &package.root];
        excluded.extend_from_slice(extra_excluded_paths);
        write_output(out, &bundle, &package.assets, &excluded)?;
    }
    Ok(bundle)
}

fn validate_bundle(bundle: &ExpansionBundle) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    let mut folded_paths = BTreeSet::new();
    let mut total_output_bytes = 0usize;
    for template in bundle.templates.keys() {
        let path = format!("ui/{template}");
        if !safe_relative(template) || !folded_paths.insert(path.to_ascii_lowercase()) {
            return Err(format!("unsafe or colliding template path: {template}"));
        }
    }
    for resource in &bundle.resources {
        if !safe_relative(&resource.path)
            || !paths.insert(resource.path.clone())
            || !folded_paths.insert(resource.path.to_ascii_lowercase())
        {
            return Err(format!(
                "unsafe or duplicate resource path: {}",
                resource.path
            ));
        }
        if resource.source.is_some() == resource.content.is_some() {
            return Err(format!(
                "resource must have exactly one source/content: {}",
                resource.path
            ));
        }
        total_output_bytes = total_output_bytes.saturating_add(resource.bytes);
        if resource.bytes > MAX_FILE {
            return Err(format!("resource exceeds 1 MiB: {}", resource.path));
        }
        if let Some(content) = &resource.content {
            if content.len() != resource.bytes || digest(content.as_bytes()) != resource.digest {
                return Err(format!(
                    "resource content digest/length mismatch: {}",
                    resource.path
                ));
            }
        }
        if let Some(source) = &resource.source {
            if !safe_relative(source) {
                return Err(format!("unsafe resource source: {source}"));
            }
            let input = bundle
                .inputs
                .iter()
                .find(|input| input.path == format!("package/{source}"));
            if input.is_none_or(|input| {
                input.bytes != resource.bytes || input.digest != resource.digest
            }) {
                return Err(format!(
                    "resource source is not preserved by locked inputs: {source}"
                ));
            }
        }
    }
    for (path, content) in &bundle.templates {
        if content.len() > MAX_FILE {
            return Err(format!("template exceeds 1 MiB: {path}"));
        }
        total_output_bytes = total_output_bytes.saturating_add(content.len());
    }
    if bundle.resources.len() > MAX_FILES || total_output_bytes > MAX_TOTAL {
        return Err("expanded output file/count/byte budget exceeded".into());
    }
    let mut input_paths = BTreeSet::new();
    let mut total_input_bytes = 0usize;
    for input in &bundle.inputs {
        let relative = input
            .path
            .strip_prefix("package/")
            .or_else(|| input.path.strip_prefix("ui/"));
        if !relative.is_some_and(safe_manifest_path)
            || !input_paths.insert(input.path.to_ascii_lowercase())
            || input.bytes > MAX_FILE
        {
            return Err(format!(
                "unsafe, duplicate, or oversized input: {}",
                input.path
            ));
        }
        total_input_bytes = total_input_bytes.saturating_add(input.bytes);
    }
    if bundle.inputs.len() > MAX_INPUTS || total_input_bytes > MAX_TOTAL {
        return Err("expanded input file/count/byte budget exceeded".into());
    }
    let mut entries = BTreeSet::new();
    for entry in &bundle.entrypoints {
        if !entries.insert(entry)
            || !bundle
                .resources
                .iter()
                .any(|resource| resource.path == *entry && resource.kind == "module")
        {
            return Err(format!(
                "entrypoint must identify a unique emitted module: {entry}"
            ));
        }
    }
    let mut output_paths = paths.clone();
    output_paths.extend(bundle.templates.keys().map(|path| format!("ui/{path}")));
    validate_no_path_prefix_collisions(&output_paths)?;
    for path in &bundle.consumed_inputs {
        if !path.starts_with("ui/")
            || !safe_relative(path)
            || !bundle.inputs.iter().any(|input| input.path == *path)
            || paths.contains(path)
            || bundle
                .templates
                .contains_key(path.trim_start_matches("ui/"))
        {
            return Err(format!(
                "consumed input must identify a captured, non-output UI resource: {path}"
            ));
        }
    }
    Ok(())
}

fn validate_no_path_prefix_collisions(paths: &BTreeSet<String>) -> Result<(), String> {
    for path in paths {
        if paths.iter().any(|other| {
            other.len() > path.len()
                && other[..path.len()].eq_ignore_ascii_case(path)
                && other.as_bytes().get(path.len()) == Some(&b'/')
        }) {
            return Err(format!("output paths collide by prefix: {path}"));
        }
    }
    Ok(())
}

fn absolute_lexical(path: &Path) -> Result<std::path::PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut clean = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other.as_os_str()),
        }
    }
    Ok(clean)
}

fn canonicalize_missing(path: &Path) -> Result<std::path::PathBuf, String> {
    let absolute = absolute_lexical(path)?;
    let mut existing = absolute.as_path();
    let mut suffix = Vec::new();
    while !existing.exists() {
        let name = existing
            .file_name()
            .ok_or("output path has no existing ancestor")?;
        suffix.push(name.to_os_string());
        existing = existing
            .parent()
            .ok_or("output path has no existing ancestor")?;
    }
    let mut resolved = fs::canonicalize(existing).map_err(|e| e.to_string())?;
    for part in suffix.iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

fn write_output(
    out: &Path,
    bundle: &ExpansionBundle,
    assets: &BTreeMap<String, Vec<u8>>,
    forbidden: &[&Path],
) -> Result<(), String> {
    validate_bundle(bundle)?;
    let output_path = canonicalize_missing(out)?;
    for path in forbidden {
        let input_path = canonicalize_missing(path)?;
        if output_path.starts_with(&input_path) || input_path.starts_with(&output_path) {
            return Err(format!(
                "--out overlaps read-only input: {}",
                path.display()
            ));
        }
    }
    let parent = out
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    match fs::symlink_metadata(out) {
        Ok(meta) => {
            if !meta.is_dir()
                || meta.file_type().is_symlink()
                || fs::read_dir(out)
                    .map_err(|e| e.to_string())?
                    .next()
                    .is_some()
            {
                return Err(format!(
                    "--out must be a new or empty real directory: {}",
                    out.display()
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", out.display())),
    }
    let staging = tempfile::Builder::new()
        .prefix(".clanker-expand-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let root = staging.path();
    for (path, content) in &bundle.templates {
        write_new(root, &format!("ui/{path}"), content.as_bytes())?;
    }
    for resource in &bundle.resources {
        let bytes = if let Some(content) = &resource.content {
            content.as_bytes().to_vec()
        } else {
            let source = resource
                .source
                .as_deref()
                .ok_or("resource has no content or source")?;
            assets
                .get(source)
                .ok_or_else(|| format!("resource source is outside locked snapshot: {source}"))?
                .clone()
        };
        if bytes.len() != resource.bytes || digest(&bytes) != resource.digest {
            return Err(format!(
                "resource digest/length mismatch: {}",
                resource.path
            ));
        }
        write_new(root, &resource.path, &bytes)?;
    }
    let existing_empty = out.exists();
    if existing_empty {
        fs::remove_dir(out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let staging_path = staging.keep();
    fs::rename(&staging_path, out).map_err(|e| {
        let _ = fs::remove_dir_all(&staging_path);
        format!("atomic output publish failed: {e}")
    })
}

fn write_new(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    if !safe_relative(relative) {
        return Err(format!("unsafe output path: {relative}"));
    }
    let path = root.join(relative);
    let parent = path.parent().ok_or("invalid output path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    use std::io::Write;
    let mut file = options
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod output_validation_tests {
    use super::*;

    #[test]
    fn captured_input_prefix_does_not_reduce_package_path_budget() {
        let mut bundle = ExpansionBundle {
            schema_version: 1,
            runtime_abi: 2,
            template_engine: "minijinja-2.12.0".into(),
            package_digest: digest(b""),
            templates: BTreeMap::new(),
            bindings: Vec::new(),
            resources: Vec::new(),
            inputs: vec![LockedInput {
                path: "package/a/b/c/d/e/f/g/file.css".into(),
                bytes: 0,
                digest: digest(b""),
            }],
            consumed_inputs: Vec::new(),
            entrypoints: Vec::new(),
        };
        assert!(validate_bundle(&bundle).is_ok());
        bundle.inputs[0].path = "package/a/b/c/d/e/f/g/h/file.css".into();
        assert!(validate_bundle(&bundle).is_err());
        bundle.inputs[0].path = "unexpected/file.css".into();
        assert!(validate_bundle(&bundle).is_err());
    }
}
