use crate::ports::{Bundle, LoadedPackage, PackageSource};
use catalog_core::alert::{self, AlertInstance};
use catalog_core::badge::{self, BadgeInstance};
use catalog_core::divider::{self, Alignment, DividerInstance, Orientation};
use catalog_core::form_field::{self, FieldKind, FormFieldInstance};
use catalog_core::icon::{self, IconInstance, IconSize};
use catalog_core::progress::{self, ProgressInstance};
use catalog_core::status_indicator::{self, StatusIndicatorInstance};
use catalog_core::tag::{self, TagInstance};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DividerFixture {
    variant: String,
    orientation: Orientation,
    #[serde(default)]
    alignment: Alignment,
    #[serde(default)]
    label: Option<String>,
    expected_html: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StaticFixture<T> {
    #[serde(flatten)]
    instance: T,
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
            if component.name == "button" {
                for field in &component.contract.required_fields {
                    if !template.contains(&format!("{{{{ {field} }}}}")) {
                        return Err(format!(
                            "{} missing required template field {field}",
                            component.name
                        ));
                    }
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
                    for name in icons.keys() {
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
                if component.name == "divider" && component.contract.role == "separator" {
                    let fixture: DividerFixture =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if fixture.variant != fixture.orientation.as_str()
                        || !component.contract.variants.contains(&fixture.variant)
                    {
                        return Err(format!("{fixture_path}: invalid divider orientation"));
                    }
                    let rendered = divider::render(
                        &DividerInstance {
                            orientation: fixture.orientation,
                            alignment: fixture.alignment,
                            label: fixture.label,
                        },
                        &template,
                    )
                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    covered.insert(fixture.variant);
                    continue;
                }
                if component.name == "badge" && component.contract.role == "status-label" {
                    let fixture: StaticFixture<BadgeInstance> =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let icons: BTreeMap<String, String> =
                        serde_json::from_slice(&source.asset(package, "icons.json")?)
                            .map_err(|error| format!("icons.json: {error}"))?;
                    let tone = fixture.instance.tone.as_str();
                    if !component
                        .contract
                        .variants
                        .iter()
                        .any(|variant| variant == tone)
                    {
                        return Err(format!("{fixture_path}: unsupported badge tone"));
                    }
                    let rendered = badge::render(&fixture.instance, &template, &icons)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    covered.insert(tone.into());
                    continue;
                }
                if component.name == "status-indicator" && component.contract.role == "text" {
                    let fixture: StaticFixture<StatusIndicatorInstance> =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let tone = fixture.instance.tone.as_str();
                    if !component
                        .contract
                        .variants
                        .iter()
                        .any(|variant| variant == tone)
                    {
                        return Err(format!("{fixture_path}: unsupported status tone"));
                    }
                    let rendered = status_indicator::render(&fixture.instance, &template)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    covered.insert(tone.into());
                    continue;
                }
                if ["tag", "alert", "progress"].contains(&component.name.as_str()) {
                    let bytes = source.asset(package, fixture_path)?;
                    let icons: BTreeMap<String, String> =
                        serde_json::from_slice(&source.asset(package, "icons.json")?)
                            .map_err(|error| format!("icons.json: {error}"))?;
                    let (rendered, expected, variants) = match component.name.as_str() {
                        "tag" if component.contract.role == "metadata-label" => {
                            let fixture: StaticFixture<TagInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            (
                                tag::render(&fixture.instance, &template, &icons)?,
                                fixture.expected_html,
                                vec![
                                    fixture.instance.tone.as_str(),
                                    fixture.instance.size.as_str(),
                                ],
                            )
                        }
                        "alert" if component.contract.role == "section" => {
                            let fixture: StaticFixture<AlertInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            (
                                alert::render(&fixture.instance, &template, &icons)?,
                                fixture.expected_html,
                                vec![fixture.instance.tone.as_str()],
                            )
                        }
                        "progress" if component.contract.role == "progressbar" => {
                            let fixture: StaticFixture<ProgressInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            (
                                progress::render(&fixture.instance, &template)?,
                                fixture.expected_html,
                                vec![
                                    fixture.instance.state.as_str(),
                                    fixture.instance.tone.as_str(),
                                    fixture.instance.size.as_str(),
                                ],
                            )
                        }
                        _ => return Err(format!("{} has an incompatible role", component.name)),
                    };
                    if rendered != expected {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    for variant in variants {
                        if !component
                            .contract
                            .variants
                            .iter()
                            .any(|item| item == variant)
                        {
                            return Err(format!(
                                "{fixture_path}: unsupported {} variant {variant}",
                                component.name
                            ));
                        }
                        covered.insert(variant.into());
                    }
                    continue;
                }
                if component.name == "form-field"
                    && component.contract.role == "native form control"
                {
                    let fixture: StaticFixture<FormFieldInstance> =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let variant = if fixture.instance.kind == FieldKind::Textarea {
                        "textarea"
                    } else {
                        fixture.instance.input_type.unwrap_or_default().as_str()
                    };
                    if !component
                        .contract
                        .variants
                        .iter()
                        .any(|item| item == variant)
                    {
                        return Err(format!("{fixture_path}: unsupported form field kind"));
                    }
                    let rendered = form_field::render(&fixture.instance, &template)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    covered.insert(variant.into());
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
