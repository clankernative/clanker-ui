//! Accessible button-group rendering; buttons are admitted as complete host children.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    #[default]
    Joined,
    Spaced,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ButtonGroupInstance {
    pub label: String,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub full_width: bool,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl ButtonGroupInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("button group label must be nonblank plain text".into());
        }
        Ok(())
    }
}
/// Render the group from two to six buttons admitted by the host. Element-level
/// button composition is a Native-host responsibility; this crate does not inspect HTML.
pub fn render(
    i: &ButtonGroupInstance,
    buttons: &[AdmittedChildren],
    fragment: &str,
) -> Result<String, String> {
    i.validate()?;
    if !(2..=6).contains(&buttons.len()) {
        return Err("button group requires 2..=6 host-admitted buttons".into());
    }
    let children = buttons.iter().map(|b| b.as_markup()).collect::<String>();
    let attrs = format!(
        "class=\"cui-button-group cui-button-group--{}{}\" data-cui-component=\"button-group\" role=\"group\" aria-label=\"{}\"",
        if i.appearance == Appearance::Joined {
            "joined"
        } else {
            "spaced"
        },
        if i.full_width {
            " cui-button-group--full-width"
        } else {
            ""
        },
        esc(&i.label)
    );
    crate::fragment::fill(
        fragment,
        &[("[[attributes]]", &attrs), ("[[buttons]]", &children)],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<div [[attributes]]>[[buttons]]</div>";
    fn i() -> ButtonGroupInstance {
        ButtonGroupInstance {
            label: "Range <selection>".into(),
            appearance: Appearance::Joined,
            full_width: false,
        }
    }
    fn buttons(n: usize) -> Vec<AdmittedChildren> {
        (0..n)
            .map(|_| AdmittedChildren::from_host_admitted("<button type=\"button\">Go</button>"))
            .collect()
    }
    #[test]
    fn renders_named_group_with_admitted_buttons() {
        let h = render(&i(), &buttons(2), F).unwrap();
        assert!(h.contains("role=\"group\" aria-label=\"Range &lt;selection&gt;\""));
        assert_eq!(h.matches("<button").count(), 2);
    }
    #[test]
    fn enforces_bounds_label_unknowns_and_template() {
        for n in [0, 1, 7] {
            assert!(render(&i(), &buttons(n), F).is_err());
        }
        let mut x = i();
        x.label = "\n".into();
        assert!(x.validate().is_err());
        assert!(ButtonGroupInstance::parse(r#"{"label":"x","appearance":"grid"}"#).is_err());
        assert!(ButtonGroupInstance::parse(r#"{"label":"x","foo":false}"#).is_err());
        assert!(render(&i(), &buttons(2), "[[attributes]]").is_err());
    }
}
