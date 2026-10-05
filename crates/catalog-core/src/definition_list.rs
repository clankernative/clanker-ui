//! Typed static definition-list rendering. Serialized instances contain text only;
//! richer values enter exclusively through host-admitted children.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Density {
    #[default]
    Standard,
    Compact,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionListInstance {
    pub items: Vec<Item>,
    #[serde(default)]
    pub density: Density,
    #[serde(default = "default_true")]
    pub dividers: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub term: String,
    pub value: String,
}
fn default_true() -> bool {
    true
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn plain(s: &str) -> bool {
    !s.chars().any(char::is_control)
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && plain(s)
}
impl DefinitionListInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.items.is_empty() || self.items.len() > 100 {
            return Err("definition list requires 1..=100 items".into());
        }
        if self
            .items
            .iter()
            .any(|item| !text(&item.term) || !plain(&item.value))
        {
            return Err(
                "definition terms must be nonblank and all text must be control-free".into(),
            );
        }
        Ok(())
    }
}
fn render_values(
    i: &DefinitionListInstance,
    values: &[String],
    fragment: &str,
) -> Result<String, String> {
    i.validate()?;
    if values.len() != i.items.len() {
        return Err("definition values must match item count".into());
    }
    let items = i.items.iter().zip(values).map(|(item, value)| format!(
        "<div class=\"cui-definition-list__item\"><dt class=\"cui-definition-list__term\">{}</dt><dd class=\"cui-definition-list__value\">{}</dd></div>", esc(&item.term), value
    )).collect::<String>();
    let attributes = format!(
        "class=\"cui-definition-list cui-definition-list--{}{}\" data-cui-component=\"definition-list\"",
        if i.density == Density::Compact {
            "compact"
        } else {
            "standard"
        },
        if i.dividers {
            ""
        } else {
            " cui-definition-list--plain"
        }
    );
    crate::fragment::fill(
        fragment,
        &[("[[attributes]]", &attributes), ("[[items]]", &items)],
    )
}
/// Render every item as escaped text.
pub fn render(i: &DefinitionListInstance, fragment: &str) -> Result<String, String> {
    let values = i
        .items
        .iter()
        .map(|item| esc(&item.value))
        .collect::<Vec<_>>();
    render_values(i, &values, fragment)
}
/// Substitute only values already admitted by the Native host. `None` retains the
/// serialized plain-text value; this function is not an HTML sanitizer.
pub fn render_with_admitted_values(
    i: &DefinitionListInstance,
    rich_values: &[Option<AdmittedChildren>],
    fragment: &str,
) -> Result<String, String> {
    if rich_values.len() != i.items.len() {
        return Err("admitted definition values must match item count".into());
    }
    let values = i
        .items
        .iter()
        .zip(rich_values)
        .map(|(item, rich)| {
            rich.as_ref()
                .map(|v| v.as_markup().to_owned())
                .unwrap_or_else(|| esc(&item.value))
        })
        .collect::<Vec<_>>();
    render_values(i, &values, fragment)
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<dl [[attributes]]>[[items]]</dl>";
    fn i() -> DefinitionListInstance {
        DefinitionListInstance {
            items: vec![Item {
                term: "Owner <team>".into(),
                value: "Platform & ops".into(),
            }],
            density: Density::Standard,
            dividers: true,
        }
    }
    #[test]
    fn escaped_default_and_rich_path() {
        let h = render(&i(), F).unwrap();
        assert!(h.contains("Owner &lt;team&gt;"));
        assert!(h.contains("Platform &amp; ops"));
        let r = render_with_admitted_values(
            &i(),
            &[Some(AdmittedChildren::from_host_admitted(
                "<strong>Ready</strong>",
            ))],
            F,
        )
        .unwrap();
        assert!(r.contains("<strong>Ready</strong>"));
    }
    #[test]
    fn validates_unknown_controls_and_rich_length() {
        assert!(DefinitionListInstance::parse(
            r#"{"items":[{"term":"x","value":"y"}],"extra":true}"#
        )
        .is_err());
        let mut x = i();
        x.items = (0..101)
            .map(|n| Item {
                term: format!("Term {n}"),
                value: "Value".into(),
            })
            .collect();
        assert!(x.validate().is_err());
        x = i();
        x.items[0].term = "\n".into();
        assert!(x.validate().is_err());
        assert!(render_with_admitted_values(&i(), &[], F).is_err());
        assert!(render(&i(), "[[attributes]]").is_err());
    }
}
