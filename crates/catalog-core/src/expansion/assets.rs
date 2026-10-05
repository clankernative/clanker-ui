use super::*;
use anyhow::Context;

const MAX_TOTAL: usize = 32 * 1024 * 1024;
const STATIC: [&str; 43] = [
    "badge",
    "divider",
    "status-indicator",
    "form-field",
    "tag",
    "alert",
    "progress",
    "avatar",
    "empty-state",
    "metric",
    "skeleton",
    "page-header",
    "container",
    "stack",
    "cluster",
    "grid",
    "split",
    "card",
    "cover",
    "layer",
    "pane",
    "reel",
    "sidebar",
    "switch",
    "select-field",
    "filter-bar",
    "data-table",
    "breadcrumbs",
    "pagination",
    "activity-feed",
    "definition-list",
    "disclosure",
    "button-group",
    "tabs",
    "segmented-control",
    "progress-steps",
    "checkbox-group",
    "radio-group",
    "toggle",
    "copy-field",
    "theme-switcher",
    "tooltip",
    "toast",
];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PropertyCatalog {
    schema_version: u32,
    package: String,
    components: Vec<PropertyComponent>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PropertyComponent {
    name: String,
    fields: serde_json::Value,
    slots: serde_json::Value,
    #[serde(default)]
    sample: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template_authoring: Option<crate::authoring::TemplateAuthoring>,
}

pub(super) fn from_assets(assets: &BTreeMap<String, Vec<u8>>) -> Result<Package> {
    ensure!(
        assets.values().all(|bytes| bytes.len() <= MAX_FILE),
        "Native UI package file exceeds limit"
    );
    ensure!(
        assets.values().map(Vec::len).sum::<usize>() <= MAX_TOTAL,
        "Native UI package input budget exceeded"
    );
    let manifest: crate::PackageManifest = serde_json::from_slice(
        assets
            .get("ui-package.json")
            .context("missing ui-package.json")?,
    )?;
    ensure!(
        manifest.schema_version == 1 && !manifest.summary.trim().is_empty(),
        "invalid package manifest"
    );
    ensure!(
        assets.contains_key(&manifest.theme),
        "package theme is not a captured asset"
    );
    for path in std::iter::once(&manifest.theme).chain(manifest.resources.iter()) {
        ensure!(
            crate::safe_path(path),
            "unsafe Native UI package asset path: {path}"
        );
        ensure!(
            assets.contains_key(path),
            "declared Native UI asset is not captured: {path}"
        );
    }
    let icons: BTreeMap<String, String> = serde_json::from_slice(
        assets
            .get("icons.json")
            .context("locked package must declare icons.json resource")?,
    )?;
    ensure!(!icons.is_empty(), "package icon catalog must not be empty");
    for (name, geometry) in &icons {
        let lower = geometry.to_ascii_lowercase();
        ensure!(
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                && !geometry.is_empty()
                && !lower.contains("<script")
                && !lower.contains("<svg")
                && !lower.contains("foreignobject")
                && !lower.contains("javascript:")
                && !lower.contains("href")
                && !lower.contains("style")
                && !geometry.contains("{{")
                && !geometry.contains("}}")
                && !geometry.contains("[[")
                && !geometry.contains('&')
                && geometry
                    .chars()
                    .all(|ch| !ch.is_control() || ch == '\n' || ch == '\t')
                && !lower.split_whitespace().any(|part| part
                    .strip_prefix('<')
                    .unwrap_or(part)
                    .starts_with("on")
                    && part.contains('=')),
            "unsafe package icon geometry: {name}"
        );
    }
    let fragment = String::from_utf8(
        assets
            .get("components/button/fragment.html")
            .context("locked package must declare button fragment resource")?
            .clone(),
    )?;
    for slot in ["[[open_tag]]", "[[icon]]", "[[label]]", "[[close_tag]]"] {
        ensure!(
            fragment.matches(slot).count() == 1,
            "button fragment must contain exactly one {slot} slot"
        );
    }
    let button = component(assets, "button")?;
    ensure!(
        button.schema_version == 1,
        "locked package button metadata is incompatible"
    );
    ensure!(
        button.name == "button"
            && button.status == crate::Status::Ready
            && button.target == "native-html"
            && button.contract.role == "button"
            && button.assets.scripts.is_empty()
            && button.assets.styles == "components/button/styles.css",
        "locked package button metadata is incompatible"
    );
    let mut styles = assets
        .get(&button.assets.styles)
        .context("button stylesheet is not a locked asset")?
        .clone();
    let icon_fragment = if let Some(bytes) = assets.get("components/icon/component.json") {
        let icon: crate::Component = serde_json::from_slice(bytes).map_err(anyhow::Error::msg)?;
        if icon.status == crate::Status::Ready {
            ensure!(
                icon.name == "icon"
                    && icon.target == "native-html"
                    && icon.contract.role == "img"
                    && icon.assets.scripts.is_empty()
                    && icon.assets.styles == "components/icon/styles.css",
                "locked package icon metadata is incompatible"
            );
            let html = String::from_utf8(
                assets
                    .get("components/icon/fragment.html")
                    .context("locked icon fragment is missing")?
                    .clone(),
            )?;
            for slot in ["[[attributes]]", "[[geometry]]"] {
                ensure!(
                    html.matches(slot).count() == 1,
                    "icon fragment must contain exactly one {slot} slot"
                );
            }
            styles.push(b'\n');
            styles.extend_from_slice(
                assets
                    .get(&icon.assets.styles)
                    .context("icon stylesheet is not a locked asset")?,
            );
            Some(html)
        } else {
            None
        }
    } else {
        None
    };
    // The host captures component manifests before selecting renderer-supported components;
    // validate every captured manifest's frozen top-level identity/status contract.
    let mut ready_component_names = BTreeSet::new();
    for (path, bytes) in assets
        .iter()
        .filter(|(path, _)| path.starts_with("components/") && path.ends_with("/component.json"))
    {
        let expected = path
            .strip_prefix("components/")
            .unwrap()
            .strip_suffix("/component.json")
            .unwrap();
        let c: crate::Component =
            serde_json::from_slice(bytes).with_context(|| format!("parse {path}"))?;
        ensure!(
            c.schema_version == 1
                && c.name == expected
                && matches!(c.status, crate::Status::Ready | crate::Status::Draft),
            "invalid component metadata: {path}"
        );
        if c.status == crate::Status::Ready {
            ready_component_names.insert(c.name.clone());
            for asset in c
                .assets
                .scripts
                .iter()
                .chain(c.assets.contracts.iter())
                .chain([&c.assets.template, &c.assets.styles])
                .chain(c.fixtures.iter())
            {
                ensure!(
                    crate::safe_path(asset),
                    "unsafe component asset path: {asset}"
                );
                ensure!(
                    assets.contains_key(asset),
                    "ready component asset is not captured: {asset}"
                );
            }
        }
    }
    let roles = [
        ("tag", "metadata-label"),
        ("alert", "section"),
        ("empty-state", "section"),
        ("progress", "progressbar"),
        ("form-field", "native form control"),
        ("avatar", "identity"),
        ("metric", "metric"),
        ("skeleton", "presentation"),
        ("page-header", "page-heading"),
        ("container", "neutral-width-container"),
        ("stack", "neutral-vertical-layout"),
        ("cluster", "neutral-wrapping-row"),
        ("grid", "responsive-grid"),
        ("split", "two-region-layout"),
        ("card", "neutral-content-surface"),
        ("cover", "vertical-cover-layout"),
        ("layer", "overlapping-region-layout"),
        ("pane", "bounded-content-pane"),
        ("reel", "horizontal-scroll-region"),
        ("sidebar", "main-and-rail-layout"),
        ("switch", "responsive-row-layout"),
        ("select-field", "native form control"),
        ("filter-bar", "presentation-only labelled control group"),
        (
            "data-table",
            "keyboard-focusable labelled responsive table region",
        ),
        ("breadcrumbs", "labelled navigation landmark"),
        ("pagination", "labelled pagination navigation landmark"),
        (
            "activity-feed",
            "labelled ordered list with machine-readable native time elements",
        ),
        ("definition-list", "native dl with paired dt and dd values"),
        ("disclosure", "native details and summary disclosure"),
        (
            "button-group",
            "named role=group containing two to six host-admitted buttons",
        ),
        ("tabs", "navigation"),
        ("segmented-control", "navigation"),
        ("progress-steps", "navigation"),
        ("checkbox-group", "native form control group"),
        ("radio-group", "native form control group"),
        ("toggle", "native Boolean form control"),
        ("copy-field", "native form control"),
        ("theme-switcher", "group"),
        ("tooltip", "tooltip"),
        ("toast", "region"),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let mut static_fragments = BTreeMap::new();
    for name in STATIC {
        let Some(bytes) = assets.get(&format!("components/{name}/component.json")) else {
            continue;
        };
        let c: crate::Component = serde_json::from_slice(bytes).map_err(anyhow::Error::msg)?;
        ensure!(c.schema_version == 1, "invalid component metadata: {name}");
        if c.status != crate::Status::Ready {
            continue;
        }
        let expected_scripts = component_browser_scripts(name);
        ensure!(
            c.name == name
                && c.target == "native-html"
                && c.assets
                    .scripts
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    == expected_scripts
                && c.assets.styles == format!("components/{name}/styles.css"),
            "locked package {name} metadata is incompatible"
        );
        if let Some(role) = roles.get(name) {
            ensure!(
                c.contract.role == *role,
                "locked package {name} role is incompatible"
            );
        }
        if name == "button-group" {
            ensure!(
                c.dependencies.iter().any(|d| d == "button"),
                "locked package button-group must depend on button"
            );
        }
        if matches!(
            name,
            "metric" | "progress-steps" | "copy-field" | "theme-switcher" | "tooltip" | "toast"
        ) {
            ensure!(
                c.dependencies.iter().any(|d| d == "icon")
                    && icon_fragment.is_some()
                    && assets.contains_key("components/icon/component.json"),
                "locked package {name} requires the ready icon component and fragment closure"
            );
        }
        let html = String::from_utf8(
            assets
                .get(&c.assets.template)
                .context("static component fragment is not a locked asset")?
                .clone(),
        )
        .with_context(|| format!("{name} fragment is not UTF-8"))?;
        let slots: &[&str] = match name {
            "badge" => &["[[attributes]]", "[[icon]]", "[[label]]"],
            "divider" => &["[[attributes]]", "[[label]]"],
            "status-indicator" => &["[[attributes]]", "[[label]]", "[[detail]]"],
            "form-field" | "select-field" => &[
                "[[attributes]]",
                "[[label]]",
                "[[control]]",
                "[[description]]",
            ],
            "tag" => &["[[tag]]"],
            "copy-field" => &["[[attributes]]", "[[control]]", "[[description]]"],
            "theme-switcher" => &["[[themeSwitcher]]"],
            "tooltip" => &["[[tooltip]]"],
            "toast" => &["[[toastRegion]]"],
            "alert" => &[
                "[[attributes]]",
                "[[icon]]",
                "[[heading]]",
                "[[body]]",
                "[[recovery]]",
            ],
            "progress" => &[
                "[[attributes]]",
                "[[label]]",
                "[[progress]]",
                "[[suffix]]",
                "[[detail]]",
            ],
            "avatar" => &["[[avatar]]"],
            "empty-state" => &["[[empty-state]]"],
            "metric" => &["[[metric]]"],
            "skeleton" => &["[[skeleton]]"],
            "page-header" => &["[[page-header]]"],
            "container" | "stack" | "cluster" | "grid" => &["[[class]]", "[[body]]"],
            "split" => &["[[class]]", "[[start]]", "[[end]]"],
            "card" => &["[[class]]", "[[header]]", "[[body]]", "[[actions]]"],
            "cover" => &["[[class]]", "[[top]]", "[[primary]]", "[[bottom]]"],
            "layer" => &["[[class]]", "[[base]]", "[[foreground]]"],
            "pane" => &["[[class]]", "[[header]]", "[[body]]", "[[footer]]"],
            "reel" => &["[[class]]", "[[label]]", "[[body]]"],
            "sidebar" => &["[[class]]", "[[regions]]"],
            "switch" => &["[[class]]", "[[body]]"],
            "filter-bar" => &[
                "[[attributes]]",
                "[[label]]",
                "[[summary]]",
                "[[controls]]",
                "[[actions]]",
                "[[applied]]",
            ],
            "data-table" => &["[[attributes]]", "[[caption]]", "[[head]]", "[[body]]"],
            "breadcrumbs" | "pagination" | "tabs" | "segmented-control" | "progress-steps" => {
                &["[[attributes]]", "[[label]]", "[[items]]"]
            }
            "activity-feed" => &["[[attributes]]", "[[label]]", "[[entries]]"],
            "definition-list" => &["[[attributes]]", "[[items]]"],
            "disclosure" => &["[[attributes]]", "[[summary]]", "[[body]]"],
            "button-group" => &["[[attributes]]", "[[buttons]]"],
            "checkbox-group" | "radio-group" => &[
                "[[attributes]]",
                "[[legend]]",
                "[[choices]]",
                "[[description]]",
            ],
            "toggle" => &["[[attributes]]", "[[label]]", "[[description]]"],
            _ => unreachable!(),
        };
        ensure!(
            slots.iter().all(|slot| html.matches(slot).count() == 1)
                && html.matches("[[").count() == slots.len(),
            "{name} fragment has missing or unsupported slots"
        );
        styles.push(b'\n');
        styles.extend_from_slice(
            assets
                .get(&c.assets.styles)
                .context("component stylesheet is not a locked asset")?,
        );
        static_fragments.insert(name.to_owned(), html);
    }
    let property_catalog = assets
        .get("property-catalog.json")
        .map(|bytes| -> Result<serde_json::Value> {
            let catalog: PropertyCatalog =
                serde_json::from_slice(bytes).context("parse package property catalog")?;
            ensure!(
                catalog.schema_version == 1
                    && catalog.package == "@clanker/vanilla"
                    && catalog.package == manifest.name,
                "Native UI property catalog identity does not match package"
            );
            let ready = ready_component_names.clone();
            let mut names = BTreeSet::new();
            for entry in &catalog.components {
                let _ = (&entry.fields, &entry.slots, &entry.sample);
                if let Some(authoring) = &entry.template_authoring {
                    authoring
                        .validate(&entry.name, "native-html")
                        .map_err(anyhow::Error::msg)?;
                }
                ensure!(
                    !entry.name.is_empty()
                        && ready.contains(entry.name.as_str())
                        && names.insert(entry.name.as_str()),
                    "invalid or duplicate component in Native UI property catalog: {}",
                    entry.name
                );
            }
            Ok(serde_json::to_value(catalog)?)
        })
        .transpose()?;
    Ok(Package {
        identity: manifest.name,
        property_catalog,
        theme: assets[&manifest.theme].clone(),
        styles,
        fragment,
        icon_fragment,
        static_fragments,
        icons,
    })
}

fn component(assets: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<crate::Component> {
    crate::Component::parse(
        assets
            .get(&format!("components/{name}/component.json"))
            .with_context(|| format!("locked package must include {name} metadata"))?,
    )
    .map_err(anyhow::Error::msg)
}
