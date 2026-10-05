//! Static linked peer-page navigation (not an ARIA tablist or client-side panel switcher).
use serde::{Deserialize, Serialize};

pub const MAX_TABS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TabsInstance {
    #[serde(default = "default_label")]
    pub label: String,
    pub items: Vec<TabItem>,
}
fn default_label() -> String {
    "Sections".into()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TabItem {
    pub label: String,
    pub href: String,
    #[serde(default)]
    pub active: bool,
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
impl TabsInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !safe_text(&self.label) {
            return Err("tabs accessible label must be nonblank plain text".into());
        }
        if !(2..=MAX_TABS).contains(&self.items.len()) {
            return Err(format!("tabs require 2 to {MAX_TABS} destinations"));
        }
        if self.items.iter().filter(|item| item.active).count() != 1 {
            return Err("tabs require exactly one active destination".into());
        }
        for item in &self.items {
            if !safe_text(&item.label) {
                return Err("tab labels must be nonblank plain text".into());
            }
            if !crate::button::safe_href(&item.href) {
                return Err("tab href must be a safe destination".into());
            }
        }
        Ok(())
    }
}
/// Renders linked navigation; the fragment has one `[[attributes]]`, `[[label]]`, and `[[items]]` slot.
pub fn render(instance: &TabsInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    let mut items = String::new();
    for item in &instance.items {
        let active = if item.active {
            " cui-tabs__link--active"
        } else {
            ""
        };
        let current = if item.active {
            " aria-current=\"page\""
        } else {
            ""
        };
        items.push_str(&format!("<li class=\"cui-tabs__item\"><a class=\"cui-tabs__link{active}\" href=\"{}\"{current}>{}</a></li>", esc(&item.href), esc(&item.label)));
    }
    let attributes = "class=\"cui-tabs\" data-cui-component=\"tabs\"";
    let label = esc(&instance.label);
    let list = format!("<ul class=\"cui-tabs__list\">{items}</ul>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", attributes),
            ("[[label]]", &label),
            ("[[items]]", &list),
        ],
    )
}
