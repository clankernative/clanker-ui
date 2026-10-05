use crate::ports::{Bundle, LoadedPackage, PackageSource};
use catalog_core::alert::{self, AlertInstance};
use catalog_core::avatar::{self, AvatarInstance};
use catalog_core::badge::{self, BadgeInstance};
use catalog_core::divider::{self, Alignment, DividerInstance, Orientation};
use catalog_core::empty_state::{self, EmptyStateInstance};
use catalog_core::form_field::{self, FieldKind, FormFieldInstance};
use catalog_core::icon::{self, IconCatalog, IconInstance, IconSize};
use catalog_core::layout::{self, AdmittedChildren};
use catalog_core::metric::{self, MetricInstance};
use catalog_core::page_header::{self, PageHeaderInstance};
use catalog_core::progress::{self, ProgressInstance};
use catalog_core::skeleton::{self, SkeletonInstance};
use catalog_core::status_indicator::{self, StatusIndicatorInstance};
use catalog_core::tag::{self, TagInstance};
use catalog_core::{breadcrumbs, data_table, filter_bar, pagination, select_field};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[path = "native_ports.rs"]
mod native_ports;

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LayoutFixture {
    options: serde_json::Value,
    children: BTreeMap<String, String>,
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

fn render_enterprise_fixture(
    name: &str,
    value: &serde_json::Value,
    fragment: &str,
) -> Result<String, String> {
    let mut options = value.get("options").unwrap_or(value).clone();
    if let Some(object) = options.as_object_mut() {
        object.remove("expectedHtml");
    }
    let json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    match name {
        "select-field" => {
            select_field::render(&select_field::SelectFieldInstance::parse(&json)?, fragment)
        }
        "breadcrumbs" => {
            breadcrumbs::render(&breadcrumbs::BreadcrumbsInstance::parse(&json)?, fragment)
        }
        "pagination" => {
            pagination::render(&pagination::PaginationInstance::parse(&json)?, fragment)
        }
        "data-table" if value.get("children").is_none() => {
            data_table::render(&data_table::DataTableInstance::parse(&json)?, fragment)
        }
        "data-table" => {
            let fixture: LayoutFixture =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if fixture.children.len() != 1 || !fixture.children.contains_key("body") {
                return Err("table fixture requires exactly body".into());
            }
            data_table::render_with_admitted_body(
                &data_table::DataTableInstance::parse(&json)?,
                fragment,
                AdmittedChildren::from_host_admitted(fixture.children["body"].clone()),
            )
        }
        "filter-bar" => {
            let fixture: LayoutFixture =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if fixture
                .children
                .keys()
                .any(|k| !["controls", "actions", "applied"].contains(&k.as_str()))
            {
                return Err("unsupported filter fixture slot".into());
            }
            let slots = filter_bar::FilterBarSlots {
                controls: AdmittedChildren::from_host_admitted(
                    fixture
                        .children
                        .get("controls")
                        .ok_or("filter fixture requires controls")?
                        .clone(),
                ),
                actions: fixture
                    .children
                    .get("actions")
                    .cloned()
                    .map(AdmittedChildren::from_host_admitted),
                applied: fixture
                    .children
                    .get("applied")
                    .cloned()
                    .map(AdmittedChildren::from_host_admitted),
            };
            filter_bar::render_with_slots(
                &filter_bar::FilterBarInstance::parse(&json)?,
                fragment,
                &slots,
            )
        }
        _ => Err(format!("unsupported enterprise component {name}")),
    }
}

impl NativeAdapter {
    /// Locked, build-time editor data; the Rust contracts remain the source of truth.
    pub fn property_catalog(package: &LoadedPackage) -> Result<serde_json::Value, String> {
        Self::property_catalog_context(package, None)
    }

    pub fn property_catalog_with_icons(
        package: &LoadedPackage,
        icons: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, String> {
        Self::property_catalog_context(package, Some(icons))
    }

    fn property_catalog_context(
        package: &LoadedPackage,
        icons: Option<&BTreeMap<String, String>>,
    ) -> Result<serde_json::Value, String> {
        let components = package
            .catalog
            .components()
            .map(|component| {
                let mut descriptor = serde_json::to_value(
                    catalog_core::properties::describe_component_properties(component)?,
                )
                .map_err(|error| error.to_string())?;
                let mut sample = match icons {
                    Some(icons) => catalog_core::properties::example_properties_with_icons(
                        &component.name,
                        icons,
                    )?,
                    None => catalog_core::properties::example_properties(&component.name)?,
                };
                if let Some(sample) = sample.as_object_mut() {
                    sample.remove("id");
                    sample.remove("component");
                }
                descriptor["sample"] = sample;
                Ok(descriptor)
            })
            .collect::<Result<Vec<serde_json::Value>, String>>()?;
        Ok(
            serde_json::json!({"schemaVersion":1,"package":package.catalog.package.name,"components":components}),
        )
    }

    /// Render a declared locked component fixture. This is component proof,
    /// not Native admission and not permission to consume raw app markup.
    pub fn fixture_html(
        source: &impl PackageSource,
        package: &LoadedPackage,
        name: &str,
        fixture: &str,
    ) -> Result<String, String> {
        let component = package
            .catalog
            .get(name)
            .ok_or("unknown or incomplete component")?;
        if !component.fixtures.iter().any(|path| path == fixture) {
            return Err("fixture is not declared by the component".into());
        }
        let value = serde_json::from_slice(&source.asset(package, fixture)?)
            .map_err(|error| format!("{fixture}: {error}"))?;
        let template = String::from_utf8(source.asset(package, &component.assets.template)?)
            .map_err(|_| "component template is not UTF-8")?;
        let icons = serde_json::from_slice(&source.asset(package, "icons.json")?)
            .map_err(|error| format!("icons.json: {error}"))?;
        let icon = package
            .catalog
            .get("icon")
            .ok_or("component proof requires complete icon")?;
        let icon_template = String::from_utf8(source.asset(package, &icon.assets.template)?)
            .map_err(|_| "icon template is not UTF-8")?;
        let rendered = if name == "date-picker" {
            let calendar = package
                .catalog
                .get("date-calendar")
                .ok_or("date-picker requires complete date-calendar")?;
            let calendar_template =
                String::from_utf8(source.asset(package, &calendar.assets.template)?)
                    .map_err(|_| "calendar template is not UTF-8")?;
            native_ports::render_with_dependencies(
                name,
                &value,
                &template,
                &icons,
                &icon_template,
                Some(&calendar_template),
            )?
        } else {
            native_ports::render(name, &value, &template, &icons, &icon_template)?
        };
        Ok(rendered.0)
    }

    pub fn verify(source: &impl PackageSource, package: &LoadedPackage) -> Result<(), String> {
        let default_theme =
            String::from_utf8(source.asset(package, &package.catalog.package.theme)?)
                .map_err(|_| "package theme is not UTF-8")?;
        let icon_geometries: BTreeMap<String, String> =
            serde_json::from_slice(&source.asset(package, "icons.json")?)
                .map_err(|error| format!("icons.json: {error}"))?;
        let _icon_catalog = IconCatalog::parse(
            &source.asset(package, "icon-catalog.json")?,
            &icon_geometries,
        )
        .map_err(|error| format!("icon-catalog.json: {error}"))?;
        if icon_geometries.len() != 100 {
            return Err("icon catalog must contain exactly 100 glyphs".into());
        }
        if package
            .catalog
            .package
            .resources
            .iter()
            .any(|path| path == "property-catalog.json")
        {
            let declared: serde_json::Value =
                serde_json::from_slice(&source.asset(package, "property-catalog.json")?)
                    .map_err(|error| format!("property-catalog.json: {error}"))?;
            if declared != Self::property_catalog_with_icons(package, &icon_geometries)? {
                return Err("property-catalog.json is stale; regenerate it with the properties command and explicitly refresh consumer locks".into());
            }
        }
        for component in package.catalog.components() {
            if component.target != "native-html" {
                return Err(format!("{} is not a native-html component", component.name));
            }
            let expected_scripts =
                catalog_core::expansion::component_browser_scripts(&component.name);
            if component
                .assets
                .scripts
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_scripts
            {
                return Err(format!(
                    "{} has unsupported browser resource declarations",
                    component.name
                ));
            }
            for contract in &component.assets.contracts {
                let bytes = source.asset(package, contract)?;
                if bytes.is_empty() || std::str::from_utf8(&bytes).is_err() {
                    return Err(format!(
                        "{} port type contract is empty or not UTF-8",
                        component.name
                    ));
                }
            }
            // Verify declared inputs, not browser execution or import admission.
            // The Native host independently admits the complete emitted graph.
            for script in expected_scripts {
                let bytes = source.asset(package, script)?;
                if bytes.is_empty() || std::str::from_utf8(&bytes).is_err() {
                    return Err(format!(
                        "{} browser resource is empty or not UTF-8",
                        component.name
                    ));
                }
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
                    || (package.catalog.package.token_metadata.is_none()
                        && !default_theme.contains(&format!("{token}:")))
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
                    for name in icon_geometries.keys() {
                        icon::render(
                            &IconInstance {
                                name: name.clone(),
                                size: IconSize::Medium,
                                label: None,
                            },
                            &template,
                            &icon_geometries,
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
                    let rendered = icon::render(&instance, &template, &icon_geometries)
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
                            let appearance = match fixture.instance.appearance {
                                catalog_core::alert::Appearance::Soft => "soft",
                                catalog_core::alert::Appearance::Outlined => "outlined",
                                catalog_core::alert::Appearance::Accent => "accent",
                            };
                            let heading = match fixture.instance.heading_level {
                                catalog_core::alert::HeadingLevel::H2 => "h2",
                                catalog_core::alert::HeadingLevel::H3 => "h3",
                                catalog_core::alert::HeadingLevel::H4 => "h4",
                            };
                            (
                                alert::render(&fixture.instance, &template, &icons)?,
                                fixture.expected_html,
                                vec![fixture.instance.tone.as_str(), appearance, heading],
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
                if component.name == "page-header" && component.contract.role == "page-heading" {
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let object = value
                        .as_object_mut()
                        .ok_or_else(|| format!("{fixture_path}: expected object"))?;
                    let expected = object
                        .remove("expectedHtml")
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .ok_or_else(|| format!("{fixture_path}: expectedHtml must be a string"))?;
                    let children = object
                        .remove("children")
                        .unwrap_or_else(|| serde_json::json!({}));
                    let children = children
                        .as_object()
                        .ok_or_else(|| format!("{fixture_path}: children must be an object"))?;
                    if children.keys().any(|key| key != "actions") {
                        return Err(format!(
                            "{fixture_path}: page-header accepts only actions child slot"
                        ));
                    }
                    let actions = children
                        .get("actions")
                        .map(|value| {
                            value
                                .as_str()
                                .map(|markup| {
                                    AdmittedChildren::from_host_admitted(markup.to_owned())
                                })
                                .ok_or_else(|| {
                                    format!("{fixture_path}: actions child must be a string")
                                })
                        })
                        .transpose()?;
                    let instance: PageHeaderInstance = serde_json::from_value(value)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let rendered =
                        page_header::render_with_actions(&instance, actions.as_ref(), &template)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != expected {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    let variants = [
                        if instance.description.is_some() {
                            "described"
                        } else {
                            "title-only"
                        },
                        if actions.is_some() { "actions" } else { "" },
                    ];
                    for variant in variants.into_iter().filter(|variant| !variant.is_empty()) {
                        if !component
                            .contract
                            .variants
                            .iter()
                            .any(|item| item == variant)
                        {
                            return Err(format!(
                                "{fixture_path}: unsupported page-header variant {variant}"
                            ));
                        }
                        covered.insert(variant.to_owned());
                    }
                    continue;
                }
                if ["avatar", "empty-state", "metric", "skeleton"]
                    .contains(&component.name.as_str())
                {
                    let bytes = source.asset(package, fixture_path)?;
                    let (rendered, expected, variants) = match component.name.as_str() {
                        "avatar" if component.contract.role == "identity" => {
                            let fixture: StaticFixture<AvatarInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            let mut variants = vec![
                                fixture.instance.size.as_str(),
                                fixture.instance.tone.as_str(),
                            ];
                            if fixture.instance.image_source.is_some() {
                                variants.push("image");
                            }
                            (
                                avatar::render(&fixture.instance, &template)?,
                                fixture.expected_html,
                                variants,
                            )
                        }
                        "empty-state" if component.contract.role == "section" => {
                            let fixture: StaticFixture<EmptyStateInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            (
                                empty_state::render(&fixture.instance, &template)?,
                                fixture.expected_html,
                                vec![
                                    fixture.instance.alignment.as_str(),
                                    fixture.instance.heading_level.as_str(),
                                ],
                            )
                        }
                        "metric" if component.contract.role == "metric" => {
                            let fixture: StaticFixture<MetricInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            let icons: BTreeMap<String, String> =
                                serde_json::from_slice(&source.asset(package, "icons.json")?)
                                    .map_err(|error| format!("icons.json: {error}"))?;
                            let mut variants = vec![fixture.instance.appearance.as_str()];
                            if let Some(trend) = fixture.instance.trend {
                                variants.push(trend.as_str());
                            }
                            if let Some(tone) = fixture.instance.trend_tone {
                                variants.push(tone.as_str());
                            }
                            (
                                metric::render(&fixture.instance, &template, &icons)?,
                                fixture.expected_html,
                                variants,
                            )
                        }
                        "skeleton" if component.contract.role == "presentation" => {
                            let fixture: StaticFixture<SkeletonInstance> =
                                serde_json::from_slice(&bytes)
                                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                            (
                                skeleton::render(&fixture.instance, &template)?,
                                fixture.expected_html,
                                vec![
                                    fixture.instance.shape.as_str(),
                                    fixture.instance.size.as_str(),
                                    fixture.instance.width.as_str(),
                                    if fixture.instance.animated {
                                        "animated"
                                    } else {
                                        "static"
                                    },
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
                if [
                    "card",
                    "cluster",
                    "container",
                    "grid",
                    "split",
                    "stack",
                    "cover",
                    "layer",
                    "pane",
                    "reel",
                    "sidebar",
                    "switch",
                ]
                .contains(&component.name.as_str())
                {
                    let expected_role = match component.name.as_str() {
                        "card" => "neutral-content-surface",
                        "cluster" => "neutral-wrapping-row",
                        "container" => "neutral-width-container",
                        "grid" => "responsive-grid",
                        "split" => "two-region-layout",
                        "stack" => "neutral-vertical-layout",
                        "cover" => "vertical-cover-layout",
                        "layer" => "overlapping-region-layout",
                        "pane" => "bounded-content-pane",
                        "reel" => "horizontal-scroll-region",
                        "sidebar" => "main-and-rail-layout",
                        "switch" => "responsive-row-layout",
                        _ => unreachable!(),
                    };
                    if component.contract.role != expected_role {
                        return Err(format!("{} has an incompatible role", component.name));
                    }
                    let fixture: LayoutFixture =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let slots = fixture
                        .children
                        .into_iter()
                        .map(|(name, markup)| (name, AdmittedChildren::from_host_admitted(markup)))
                        .collect();
                    let rendered =
                        layout::render(&component.name, &fixture.options, &slots, &template)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if rendered != fixture.expected_html {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    let options = &fixture.options;
                    let variant = match component.name.as_str() {
                        "card" => serde_json::from_value::<layout::CardConfig>(options.clone())
                            .map(|config| format!("{:?}", config.padding).to_ascii_lowercase()),
                        "cluster" => {
                            serde_json::from_value::<layout::ClusterConfig>(options.clone())
                                .map(|config| format!("{:?}", config.wrapping).to_ascii_lowercase())
                        }
                        "container" => {
                            serde_json::from_value::<layout::ContainerConfig>(options.clone())
                                .map(|config| format!("{:?}", config.width).to_ascii_lowercase())
                        }
                        "grid" => serde_json::from_value::<layout::GridConfig>(options.clone())
                            .map(|config| format!("{:?}", config.columns).to_ascii_lowercase()),
                        "split" => serde_json::from_value::<layout::SplitConfig>(options.clone())
                            .map(|config| format!("{:?}", config.ratio).to_ascii_lowercase()),
                        "stack" => serde_json::from_value::<layout::StackConfig>(options.clone())
                            .map(|config| format!("{:?}", config.gap).to_ascii_lowercase()),
                        "cover" => serde_json::from_value::<layout::CoverConfig>(options.clone())
                            .map(|config| format!("{:?}", config.height).to_ascii_lowercase()),
                        "layer" => serde_json::from_value::<layout::LayerConfig>(options.clone())
                            .map(|config| format!("{:?}", config.placement).to_ascii_lowercase()),
                        "pane" => serde_json::from_value::<layout::PaneConfig>(options.clone())
                            .map(|config| format!("{:?}", config.height).to_ascii_lowercase()),
                        "reel" => serde_json::from_value::<layout::ReelConfig>(options.clone())
                            .map(|config| format!("{:?}", config.item_width).to_ascii_lowercase()),
                        "sidebar" => {
                            serde_json::from_value::<layout::SidebarConfig>(options.clone())
                                .map(|config| format!("{:?}", config.side).to_ascii_lowercase())
                        }
                        "switch" => serde_json::from_value::<layout::SwitchConfig>(options.clone())
                            .map(|config| format!("{:?}", config.sizing).to_ascii_lowercase()),
                        _ => unreachable!(),
                    }
                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let variant = match variant.as_str() {
                        "nowrap" => "nowrap",
                        "wrap" => "wrap",
                        "reading" => "reading",
                        "compact" => "compact",
                        "standard" => "standard",
                        "wide" => "wide",
                        "full" => "full",
                        "none" => "none",
                        "spacious" => "spacious",
                        "generous" => "generous",
                        "responsive" => "responsive",
                        "two" => "two",
                        "three" => "three",
                        "four" => "four",
                        "equal" => "equal",
                        "startwide" => "start-wide",
                        "endwide" => "end-wide",
                        "startdominant" => "start-dominant",
                        "enddominant" => "end-dominant",
                        "topstart" => "top-start",
                        "topend" => "top-end",
                        "bottomstart" => "bottom-start",
                        "bottomend" => "bottom-end",
                        _ => variant.as_str(),
                    };
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
                    covered.insert(variant.to_owned());
                    continue;
                }
                if [
                    "activity-feed",
                    "definition-list",
                    "disclosure",
                    "button-group",
                    "checkbox-group",
                    "radio-group",
                    "toggle",
                    "tabs",
                    "segmented-control",
                    "progress-steps",
                    "copy-field",
                    "modal",
                    "drawer",
                    "popover",
                    "tooltip",
                    "toast",
                    "theme-switcher",
                    "command-menu",
                    "confirm-dialog",
                    "date-calendar",
                    "date-picker",
                    "file-upload",
                    "data-viewport",
                ]
                .contains(&component.name.as_str())
                {
                    let value: serde_json::Value =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let icons: BTreeMap<String, String> =
                        serde_json::from_slice(&source.asset(package, "icons.json")?)
                            .map_err(|error| format!("icons.json: {error}"))?;
                    let icon = package
                        .catalog
                        .get("icon")
                        .ok_or("native fixtures require a ready icon contract")?;
                    let icon_fragment =
                        String::from_utf8(source.asset(package, &icon.assets.template)?)
                            .map_err(|_| "icon template is not UTF-8")?;
                    if value
                        .get("expectedHtml")
                        .and_then(serde_json::Value::as_str)
                        .is_none()
                    {
                        return Err(format!(
                            "{fixture_path}: ready native fixtures require exact expectedHtml"
                        ));
                    }
                    let calendar = if component.name == "date-picker" {
                        let dependency = package
                            .catalog
                            .get("date-calendar")
                            .ok_or("date-picker requires a complete date-calendar contract")?;
                        Some(
                            String::from_utf8(source.asset(package, &dependency.assets.template)?)
                                .map_err(|_| "calendar template is not UTF-8")?,
                        )
                    } else {
                        None
                    };
                    let (_, variants) = native_ports::render_with_dependencies(
                        &component.name,
                        &value,
                        &template,
                        &icons,
                        &icon_fragment,
                        calendar.as_deref(),
                    )
                    .map_err(|error| format!("{fixture_path}: {error}"))?;
                    for variant in variants {
                        if !component.contract.variants.iter().any(|v| v == &variant) {
                            return Err(format!(
                                "{fixture_path}: unsupported {} variant {variant}",
                                component.name
                            ));
                        }
                        covered.insert(variant);
                    }
                    continue;
                }
                if [
                    "select-field",
                    "filter-bar",
                    "breadcrumbs",
                    "pagination",
                    "data-table",
                ]
                .contains(&component.name.as_str())
                {
                    let expected_role = match component.name.as_str() {
                        "select-field" => "native form control",
                        "filter-bar" => "presentation-only labelled control group",
                        "breadcrumbs" => "labelled navigation landmark",
                        "pagination" => "labelled pagination navigation landmark",
                        "data-table" => "keyboard-focusable labelled responsive table region",
                        _ => unreachable!(),
                    };
                    if component.contract.role != expected_role {
                        return Err(format!(
                            "{} role differs from typed contract",
                            component.name
                        ));
                    }
                    let value: serde_json::Value =
                        serde_json::from_slice(&source.asset(package, fixture_path)?)
                            .map_err(|error| format!("{fixture_path}: {error}"))?;
                    let rendered = render_enterprise_fixture(&component.name, &value, &template)
                        .map_err(|error| format!("{fixture_path}: {error}"))?;
                    if value
                        .get("expectedHtml")
                        .and_then(serde_json::Value::as_str)
                        != Some(rendered.as_str())
                    {
                        return Err(format!(
                            "{fixture_path}: rendered HTML differs from expected fixture"
                        ));
                    }
                    let options = value.get("options").unwrap_or(&value);
                    let variants: Vec<&str> = match component.name.as_str() {
                        "select-field" => vec![if options.get("error").is_some() {
                            "invalid"
                        } else if options.get("disabled").and_then(serde_json::Value::as_bool)
                            == Some(true)
                        {
                            "disabled"
                        } else {
                            "standard"
                        }],
                        "filter-bar" => vec![if options.get("resultSummary").is_some() {
                            "with-summary"
                        } else {
                            "without-summary"
                        }],
                        "breadcrumbs" => vec!["standard"],
                        "pagination" => vec![options
                            .get("variant")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("standard")],
                        "data-table" => vec![
                            if options
                                .get("captionVisible")
                                .and_then(serde_json::Value::as_bool)
                                == Some(true)
                            {
                                "caption-visible"
                            } else {
                                "caption-hidden"
                            },
                            if value.get("children").is_some() {
                                "host-composed-body"
                            } else {
                                "text-rows"
                            },
                        ],
                        _ => unreachable!(),
                    };
                    for variant in variants {
                        if !component.contract.variants.iter().any(|v| v == variant) {
                            return Err(format!("{fixture_path}: unsupported variant {variant}"));
                        }
                        covered.insert(variant.to_owned());
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
        if package.catalog.package.token_metadata.is_some() {
            crate::theming::token_catalog(package)?;
        }
        // Verify guidance after ordinary resource/fixture checks, retaining their
        // diagnostics. This is expansion/syntax proof, not host admission.
        if package
            .catalog
            .components()
            .any(|component| component.template_authoring.is_some())
        {
            let captured = catalog_core::expansion::Package::from_assets(&package.assets)
                .map_err(|error| format!("authoring example package: {error:#}"))?;
            for component in package.catalog.components() {
                if let Some(authoring) = &component.template_authoring {
                    for example in &authoring.examples {
                        let expanded =
                            catalog_core::expansion::expand(&example.template, &captured).map_err(
                                |error| {
                                    format!(
                                        "{} authoring example {}: {error:#}",
                                        component.name, example.name
                                    )
                                },
                            )?;
                        let mut templates = minijinja::Environment::new();
                        templates
                            .add_template("authoring.html", &expanded.html)
                            .map_err(|error| {
                                format!(
                                    "{} authoring example {} template syntax: {error}",
                                    component.name, example.name
                                )
                            })?;
                        if !expanded.used_components.contains(&component.name) {
                            return Err(format!(
                                "{} authoring example {} does not use its component",
                                component.name, example.name
                            ));
                        }
                    }
                }
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
