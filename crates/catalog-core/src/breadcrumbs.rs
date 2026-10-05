//! Typed breadcrumb navigation with linked ancestors and one current page.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreadcrumbsInstance {
    #[serde(default = "default_label")]
    pub label: String,
    pub items: Vec<BreadcrumbItem>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreadcrumbItem {
    pub label: String,
    #[serde(default)]
    pub href: Option<String>,
}
fn default_label() -> String {
    "Breadcrumbs".into()
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl BreadcrumbsInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("breadcrumb label must be nonblank plain text".into());
        }
        if self.items.len() < 2 {
            return Err("breadcrumbs require at least one ancestor and a current item".into());
        }
        for (n, item) in self.items.iter().enumerate() {
            if item.label.trim().is_empty() || item.label.chars().any(char::is_control) {
                return Err("breadcrumb item labels must be nonblank plain text".into());
            }
            if n + 1 == self.items.len() {
                if item.href.is_some() {
                    return Err("the final breadcrumb must be a nonlink current page".into());
                }
            } else {
                let h = item
                    .href
                    .as_deref()
                    .ok_or("every ancestor breadcrumb must be linked")?;
                if !crate::button::safe_href(h) {
                    return Err("breadcrumb href must be a safe destination".into());
                }
            }
        }
        Ok(())
    }
}
pub fn render(i: &BreadcrumbsInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let mut li = String::new();
    for item in &i.items {
        let content = if let Some(h) = &item.href {
            format!(
                "<a class=\"cui-breadcrumbs__link\" href=\"{}\">{}</a>",
                esc(h),
                esc(&item.label)
            )
        } else {
            format!(
                "<span class=\"cui-breadcrumbs__current\" aria-current=\"page\">{}</span>",
                esc(&item.label)
            )
        };
        li.push_str(&format!(
            "<li class=\"cui-breadcrumbs__item\">{content}</li>"
        ));
    }
    let attr = "class=\"cui-breadcrumbs\" data-cui-component=\"breadcrumbs\"";
    let navlabel = esc(&i.label);
    let items = format!("<ol class=\"cui-breadcrumbs__list\">{li}</ol>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", attr),
            ("[[label]]", &navlabel),
            ("[[items]]", &items),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<nav [[attributes]] aria-label=\"[[label]]\">[[items]]</nav>";
    fn i() -> BreadcrumbsInstance {
        BreadcrumbsInstance {
            label: "Breadcrumbs".into(),
            items: vec![
                BreadcrumbItem {
                    label: "Home".into(),
                    href: Some("/".into()),
                },
                BreadcrumbItem {
                    label: "Reports <&>".into(),
                    href: None,
                },
            ],
        }
    }
    #[test]
    fn renders_linked_ancestors_and_current_page() {
        let h = render(&i(), F).unwrap();
        assert!(h.contains("href=\"/\">Home</a>"));
        assert!(h.contains("aria-current=\"page\">Reports &lt;&amp;&gt;"));
    }
    #[test]
    fn rejects_missing_current_or_unsafe_href() {
        let mut x = i();
        x.items[1].href = Some("/bad".into());
        assert!(x.validate().is_err());
        x.items[1].href = None;
        x.items[0].href = Some("javascript:alert(1)".into());
        assert!(x.validate().is_err());
    }
    #[test]
    fn exact_slots_and_default_label_parse() {
        let x =
            BreadcrumbsInstance::parse(r#"{"items":[{"label":"A","href":"/a"},{"label":"B"}]}"#)
                .unwrap();
        assert_eq!(x.label, "Breadcrumbs");
        assert!(render(&x, "[[attributes]][[label]]").is_err());
    }
}
