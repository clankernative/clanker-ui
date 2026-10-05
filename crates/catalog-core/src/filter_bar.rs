//! Presentation-only filter group; form/query ownership remains with the host.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilterBarInstance {
    pub label: String,
    #[serde(default)]
    pub result_summary: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterBarSlots {
    pub controls: AdmittedChildren,
    pub actions: Option<AdmittedChildren>,
    pub applied: Option<AdmittedChildren>,
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl FilterBarInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("filter bar label must be nonblank plain text".into());
        }
        if self
            .result_summary
            .as_deref()
            .is_some_and(|s| s.trim().is_empty() || s.chars().any(char::is_control))
        {
            return Err("result summary must be nonblank plain text".into());
        }
        Ok(())
    }
}
pub fn render_with_slots(
    i: &FilterBarInstance,
    fragment: &str,
    slots: &FilterBarSlots,
) -> Result<String, String> {
    i.validate()?;
    if slots.controls.as_markup().trim().is_empty()
        || slots
            .actions
            .as_ref()
            .is_some_and(|s| s.as_markup().trim().is_empty())
        || slots
            .applied
            .as_ref()
            .is_some_and(|s| s.as_markup().trim().is_empty())
    {
        return Err("filter slots must contain nonblank admitted children".into());
    }
    let attr = "class=\"cui-filter-bar\" data-cui-component=\"filter-bar\" role=\"group\"";
    let label = escape(&i.label);
    let summary = i
        .result_summary
        .as_ref()
        .map(|s| {
            format!(
                "<p class=\"cui-filter-bar__summary\" role=\"status\">{}</p>",
                escape(s)
            )
        })
        .unwrap_or_default();
    let controls = slots.controls.as_markup();
    let actions = slots
        .actions
        .as_ref()
        .map(AdmittedChildren::as_markup)
        .unwrap_or("");
    let applied = slots
        .applied
        .as_ref()
        .map(AdmittedChildren::as_markup)
        .unwrap_or("");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", attr),
            ("[[label]]", &label),
            ("[[summary]]", &summary),
            ("[[controls]]", controls),
            ("[[actions]]", actions),
            ("[[applied]]", applied),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<div [[attributes]] aria-label=\"[[label]]\"><div>[[controls]][[actions]]</div>[[applied]][[summary]]</div>";
    #[test]
    fn renders_only_group_presentation_and_trusted_slots() {
        let i = FilterBarInstance {
            label: "Filters <&>".into(),
            result_summary: Some("4 results".into()),
        };
        let slots = FilterBarSlots {
            controls: AdmittedChildren::from_host_admitted("<input>"),
            actions: None,
            applied: Some(AdmittedChildren::from_host_admitted("<b>Active</b>")),
        };
        let h = render_with_slots(&i, F, &slots).unwrap();
        assert!(h.contains("role=\"group\""));
        assert!(h.contains("Filters &lt;&amp;&gt;"));
        assert!(h.contains("<input>"));
        assert!(!h.contains("<form"));
    }
    #[test]
    fn allows_empty_optional_slots_and_escapes_summary() {
        let i = FilterBarInstance {
            label: "Filters".into(),
            result_summary: Some("<2".into()),
        };
        let h = render_with_slots(
            &i,
            F,
            &FilterBarSlots {
                controls: AdmittedChildren::from_host_admitted("<input aria-label=\"Search\">"),
                actions: None,
                applied: None,
            },
        )
        .unwrap();
        assert!(h.contains("&lt;2"));
        assert!(h.contains("<div><input aria-label=\"Search\"></div>"));
    }
    #[test]
    fn rejects_bad_values_or_slots() {
        let i = FilterBarInstance {
            label: " ".into(),
            result_summary: None,
        };
        assert!(render_with_slots(
            &i,
            F,
            &FilterBarSlots {
                controls: AdmittedChildren::from_host_admitted(""),
                actions: None,
                applied: None
            }
        )
        .is_err());
        let i = FilterBarInstance {
            label: "Filters".into(),
            result_summary: None,
        };
        assert!(render_with_slots(
            &i,
            "[[attributes]][[label]]",
            &FilterBarSlots {
                controls: AdmittedChildren::from_host_admitted("<input aria-label=\"Search\">"),
                actions: None,
                applied: None
            }
        )
        .is_err());
    }
}
