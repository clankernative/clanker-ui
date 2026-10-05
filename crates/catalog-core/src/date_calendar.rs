//! Deterministic localized 42-cell Gregorian calendar contract.
use crate::date::Date;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectionMode {
    #[default]
    Single,
    Range,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Selection {
    None,
    Single { date: Date },
    Range { start: Date, end: Option<Date> },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarInstance {
    pub id: String,
    pub month: Date,
    pub today: Date,
    pub label: String,
    pub month_labels: Vec<String>,
    pub weekday_short_labels: Vec<String>,
    pub weekday_full_labels: Vec<String>,
    pub previous_label: String,
    pub next_label: String,
    pub first_day: u8,
    #[serde(default)]
    pub minimum: Option<Date>,
    #[serde(default)]
    pub maximum: Option<Date>,
    #[serde(default)]
    pub unavailable_dates: Vec<Date>,
    #[serde(default)]
    pub selection_mode: SelectionMode,
    #[serde(default = "no_selection")]
    pub selection: Selection,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub disabled: bool,
}
fn no_selection() -> Selection {
    Selection::None
}
fn safe_token(s: &str) -> bool {
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
impl CalendarInstance {
    pub fn validate(&self) -> Result<(), String> {
        if !safe_token(&self.id)
            || !text(&self.label)
            || !text(&self.previous_label)
            || !text(&self.next_label)
        {
            return Err("calendar id and labels must be safe nonblank text".into());
        }
        if self.month.day() != 1 {
            return Err("calendar month must be its first day".into());
        }
        if self.first_day > 6 {
            return Err("firstDay must be between 0 and 6".into());
        }
        if self.month_labels.len() != 12
            || self.weekday_short_labels.len() != 7
            || self.weekday_full_labels.len() != 7
            || self
                .month_labels
                .iter()
                .chain(&self.weekday_short_labels)
                .chain(&self.weekday_full_labels)
                .any(|s| !text(s))
        {
            return Err(
                "calendar requires 12 month labels and seven short/full weekday labels".into(),
            );
        }
        if self.minimum.zip(self.maximum).is_some_and(|(a, b)| a > b) {
            return Err("minimum must not exceed maximum".into());
        }
        if let Selection::Range {
            start,
            end: Some(end),
        } = self.selection
        {
            if end < start {
                return Err("range end must be on or after start".into());
            }
        }
        let selected_dates: Vec<Date> = match self.selection {
            Selection::None => Vec::new(),
            Selection::Single { date } => vec![date],
            Selection::Range { start, end } => std::iter::once(start).chain(end).collect(),
        };
        if selected_dates.iter().any(|date| {
            self.minimum.is_some_and(|minimum| *date < minimum)
                || self.maximum.is_some_and(|maximum| *date > maximum)
                || self.unavailable_dates.contains(date)
        }) {
            return Err("selected date violates calendar constraints".into());
        }
        match (&self.selection_mode, &self.selection) {
            (SelectionMode::Single, Selection::Range { .. })
            | (SelectionMode::Range, Selection::Single { .. }) => {
                return Err("selection must match selectionMode".into());
            }
            _ => (),
        }
        if self.unavailable_dates.iter().collect::<BTreeSet<_>>().len()
            != self.unavailable_dates.len()
        {
            return Err("unavailableDates must be unique".into());
        }
        Ok(())
    }
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::from([self.id.clone(), format!("{}-heading", self.id)])
    }
    /// Text controls are the context-free fallback; a host can supply its
    /// exact locked icon map through render_with_icons.
    pub fn render(&self, fragment: &str) -> Result<String, String> {
        self.render_controls(fragment, &esc(&self.previous_label), &esc(&self.next_label))
    }
    pub fn render_with_icons(
        &self,
        fragment: &str,
        icons: &std::collections::BTreeMap<String, String>,
    ) -> Result<String, String> {
        let arrow = |name: &str| {
            crate::icon::render(
                &crate::icon::IconInstance {
                    name: name.into(),
                    size: crate::icon::IconSize::Small,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )
        };
        self.render_controls(fragment, &arrow("chevron-left")?, &arrow("chevron-right")?)
    }
    fn render_controls(
        &self,
        fragment: &str,
        previous: &str,
        next: &str,
    ) -> Result<String, String> {
        self.validate()?;
        if fragment.matches("[[calendar]]").count() != 1 || fragment.matches("[[").count() != 1 {
            return Err("calendar fragment requires exactly one [[calendar]] slot".into());
        }
        let y = self.month.year();
        let m = self.month.month();
        let offset = (self.month.weekday() as i32 - self.first_day as i32 + 7) % 7;
        let mut h = format!(
            "<section class=\"cui-date-calendar{}{}\" id=\"{}\" data-cui-component=\"date-calendar\" data-cui-date-calendar data-cui-date-interactive=\"{}\" data-cui-date-disabled=\"{}\" data-cui-date-selection-mode=\"{}\" data-cui-date-selection-start=\"{}\" data-cui-date-selection-end=\"{}\" data-cui-date-month=\"{}\" data-cui-date-first-day=\"{}\" data-cui-date-today=\"{}\" data-cui-date-month-labels=\"{}\" data-cui-date-weekday-short=\"{}\" data-cui-date-weekday-full=\"{}\"{}{} role=\"group\" aria-label=\"{}\"><header class=\"cui-date-calendar__header\">",
            if self.interactive {
                " cui-date-calendar--interactive"
            } else {
                ""
            },
            if self.disabled {
                " cui-date-calendar--disabled"
            } else {
                ""
            },
            esc(&self.id),
            self.interactive,
            self.disabled,
            if self.selection_mode == SelectionMode::Range {
                "range"
            } else {
                "single"
            },
            selection_start(&self.selection),
            selection_end(&self.selection),
            self.month,
            self.first_day,
            self.today,
            esc(&serde_json::to_string(&self.month_labels).map_err(|error| error.to_string())?),
            esc(&serde_json::to_string(&self.weekday_short_labels)
                .map_err(|error| error.to_string())?),
            esc(&serde_json::to_string(&self.weekday_full_labels)
                .map_err(|error| error.to_string())?),
            self.minimum
                .map(|date| format!(" data-cui-date-minimum=\"{date}\""))
                .unwrap_or_default(),
            self.maximum
                .map(|date| format!(" data-cui-date-maximum=\"{date}\""))
                .unwrap_or_default(),
            esc(&self.label)
        );
        let (pl, nl) = (&self.previous_label, &self.next_label);
        if self.interactive {
            h.push_str(&format!(
                "<button type=\"button\" data-cui-date-previous aria-label=\"{}\"{}>{previous}</button>",
                esc(pl),
                if self.disabled || self.month.add_months(-1).is_none() { " disabled" } else { "" }
            ))
        } else {
            h.push_str("<span aria-hidden=\"true\"></span>")
        }
        h.push_str(&format!(
            "<h2 id=\"{}-heading\" data-cui-date-month-label aria-live=\"polite\">{} {}</h2>",
            esc(&self.id),
            esc(&self.month_labels[(m - 1) as usize]),
            y
        ));
        if self.interactive {
            h.push_str(&format!(
                "<button type=\"button\" data-cui-date-next aria-label=\"{}\"{}>{next}</button>",
                esc(nl),
                if self.disabled || self.month.add_months(1).is_none() {
                    " disabled"
                } else {
                    ""
                }
            ))
        } else {
            h.push_str("<span aria-hidden=\"true\"></span>")
        }
        h.push_str(&format!("</header><table class=\"cui-date-calendar__grid\" role=\"grid\" aria-labelledby=\"{}-heading\"><thead><tr>",esc(&self.id)));
        for i in 0..7 {
            let idx = (self.first_day as usize + i) % 7;
            h.push_str(&format!(
                "<th scope=\"col\"><abbr title=\"{}\">{}</abbr></th>",
                esc(&self.weekday_full_labels[idx]),
                esc(&self.weekday_short_labels[idx])
            ))
        }
        h.push_str("</tr></thead><tbody data-cui-date-grid-body>");
        let cells = (0..42)
            .filter_map(|index| self.month.add_days(index - offset))
            .collect::<Vec<_>>();
        let selected = match self.selection {
            Selection::Single { date } => Some(date),
            Selection::Range { start, .. } => Some(start),
            Selection::None => None,
        };
        let enabled = |date: &Date| cells.contains(date) && !self.unavailable(*date);
        let focus = selected
            .filter(enabled)
            .or_else(|| Some(self.today).filter(enabled))
            .or_else(|| {
                cells
                    .iter()
                    .copied()
                    .find(|date| date.month() == m && !self.unavailable(*date))
            })
            .or_else(|| cells.iter().copied().find(|date| !self.unavailable(*date)));
        for w in 0..6 {
            h.push_str("<tr>");
            for d in 0..7 {
                let Some(date) = self.month.add_days(w * 7 + d - offset) else {
                    h.push_str("<td class=\"cui-date-calendar__cell\" aria-hidden=\"true\"></td>");
                    continue;
                };
                let out = date.year() != y || date.month() != m;
                let (rs, re, sel, range) = match self.selection {
                    Selection::None => (false, false, false, false),
                    Selection::Single { date: s } => (false, false, date == s, false),
                    Selection::Range {
                        start: s,
                        end: None,
                    } => (date == s, false, date == s, false),
                    Selection::Range {
                        start: s,
                        end: Some(e),
                    } => (
                        date == s,
                        date == e,
                        date == s || date == e,
                        date >= s && date <= e,
                    ),
                };
                let unavailable = self.unavailable(date);
                let cls = format!(
                    "cui-date-calendar__cell{}{}{}{}{}{}{}",
                    if out {
                        " cui-date-calendar__cell--outside"
                    } else {
                        ""
                    },
                    if date == self.today {
                        " cui-date-calendar__cell--today"
                    } else {
                        ""
                    },
                    if sel {
                        " cui-date-calendar__cell--selected"
                    } else {
                        ""
                    },
                    if rs {
                        " cui-date-calendar__cell--range-start"
                    } else {
                        ""
                    },
                    if re {
                        " cui-date-calendar__cell--range-end"
                    } else {
                        ""
                    },
                    if range {
                        " cui-date-calendar__cell--in-range"
                    } else {
                        ""
                    },
                    if unavailable {
                        " cui-date-calendar__cell--unavailable"
                    } else {
                        ""
                    }
                );
                h.push_str(&format!(
                    "<td class=\"{}\" role=\"gridcell\" aria-selected=\"{}\"{}>",
                    cls,
                    range || sel,
                    if date == self.today {
                        " aria-current=\"date\""
                    } else {
                        ""
                    }
                ));
                if self.interactive {
                    h.push_str(&format!("<button class=\"cui-date-calendar__day\" type=\"button\" data-cui-date-day data-cui-date-value=\"{}\" aria-label=\"{} {} {}, {}\" tabindex=\"{}\"{}>{}</button>",date, date.day(),esc(&self.month_labels[(date.month()-1) as usize]),date.year(),esc(&self.label),if Some(date)==focus&&!unavailable{"0"}else{"-1"},if unavailable{" disabled"}else{""},date.day()))
                } else {
                    h.push_str(&format!(
                        "<span class=\"cui-date-calendar__day\" aria-label=\"{} {} {}\">{}</span>",
                        date.day(),
                        esc(&self.month_labels[(date.month() - 1) as usize]),
                        date.year(),
                        date.day()
                    ))
                }
                h.push_str("</td>")
            }
            h.push_str("</tr>")
        }
        h.push_str("</tbody></table><div hidden>");
        for date in &self.unavailable_dates {
            h.push_str(&format!(
                "<data data-cui-date-unavailable value=\"{date}\"></data>"
            ));
        }
        h.push_str("</div></section>");
        crate::fragment::fill(fragment, &[("[[calendar]]", &h)])
    }
    fn unavailable(&self, d: Date) -> bool {
        self.disabled
            || self.minimum.is_some_and(|x| d < x)
            || self.maximum.is_some_and(|x| d > x)
            || self.unavailable_dates.contains(&d)
    }
}
fn selection_start(s: &Selection) -> String {
    match s {
        Selection::Single { date } => date.iso(),
        Selection::Range { start, .. } => start.iso(),
        Selection::None => String::new(),
    }
}
fn selection_end(s: &Selection) -> String {
    match s {
        Selection::Range {
            end: Some(date), ..
        } => date.iso(),
        _ => String::new(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_dates_must_respect_bounds_and_unavailable_dates() {
        let json = r#"{"id":"x","month":"2024-02-01","today":"2024-02-14","label":"Calendar","monthLabels":["January","February","March","April","May","June","July","August","September","October","November","December"],"weekdayShortLabels":["Su","Mo","Tu","We","Th","Fr","Sa"],"weekdayFullLabels":["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],"previousLabel":"Previous","nextLabel":"Next","firstDay":0,"selectionMode":"single","selection":{"kind":"single","date":"2024-02-10"},"interactive":true}"#;
        let mut calendar: CalendarInstance = serde_json::from_str(json).unwrap();
        calendar.minimum = Some(Date::parse("2024-02-11").unwrap());
        assert!(calendar.validate().is_err());
        calendar.minimum = None;
        calendar.unavailable_dates = vec![Date::parse("2024-02-10").unwrap()];
        assert!(calendar.validate().is_err());
    }

    #[test]
    fn strict_and_range_order() {
        let j = r#"{"id":"x","month":"2024-02-01","today":"2024-02-29","label":"Calendar","monthLabels":["January","February","March","April","May","June","July","August","September","October","November","December"],"weekdayShortLabels":["Su","Mo","Tu","We","Th","Fr","Sa"],"weekdayFullLabels":["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],"previousLabel":"Previous","nextLabel":"Next","firstDay":0,"selectionMode":"range","selection":{"kind":"range","start":"2024-02-10","end":"2024-02-29"},"interactive":true}"#;
        let c: CalendarInstance = serde_json::from_str(j).unwrap();
        let html = c.render("[[calendar]]").unwrap();
        assert_eq!(html.matches("data-cui-date-day").count(), 42);
        assert!(html.contains("data-cui-date-value=\"2024-02-29\""));
        assert!(html.contains("--range-end"));
        let mut unavailable = c.clone();
        unavailable.selection = Selection::None;
        unavailable.unavailable_dates = vec![Date::parse("2024-02-22").unwrap()];
        assert!(unavailable
            .render("[[calendar]]")
            .unwrap()
            .contains("data-cui-date-unavailable value=\"2024-02-22\""));
        assert!(serde_json::from_str::<CalendarInstance>(
            &j.replace("\"interactive\":true", "\"interactive\":true,\"oops\":1")
        )
        .is_err());
        let mut c = c;
        c.selection = Selection::Range {
            start: Date::parse("2024-02-20").unwrap(),
            end: Some(Date::parse("2024-02-10").unwrap()),
        };
        assert!(c.validate().is_err());
        let mut edge = c;
        edge.month = Date::parse("0001-01-01").unwrap();
        edge.today = edge.month;
        edge.selection = Selection::None;
        let html = edge.render("[[calendar]]").unwrap();
        assert_eq!(html.matches("<td ").count(), 42);
        assert!(html.contains("aria-hidden=\"true\"></td>"));
        edge.month = Date::parse("9999-12-01").unwrap();
        assert_eq!(
            edge.render("[[calendar]]").unwrap().matches("<td ").count(),
            42
        );
    }
}
