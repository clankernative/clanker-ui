//! Clanker Native build-time composition. No app I/O or browser execution.

use crate::ports::{Bundle, LoadedPackage, PackageSource};
use catalog_core::button::{
    BoolValue, ButtonInstance, Destination, HrefValue, Size, TextValue, Variant,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ButtonInstances {
    pub schema_version: u32,
    pub instances: Vec<ButtonInstance>,
}

impl ButtonInstances {
    pub fn parse(bytes: &[u8], package: &LoadedPackage) -> Result<Self, String> {
        let config: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if config.schema_version != 1 || config.instances.is_empty() || config.instances.len() > 64
        {
            return Err("button usage must contain 1–64 instances at schema version 1".into());
        }
        let mut names = BTreeSet::new();
        for instance in &config.instances {
            instance.validate(&package.catalog.package.name)?;
            if !names.insert(&instance.id) {
                return Err(format!("duplicate button instance: {}", instance.id));
            }
        }
        Ok(config)
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn text(value: &TextValue) -> String {
    match value {
        TextValue::Literal { value } => escape(value),
        TextValue::Field { path } => format!("{{{{ {path} }}}}"),
    }
}

fn variant(value: Variant) -> &'static str {
    match value {
        Variant::Primary => "primary",
        Variant::Secondary => "secondary",
        Variant::Danger => "danger",
        Variant::Quiet => "quiet",
    }
}

fn size(value: Size) -> &'static str {
    match value {
        Size::Compact => "compact",
        Size::Standard => "standard",
    }
}

pub fn render(
    instance: &ButtonInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    render_states(
        instance,
        fragment,
        icons,
        instance.busy.literal(),
        instance.disabled.literal(),
    )
}

fn render_states(
    instance: &ButtonInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
    busy: Option<bool>,
    disabled: Option<bool>,
) -> Result<String, String> {
    if busy.is_none() {
        let BoolValue::Field { path } = &instance.busy else {
            unreachable!()
        };
        return Ok(format!(
            "{{% if {path} %}}{}{{% else %}}{}{{% endif %}}",
            render_states(instance, fragment, icons, Some(true), disabled)?,
            render_states(instance, fragment, icons, Some(false), disabled)?
        ));
    }
    if disabled.is_none() {
        let BoolValue::Field { path } = &instance.disabled else {
            unreachable!()
        };
        return Ok(format!(
            "{{% if {path} %}}{}{{% else %}}{}{{% endif %}}",
            render_states(instance, fragment, icons, busy, Some(true))?,
            render_states(instance, fragment, icons, busy, Some(false))?
        ));
    }
    render_concrete(instance, fragment, icons, busy.unwrap(), disabled.unwrap())
}

fn render_concrete(
    instance: &ButtonInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
    busy: bool,
    disabled_state: bool,
) -> Result<String, String> {
    let disabled = disabled_state || busy;
    let classes = format!(
        "cui-button cui-button--{} cui-button--{}{}",
        variant(instance.variant),
        size(instance.size),
        if instance.edge_aligned {
            " cui-button--edge-aligned"
        } else {
            ""
        }
    );
    let mut attrs = format!("class=\"{classes}\" data-cui-component=\"button\"");
    let (tag, close) = match &instance.destination {
        Destination::Action => {
            attrs.push_str(" type=\"button\"");
            ("button", "button")
        }
        Destination::Submit => {
            attrs.push_str(" type=\"submit\"");
            ("button", "button")
        }
        Destination::Link { href } if disabled => {
            let _ = href;
            attrs.push_str(" role=\"link\" aria-disabled=\"true\"");
            ("span", "span")
        }
        Destination::Link { href } => {
            let value = match href {
                HrefValue::Literal { value } => escape(value),
                HrefValue::Route { name } => format!("{{{{ routes.{name}() }}}}"),
            };
            attrs.push_str(&format!(" href=\"{value}\""));
            ("a", "a")
        }
    };
    if disabled && tag == "button" {
        attrs.push_str(" disabled");
    }
    if busy {
        attrs.push_str(" aria-busy=\"true\"");
    }
    let icon = match &instance.leading_icon {
        Some(name) => {
            let geometry = icons
                .get(name)
                .ok_or_else(|| format!("unknown icon: {name}"))?;
            format!("<span class=\"cui-button__icon\"><svg data-cui-icon=\"{name}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" focusable=\"false\" aria-hidden=\"true\">{geometry}</svg></span>")
        }
        None => String::new(),
    };
    let label = text(if busy {
        instance.busy_label.as_ref().ok_or("busy label missing")?
    } else {
        &instance.label
    });
    catalog_core::fragment::fill(
        fragment,
        &[
            ("[[open_tag]]", &format!("<{tag} {attrs}>")),
            ("[[close_tag]]", &format!("</{close}>")),
            ("[[icon]]", &icon),
            ("[[label]]", &label),
        ],
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn compose(
    source: &impl PackageSource,
    package: &LoadedPackage,
    config: &ButtonInstances,
    base_css: &[u8],
    app_theme: &[u8],
) -> Result<Bundle, String> {
    let component = package.catalog.get("button").ok_or("button not ready")?;
    if component.target != "native-html" || !component.assets.scripts.is_empty() {
        return Err("button requires supported native-html assets".into());
    }
    let fragment = String::from_utf8(source.asset(package, "components/button/fragment.html")?)
        .map_err(|_| "button fragment is not UTF-8")?;
    let icons: BTreeMap<String, String> =
        serde_json::from_slice(&source.asset(package, "icons.json")?)
            .map_err(|e| format!("icons.json: {e}"))?;
    if icons.len() != 100
        || icons
            .values()
            .any(|geometry| geometry.contains("{{") || geometry.contains("<script"))
    {
        return Err("invalid closed icon catalog".into());
    }
    let mut files = BTreeMap::new();
    let mut instances = Vec::new();
    for instance in &config.instances {
        instance.validate(&package.catalog.package.name)?;
        if !component
            .contract
            .variants
            .contains(&variant(instance.variant).into())
        {
            return Err(format!(
                "unsupported button variant: {}",
                variant(instance.variant)
            ));
        }
        let html = render(instance, &fragment, &icons)?;
        files.insert(
            format!("ui/components/cui-{}.html", instance.id),
            html.into_bytes(),
        );
        instances.push(instance.id.clone());
    }
    let mut css = base_css.to_vec();
    css.extend_from_slice(b"\n/* Clanker Native UI package defaults */\n");
    css.extend_from_slice(&source.asset(package, &package.catalog.package.theme)?);
    css.extend_from_slice(b"\n/* Clanker Native UI button */\n");
    css.extend_from_slice(&source.asset(package, &component.assets.styles)?);
    css.extend_from_slice(b"\n/* app-owned component token overrides */\n");
    css.extend_from_slice(app_theme);
    css.push(b'\n');
    files.insert("ui/app.bundle.css".into(), css);
    let index = format!(
        "# {} / actions\n\n- [button](actions.md): {}\n",
        package.catalog.package.name, component.summary
    );
    files.insert("catalog/index.md".into(), index.into_bytes());
    files.insert(
        "catalog/actions.md".into(),
        format!(
            "# button\n\n{}\n\n- Use when: {}\n- Avoid when: {}\n- Variants: {}\n",
            component.summary,
            component.agent.use_when,
            component.agent.avoid_when,
            component.contract.variants.join(", ")
        )
        .into_bytes(),
    );
    let hashes: BTreeMap<&str, String> = files
        .iter()
        .map(|(path, bytes)| (path.as_str(), digest(bytes)))
        .collect();
    let manifest = serde_json::json!({
        "schemaVersion": 1, "package": package.catalog.package.name,
        "packageDigest": package.digest, "components": [component.id(&package.catalog.package.name)],
        "instances": instances, "files": hashes
    });
    files.insert(
        "build-manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    );
    Ok(Bundle {
        files,
        package_digest: package.digest.clone(),
        components: vec![component.id(&package.catalog.package.name)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(destination: Destination) -> ButtonInstance {
        ButtonInstance {
            id: "save".into(),
            component: "@clanker/vanilla/button".into(),
            label: TextValue::Literal {
                value: "Save <record>".into(),
            },
            destination,
            variant: Variant::Primary,
            size: Size::Compact,
            leading_icon: None,
            edge_aligned: false,
            disabled: BoolValue::Literal(false),
            busy: BoolValue::Literal(false),
            busy_label: None,
        }
    }

    fn fragment() -> &'static str {
        "[[open_tag]][[icon]]<span>[[label]]</span>[[close_tag]]"
    }

    #[test]
    fn submit_escapes_label_and_preserves_native_semantics() {
        let html = render(&button(Destination::Submit), fragment(), &BTreeMap::new()).unwrap();
        assert!(html.contains("type=\"submit\""));
        assert!(html.contains("Save &lt;record&gt;"));
    }

    #[test]
    fn busy_and_disabled_links_never_expose_href() {
        let mut instance = button(Destination::Link {
            href: HrefValue::Literal {
                value: "/links".into(),
            },
        });
        instance.disabled = BoolValue::Literal(true);
        let html = render(&instance, fragment(), &BTreeMap::new()).unwrap();
        assert!(html.contains("role=\"link\" aria-disabled=\"true\""));
        assert!(!html.contains("href="));
        instance.disabled = BoolValue::Literal(false);
        instance.busy = BoolValue::Literal(true);
        instance.busy_label = Some(TextValue::Literal {
            value: "Loading".into(),
        });
        let html = render(&instance, fragment(), &BTreeMap::new()).unwrap();
        assert!(html.contains("Loading") && html.contains("aria-busy=\"true\""));
        assert!(!html.contains("href="));
    }

    #[test]
    fn typed_page_state_selects_busy_and_disabled_markup() {
        let mut instance = button(Destination::Link {
            href: HrefValue::Route {
                name: "home".into(),
            },
        });
        instance.busy = BoolValue::Field {
            path: "button_busy".into(),
        };
        instance.disabled = BoolValue::Field {
            path: "button_disabled".into(),
        };
        instance.busy_label = Some(TextValue::Field {
            path: "busy_label".into(),
        });
        let html = render(&instance, fragment(), &BTreeMap::new()).unwrap();
        assert!(html.contains("{% if button_busy %}{% if button_disabled %}"));
        assert!(html.contains("{% if button_disabled %}"));
        assert!(html.contains("{{ busy_label }}"));
        assert!(html.contains("href=\"{{ routes.home() }}\""));
        assert_eq!(
            html.matches("role=\"link\" aria-disabled=\"true\"").count(),
            3
        );
    }

    #[test]
    fn quiet_edge_alignment_and_icon_are_explicit() {
        let mut instance = button(Destination::Action);
        instance.variant = Variant::Quiet;
        instance.edge_aligned = true;
        instance.leading_icon = Some("check".into());
        let icons = BTreeMap::from([("check".into(), "<path d=\"M1 1\"></path>".into())]);
        let html = render(&instance, fragment(), &icons).unwrap();
        assert!(html.contains("cui-button--quiet cui-button--compact cui-button--edge-aligned"));
        assert!(html.contains("aria-hidden=\"true\""));
    }
}
