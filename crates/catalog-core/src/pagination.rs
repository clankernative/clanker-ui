//! Static linked pagination with explicit current, gap, and disabled controls.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    #[default]
    Standard,
    Outlined,
    Compact,
}
impl Variant {
    fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Outlined => "outlined",
            Self::Compact => "compact",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Page,
    Current,
    Previous,
    Next,
    Gap,
}
impl ItemKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Current => "current",
            Self::Previous => "previous",
            Self::Next => "next",
            Self::Gap => "gap",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaginationItem {
    pub kind: ItemKind,
    pub label: String,
    #[serde(default)]
    pub href: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaginationInstance {
    #[serde(default = "label_default")]
    pub label: String,
    #[serde(default)]
    pub variant: Variant,
    pub items: Vec<PaginationItem>,
}
fn label_default() -> String {
    "Pagination".into()
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl PaginationInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("pagination label must be nonblank plain text".into());
        }
        if self.items.is_empty()
            || self
                .items
                .iter()
                .filter(|i| i.kind == ItemKind::Current)
                .count()
                != 1
        {
            return Err("pagination requires items and exactly one current item".into());
        }
        for item in &self.items {
            if item.label.trim().is_empty() || item.label.chars().any(char::is_control) {
                return Err("pagination labels must be nonblank plain text".into());
            }
            match item.kind {
                ItemKind::Current | ItemKind::Gap if item.href.is_some() => {
                    return Err("current and gap items cannot have hrefs".into());
                }
                ItemKind::Page if item.href.is_none() => {
                    return Err("page items require a safe href".into());
                }
                _ => {}
            }
            if let Some(h) = &item.href {
                if !crate::button::safe_href(h) {
                    return Err("pagination href must be a safe destination".into());
                }
            }
        }
        Ok(())
    }
}
pub fn render(i: &PaginationInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let mut list = String::new();
    for item in &i.items {
        let content = if let Some(h) = &item.href {
            let rel = match item.kind {
                ItemKind::Previous => " rel=\"prev\"",
                ItemKind::Next => " rel=\"next\"",
                _ => "",
            };
            format!(
                "<a class=\"cui-pagination__link\" href=\"{}\"{}>{}</a>",
                esc(h),
                rel,
                esc(&item.label)
            )
        } else if item.kind == ItemKind::Current {
            format!(
                "<span class=\"cui-pagination__current\" aria-current=\"page\">{}</span>",
                esc(&item.label)
            )
        } else if matches!(item.kind, ItemKind::Previous | ItemKind::Next) {
            format!(
                "<span class=\"cui-pagination__disabled\" aria-disabled=\"true\">{}</span>",
                esc(&item.label)
            )
        } else {
            format!(
                "<span class=\"cui-pagination__gap\" aria-hidden=\"true\">{}</span>",
                esc(&item.label)
            )
        };
        list.push_str(&format!(
            "<li class=\"cui-pagination__item cui-pagination__item--{}\">{content}</li>",
            item.kind.as_str()
        ));
    }
    let attr = format!(
        "class=\"cui-pagination cui-pagination--{}\" data-cui-component=\"pagination\"",
        i.variant.as_str()
    );
    let label = esc(&i.label);
    let items = format!("<ol class=\"cui-pagination__list\">{list}</ol>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attr),
            ("[[label]]", &label),
            ("[[items]]", &items),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<nav [[attributes]] aria-label=\"[[label]]\">[[items]]</nav>";
    fn fixture() -> PaginationInstance {
        PaginationInstance {
            label: "Results".into(),
            variant: Variant::Outlined,
            items: vec![
                PaginationItem {
                    kind: ItemKind::Previous,
                    label: "Previous".into(),
                    href: Some("/p/1".into()),
                },
                PaginationItem {
                    kind: ItemKind::Page,
                    label: "1".into(),
                    href: Some("/p/1".into()),
                },
                PaginationItem {
                    kind: ItemKind::Current,
                    label: "2".into(),
                    href: None,
                },
                PaginationItem {
                    kind: ItemKind::Gap,
                    label: "…".into(),
                    href: None,
                },
                PaginationItem {
                    kind: ItemKind::Next,
                    label: "Next".into(),
                    href: None,
                },
            ],
        }
    }
    #[test]
    fn renders_links_current_gap_disabled_and_variant() {
        let h = render(&fixture(), F).unwrap();
        assert!(h.contains("rel=\"prev\""));
        assert!(h.contains("aria-current=\"page\""));
        assert!(h.contains("aria-hidden=\"true\">…"));
        assert!(h.contains("aria-disabled=\"true\""));
        assert!(h.contains("cui-pagination--outlined"));
    }
    #[test]
    fn rejects_wrong_current_and_unsafe_destinations() {
        let mut x = fixture();
        x.items[2].kind = ItemKind::Page;
        assert!(x.validate().is_err());
        x = fixture();
        x.items[1].href = Some("javascript:bad".into());
        assert!(x.validate().is_err());
    }
    #[test]
    fn accepts_compact_and_rejects_fragment_slots() {
        let mut x = fixture();
        x.variant = Variant::Compact;
        assert!(render(&x, F).unwrap().contains("--compact"));
        assert!(render(&x, "[[attributes]]").is_err());
    }
}
