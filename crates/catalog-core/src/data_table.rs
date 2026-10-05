//! Text-only tables and an explicit trust boundary for host-composed table bodies.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataTableInstance {
    pub caption: String,
    #[serde(default)]
    pub caption_visible: bool,
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<TextRow>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextRow {
    #[serde(default)]
    pub id: Option<String>,
    pub cells: Vec<String>,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
}
fn valid_text(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
impl DataTableInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !valid_text(&self.caption) {
            return Err("table caption must be nonblank plain text".into());
        }
        if !(1..=16).contains(&self.columns.len()) || self.columns.iter().any(|s| !valid_text(s)) {
            return Err("data table requires 1..=16 nonblank column labels".into());
        }
        if self.rows.len() > 100 {
            return Err("data table supports at most 100 text rows".into());
        }
        let mut ids = BTreeSet::new();
        for row in &self.rows {
            if row.cells.len() != self.columns.len()
                || row.cells.iter().any(|s| s.chars().any(char::is_control))
            {
                return Err("each row must have one plain-text cell per column".into());
            }
            if let Some(id) = &row.id {
                if !token(id) || !ids.insert(id) {
                    return Err("row ids must be unique safe tokens".into());
                }
            }
        }
        Ok(())
    }
}
fn attrs(i: &DataTableInstance) -> String {
    format!(
        "class=\"cui-data-table\" data-cui-component=\"data-table\" tabindex=\"0\" role=\"region\" aria-label=\"{}\"",
        esc(&i.caption)
    )
}
fn caption(i: &DataTableInstance) -> String {
    format!(
        "<caption class=\"cui-data-table__caption{}\">{}</caption>",
        if i.caption_visible {
            " cui-data-table__caption--visible"
        } else {
            ""
        },
        esc(&i.caption)
    )
}
fn head(i: &DataTableInstance) -> String {
    format!(
        "<thead class=\"cui-data-table__head\"><tr>{}</tr></thead>",
        i.columns
            .iter()
            .map(|c| format!("<th scope=\"col\">{}</th>", esc(c)))
            .collect::<String>()
    )
}
fn body(i: &DataTableInstance) -> String {
    let mut out = String::from("<tbody class=\"cui-data-table__body\">");
    for row in &i.rows {
        out.push_str("<tr");
        if let Some(id) = &row.id {
            out.push_str(&format!(" id=\"{}\"", esc(id)));
        }
        out.push('>');
        for (idx, cell) in row.cells.iter().enumerate() {
            out.push_str(&format!(
                "<td data-label=\"{}\">{}</td>",
                esc(&i.columns[idx]),
                esc(cell)
            ));
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody>");
    out
}
fn render_parts(i: &DataTableInstance, fragment: &str, b: &str) -> Result<String, String> {
    i.validate()?;
    let a = attrs(i);
    let c = caption(i);
    let h = head(i);
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &a),
            ("[[caption]]", &c),
            ("[[head]]", &h),
            ("[[body]]", b),
        ],
    )
}
/// Render validated text fixture rows with HTML escaping.
pub fn render(i: &DataTableInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let b = body(i);
    render_parts(i, fragment, &b)
}
/// Render a host-composed `<tbody>` only after the host has proved row/cell structure,
/// column-label association, budgets, and stable row IDs. This marker does not sanitize HTML.
pub fn render_with_admitted_body(
    i: &DataTableInstance,
    fragment: &str,
    body: AdmittedChildren,
) -> Result<String, String> {
    if !i.rows.is_empty() {
        return Err("admitted body cannot be combined with text rows".into());
    }
    render_parts(i, fragment, body.as_markup())
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<div [[attributes]]><table class=\"cui-data-table__table\">[[caption]][[head]][[body]]</table></div>";
    fn i() -> DataTableInstance {
        DataTableInstance {
            caption: "Accounts <&>".into(),
            caption_visible: true,
            columns: vec!["Name".into(), "Status".into()],
            rows: vec![TextRow {
                id: Some("row-1".into()),
                cells: vec!["A <B>".into(), "Active & ready".into()],
            }],
        }
    }
    #[test]
    fn renders_responsive_accessible_table_and_escaped_text() {
        let h = render(&i(), F).unwrap();
        assert!(h.contains("role=\"region\" aria-label=\"Accounts &lt;&amp;&gt;\""));
        assert!(h.contains("<th scope=\"col\">Name</th>"));
        assert!(h.contains("id=\"row-1\""));
        assert!(h.contains("A &lt;B&gt;"));
        assert!(h.contains("data-label=\"Status\""));
    }
    #[test]
    fn admits_host_body_without_claiming_sanitization() {
        let body = AdmittedChildren::from_host_admitted(
            "<tbody><tr id=\"stable\"><td><a href=\"/x\">Link</a></td><td>Rich</td></tr></tbody>",
        );
        let mut instance = i();
        instance.rows.clear();
        let h = render_with_admitted_body(&instance, F, body).unwrap();
        assert!(h.contains("<a href=\"/x\">Link</a>"));
    }
    #[test]
    fn rejects_counts_shape_ids_and_bad_slots() {
        let mut x = i();
        x.rows[0].cells.pop();
        assert!(x.validate().is_err());
        x = i();
        x.rows.push(TextRow {
            id: Some("row-1".into()),
            cells: vec!["x".into(), "y".into()],
        });
        assert!(x.validate().is_err());
        assert!(render(&i(), "[[attributes]][[caption]]").is_err());
        x = i();
        x.rows[0].cells.push("unexpected third cell".into());
        assert!(render(&x, F).is_err());
        assert!(render_with_admitted_body(
            &i(),
            F,
            AdmittedChildren::from_host_admitted("<tbody></tbody>")
        )
        .is_err());
    }
    #[test]
    fn allows_empty_body_and_hidden_caption() {
        let mut x = i();
        x.rows.clear();
        x.caption_visible = false;
        let h = render(&x, F).unwrap();
        assert!(h.contains("<tbody class=\"cui-data-table__body\"></tbody>"));
        assert!(h.contains("cui-data-table__caption\""));
    }
}
