//! Static native form controls rendered through their typed package contracts.
use super::*;
use serde_json::{Map, Value};

fn json_attrs(
    declaration: &Button,
    allowed: &[&str],
    component: &str,
) -> Result<Map<String, Value>> {
    let mut value = Map::new();
    for (key, raw) in &declaration.attrs {
        ensure!(
            allowed.contains(&key.as_str()),
            "unknown cui-{component} attribute: {key}"
        );
        ensure!(
            !raw.contains("{{") && !raw.contains("}}"),
            "cui-{component} only accepts literal attributes here"
        );
        let parsed = match key.as_str() {
            "choices" => serde_json::from_str(raw).context("invalid choices JSON")?,
            "checked" | "required" | "disabled" => {
                ensure!(
                    raw == "true" || raw == "false",
                    "{key} must be true or false"
                );
                Value::Bool(raw == "true")
            }
            _ => Value::String(raw.clone()),
        };
        value.insert(key.clone(), parsed);
    }
    Ok(value)
}

fn render_group(declaration: &Button, package: &Package, checkbox: bool) -> Result<String> {
    let name = if checkbox {
        "checkbox-group"
    } else {
        "radio-group"
    };
    let allowed = if checkbox {
        &[
            "name",
            "id",
            "legend",
            "choices",
            "description",
            "error",
            "layout",
            "disabled",
        ][..]
    } else {
        &[
            "name",
            "id",
            "legend",
            "choices",
            "selected",
            "description",
            "error",
            "required",
            "layout",
            "disabled",
        ][..]
    };
    let json = serde_json::to_string(&Value::Object(json_attrs(declaration, allowed, name)?))?;
    let fragment = static_fragment(package, name)?;
    let html = if checkbox {
        let instance = crate::checkbox_group::CheckboxGroupInstance::parse(&json)
            .map_err(anyhow::Error::msg)?;
        crate::checkbox_group::render(&instance, fragment).map_err(anyhow::Error::msg)?
    } else {
        let instance =
            crate::radio_group::RadioGroupInstance::parse(&json).map_err(anyhow::Error::msg)?;
        crate::radio_group::render(&instance, fragment).map_err(anyhow::Error::msg)?
    };
    Ok(html)
}

pub(super) fn render(
    component: &str,
    declaration: &Button,
    package: &Package,
    _bindings: &mut Vec<Binding>,
) -> Result<String> {
    match component {
        "checkbox-group" => render_group(declaration, package, true),
        "radio-group" => render_group(declaration, package, false),
        "toggle" => {
            let checked_binding = declaration
                .attrs
                .get("checked")
                .and_then(|v| interpolation(v));
            if let Some(raw) = declaration.attrs.get("checked") {
                ensure!(
                    raw == "true" || raw == "false" || checked_binding.is_some(),
                    "checked must be true, false, or a whole checked page field"
                );
            }
            let mut attrs = declaration.clone();
            if checked_binding.is_some() {
                attrs.attrs.insert("checked".into(), "false".into());
            }
            let json = serde_json::to_string(&Value::Object(json_attrs(
                &attrs,
                &[
                    "name", "id", "label", "hint", "error", "checked", "required", "disabled",
                ],
                "toggle",
            )?))?;
            let instance =
                crate::toggle::ToggleInstance::parse(&json).map_err(anyhow::Error::msg)?;
            let mut html = crate::toggle::render(&instance, static_fragment(package, "toggle")?)
                .map_err(anyhow::Error::msg)?;
            if let Some(path) = checked_binding {
                let needle = " value=\"true\"";
                ensure!(
                    html.contains(needle),
                    "toggle renderer produced unexpected control markup"
                );
                html = html.replacen(
                    needle,
                    &format!(" data-cui-checked-flag=\"{{{{ {path} }}}}\"{needle}"),
                    1,
                );
            }
            Ok(html)
        }
        _ => anyhow::bail!("unsupported native control: {component}"),
    }
}
