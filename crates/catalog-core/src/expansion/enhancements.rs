//! Closed build-time configuration for browser-local progressive enhancements.
use super::composition::Children;
use super::*;
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
enum Kind {
    Text,
    Json,
    Boolean,
    NullableText,
}

fn options(declaration: &Button, fields: &[(&str, &str, Kind)]) -> Result<Value> {
    let allowed = fields
        .iter()
        .map(|(attribute, _, _)| *attribute)
        .collect::<Vec<_>>();
    checked_attributes(declaration, &allowed, "enhancement")?;
    let mut values = Map::new();
    for (attribute, field, kind) in fields {
        let Some(raw) = declaration.attrs.get(*attribute) else {
            continue;
        };
        let value = match kind {
            Kind::Text => Value::String(raw.clone()),
            Kind::NullableText if raw == "null" => Value::Null,
            Kind::NullableText => Value::String(raw.clone()),
            Kind::Boolean => {
                ensure!(
                    raw == "true" || raw == "false",
                    "{attribute} must be literal true or false"
                );
                serde_json::from_str(raw)?
            }
            Kind::Json => serde_json::from_str(raw)
                .with_context(|| format!("{attribute} must be closed JSON data"))?,
        };
        ensure!(
            plain_data(&value),
            "{attribute} must be literal configuration, not a template expression"
        );
        values.insert((*field).into(), value);
    }
    Ok(Value::Object(values))
}
fn plain_data(value: &Value) -> bool {
    match value {
        Value::String(s) => !["{{", "}}", "{%", "%}"]
            .iter()
            .any(|marker| s.contains(marker)),
        Value::Array(a) => a.iter().all(plain_data),
        Value::Object(o) => o.values().all(plain_data),
        _ => true,
    }
}

pub(super) fn render(
    name: &str,
    declaration: &Button,
    children: &Children,
    package: &Package,
) -> Result<String> {
    ensure!(
        children.body.trim().is_empty()
            && children.slots.is_empty()
            && children.definitions.is_empty(),
        "cui-{name} accepts no child markup"
    );
    let fragment = package
        .static_fragments
        .get(name)
        .with_context(|| format!("cui-{name} requires its ready locked fragment"))?;
    use Kind::*;
    let html = match name {
        "copy-field" => {
            let value = options(
                declaration,
                &[
                    ("name", "name", Text),
                    ("id", "id", Text),
                    ("label", "label", Text),
                    ("value", "value", Text),
                    ("hint", "hint", Text),
                    ("idle-label", "idleLabel", Text),
                    ("copied-label", "copiedLabel", Text),
                    ("failed-label", "failedLabel", Text),
                ],
            )?;
            let instance = crate::copy_field::CopyFieldInstance::parse(&value.to_string())
                .map_err(anyhow::Error::msg)?;
            crate::copy_field::render(&instance, fragment, &package.icons)
        }
        "theme-switcher" => {
            let value = options(
                declaration,
                &[
                    ("label", "label", Text),
                    ("choices", "choices", Json),
                    ("presentation", "presentation", Text),
                    ("default-choice", "defaultChoice", Text),
                    ("system-light-theme", "systemLightTheme", Text),
                    ("system-dark-theme", "systemDarkTheme", Text),
                    ("storage-key", "storageKey", NullableText),
                    ("target-id", "targetId", Text),
                ],
            )?;
            let instance = crate::theme_switcher::ThemeSwitcherInstance::parse(
                &serde_json::to_vec(&value)?,
                &package.icons,
            )
            .map_err(anyhow::Error::msg)?;
            crate::theme_switcher::render(&instance, fragment, &package.icons)
        }
        "tooltip" => {
            let value = options(
                declaration,
                &[
                    ("id", "id", Text),
                    ("trigger-label", "triggerLabel", Text),
                    ("title", "title", Text),
                    ("description", "description", Text),
                    ("icon", "icon", Text),
                    ("placement", "placement", Text),
                    ("arrow", "arrow", Boolean),
                ],
            )?;
            let instance = crate::tooltip::TooltipInstance::parse(
                &serde_json::to_vec(&value)?,
                &package.icons,
            )
            .map_err(anyhow::Error::msg)?;
            crate::tooltip::render(&instance, fragment, &package.icons)
        }
        "toast" => {
            let value = options(
                declaration,
                &[
                    ("label", "label", Text),
                    ("messages", "messages", Json),
                    ("position", "position", Text),
                    ("history-policy", "historyPolicy", Text),
                ],
            )?;
            let instance =
                crate::toast::ToastInstance::parse(&serde_json::to_vec(&value)?, &package.icons)
                    .map_err(anyhow::Error::msg)?;
            crate::toast::render(&instance, fragment, &package.icons)
        }
        _ => anyhow::bail!("unsupported progressive enhancement: {name}"),
    };
    html.map_err(anyhow::Error::msg)
}
