//! Pure, deterministic expansion of Native UI declarations from captured package assets.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation;

const MAX_FILE: usize = 1_048_576;
const MAX_BUTTONS: usize = 64;
const MAX_ICONS: usize = 256;
const MAX_FORM_FIELDS: usize = 128;
const MAX_STATIC: usize = 256;

mod assets;
mod bindings;
mod chart;
mod composition;
mod enhancements;
mod enterprise;
mod native_controls;
mod ports;
use bindings::{render_avatar, render_progress};

/// Closed browser entrypoints declared by complete component contracts.
/// Native expansion selects these only for supported declarations. This is
/// resource selection, not import-graph admission or backend integration.
pub fn component_browser_scripts(name: &str) -> &'static [&'static str] {
    match name {
        "copy-field" => &["components/copy-field/interaction.js"],
        "theme-switcher" => &["components/theme-switcher/install.js"],
        "tooltip" => &["components/tooltip/install.js"],
        "toast" => &["components/toast/install.js"],
        "modal" => &["components/modal/interaction.js"],
        "drawer" => &["components/drawer/interaction.js"],
        "popover" => &["components/popover/interaction.js"],
        "command-menu" => &["components/command-menu/interaction.js"],
        "confirm-dialog" => &["components/confirm-dialog/interaction.js"],
        "date-calendar" => &["components/date-calendar/interaction.js"],
        "date-picker" => &["components/date-picker/interaction.js"],
        "file-upload" => &["components/file-upload/interaction.js"],
        "data-viewport" => &["components/data-viewport/interaction.js"],
        "chart" => &["components/chart/interaction.js"],
        _ => &[],
    }
}

pub struct Package {
    identity: String,
    property_catalog: Option<serde_json::Value>,
    theme: Vec<u8>,
    styles: Vec<u8>,
    fragment: String,
    icon_fragment: Option<String>,
    static_fragments: BTreeMap<String, String>,
    icons: BTreeMap<String, String>,
    presentation_contracts: BTreeMap<String, PresentationContract>,
}

#[derive(Default, Clone)]
struct Button {
    attrs: BTreeMap<String, String>,
}

fn parse_opening(input: &str, start: usize, tag: &str) -> Result<(usize, Button, bool)> {
    let bytes = input.as_bytes();
    let mut i = start + tag.len();
    ensure!(
        bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'/' || *b == b'>'),
        "unsupported/malformed cui syntax"
    );
    let mut attrs = BTreeMap::new();
    loop {
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if bytes.get(i) == Some(&b'>') {
            return Ok((i + 1, Button { attrs }, false));
        }
        if bytes.get(i) == Some(&b'/') {
            ensure!(
                bytes.get(i + 1) == Some(&b'>'),
                "Native UI declarations must be self-closing"
            );
            return Ok((i + 2, Button { attrs }, true));
        }
        ensure!(
            bytes.get(i) == Some(&b'"') || bytes.get(i).is_some_and(|b| b.is_ascii_alphabetic()),
            "malformed Native UI attributes"
        );
        ensure!(bytes.get(i) != Some(&b'"'), "malformed attribute name");
        let name_start = i;
        while bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
        {
            i += 1;
        }
        ensure!(i > name_start, "malformed attribute name");
        let name = &input[name_start..i];
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        ensure!(
            bytes.get(i) == Some(&b'='),
            "boolean/unsupported attribute syntax is not allowed"
        );
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let quote = *bytes.get(i).context("unterminated Native UI declaration")?;
        ensure!(
            quote == b'\'' || quote == b'"',
            "attribute values must be quoted"
        );
        i += 1;
        let value_start = i;
        while bytes.get(i).is_some_and(|b| *b != quote) {
            i += 1;
        }
        ensure!(
            bytes.get(i) == Some(&quote),
            "unterminated Native UI attribute"
        );
        let value = &input[value_start..i];
        i += 1;
        ensure!(
            attrs.insert(name.to_owned(), value.to_owned()).is_none(),
            "duplicate Native UI attribute: {name}"
        );
    }
}

fn dynamic(value: &str) -> bool {
    value.starts_with("{{") && value.ends_with("}}") && value.len() > 4
}
fn field_path(value: &str) -> Option<&str> {
    let path = value.trim();
    (!path.is_empty()
        && path.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }))
    .then_some(path)
}
fn interpolation(value: &str) -> Option<&str> {
    dynamic(value)
        .then(|| field_path(&value[2..value.len() - 2]))
        .flatten()
}

/// Closed set of declaration bindings with a renderer-enforced value type.
fn binding_kind(component: &str, attribute: &str) -> Option<&'static str> {
    match (component, attribute) {
        ("button", "label" | "busy-label") => Some("string"),
        ("button", "variant" | "size") => Some("integer"),
        (_, "busy" | "disabled" | "hidden" | "checked" | "required" | "current") => Some("boolean"),
        ("progress", "value" | "maximum") => Some("number"),
        ("pagination", "current-page" | "total-pages" | "page-size") => Some("integer"),
        ("data-table", "rows") | ("select-field", "options") => Some("list"),
        ("avatar", "initials") => Some("string"),
        // All remaining accepted whole-field attributes pass through the
        // renderer's checked text/token/URL helpers. Rendering still rejects
        // unknown attributes and unsupported interpolation before returning.
        _ => Some("string"),
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
fn bool_state(value: Option<&String>, key: &str) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    ensure!(
        value == "true" || value == "false" || interpolation(value).is_some(),
        "{key} must be true, false, or {{ page.field }}"
    );
    Ok(Some(value.clone()))
}
fn render(button: &Button, package: &Package) -> Result<String> {
    let allowed: BTreeSet<&str> = [
        "kind",
        "label",
        "busy-label",
        "variant",
        "size",
        "icon",
        "edge-aligned",
        "disabled",
        "busy",
        "href",
        "route",
        "id",
    ]
    .into_iter()
    .collect();
    for key in button.attrs.keys() {
        ensure!(
            allowed.contains(key.as_str()),
            "unknown cui-button attribute: {key}"
        );
    }
    let attrs = &button.attrs;
    // Closed ordinal bindings keep a typed configurator bounded without exposing
    // arbitrary classes, icon geometry, expressions, or HTML strings.
    for (property, fallback) in [("size", "standard"), ("variant", "secondary")] {
        if let Some(path) = attrs.get(property).and_then(|value| interpolation(value)) {
            let mut literal = button.clone();
            literal
                .attrs
                .insert(property.to_owned(), fallback.to_owned());
            let rendered = render(&literal, package)?;
            return Ok(if property == "variant" {
                rendered.replace(
                    "cui-button--secondary",
                    &format!("{{% if ui_integer({path}, 0, 3) == \"0\" %}}cui-button--primary{{% elif ui_integer({path}, 0, 3) == \"1\" %}}cui-button--secondary{{% elif ui_integer({path}, 0, 3) == \"2\" %}}cui-button--quiet{{% else %}}cui-button--danger{{% endif %}}"),
                )
            } else {
                rendered.replace(
                    "class=\"cui-button ",
                    &format!("class=\"cui-button {{% if ui_integer({path}, 0, 1) == \"0\" %}}cui-button--compact {{% endif %}}"),
                )
            });
        }
    }
    for state in ["busy", "disabled"] {
        if let Some(value) = attrs.get(state).and_then(|v| interpolation(v)) {
            let mut yes = button.clone();
            let mut no = button.clone();
            yes.attrs.insert(state.to_owned(), "true".to_owned());
            no.attrs.insert(state.to_owned(), "false".to_owned());
            return Ok(format!(
                "{{% if {value} %}}{}{{% else %}}{}{{% endif %}}",
                render(&yes, package)?,
                render(&no, package)?
            ));
        }
    }
    let kind = attrs.get("kind").map(String::as_str).unwrap_or("action");
    ensure!(
        ["action", "submit", "link"].contains(&kind),
        "invalid button kind"
    );
    let label = attrs.get("label").context("cui-button requires label")?;
    let label_value = if dynamic(label) {
        ensure!(
            interpolation(label).is_some(),
            "label interpolation must be a full {{ page.field }}"
        );
        format!(
            "{{{{ ui_text({}, \"nonblank\", 1, 0) }}}}",
            interpolation(label).context("invalid page field interpolation")?
        )
    } else {
        ensure!(
            !label.contains("{{") && !label.contains("}}"),
            "mixed label interpolation is unsupported"
        );
        escape(label)
    };
    let busy_label = attrs
        .get("busy-label")
        .map(|v| {
            if dynamic(v) {
                ensure!(
                    interpolation(v).is_some(),
                    "busy-label interpolation must be a full page field"
                );
                Ok(format!(
                    "{{{{ ui_text({}, \"nonblank\", 1, 0) }}}}",
                    interpolation(v).context("invalid busy-label interpolation")?
                ))
            } else {
                ensure!(
                    !v.contains("{{") && !v.contains("}}"),
                    "mixed busy-label interpolation is unsupported"
                );
                Ok(escape(v))
            }
        })
        .transpose()?;
    let busy = bool_state(attrs.get("busy"), "busy")?;
    let busy_can_be_true = busy.as_deref() == Some("true")
        || busy.as_ref().is_some_and(|v| interpolation(v).is_some());
    ensure!(
        !busy_can_be_true || busy_label.is_some(),
        "busy=true or dynamic busy requires busy-label"
    );
    let disabled = bool_state(attrs.get("disabled"), "disabled")?;
    let edge = attrs
        .get("edge-aligned")
        .map(String::as_str)
        .unwrap_or("false");
    ensure!(
        edge == "true" || edge == "false",
        "edge-aligned must be true or false"
    );
    let variant = attrs
        .get("variant")
        .map(String::as_str)
        .unwrap_or("secondary");
    ensure!(
        ["primary", "secondary", "danger", "quiet"].contains(&variant),
        "invalid button variant"
    );
    let size = attrs.get("size").map(String::as_str).unwrap_or("standard");
    ensure!(
        ["compact", "standard"].contains(&size),
        "invalid button size"
    );
    let icon = attrs.get("icon");
    if let Some(icon) = icon {
        ensure!(
            package.icons.contains_key(icon),
            "unknown package icon: {icon}"
        );
    }
    ensure!(
        !(attrs.contains_key("href") && attrs.contains_key("route")),
        "route and href are mutually exclusive"
    );
    if kind == "link" {
        ensure!(
            attrs.contains_key("href") || attrs.contains_key("route"),
            "link button requires href or route"
        );
    }
    if let Some(route) = attrs.get("route") {
        ensure!(
            route.split('.').count() == 1 && field_path(route).is_some(),
            "invalid route name"
        );
    }
    let mut classes = format!("cui-button cui-button--{variant}");
    if size == "compact" {
        classes.push_str(" cui-button--compact");
    }
    if edge == "true" {
        classes.push_str(" cui-button--edge-aligned");
    }
    let noninteractive_link =
        kind == "link" && (disabled.as_deref() == Some("true") || busy.as_deref() == Some("true"));
    let mut open = if kind == "link" && noninteractive_link {
        "<span".to_owned()
    } else if kind == "link" {
        "<a".to_owned()
    } else {
        "<button".to_owned()
    };
    open.push_str(&format!(" class=\"{}\"", escape(&classes)));
    if let Some(id) = attrs.get("id") {
        ensure!(
            !id.is_empty()
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_:.".contains(&b)),
            "invalid app-owned button id"
        );
        open.push_str(&format!(" id=\"{}\"", escape(id)));
    }
    if kind == "link" {
        if disabled != Some("true".into())
            && busy != Some("true".into())
            && disabled.as_ref().is_none_or(|v| interpolation(v).is_none())
            && busy.as_ref().is_none_or(|v| interpolation(v).is_none())
        {
            if let Some(route) = attrs.get("route") {
                ensure!(
                    !attrs.contains_key("href")
                        && route.split('.').count() == 1
                        && field_path(route).is_some(),
                    "invalid route name or route/href combination"
                );
                open.push_str(&format!(" href=\"{{{{ routes.{route}() }}}}\""));
            } else {
                let href = attrs
                    .get("href")
                    .context("link button requires href or route")?;
                ensure!(safe_href(href), "unsafe link href");
                open.push_str(&format!(" href=\"{}\"", escape(href)));
            }
        }
        if noninteractive_link {
            open.push_str(" role=\"link\" aria-disabled=\"true\"");
        }
    } else {
        ensure!(
            !attrs.contains_key("href") && !attrs.contains_key("route"),
            "href and route are only valid for link buttons"
        );
        open.push_str(&format!(
            " type=\"{}\"",
            if kind == "submit" { "submit" } else { "button" }
        ));
        if disabled.as_deref() == Some("true") || busy.as_deref() == Some("true") {
            open.push_str(" disabled");
        }
    }
    if let Some(value) = busy.as_ref() {
        open.push_str(&format!(" aria-busy=\"{}\"", value));
    }
    open.push('>');
    let icon_html = icon.map(|name| format!("<span class=\"cui-button__icon\" aria-hidden=\"true\"><svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.75\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg></span>", package.icons[name])).unwrap_or_default();
    let shown_label = if busy.as_deref() == Some("true") {
        busy_label.clone().unwrap_or_default()
    } else {
        label_value
    };
    let close = if noninteractive_link {
        "</span>"
    } else if kind == "link" {
        "</a>"
    } else {
        "</button>"
    };
    fill_slots(
        &package.fragment,
        &[
            ("[[open_tag]]", &open),
            ("[[icon]]", &icon_html),
            ("[[label]]", &shown_label),
            ("[[close_tag]]", close),
        ],
    )
}
fn render_icon(icon: &Button, package: &Package) -> Result<String> {
    let fragment = package
        .icon_fragment
        .as_ref()
        .context("cui-icon requires a ready locked icon component")?;
    for key in icon.attrs.keys() {
        ensure!(
            ["name", "size", "label"].contains(&key.as_str()),
            "unknown cui-icon attribute: {key}"
        );
    }
    let name = icon.attrs.get("name").context("cui-icon requires name")?;
    let geometry = package
        .icons
        .get(name)
        .with_context(|| format!("unknown package icon: {name}"))?;
    let size = icon
        .attrs
        .get("size")
        .map(String::as_str)
        .unwrap_or("medium");
    ensure!(
        ["small", "medium", "large"].contains(&size),
        "invalid icon size"
    );
    let mut attributes = format!(
        "class=\"cui-icon cui-icon--{size}\" data-cui-component=\"icon\" data-cui-icon=\"{name}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" focusable=\"false\""
    );
    if let Some(label) = icon.attrs.get("label") {
        let label = checked_template_text(label, "icon label")?;
        attributes.push_str(&format!(" role=\"img\" aria-label=\"{label}\""));
    } else {
        attributes.push_str(" aria-hidden=\"true\"");
    }
    fill_slots(
        fragment,
        &[("[[attributes]]", &attributes), ("[[geometry]]", geometry)],
    )
}

fn static_fragment<'a>(package: &'a Package, name: &str) -> Result<&'a str> {
    package
        .static_fragments
        .get(name)
        .map(String::as_str)
        .with_context(|| format!("cui-{name} requires a ready locked component"))
}

fn checked_attributes(declaration: &Button, allowed: &[&str], component: &str) -> Result<()> {
    for key in declaration.attrs.keys() {
        ensure!(
            allowed.contains(&key.as_str()),
            "unknown cui-{component} attribute: {key}"
        );
    }
    Ok(())
}

fn fill_slots(fragment: &str, replacements: &[(&str, &str)]) -> Result<String> {
    let mut output = String::with_capacity(fragment.len());
    let mut cursor = 0;
    while let Some(relative) = fragment[cursor..].find("[[") {
        let start = cursor + relative;
        output.push_str(&fragment[cursor..start]);
        let end = fragment[start..]
            .find("]]")
            .map(|offset| start + offset + 2)
            .context("component fragment contains an unterminated slot")?;
        let slot = &fragment[start..end];
        let value = replacements
            .iter()
            .find_map(|(candidate, value)| (*candidate == slot).then_some(*value))
            .with_context(|| format!("component fragment contains unsupported slot {slot}"))?;
        output.push_str(value);
        cursor = end;
    }
    output.push_str(&fragment[cursor..]);
    Ok(output)
}

fn fill_static(fragment: &str, replacements: &[(&str, &str)]) -> Result<String> {
    fill_slots(fragment, replacements)
}

fn render_badge(badge: &Button, package: &Package) -> Result<String> {
    checked_attributes(badge, &["label", "tone", "icon", "show-icon"], "badge")?;
    let label = checked_template_text(
        badge
            .attrs
            .get("label")
            .context("cui-badge requires label")?,
        "badge label",
    )?;
    let tone = badge
        .attrs
        .get("tone")
        .map(String::as_str)
        .unwrap_or("neutral");
    ensure!(
        ["neutral", "info", "success", "warning", "danger", "running"].contains(&tone),
        "invalid badge tone"
    );
    let show_icon = badge
        .attrs
        .get("show-icon")
        .map(String::as_str)
        .unwrap_or("true");
    ensure!(
        show_icon == "true" || show_icon == "false",
        "show-icon must be true or false"
    );
    if let Some(name) = badge.attrs.get("icon") {
        ensure!(
            package.icons.contains_key(name),
            "unknown package icon: {name}"
        );
    }
    let icon_name = if show_icon == "true" {
        badge
            .attrs
            .get("icon")
            .map(String::as_str)
            .or_else(|| match tone {
                "neutral" => None,
                "info" => Some("info"),
                "success" => Some("check"),
                "warning" | "danger" => Some("alert"),
                "running" => Some("restore"),
                _ => unreachable!(),
            })
    } else {
        None
    };
    let icon = match icon_name {
        Some(name) => {
            let geometry = package
                .icons
                .get(name)
                .context("badge default icon is missing")?;
            format!(
                "<span class=\"cui-badge__icon\"><svg class=\"cui-icon cui-icon--small\" data-cui-component=\"icon\" data-cui-icon=\"{name}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" focusable=\"false\" aria-hidden=\"true\">{geometry}</svg></span>"
            )
        }
        None => String::new(),
    };
    let attributes = format!(
        "class=\"cui-badge cui-badge--{tone}\" data-cui-component=\"badge\" data-cui-tone=\"{tone}\""
    );
    fill_static(
        static_fragment(package, "badge")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[icon]]", &icon),
            ("[[label]]", &label),
        ],
    )
}

fn render_divider(divider: &Button, package: &Package) -> Result<String> {
    checked_attributes(divider, &["orientation", "alignment", "label"], "divider")?;
    let orientation = divider
        .attrs
        .get("orientation")
        .map(String::as_str)
        .unwrap_or("horizontal");
    ensure!(
        ["horizontal", "vertical"].contains(&orientation),
        "invalid divider orientation"
    );
    let alignment = divider
        .attrs
        .get("alignment")
        .map(String::as_str)
        .unwrap_or("center");
    ensure!(
        ["start", "center", "end"].contains(&alignment),
        "invalid divider alignment"
    );
    let label = divider
        .attrs
        .get("label")
        .map(|value| checked_template_text(value, "divider label"))
        .transpose()?;
    ensure!(
        orientation != "vertical" || label.is_none(),
        "vertical divider cannot have a label"
    );
    let mut classes = format!("cui-divider cui-divider--{orientation}");
    let label_html = if let Some(label) = label.as_deref() {
        classes.push_str(&format!(" cui-divider--labelled cui-divider--{alignment}"));
        format!("<span class=\"cui-divider__label\">{label}</span>")
    } else {
        String::new()
    };
    let attributes = format!(
        "class=\"{classes}\" data-cui-component=\"divider\" role=\"separator\" aria-orientation=\"{orientation}\"{}",
        label
            .as_deref()
            .map(|value| format!(" aria-label=\"{value}\""))
            .unwrap_or_default()
    );
    fill_static(
        static_fragment(package, "divider")?,
        &[("[[attributes]]", &attributes), ("[[label]]", &label_html)],
    )
}

fn render_status_indicator(status: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        status,
        &["label", "tone", "detail", "size", "pulse"],
        "status-indicator",
    )?;
    let label = checked_template_text(
        status
            .attrs
            .get("label")
            .context("cui-status-indicator requires label")?,
        "status label",
    )?;
    let tone = status
        .attrs
        .get("tone")
        .map(String::as_str)
        .unwrap_or("neutral");
    ensure!(
        ["neutral", "info", "success", "warning", "danger"].contains(&tone),
        "invalid status tone"
    );
    let size = status
        .attrs
        .get("size")
        .map(String::as_str)
        .unwrap_or("small");
    ensure!(["small", "large"].contains(&size), "invalid status size");
    let pulse = status
        .attrs
        .get("pulse")
        .map(String::as_str)
        .unwrap_or("false");
    ensure!(
        pulse == "true" || pulse == "false",
        "pulse must be true or false"
    );
    let detail = status
        .attrs
        .get("detail")
        .map(|value| checked_template_text(value, "status detail"))
        .transpose()?;
    let attributes = format!(
        "class=\"cui-status-indicator cui-status-indicator--{tone} cui-status-indicator--{size}{}\" data-cui-component=\"status-indicator\"",
        if pulse == "true" {
            " cui-status-indicator--pulse"
        } else {
            ""
        }
    );
    let detail_html = detail
        .map(|value| format!("<span class=\"cui-status-indicator__detail\">{value}</span>"))
        .unwrap_or_default();
    fill_static(
        static_fragment(package, "status-indicator")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[detail]]", &detail_html),
        ],
    )
}

/// A form-control id: a literal token, or literal token text around exactly one
/// `{{ page.field }}` so controls rendered inside a loop get unique ids (and their
/// labels and descriptions stay attached). The literal prefix is required so a
/// generated id cannot collide with unrelated ids. Native escapes the rendered value.
fn control_id(value: &str, component: &str) -> Result<String> {
    let Some(open) = value.find("{{") else {
        ensure!(form_field_token(value), "invalid {component} id");
        return Ok(value.to_owned());
    };
    let close = value[open..]
        .find("}}")
        .map(|offset| open + offset)
        .with_context(|| format!("unclosed interpolation in {component} id"))?;
    let (prefix, inner, suffix) = (&value[..open], &value[open + 2..close], &value[close + 2..]);
    let path = field_path(inner)
        .with_context(|| format!("{component} id interpolation must be {{{{ page.field }}}}"))?;
    let literal = |text: &str| {
        text.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
    };
    ensure!(
        !prefix.is_empty() && literal(prefix) && literal(suffix) && value.len() <= 128,
        "invalid {component} id: use a literal prefix and one {{{{ page.field }}}}, e.g. buyer-name-{{{{ customer.id }}}}"
    );
    Ok(format!("{prefix}{{{{ {path} }}}}{suffix}"))
}

fn form_field_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn render_form_field(field: &Button, package: &Package) -> Result<String> {
    let allowed = [
        "id",
        "name",
        "label",
        "kind",
        "input-type",
        "rows",
        "value",
        "placeholder",
        "hint",
        "error",
        "autocomplete",
        "maxlength",
        "required",
        "readonly",
        "disabled",
    ];
    checked_attributes(field, &allowed, "form-field")?;
    let attrs = &field.attrs;
    let id = &control_id(
        attrs.get("id").context("cui-form-field requires id")?,
        "form-field",
    )?;
    let name = attrs.get("name").context("cui-form-field requires name")?;
    ensure!(
        form_field_token(name) && !name.starts_with('_'),
        "invalid/reserved form-field name"
    );
    let label = attrs
        .get("label")
        .context("cui-form-field requires label")?;
    let label_html_text = checked_template_text(label, "form-field label")?;
    let kind = attrs.get("kind").map(String::as_str).unwrap_or("input");
    ensure!(
        ["input", "textarea"].contains(&kind),
        "invalid form-field kind"
    );
    let input_type = attrs.get("input-type").map(String::as_str);
    if let Some(input_type) = input_type {
        ensure!(
            ["text", "email", "url", "number", "password", "search", "date", "tel"]
                .contains(&input_type),
            "invalid form-field input-type"
        );
    }
    let rows = attrs
        .get("rows")
        .map(|value| {
            ensure!(
                value.bytes().all(|byte| byte.is_ascii_digit()),
                "form-field rows must be an integer literal"
            );
            value
                .parse::<u8>()
                .context("form-field rows must be an integer literal")
        })
        .transpose()?;
    if kind == "input" {
        ensure!(rows.is_none(), "rows are only valid for textarea fields");
    } else {
        ensure!(
            input_type.is_none(),
            "input-type is only valid for input fields"
        );
        ensure!(
            rows.is_none_or(|value| (2..=40).contains(&value)),
            "textarea rows must be between 2 and 40"
        );
    }
    let required = attrs.get("required").map(String::as_str).unwrap_or("false");
    let readonly = attrs.get("readonly").map(String::as_str).unwrap_or("false");
    let disabled = attrs.get("disabled").map(String::as_str).unwrap_or("false");
    ensure!(
        ["true", "false"].contains(&required),
        "form-field required must be true or false"
    );
    ensure!(
        ["true", "false"].contains(&readonly),
        "form-field readonly must be true or false"
    );
    ensure!(
        ["true", "false"].contains(&disabled),
        "form-field disabled must be true or false"
    );
    ensure!(
        !(required == "true" && readonly == "true"),
        "readonly fields cannot be required"
    );
    let mut descriptions = Vec::new();
    for (attribute, suffix) in [("hint", "hint"), ("error", "error")] {
        if let Some(value) = attrs.get(attribute) {
            let html = field_text(value, attribute, true, false)?;
            descriptions.push((suffix, html));
        }
    }
    let autocomplete = attrs.get("autocomplete");
    if let Some(value) = autocomplete {
        ensure!(
            form_field_token(value),
            "invalid form-field autocomplete token"
        );
    }
    let placeholder = attrs
        .get("placeholder")
        .map(|value| field_text(value, "placeholder", false, true))
        .transpose()?;
    let value = attrs.get("value").map(String::as_str).unwrap_or("");
    let value_html = if let Some(path) = interpolation(value) {
        format!("{{{{ {path} }}}}")
    } else {
        ensure!(
            !value.contains("{{") && !value.contains("}}"),
            "form-field value interpolation must be a full {{ page.field }}"
        );
        ensure!(
            !value.chars().any(|character| {
                character.is_control()
                    && !(kind == "textarea" && matches!(character, '\n' | '\r' | '\t'))
            }),
            "form-field value contains unsupported control characters"
        );
        escape(value)
    };
    let max_length = attrs
        .get("maxlength")
        .map(|value| {
            ensure!(
                value.bytes().all(|byte| byte.is_ascii_digit()),
                "form-field maxlength must be a positive bounded integer literal"
            );
            let length = value
                .parse::<u32>()
                .context("form-field maxlength must be a positive bounded integer literal")?;
            ensure!(
                (1..=65_535).contains(&length),
                "form-field maxlength must be between 1 and 65535"
            );
            Ok::<u32, anyhow::Error>(length)
        })
        .transpose()?;
    ensure!(
        max_length.is_none()
            || kind == "textarea"
            || !matches!(input_type.unwrap_or("text"), "number" | "date"),
        "form-field maxlength is only valid for textual input types"
    );
    let invalid = attrs.contains_key("error");
    let attributes = format!(
        "class=\"cui-form-field{}\" data-cui-component=\"form-field\" data-cui-kind=\"{kind}\"",
        if invalid {
            " cui-form-field--invalid"
        } else {
            ""
        }
    );
    let required_mark = if required == "true" {
        " <span class=\"cui-form-field__required\" aria-hidden=\"true\">*</span>"
    } else {
        ""
    };
    let label_html = format!(
        "<label class=\"cui-form-field__label\" for=\"{}\">{}{}</label>",
        escape(id),
        label_html_text,
        required_mark
    );
    let has_description = !descriptions.is_empty();
    let described_by = if has_description {
        format!(
            " aria-describedby=\"{}\"",
            escape(
                &descriptions
                    .iter()
                    .map(|(suffix, _)| format!("{id}-{suffix}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        )
    } else {
        String::new()
    };
    let invalid_attr = if invalid {
        " aria-invalid=\"true\""
    } else {
        ""
    };
    let required_attr = if required == "true" { " required" } else { "" };
    let readonly_attr = if readonly == "true" { " readonly" } else { "" };
    let disabled_attr = if disabled == "true" { " disabled" } else { "" };
    let placeholder_attr = placeholder
        .map(|text| format!(" placeholder=\"{text}\""))
        .unwrap_or_default();
    let control = if kind == "input" {
        format!(
            "<input class=\"cui-form-field__control\" id=\"{}\" name=\"{}\" type=\"{}\" value=\"{}\"{}{}{}{}{}{}{}{}>",
            escape(id),
            escape(name),
            input_type.unwrap_or("text"),
            value_html,
            placeholder_attr,
            autocomplete
                .map(|text| format!(" autocomplete=\"{}\"", escape(text)))
                .unwrap_or_default(),
            max_length
                .map(|length| format!(" maxlength=\"{length}\""))
                .unwrap_or_default(),
            described_by,
            invalid_attr,
            required_attr,
            readonly_attr,
            disabled_attr
        )
    } else {
        format!(
            "<textarea class=\"cui-form-field__control cui-form-field__control--textarea\" id=\"{}\" name=\"{}\" rows=\"{}\"{}{}{}{}{}{}{}{}>{}</textarea>",
            escape(id),
            escape(name),
            rows.unwrap_or(3),
            placeholder_attr,
            autocomplete
                .map(|text| format!(" autocomplete=\"{}\"", escape(text)))
                .unwrap_or_default(),
            max_length
                .map(|length| format!(" maxlength=\"{length}\""))
                .unwrap_or_default(),
            described_by,
            invalid_attr,
            required_attr,
            readonly_attr,
            disabled_attr,
            value_html
        )
    };
    let description = if has_description {
        let mut html = "<div class=\"cui-form-field__description\">".to_owned();
        for (suffix, text) in descriptions {
            html.push_str(&format!(
                "<p class=\"cui-form-field__{suffix}\" id=\"{}-{suffix}\">{}</p>",
                escape(id),
                text
            ));
        }
        html.push_str("</div>");
        html
    } else {
        String::new()
    };
    fill_slots(
        static_fragment(package, "form-field")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label_html),
            ("[[control]]", &control),
            ("[[description]]", &description),
        ],
    )
}

fn field_text(value: &str, field: &str, multiline: bool, allow_blank: bool) -> Result<String> {
    if let Some(path) = interpolation(value) {
        let policy = if multiline {
            "multiline"
        } else if allow_blank {
            "plain"
        } else {
            "nonblank"
        };
        let minimum = if allow_blank { 0 } else { 1 };
        return Ok(format!(
            "{{{{ ui_text({path}, \"{policy}\", {minimum}, 0) }}}}"
        ));
    }
    ensure!(
        !value.contains("{{") && !value.contains("}}"),
        "{field} requires a whole field binding"
    );
    ensure!(
        allow_blank || !value.trim().is_empty(),
        "{field} must be nonblank when supplied"
    );
    ensure!(
        !value
            .chars()
            .any(|ch| ch.is_control() && !(multiline && matches!(ch, '\n' | '\r' | '\t'))),
        "{field} contains unsupported controls"
    );
    Ok(escape(value))
}

fn checked_template_text(value: &str, field: &str) -> Result<String> {
    if let Some(path) = interpolation(value) {
        // The build checks a complete field path; only the Native runtime can
        // know whether the bound page value is nonblank.
        return Ok(format!("{{{{ ui_text({path}, \"nonblank\", 1, 0) }}}}"));
    }
    ensure!(
        !value.contains("{{") && !value.contains("}}"),
        "{field} interpolation must be a complete checked page field"
    );
    ensure!(
        !value.trim().is_empty() && !value.chars().any(char::is_control),
        "{field} must be nonblank safe text"
    );
    Ok(escape(value))
}

fn route_destination(route: &str, field: &str) -> Result<String> {
    ensure!(
        route.split('.').count() == 1 && field_path(route).is_some(),
        "invalid {field} route name"
    );
    Ok(format!("{{{{ routes.{route}() }}}}"))
}

fn declaration_href(
    attrs: &BTreeMap<String, String>,
    href_key: &str,
    route_key: &str,
) -> Result<Option<String>> {
    ensure!(
        !(attrs.contains_key(href_key) && attrs.contains_key(route_key)),
        "{href_key} and {route_key} are mutually exclusive"
    );
    if let Some(route) = attrs.get(route_key) {
        return Ok(Some(route_destination(route, route_key)?));
    }
    attrs
        .get(href_key)
        .map(|href| {
            ensure!(safe_href(href), "unsafe {href_key}");
            Ok(escape(href))
        })
        .transpose()
}

fn render_package_icon(name: &str, size: &str, package: &Package) -> Result<String> {
    ensure!(
        package.icons.contains_key(name),
        "unknown package icon: {name}"
    );
    let icon = Button {
        attrs: BTreeMap::from([("name".into(), name.into()), ("size".into(), size.into())]),
    };
    render_icon(&icon, package)
}

fn render_tag(tag: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        tag,
        &[
            "label",
            "tone",
            "size",
            "href",
            "route",
            "icon",
            "count",
            "count-label",
            "remove-href",
            "remove-route",
            "remove-label",
        ],
        "tag",
    )?;
    let attrs = &tag.attrs;
    let label = attrs.get("label").context("cui-tag requires label")?;
    let label_html = checked_template_text(label, "tag label")?;
    let tone = attrs.get("tone").map(String::as_str).unwrap_or("neutral");
    ensure!(
        ["neutral", "brand", "info", "success", "warning", "danger"].contains(&tone),
        "invalid tag tone"
    );
    let size = attrs.get("size").map(String::as_str).unwrap_or("medium");
    ensure!(
        ["small", "medium", "large"].contains(&size),
        "invalid tag size"
    );
    let href = declaration_href(attrs, "href", "route")?;
    let remove_href = declaration_href(attrs, "remove-href", "remove-route")?;
    ensure!(
        href.is_none() || remove_href.is_none(),
        "a linked tag cannot contain a second removal link"
    );
    let count = attrs.get("count");
    let count_label = attrs.get("count-label");
    ensure!(
        count.is_some() == count_label.is_some(),
        "tag count and count-label must be supplied together"
    );
    for (key, value) in [
        ("count", count),
        ("count-label", count_label),
        ("remove-label", attrs.get("remove-label")),
    ] {
        if let Some(value) = value {
            checked_template_text(value, &format!("tag {key}"))?;
        }
    }
    ensure!(
        remove_href.is_some() == attrs.contains_key("remove-label"),
        "tag remove destination and remove-label must be supplied together"
    );
    let icon = attrs
        .get("icon")
        .map(|name| {
            let icon = render_package_icon(name, "small", package)?;
            Ok::<String, anyhow::Error>(format!("<span class=\"cui-tag__icon\">{icon}</span>"))
        })
        .transpose()?
        .unwrap_or_default();
    let count_html = match (count, count_label) {
        (Some(count), Some(count_label)) => format!(
            "<span class=\"cui-tag__count\" aria-hidden=\"true\">{}</span><span class=\"cui-tag__count-label\">{}</span>",
            checked_template_text(count, "tag count")?,
            checked_template_text(count_label, "tag count-label")?
        ),
        _ => String::new(),
    };
    let mut classes = format!("cui-tag cui-tag--{tone} cui-tag--{size}");
    let content = format!("{icon}<span class=\"cui-tag__label\">{label_html}</span>{count_html}");
    let tag_html = if let Some(href) = href {
        classes.push_str(" cui-tag--linked");
        format!(
            "<a class=\"{classes}\" href=\"{href}\" data-cui-component=\"tag\" data-cui-tone=\"{tone}\">{content}</a>"
        )
    } else if let (Some(href), Some(remove_label)) = (remove_href, attrs.get("remove-label")) {
        let icon = render_package_icon("close", "small", package)?;
        format!(
            "<span class=\"{classes}\" data-cui-component=\"tag\" data-cui-tone=\"{tone}\">{content}<a class=\"cui-tag__remove\" href=\"{href}\" aria-label=\"{}\">{icon}</a></span>",
            checked_template_text(remove_label, "tag remove-label")?
        )
    } else {
        format!(
            "<span class=\"{classes}\" data-cui-component=\"tag\" data-cui-tone=\"{tone}\">{content}</span>"
        )
    };
    fill_slots(static_fragment(package, "tag")?, &[("[[tag]]", &tag_html)])
}

fn render_alert(alert: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        alert,
        &[
            "title",
            "body",
            "tone",
            "recovery-label",
            "recovery-href",
            "recovery-route",
            "announcement",
            "appearance",
            "heading-level",
        ],
        "alert",
    )?;
    let attrs = &alert.attrs;
    let title = checked_template_text(
        attrs.get("title").context("cui-alert requires title")?,
        "alert title",
    )?;
    let body = checked_template_text(
        attrs.get("body").context("cui-alert requires body")?,
        "alert body",
    )?;
    let tone = attrs.get("tone").context("cui-alert requires tone")?;
    ensure!(
        ["info", "success", "warning", "danger"].contains(&tone.as_str()),
        "invalid alert tone"
    );
    let announcement = attrs.get("announcement").map(String::as_str);
    let role = match announcement {
        None => "",
        Some("polite") => " role=\"status\" aria-live=\"polite\"",
        Some("assertive") => " role=\"alert\" aria-live=\"assertive\"",
        Some(_) => anyhow::bail!("invalid alert announcement"),
    };
    let appearance = attrs
        .get("appearance")
        .map(String::as_str)
        .unwrap_or("soft");
    ensure!(
        ["soft", "outlined", "accent"].contains(&appearance),
        "invalid alert appearance"
    );
    let heading = attrs
        .get("heading-level")
        .map(String::as_str)
        .unwrap_or("h2");
    ensure!(
        ["h2", "h3", "h4"].contains(&heading),
        "invalid alert heading-level"
    );
    let heading_html = format!("<{heading} class=\"cui-alert__title\">{title}</{heading}>");
    let attributes = format!(
        "class=\"cui-alert cui-alert--{tone} cui-alert--{appearance}\" data-cui-component=\"alert\" data-cui-tone=\"{tone}\"{role}"
    );
    let icon_name = match tone.as_str() {
        "info" => "info",
        "success" => "check",
        "warning" | "danger" => "alert",
        _ => unreachable!(),
    };
    let icon = format!(
        "<span class=\"cui-alert__icon\">{}</span>",
        render_package_icon(icon_name, "medium", package)?
    );
    let recovery_label = attrs.get("recovery-label");
    let recovery_href = declaration_href(attrs, "recovery-href", "recovery-route")?;
    ensure!(
        recovery_label.is_some() == recovery_href.is_some(),
        "alert recovery-label and recovery destination must be supplied together"
    );
    let recovery = match (recovery_label, recovery_href) {
        (Some(label), Some(href)) => {
            let label = checked_template_text(label, "alert recovery-label")?;
            format!("<a class=\"cui-alert__recovery\" href=\"{href}\">{label}</a>")
        }
        _ => String::new(),
    };
    fill_slots(
        static_fragment(package, "alert")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[icon]]", &icon),
            ("[[heading]]", &heading_html),
            ("[[body]]", &body),
            ("[[recovery]]", &recovery),
        ],
    )
}

fn render_progress_literal(progress: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        progress,
        &[
            "label", "state", "value", "maximum", "suffix", "detail", "tone", "size",
        ],
        "progress",
    )?;
    let attrs = &progress.attrs;
    let label = checked_template_text(
        attrs.get("label").context("cui-progress requires label")?,
        "progress label",
    )?;
    let state = attrs
        .get("state")
        .context("cui-progress requires state")?
        .as_str();
    ensure!(
        ["determinate", "indeterminate"].contains(&state),
        "invalid progress state"
    );
    let parse_number = |key: &str| -> Result<Option<f64>> {
        let Some(text) = attrs.get(key) else {
            return Ok(None);
        };
        ensure!(
            interpolation(text).is_none() && !text.contains("{{") && !text.contains("}}"),
            "progress {key} must be a numeric literal; interpolation is unsupported"
        );
        let value = text
            .parse::<f64>()
            .with_context(|| format!("progress {key} must be a numeric literal"))?;
        ensure!(value.is_finite(), "progress {key} must be finite");
        Ok(Some(value))
    };
    let value = parse_number("value")?;
    let maximum = parse_number("maximum")?;
    if state == "determinate" {
        let (Some(value), Some(maximum)) = (value, maximum) else {
            anyhow::bail!("determinate progress requires value and maximum")
        };
        ensure!(
            maximum > 0.0 && value >= 0.0 && value <= maximum,
            "progress requires maximum > 0 and 0 <= value <= maximum"
        );
    } else {
        ensure!(
            value.is_none() && maximum.is_none(),
            "indeterminate progress cannot include value or maximum"
        );
        ensure!(
            !attrs.contains_key("suffix"),
            "indeterminate progress cannot include suffix"
        );
    }
    let suffix = attrs.get("suffix");
    let suffix = suffix
        .map(|text| checked_template_text(text, "progress suffix"))
        .transpose()?;
    let detail = attrs
        .get("detail")
        .map(|value| checked_template_text(value, "progress detail"))
        .transpose()?;
    let tone = attrs.get("tone").map(String::as_str).unwrap_or("info");
    ensure!(
        ["info", "success", "warning", "danger"].contains(&tone),
        "invalid progress tone"
    );
    let size = attrs.get("size").map(String::as_str).unwrap_or("regular");
    ensure!(
        ["regular", "compact"].contains(&size),
        "invalid progress size"
    );
    let complete = state == "determinate" && value == maximum;
    let attributes = format!(
        "class=\"cui-progress cui-progress--{tone} cui-progress--{size} cui-progress--{state}{}\" data-cui-component=\"progress\" data-cui-tone=\"{tone}\" data-cui-size=\"{size}\" data-cui-state=\"{state}\"",
        if complete {
            " cui-progress--complete"
        } else {
            ""
        }
    );
    let progress_html = match (value, maximum) {
        (Some(value), Some(maximum)) => format!(
            "<progress class=\"cui-progress__track\" value=\"{}\" max=\"{}\" aria-label=\"{label}\"></progress>",
            number(value),
            number(maximum)
        ),
        _ => format!("<progress class=\"cui-progress__track\" aria-label=\"{label}\"></progress>"),
    };
    let suffix_html = suffix
        .map(|text| format!("<span class=\"cui-progress__suffix\">{text}</span>"))
        .unwrap_or_default();
    let detail_html = detail
        .map(|text| format!("<p class=\"cui-progress__detail\">{text}</p>"))
        .unwrap_or_default();
    fill_slots(
        static_fragment(package, "progress")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[progress]]", &progress_html),
            ("[[suffix]]", &suffix_html),
            ("[[detail]]", &detail_html),
        ],
    )
}

fn render_avatar_literal(avatar: &Button, package: &Package) -> Result<String> {
    checked_attributes(avatar, &["initials", "label", "size", "tone"], "avatar")?;
    let initials = avatar
        .attrs
        .get("initials")
        .context("cui-avatar requires initials")?;
    ensure!(
        !initials.trim().is_empty()
            && !initials.chars().any(char::is_control)
            && !initials.contains("{{")
            && !initials.contains("}}"),
        "avatar initials must be nonblank literal text without controls"
    );
    let graphemes = initials.graphemes(true).count();
    ensure!(
        (1..=3).contains(&graphemes),
        "avatar initials must contain one to three Unicode grapheme clusters"
    );
    let label = checked_template_text(
        avatar
            .attrs
            .get("label")
            .context("cui-avatar requires label")?,
        "avatar label",
    )?;
    let size = avatar
        .attrs
        .get("size")
        .map(String::as_str)
        .unwrap_or("medium");
    ensure!(
        ["extra-small", "small", "medium", "large", "extra-large"].contains(&size),
        "invalid avatar size"
    );
    let tone = avatar
        .attrs
        .get("tone")
        .map(String::as_str)
        .unwrap_or("neutral");
    ensure!(
        ["neutral", "brand", "success"].contains(&tone),
        "invalid avatar tone"
    );
    let markup = format!(
        "<span class=\"cui-avatar cui-avatar--{size} cui-avatar--{tone}\" role=\"img\" aria-label=\"{label}\" data-cui-component=\"avatar\"><span class=\"cui-avatar__initials\" aria-hidden=\"true\">{}</span></span>",
        escape(initials)
    );
    fill_static(
        static_fragment(package, "avatar")?,
        &[("[[avatar]]", &markup)],
    )
}

fn render_empty_state(state: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        state,
        &[
            "title",
            "body",
            "alignment",
            "heading-level",
            "action-label",
            "action-href",
            "action-route",
        ],
        "empty-state",
    )?;
    let title = checked_template_text(
        state
            .attrs
            .get("title")
            .context("cui-empty-state requires title")?,
        "empty-state title",
    )?;
    let body = checked_template_text(
        state
            .attrs
            .get("body")
            .context("cui-empty-state requires body")?,
        "empty-state body",
    )?;
    let alignment = state
        .attrs
        .get("alignment")
        .map(String::as_str)
        .unwrap_or("start");
    ensure!(
        ["start", "center"].contains(&alignment),
        "invalid empty-state alignment"
    );
    let heading = state
        .attrs
        .get("heading-level")
        .map(String::as_str)
        .unwrap_or("h2");
    ensure!(
        ["h2", "h3", "h4"].contains(&heading),
        "invalid empty-state heading level"
    );
    let destination = declaration_href(&state.attrs, "action-href", "action-route")?;
    let action_label = state
        .attrs
        .get("action-label")
        .map(|value| checked_template_text(value, "empty-state action label"))
        .transpose()?;
    ensure!(
        destination.is_some() == action_label.is_some(),
        "empty-state action destination and action-label must be supplied together"
    );
    let action = match (destination, action_label) {
        (Some(href), Some(label)) => {
            format!("<a class=\"cui-empty-state__action\" href=\"{href}\">{label}</a>")
        }
        _ => String::new(),
    };
    let action_class = if action.is_empty() {
        ""
    } else {
        " cui-empty-state--with-action"
    };
    let markup = format!(
        "<section class=\"cui-empty-state cui-empty-state--{alignment}{action_class}\" data-cui-component=\"empty-state\"><div class=\"cui-empty-state__layout\"><div class=\"cui-empty-state__content\"><{heading} class=\"cui-empty-state__title\">{title}</{heading}><p class=\"cui-empty-state__body\">{body}</p></div>{action}</div></section>"
    );
    fill_static(
        static_fragment(package, "empty-state")?,
        &[("[[empty-state]]", &markup)],
    )
}

fn render_metric(metric: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        metric,
        &[
            "label",
            "value",
            "detail",
            "appearance",
            "icon",
            "trend",
            "trend-tone",
            "trend-label",
            "trend-announcement",
        ],
        "metric",
    )?;
    let label = checked_template_text(
        metric
            .attrs
            .get("label")
            .context("cui-metric requires label")?,
        "metric label",
    )?;
    let value = checked_template_text(
        metric
            .attrs
            .get("value")
            .context("cui-metric requires value")?,
        "metric value",
    )?;
    let detail = metric
        .attrs
        .get("detail")
        .map(|text| checked_template_text(text, "metric detail"))
        .transpose()?;
    let appearance = metric
        .attrs
        .get("appearance")
        .map(String::as_str)
        .unwrap_or("plain");
    ensure!(
        ["plain", "contained"].contains(&appearance),
        "invalid metric appearance"
    );
    let trend = metric.attrs.get("trend").map(String::as_str);
    let trend_tone = metric.attrs.get("trend-tone").map(String::as_str);
    let trend_label = metric
        .attrs
        .get("trend-label")
        .map(|text| checked_template_text(text, "metric trend label"))
        .transpose()?;
    let trend_announcement = metric
        .attrs
        .get("trend-announcement")
        .map(|text| checked_template_text(text, "metric trend announcement"))
        .transpose()?;
    let has_trend = trend.is_some() && trend_tone.is_some() && trend_label.is_some();
    ensure!(
        (trend.is_some() || trend_tone.is_some() || trend_label.is_some()) == has_trend,
        "metric trend, trend-tone, and trend-label must be supplied together"
    );
    ensure!(
        trend_announcement.is_none() || has_trend,
        "metric trend-announcement requires a trend"
    );
    if let Some(name) = metric.attrs.get("icon") {
        ensure!(
            package.icons.contains_key(name),
            "unknown package icon: {name}"
        );
    }
    let mut content = String::new();
    if let Some(name) = metric.attrs.get("icon") {
        let icon = render_package_icon(name, "small", package)?;
        content.push_str(&format!(
            "<span class=\"cui-metric__icon\" aria-hidden=\"true\">{icon}</span>"
        ));
    }
    content.push_str(&format!(
        "<dl class=\"cui-metric__content\"><dt class=\"cui-metric__label\">{label}</dt><dd class=\"cui-metric__value\">{value}</dd></dl>"
    ));
    let mut footer = String::new();
    if let (Some(direction), Some(tone), Some(label)) = (trend, trend_tone, trend_label.as_deref())
    {
        ensure!(
            ["up", "down", "flat"].contains(&direction),
            "invalid metric trend"
        );
        ensure!(
            ["positive", "negative", "neutral"].contains(&tone),
            "invalid metric trend tone"
        );
        let icon_name = match direction {
            "up" => "arrow-up",
            "down" => "arrow-down",
            "flat" => "minus",
            _ => unreachable!(),
        };
        let icon = render_package_icon(icon_name, "small", package)?;
        let direction_text = match direction {
            "up" => "Increased",
            "down" => "Decreased",
            _ => "Unchanged",
        };
        let tone_text = match tone {
            "positive" => "favorable change",
            "negative" => "unfavorable change",
            _ => "neutral change",
        };
        let announcement =
            trend_announcement.unwrap_or_else(|| format!("{direction_text}, {tone_text}"));
        footer.push_str(&format!(
            "<span class=\"cui-metric__change cui-metric__change--{direction} cui-metric__change--{tone}\"><span class=\"cui-visually-hidden\">{announcement}: </span>{icon}<span>{label}</span></span>"
        ));
    }
    if let Some(detail) = detail {
        footer.push_str(&format!(
            "<span class=\"cui-metric__detail\">{detail}</span>"
        ));
    }
    if !footer.is_empty() {
        content.push_str(&format!("<p class=\"cui-metric__footer\">{footer}</p>"));
    }
    let markup = format!(
        "<article class=\"cui-metric cui-metric--{appearance}\" data-cui-component=\"metric\">{content}</article>"
    );
    fill_static(
        static_fragment(package, "metric")?,
        &[("[[metric]]", &markup)],
    )
}

fn render_skeleton(skeleton: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        skeleton,
        &["shape", "size", "width", "animated"],
        "skeleton",
    )?;
    let shape = skeleton
        .attrs
        .get("shape")
        .context("cui-skeleton requires shape")?;
    ensure!(
        ["text", "rectangle", "circle"].contains(&shape.as_str()),
        "invalid skeleton shape"
    );
    let size = skeleton
        .attrs
        .get("size")
        .map(String::as_str)
        .unwrap_or("medium");
    ensure!(
        ["small", "medium", "large"].contains(&size),
        "invalid skeleton size"
    );
    let width = skeleton
        .attrs
        .get("width")
        .map(String::as_str)
        .unwrap_or("full");
    ensure!(
        ["short", "medium", "full"].contains(&width),
        "invalid skeleton width"
    );
    let animated = skeleton
        .attrs
        .get("animated")
        .map(String::as_str)
        .unwrap_or("true");
    ensure!(
        ["true", "false"].contains(&animated),
        "skeleton animated must be true or false"
    );
    let animation = if animated == "true" {
        "animated"
    } else {
        "static"
    };
    let markup = format!(
        "<span class=\"cui-skeleton cui-skeleton--{shape} cui-skeleton--{size} cui-skeleton--width-{width} cui-skeleton--{animation}\" data-cui-component=\"skeleton\" aria-hidden=\"true\"></span>"
    );
    fill_static(
        static_fragment(package, "skeleton")?,
        &[("[[skeleton]]", &markup)],
    )
}

fn render_page_header(header: &Button, package: &Package) -> Result<String> {
    checked_attributes(header, &["title", "description"], "page-header")?;
    let title = checked_template_text(
        header
            .attrs
            .get("title")
            .context("cui-page-header requires title")?,
        "page-header title",
    )?;
    let description = header
        .attrs
        .get("description")
        .map(|text| checked_template_text(text, "page-header description"))
        .transpose()?
        .map(|text| format!("<p class=\"cui-page-header__description\">{text}</p>"))
        .unwrap_or_default();
    let markup = format!(
        "<header class=\"cui-page-header\" data-cui-component=\"page-header\"><div class=\"cui-page-header__content\"><h1 class=\"cui-page-header__title\">{title}</h1>{description}</div></header>"
    );
    fill_static(
        static_fragment(package, "page-header")?,
        &[("[[page-header]]", &markup)],
    )
}

fn number(value: f64) -> String {
    let value = value.to_string();
    value.strip_suffix(".0").unwrap_or(&value).to_owned()
}

fn safe_href(value: &str) -> bool {
    if value.is_empty()
        || value != value.trim()
        || value.bytes().any(|b| b.is_ascii_control() || b == b'\\')
        || value.starts_with("//")
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }
    if let Some((scheme, _)) = value.split_once(':') {
        if !scheme.contains('/') && !scheme.contains('?') && !scheme.contains('#') {
            let lower = scheme.to_ascii_lowercase();
            if lower != "http" && lower != "https" {
                return false;
            }
            let Some(authority) = value[scheme.len() + 1..].strip_prefix("//") else {
                return false;
            };
            let host = authority.split(['/', '?', '#']).next().unwrap_or("");
            return !host.is_empty()
                && !host.starts_with('@')
                && !host.ends_with(':')
                && !host.contains('@')
                && host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b));
        }
    }
    true
}

fn transform_html_with_bindings<const N: usize>(
    input: &str,
    package: Option<&Package>,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<String> {
    let mut browser_scripts = BTreeSet::new();
    transform_html_with_selected_scripts(input, package, counts, bindings, &mut browser_scripts)
}

fn transform_html_with_selected_scripts<const N: usize>(
    input: &str,
    package: Option<&Package>,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
    browser_scripts: &mut BTreeSet<String>,
) -> Result<String> {
    composition::expand(input, package, counts, bindings, browser_scripts)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub field_path: String,
    pub expected_kind: String,
    pub component: String,
    pub attribute: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationContract {
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expansion {
    pub html: String,
    pub used_components: BTreeSet<String>,
    pub bindings: Vec<Binding>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub browser_scripts: BTreeSet<String>,
}

impl Package {
    /// Build a pure expansion context from assets already captured and admitted by the host.
    pub fn from_assets(assets: &BTreeMap<String, Vec<u8>>) -> Result<Self> {
        assets::from_assets(assets)
    }

    pub fn theme(&self) -> &[u8] {
        &self.theme
    }
    pub fn styles(&self) -> &[u8] {
        &self.styles
    }
    pub fn property_catalog(&self) -> Option<&serde_json::Value> {
        self.property_catalog.as_ref()
    }
    pub fn icons(&self) -> &BTreeMap<String, String> {
        &self.icons
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn presentation_contracts(&self) -> &BTreeMap<String, PresentationContract> {
        &self.presentation_contracts
    }
}

/// Expand supported declarations in a captured HTML source string.
pub fn expand(input: &str, package: &Package) -> Result<Expansion> {
    let mut counts = [0usize; composition::NAMES.len()];
    let mut bindings = Vec::new();
    let mut browser_scripts = BTreeSet::new();
    let html = transform_html_with_selected_scripts(
        input,
        Some(package),
        &mut counts,
        &mut bindings,
        &mut browser_scripts,
    )?;
    let used_components = composition::NAMES
        .iter()
        .zip(counts)
        .filter(|(_, count)| *count > 0)
        .map(|(name, _)| (*name).to_owned())
        .collect();
    Ok(Expansion {
        html,
        used_components,
        bindings,
        browser_scripts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package_assets() -> BTreeMap<String, Vec<u8>> {
        fn collect(
            root: &std::path::Path,
            base: &std::path::Path,
            assets: &mut BTreeMap<String, Vec<u8>>,
        ) {
            for entry in std::fs::read_dir(root).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    collect(&entry.path(), base, assets);
                } else {
                    let key = entry
                        .path()
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    assets.insert(key, std::fs::read(entry.path()).unwrap());
                }
            }
        }
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
        let mut assets = BTreeMap::new();
        collect(&base, &base, &mut assets);
        assets
    }

    fn package() -> Package {
        Package::from_assets(&package_assets()).unwrap()
    }

    #[test]
    fn expands_button_with_exact_renderer_output() {
        let result = expand(r#"<cui-button label="Save" kind="submit"/>"#, &package()).unwrap();
        assert_eq!(
            result.html,
            "<button class=\"cui-button cui-button--secondary\" type=\"submit\"><span>Save</span></button>\n"
        );
        assert_eq!(
            result.used_components,
            BTreeSet::from(["button".to_owned()])
        );
        assert!(result.bindings.is_empty());
        let bound = expand(r#"<cui-button label="{{ page.title }}"/>"#, &package()).unwrap();
        assert_eq!(
            bound.html,
            "<button class=\"cui-button cui-button--secondary\" type=\"button\"><span>{{ ui_text(page.title, \"nonblank\", 1, 0) }}</span></button>\n"
        );
        assert_eq!(
            bound.bindings,
            vec![Binding {
                field_path: "page.title".into(),
                expected_kind: "string".into(),
                component: "button".into(),
                attribute: "label".into()
            }]
        );
    }

    #[test]
    fn rejects_unsupported_declarations_and_malformed_metadata() {
        let package = package();
        assert!(expand("<cui-unknown/>", &package).is_err());
        assert!(expand("<cui-button label='{{ page.name + 1 }}'/>", &package).is_err());
        let invalid = BTreeMap::from([("ui-package.json".to_owned(), b"{}".to_vec())]);
        assert!(Package::from_assets(&invalid).is_err());
    }

    #[test]
    fn captured_package_metadata_is_checked_before_expansion() {
        let mutate = |path: &str, f: &mut dyn FnMut(&mut Vec<u8>)| {
            let mut assets = package_assets();
            f(assets.get_mut(path).unwrap());
            assert!(
                Package::from_assets(&assets).is_err(),
                "accepted invalid {path}"
            );
        };
        mutate("components/card/component.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["contract"]["role"] = "unexpected".into();
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        mutate("components/card/fragment.html", &mut |bytes| {
            bytes.extend_from_slice(b"[[extra]]")
        });
        mutate("components/card/component.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["assets"]["styles"] = "../escape.css".into();
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        mutate("icons.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["unsafe"] = "<path onload=\\\"x\\\"/>".into();
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        mutate("icons.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["unsafe"] = "<path d=\"M0 0\"/>\u{1}".into();
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        mutate("property-catalog.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            value["package"] = "@other/package".into();
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        mutate("property-catalog.json", &mut |bytes| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            let first = value["components"][0].clone();
            value["components"].as_array_mut().unwrap().push(first);
            *bytes = serde_json::to_vec(&value).unwrap();
        });
    }
}

#[cfg(test)]
mod control_id_tests {
    use super::control_id;

    #[test]
    fn control_ids_allow_one_field_after_a_literal_prefix() {
        assert_eq!(
            control_id("buyer-name", "form-field").unwrap(),
            "buyer-name"
        );
        assert_eq!(
            control_id("buyer-name-{{ customer.customer_id }}", "form-field").unwrap(),
            "buyer-name-{{ customer.customer_id }}"
        );
        assert_eq!(
            control_id("deal-{{deal.id}}-stage", "select-field").unwrap(),
            "deal-{{ deal.id }}-stage"
        );
        for bad in [
            "",
            "has space",
            "{{ customer.id }}",
            "x-{{ customer.id | upper }}",
            "x-{{ customer.id }}-{{ deal.id }}",
            "x-{{ customer.id",
            "x\"-{{ customer.id }}",
            "x-{{ Customer.Id }}",
            "x-{% if a %}",
        ] {
            assert!(control_id(bad, "form-field").is_err(), "{bad}");
        }
    }
}
