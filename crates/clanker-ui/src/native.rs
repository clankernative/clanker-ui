use crate::ports::{Bundle, LoadedPackage, PackageSource};
use catalog_core::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub struct NativeAdapter;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    variant: String,
    button_label: String,
    expected_html: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IconFixture {
    name: String,
    size: IconSize,
    label: Option<String>,
    expected_html: String,
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildManifest<'a> {
    schema_version: u32,
    package: &'a str,
    package_digest: &'a str,
    components: &'a [String],
    files: BTreeMap<&'a str, String>,
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

impl NativeAdapter {
    pub fn verify(source: &impl PackageSource, package: &LoadedPackage) -> Result<(), String> {
        let default_theme =
            String::from_utf8(source.asset(package, &package.catalog.package.theme)?)
                .map_err(|_| "package theme is not UTF-8")?;
        for component in package.catalog.components() {
            if component.target != "native-html" {
                return Err(format!("{} is not a native-html component", component.name));
            }
            if !component.assets.scripts.is_empty() {
                return Err(format!(
                    "{} requires JavaScript; import-graph admission is not implemented",
                    component.name
                ));
            }
            let template = String::from_utf8(source.asset(package, &component.assets.template)?)
                .map_err(|_| format!("{} template is not UTF-8", component.name))?;
            let css = String::from_utf8(source.asset(package, &component.assets.styles)?)
                .map_err(|_| format!("{} styles are not UTF-8", component.name))?;
            for field in &component.contract.required_fields {
                if !template.contains(&format!("{{{{ {field} }}}}")) {
                    return Err(format!(
                        "{} missing required template field {field}",
                        component.name
                    ));
                }
            }
            if component.contract.role == "button"
                && template
                    .replace("{{variant}}", "")
                    .replace("{{ button_label }}", "")
                    .contains("{{")
            {
                return Err(format!(
                    "{} has an undeclared template field",
                    component.name
                ));
            }
            for token in &component.tokens {
                if !token.starts_with("--cui-")
                    || !default_theme.contains(&format!("{token}:"))
                    || !css.contains(token)
                {
                    return Err(format!(
                        "{} token {token} missing from CSS or defaults",
                        component.name
                    ));
                }
            }
            let mut covered = BTreeSet::new();
            for fixture_path in &component.fixtures {
                if component.contract.role == "img" {
                    if component.name != "icon"
                        || template.matches("[[attributes]]").count() != 1
                        || template.matches("[[geometry]]").count() != 1
                        || template.matches("[[").count() != 2
                    {
                        return Err(format!(
                            "{} must declare one attributes and geometry fragment slot",
                            component.name
                        ));
                    }
                    let icons: BTreeMap<String, String> =
                        serde_json::from_slice(&source.asset(package, "icons.json")?)
                            .map_err(|error| format!("icons.json: {error}"))?;
                    if icons.len() != 100 {
                        return Err("icon catalog must contain exactly 100 glyphs".into());
                    }
                    for (name, _) in &icons {
                        icon::render(
                            &IconInstance {
                                name: name.clone(),
                                size: IconSize::Medium,
                                label: None,
                            },
                            &template,
                            &icons,
                        )
                        .map_err(|error| format!("icons.json ({name}): {error}"))?;
                    }
                    let fixture: IconFixture =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if !component
                        .contract
                        .variants
                        .contains(&fixture.size.as_str().to_string())
                    {
                        return Err(format!("{fixture_path}: unsupported icon size"));
                    }
                    let instance = IconInstance {
                        name: fixture.name,
                        size: fixture.size,
                        label: fixture.label,
                    };
                    let rendered = icon::render(&instance, &template, &icons)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    covered.insert(fixture.size.as_str().to_string());
                    continue;
                }
                if component.contract.role != "button" {
                    return Err(format!(
                        "{} fixture renderer not supported yet",
                        component.name
                    ));
                }
                let fixture: Fixture =
                    serde_json::from_slice(&source.asset(package, fixture_path)?)
                        .map_err(|e| format!("{fixture_path}: {e}"))?;
                if fixture.button_label.trim().is_empty()
                    || !component.contract.variants.contains(&fixture.variant)
                {
                    return Err(format!("{fixture_path}: invalid label or variant"));
                }
                covered.insert(fixture.variant.clone());
                let rendered = template
                    .replace("{{variant}}", &fixture.variant)
                    .replace("{{ button_label }}", &escape_html(&fixture.button_label));
                if rendered != fixture.expected_html {
                    return Err(format!(
                        "{fixture_path}: rendered HTML differs from expected fixture"
                    ));
                }
            }
            if covered.len() != component.contract.variants.len() {
                return Err(format!(
                    "{} fixtures do not cover every variant",
                    component.name
                ));
            }
        }
        Ok(())
    }

    pub fn plan(
        source: &impl PackageSource,
        package: &LoadedPackage,
        name: &str,
        variant: &str,
        app_theme: &[u8],
    ) -> Result<Bundle, String> {
        let chosen = package
            .catalog
            .get(name)
            .ok_or_else(|| format!("unknown or unready component: {name}"))?;
        if !chosen.contract.variants.iter().any(|v| v == variant) {
            return Err(format!("unsupported {name} variant: {variant}"));
        }
        let closure = package.catalog.resolve(name)?;
        let mut files = BTreeMap::new();
        let mut css = source.asset(package, &package.catalog.package.theme)?;
        css.extend_from_slice(b"\n/* package components */\n");
        let mut components = Vec::new();
        for component in closure {
            if component.target != "native-html" {
                return Err(format!("{} does not support native-html", component.name));
            }
            if !component.assets.scripts.is_empty() {
                return Err(format!(
                    "{} requires JavaScript; import-graph admission is not implemented",
                    component.name
                ));
            }
            let selected_variant = if component.name == name {
                variant
            } else {
                &component.contract.variants[0]
            };
            let template = String::from_utf8(source.asset(package, &component.assets.template)?)
                .map_err(|_| format!("{} template is not UTF-8", component.name))?;
            if !template.contains("{{variant}}") {
                return Err(format!("{} template lacks variant slot", component.name));
            }
            let rendered = template.replace("{{variant}}", selected_variant);
            let path = format!("ui/components/{}.html", component.name);
            files.insert(path, rendered.into_bytes());
            css.extend_from_slice(&source.asset(package, &component.assets.styles)?);
            css.push(b'\n');
            components.push(component.id(&package.catalog.package.name));
        }
        css.extend_from_slice(b"\n/* app theme overrides */\n");
        css.extend_from_slice(app_theme);
        css.push(b'\n');
        files.insert("ui/app.css".into(), css);
        // Agent navigation is derived from ready metadata, never a parallel handwritten list.
        let mut categories: BTreeMap<&str, Vec<&catalog_core::Component>> = BTreeMap::new();
        for component in package
            .catalog
            .components()
            .filter(|c| c.target == "native-html")
        {
            categories
                .entry(&component.category)
                .or_default()
                .push(component);
        }
        let mut index = format!(
            "# {}\n\nLocked package: `{}`\n\n",
            package.catalog.package.name, package.digest
        );
        for (category, entries) in categories {
            index.push_str(&format!(
                "- [{}]({category}.md) — {} components\n",
                category,
                entries.len()
            ));
            let mut page = format!("# {} / {}\n\n", package.catalog.package.name, category);
            for component in entries {
                page.push_str(&format!(
                    "## {}\n\n{}\n\n- Use when: {}\n- Avoid when: {}\n- Variants: {}\n- Inspect: `clanker-ui describe {} --lock <app-lock>`\n\n",
                    component.name, component.summary, component.agent.use_when,
                    component.agent.avoid_when, component.contract.variants.join(", "), component.name
                ));
            }
            files.insert(format!("catalog/{category}.md"), page.into_bytes());
        }
        files.insert("catalog/index.md".into(), index.into_bytes());
        let digests = files
            .iter()
            .map(|(path, bytes)| (path.as_str(), hash(bytes)))
            .collect();
        let manifest = BuildManifest {
            schema_version: 1,
            package: &package.catalog.package.name,
            package_digest: &package.digest,
            components: &components,
            files: digests,
        };
        files.insert(
            "build-manifest.json".into(),
            serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        );
        Ok(Bundle {
            files,
            package_digest: package.digest.clone(),
            components,
        })
    }
}
