//! Bounded static list/table viewport contracts. Window events carry metadata only; data stays application-owned.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Height {
    Compact,
    #[default]
    Standard,
    Viewport,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Paged,
    Incremental,
    Windowed,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Window {
    pub start: usize,
    pub total: usize,
    pub item_size: usize,
    pub overscan: usize,
    #[serde(default)]
    pub anchor: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewportInstance {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub height: Height,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub window: Option<Window>,
    #[serde(default)]
    pub items: Vec<TextItem>,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<TextRow>,
    /// Plain content only: serialized instances never carry arbitrary markup.
    pub pagination: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextItem {
    pub id: String,
    pub content: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextRow {
    pub id: String,
    pub cells: Vec<String>,
    #[serde(default)]
    pub highlighted: bool,
}
/// A host-admitted item is constructed only after ordinary app HTML admission.
#[derive(Clone, Debug)]
pub struct AdmittedItem {
    pub id: String,
    pub content: AdmittedChildren,
}
#[derive(Clone, Debug)]
pub struct AdmittedCellRow {
    pub id: String,
    pub cells: Vec<AdmittedChildren>,
    pub highlighted: bool,
}
#[derive(Clone, Debug)]
pub struct Targets {
    pub region: String,
    pub items: String,
    pub navigation: String,
    pub label: String,
    pub scroll: String,
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn safe_fragment(fragment: &str, required: &[&str]) -> Result<(), String> {
    for slot in required {
        if fragment.matches(slot).count() != 1 {
            return Err(format!("fragment must contain exactly one {slot} slot"));
        }
    }
    if fragment.matches("[[").count() != required.len() {
        return Err("fragment contains unsupported or incomplete slots".into());
    }
    Ok(())
}
impl ViewportInstance {
    pub fn targets(&self) -> Targets {
        Targets {
            region: self.id.clone(),
            items: format!("{}-items", self.id),
            navigation: format!("{}-navigation", self.id),
            label: format!("{}-label", self.id),
            scroll: format!("{}-scroll", self.id),
        }
    }
    pub fn parse(json: &str) -> Result<Self, String> {
        let x: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        x.validate()?;
        Ok(x)
    }
    pub fn validate(&self) -> Result<(), String> {
        let t = self.targets();
        if !token(&self.id) || !text(&self.label) || !text(&self.pagination) {
            return Err(
                "viewport requires safe id, nonblank label, and admitted navigation content".into(),
            );
        }
        let list = !self.items.is_empty();
        let table = !self.rows.is_empty() || !self.columns.is_empty();
        if list == table {
            return Err("viewport must contain exactly one list or table layout".into());
        }
        let ids: Vec<&str> = if list {
            self.items.iter().map(|x| x.id.as_str()).collect()
        } else {
            self.rows.iter().map(|x| x.id.as_str()).collect()
        };
        if !(1..=500).contains(&ids.len()) {
            return Err("viewport requires 1..=500 bounded items".into());
        }
        let mut seen = BTreeSet::new();
        let reserved = [
            t.region.as_str(),
            t.items.as_str(),
            t.navigation.as_str(),
            t.label.as_str(),
            t.scroll.as_str(),
        ];
        if ids
            .iter()
            .any(|id| !token(id) || !seen.insert(*id) || reserved.contains(id))
        {
            return Err(
                "viewport item IDs must be unique safe tokens without owned-ID collisions".into(),
            );
        }
        if list {
            if self.items.iter().any(|x| !text(&x.content)) {
                return Err("list content must be nonblank plain text".into());
            }
            if !self.columns.is_empty() || !self.rows.is_empty() {
                return Err("list cannot also contain table data".into());
            }
        } else {
            if !(1..=12).contains(&self.columns.len()) || self.columns.iter().any(|x| !text(x)) {
                return Err("table requires 1..=12 nonblank column labels".into());
            }
            if self.rows.iter().any(|r| {
                r.cells.len() != self.columns.len()
                    || r.cells.iter().any(|v| v.chars().any(char::is_control))
            }) {
                return Err(
                    "every table row must have exactly one safe text cell per column".into(),
                );
            }
            if !self.items.is_empty() {
                return Err("table cannot also contain list items".into());
            }
        }
        match (self.mode, &self.window) {
            (Mode::Windowed, Some(w)) => {
                let n = ids.len();
                if w.total < n
                    || w.total > 10_000_000
                    || w.start >= w.total
                    || w.start.checked_add(n).is_none_or(|end| end > w.total)
                    || w.start.checked_mul(w.item_size).is_none()
                    || w.total.checked_mul(w.item_size).is_none()
                {
                    return Err("window bounds must contain all rendered items".into());
                }
                if !(16..=4096).contains(&w.item_size) || !(1..=100).contains(&w.overscan) {
                    return Err("window item size must be 16..=4096 and overscan 1..=100".into());
                }
                if let Some(anchor) = &w.anchor {
                    if !ids.contains(&anchor.as_str()) {
                        return Err("window anchor must identify a rendered item".into());
                    }
                }
            }
            (Mode::Windowed, None) => {
                return Err("windowed mode requires window configuration".into());
            }
            (_, Some(_)) => return Err("window config is only valid in windowed mode".into()),
            _ => {}
        }
        Ok(())
    }
}
/// Render plain text fixture items. Pagination is plain text unless host rendering substitutes it separately.
pub fn render(i: &ViewportInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    render_parts(i, fragment, &[], &[], false, None)
}
/// Render a host-admitted list after the host has checked item structure, IDs, and budgets.
pub fn render_admitted_list(
    i: &ViewportInstance,
    items: &[AdmittedItem],
    fragment: &str,
) -> Result<String, String> {
    if !i.items.is_empty() || !i.rows.is_empty() || !i.columns.is_empty() {
        return Err("admitted items cannot be mixed with serialized fixture content".into());
    }
    let mut owned = i.clone();
    owned.items = items
        .iter()
        .map(|v| TextItem {
            id: v.id.clone(),
            content: "admitted".into(),
        })
        .collect();
    owned.columns.clear();
    owned.rows.clear();
    owned.validate()?;
    render_parts(&owned, fragment, items, &[], true, None)
}
/// Render a host-admitted table after host admission has proved cells/headers/IDs and budgets.
pub fn render_admitted_table(
    i: &ViewportInstance,
    rows: &[AdmittedCellRow],
    fragment: &str,
) -> Result<String, String> {
    if !i.items.is_empty() || !i.rows.is_empty() {
        return Err("admitted rows cannot be mixed with serialized fixture rows".into());
    }
    let mut owned = i.clone();
    owned.rows = rows
        .iter()
        .map(|r| TextRow {
            id: r.id.clone(),
            cells: vec!["admitted".to_owned(); i.columns.len()],
            highlighted: r.highlighted,
        })
        .collect();
    owned.validate()?;
    if rows.iter().any(|r| r.cells.len() != i.columns.len()) {
        return Err("every admitted row must have exactly one cell per column".into());
    }
    render_parts(&owned, fragment, &[], rows, true, None)
}
/// Render app-owned paging/navigation admitted by the Native host's ordinary child-content path.
pub fn render_with_admitted_navigation(
    i: &ViewportInstance,
    navigation: &AdmittedChildren,
    fragment: &str,
) -> Result<String, String> {
    i.validate()?;
    render_parts(i, fragment, &[], &[], false, Some(navigation))
}
fn render_parts(
    i: &ViewportInstance,
    fragment: &str,
    admitted_items: &[AdmittedItem],
    admitted_rows: &[AdmittedCellRow],
    admitted: bool,
    admitted_navigation: Option<&AdmittedChildren>,
) -> Result<String, String> {
    let slots = ["[[attributes]]", "[[label]]", "[[items]]", "[[navigation]]"];
    safe_fragment(fragment, &slots)?;
    let target = i.targets();
    let is_table = !i.columns.is_empty();
    let count = if is_table {
        i.rows.len()
    } else {
        i.items.len()
    };
    let w = i.window.as_ref();
    let (start, total, size, overscan, before, after) = if let Some(w) = w {
        let after = (w.total - w.start - count) * w.item_size;
        (
            w.start,
            w.total,
            w.item_size,
            w.overscan,
            w.start * w.item_size,
            after,
        )
    } else {
        (0, count, 0, 0, 0, 0)
    };
    let attrs=format!("class=\"cui-data-viewport cui-data-viewport--{} cui-data-viewport--{}{}\" id=\"{}\" role=\"region\" aria-labelledby=\"{}\" data-cui-component=\"data-viewport\" data-cui-patch-region data-cui-viewport-mode=\"{}\"{}",match i.height{Height::Compact=>"compact",Height::Standard=>"standard",Height::Viewport=>"viewport"},match i.mode{Mode::Paged=>"paged",Mode::Incremental=>"incremental",Mode::Windowed=>"windowed"},if is_table{" cui-data-viewport--table"}else{""},esc(&target.region),esc(&target.label),match i.mode{Mode::Paged=>"paged",Mode::Incremental=>"incremental",Mode::Windowed=>"windowed"},w.map(|x|format!(" data-cui-window-start=\"{}\" data-cui-window-total=\"{}\" data-cui-window-item-size=\"{}\" data-cui-window-overscan=\"{}\"{}",x.start,x.total,x.item_size,x.overscan,x.anchor.as_ref().map(|a|format!(" data-cui-window-anchor=\"{}\"",esc(a))).unwrap_or_default())).unwrap_or_default());
    let label = format!(
        "<h2 class=\"cui-visually-hidden\" id=\"{}\">{}</h2>",
        esc(&target.label),
        esc(&i.label)
    );
    let mut content = String::new();
    if is_table {
        content.push_str(&format!("<table class=\"cui-data-viewport__table\" aria-rowcount=\"{}\"><caption class=\"cui-visually-hidden\">{}</caption><thead><tr>",total+1,esc(&i.label)));
        for c in &i.columns {
            content.push_str(&format!("<th scope=\"col\">{}</th>", esc(c)));
        }
        content.push_str(&format!(
            "</tr></thead><tbody id=\"{}\">",
            esc(&target.items)
        ));
        if w.is_some() {
            content.push_str(&format!(
                "<tr aria-hidden=\"true\"><td colspan=\"{}\" style=\"block-size:{}px\"></td></tr>",
                i.columns.len(),
                before
            ));
        }
        if admitted {
            for (idx, r) in admitted_rows.iter().enumerate() {
                content.push_str(&format!("<tr class=\"cui-data-viewport__row{}\" id=\"{}\" data-cui-patch-item data-cui-viewport-item aria-rowindex=\"{}\">",if r.highlighted{" cui-data-viewport__row--highlighted"}else{""},esc(&r.id),start+idx+2));
                for (col, cell) in i.columns.iter().zip(&r.cells) {
                    let body = if w.is_some() {
                        format!(
                            "<div class=\"cui-data-viewport__cell-content\" tabindex=\"0\">{}</div>",
                            cell.as_markup()
                        )
                    } else {
                        cell.as_markup().to_owned()
                    };
                    content.push_str(&format!("<td data-label=\"{}\">{}</td>", esc(col), body));
                }
                content.push_str("</tr>");
            }
        } else {
            for (idx, r) in i.rows.iter().enumerate() {
                content.push_str(&format!(
                    "<tr class=\"cui-data-viewport__row{}\" id=\"{}\" data-cui-patch-item data-cui-viewport-item{}>",
                    if r.highlighted { " cui-data-viewport__row--highlighted" } else { "" },
                    esc(&r.id),
                    if w.is_some() {
                        format!(" aria-rowindex=\"{}\"", start + idx + 2)
                    } else {
                        String::new()
                    }
                ));
                for (col, cell) in i.columns.iter().zip(&r.cells) {
                    let text = esc(cell);
                    let body = if w.is_some() {
                        format!(
                            "<div class=\"cui-data-viewport__cell-content\" tabindex=\"0\">{text}</div>"
                        )
                    } else {
                        text
                    };
                    content.push_str(&format!("<td data-label=\"{}\">{body}</td>", esc(col)));
                }
                content.push_str("</tr>");
            }
        }
        if w.is_some() {
            content.push_str(&format!(
                "<tr aria-hidden=\"true\"><td colspan=\"{}\" style=\"block-size:{}px\"></td></tr>",
                i.columns.len(),
                after
            ));
        }
        content.push_str("</tbody></table>");
    } else {
        content.push_str(&format!("<div class=\"cui-data-viewport__items\" id=\"{}\" role=\"list\" aria-labelledby=\"{}\">",esc(&target.items),esc(&target.label)));
        if w.is_some() {
            content.push_str(&format!("<div aria-hidden=\"true\" class=\"cui-data-viewport__spacer\" style=\"block-size:{}px\"></div>",before));
        }
        if admitted {
            for (idx, item) in admitted_items.iter().enumerate() {
                content.push_str(&format!("<div class=\"cui-data-viewport__item\" id=\"{}\" role=\"listitem\" data-cui-patch-item data-cui-viewport-item{}{}>{}</div>",esc(&item.id),if w.is_some(){format!(" aria-posinset=\"{}\" aria-setsize=\"{}\"",start+idx+1,total)}else{String::new()},if w.is_some(){" tabindex=\"0\""}else{""},item.content.as_markup()));
            }
        } else {
            for (idx, item) in i.items.iter().enumerate() {
                content.push_str(&format!("<div class=\"cui-data-viewport__item\" id=\"{}\" role=\"listitem\" data-cui-patch-item data-cui-viewport-item{}{}>{}</div>",esc(&item.id),if w.is_some(){format!(" aria-posinset=\"{}\" aria-setsize=\"{}\"",start+idx+1,total)}else{String::new()},if w.is_some(){" tabindex=\"0\""}else{""},esc(&item.content)));
            }
        }
        if w.is_some() {
            content.push_str(&format!("<div aria-hidden=\"true\" class=\"cui-data-viewport__spacer\" style=\"block-size:{}px\"></div>",after));
        }
        content.push_str("</div>");
    }
    let scroller = format!(
        "<div class=\"cui-data-viewport__scroller\" id=\"{}\" tabindex=\"0\" data-cui-scroll data-cui-scroll-key=\"{}\" aria-labelledby=\"{}\"{}>{}</div>",
        esc(&target.scroll),
        esc(&target.scroll),
        esc(&target.label),
        w.map(|window| format!(
            " style=\"--cui-data-viewport-item-size:{}px\"",
            window.item_size
        ))
        .unwrap_or_default(),
        content
    );
    let nav_content = admitted_navigation
        .map(AdmittedChildren::as_markup)
        .unwrap_or(&i.pagination);
    let nav = format!(
        "<footer id=\"{}\" class=\"cui-data-viewport__navigation\">{}</footer>",
        esc(&target.navigation),
        if admitted_navigation.is_some() {
            nav_content.to_owned()
        } else {
            esc(nav_content)
        }
    );
    let body = format!(
        "{}<p class=\"cui-data-viewport__status\" data-cui-viewport-status data-cui-window-error-label=\"The visible window could not be updated.\" data-cui-navigation-error-label=\"Navigation could not be requested.\" role=\"status\" aria-live=\"polite\" hidden></p>",
        scroller
    );
    let _ = (size, overscan, admitted);
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attrs),
            ("[[label]]", &label),
            ("[[items]]", &body),
            ("[[navigation]]", &nav),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<section [[attributes]]>[[label]][[items]][[navigation]]</section>";
    fn sample() -> ViewportInstance {
        ViewportInstance {
            id: "results".into(),
            label: "Results".into(),
            height: Height::Standard,
            mode: Mode::Windowed,
            window: Some(Window {
                start: 10,
                total: 100,
                item_size: 40,
                overscan: 3,
                anchor: Some("r-10".into()),
            }),
            items: vec![
                TextItem {
                    id: "r-10".into(),
                    content: "A < B".into(),
                },
                TextItem {
                    id: "r-11".into(),
                    content: "second".into(),
                },
            ],
            columns: vec![],
            rows: vec![],
            pagination: "Next".into(),
        }
    }
    #[test]
    fn renders_escaped_text_window_positions_and_owned_targets() {
        let i = sample();
        let h = render(&i, F).unwrap();
        assert!(h.contains("aria-posinset=\"11\""));
        assert!(h.contains("aria-setsize=\"100\""));
        assert!(h.contains("A &lt; B"));
        assert_eq!(i.targets().items, "results-items");
    }
    #[test]
    fn rejects_bounds_counts_ids_cells_and_fragment_slots() {
        let mut i = sample();
        i.window.as_mut().unwrap().start = 99;
        assert!(i.validate().is_err());
        let mut i = sample();
        i.items.push(i.items[0].clone());
        assert!(i.validate().is_err());
        let mut empty = sample();
        empty.items.clear();
        assert!(
            empty.validate().is_err(),
            "empty results use the app-owned EmptyState boundary, not fabricated rows"
        );
        assert!(render(&sample(), "[[attributes]]").is_err());
    }
    #[test]
    fn host_markup_is_explicit_and_not_serialized() {
        let mut i = sample();
        i.items.clear();
        i.window = None;
        i.mode = Mode::Paged;
        i.pagination = "Next".into();
        let item = AdmittedItem {
            id: "rich-1".into(),
            content: AdmittedChildren::from_host_admitted("<strong>Rich</strong>"),
        };
        let h = render_admitted_list(&i, &[item], F).unwrap();
        assert!(h.contains("<strong>Rich</strong>"));
        let nav = AdmittedChildren::from_host_admitted(
            "<nav><a href=\"/next\" data-cui-viewport-navigation>Next</a></nav>",
        );
        i.items.push(TextItem {
            id: "plain-1".into(),
            content: "Plain".into(),
        });
        assert!(render_with_admitted_navigation(&i, &nav, F)
            .unwrap()
            .contains("href=\"/next\" data-cui-viewport-navigation"));

        i.items.clear();
        i.columns = vec!["Name".into(), "Status".into()];
        let row = AdmittedCellRow {
            id: "rich-row".into(),
            cells: vec![
                AdmittedChildren::from_host_admitted("<strong>Acme</strong>"),
                AdmittedChildren::from_host_admitted("Active"),
            ],
            highlighted: true,
        };
        let h = render_admitted_table(&i, &[row], F).unwrap();
        assert!(h.contains("<strong>Acme</strong>"));
        assert!(h.contains("cui-data-viewport__row--highlighted"));
    }
}
