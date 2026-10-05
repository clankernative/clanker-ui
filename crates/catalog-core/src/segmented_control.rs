//! Static mutually exclusive navigation links; this is not a form control or tablist.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Link,
    Current,
    Disabled,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentedItem {
    pub kind: ItemKind,
    pub label: String,
    #[serde(default)]
    pub href: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentedControlInstance {
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(default)]
    pub equal_width: bool,
    pub items: Vec<SegmentedItem>,
}
fn default_label() -> String {
    "Options".into()
}
fn safe_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl SegmentedControlInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !safe_text(&self.label) {
            return Err("segmented control label must be nonblank plain text".into());
        }
        if !(2..=8).contains(&self.items.len()) {
            return Err("segmented control requires 2 to 8 items".into());
        }
        if self
            .items
            .iter()
            .filter(|item| item.kind == ItemKind::Current)
            .count()
            != 1
        {
            return Err("segmented control requires exactly one current item".into());
        }
        for item in &self.items {
            if !safe_text(&item.label) {
                return Err("segment labels must be nonblank plain text".into());
            }
            match item.kind {
                ItemKind::Link => {
                    let href = item.href.as_deref().ok_or("linked segment requires href")?;
                    if !crate::button::safe_href(href) {
                        return Err("segment href must be a safe destination".into());
                    }
                }
                ItemKind::Current | ItemKind::Disabled if item.href.is_some() => {
                    return Err("current and disabled segments forbid href".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
/// Renders native linked navigation. The fragment needs one `[[attributes]]`, `[[label]]`, and `[[items]]` slot.
pub fn render(instance: &SegmentedControlInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    let mut items = String::new();
    for item in &instance.items {
        let (class, content) = match item.kind {
            ItemKind::Link => (
                "link",
                format!(
                    "<a class=\"cui-segmented-control__link\" href=\"{}\">{}</a>",
                    esc(item.href.as_deref().expect("validated href")),
                    esc(&item.label)
                ),
            ),
            ItemKind::Current => (
                "current",
                format!(
                    "<span class=\"cui-segmented-control__current\" aria-current=\"page\">{}</span>",
                    esc(&item.label)
                ),
            ),
            ItemKind::Disabled => (
                "disabled",
                format!(
                    "<span class=\"cui-segmented-control__disabled\" aria-disabled=\"true\">{}</span>",
                    esc(&item.label)
                ),
            ),
        };
        items.push_str(&format!("<li class=\"cui-segmented-control__item cui-segmented-control__item--{class}\">{content}</li>"));
    }
    let modifier = if instance.equal_width {
        " cui-segmented-control--equal"
    } else {
        ""
    };
    let attributes = format!(
        "class=\"cui-segmented-control{modifier}\" data-cui-component=\"segmented-control\""
    );
    let label = esc(&instance.label);
    let list = format!("<ul class=\"cui-segmented-control__list\">{items}</ul>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[items]]", &list),
        ],
    )
}
