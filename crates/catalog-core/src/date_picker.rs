//! Native date-input composition around the bounded date calendar.
use crate::{
    date::Date,
    date_calendar::{CalendarInstance, Selection, SelectionMode},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Presentation {
    #[default]
    Inline,
    Dropdown,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum Value {
    Single {
        #[serde(default)]
        date: Option<Date>,
    },
    Range {
        #[serde(default)]
        start: Option<Date>,
        #[serde(default)]
        end: Option<Date>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickerInstance {
    pub id: String,
    pub label: String,
    pub value: Value,
    pub month: Date,
    pub today: Date,
    pub month_labels: Vec<String>,
    pub weekday_short_labels: Vec<String>,
    pub weekday_full_labels: Vec<String>,
    pub first_day: u8,
    pub previous_label: String,
    pub next_label: String,
    pub start_label: String,
    pub end_label: String,
    pub calendar_label: String,
    pub open_calendar_label: String,
    pub minimum: Option<Date>,
    pub maximum: Option<Date>,
    #[serde(default)]
    pub unavailable_dates: Vec<Date>,
    #[serde(default)]
    pub presentation: Presentation,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
    pub name: Option<String>,
    pub start_name: Option<String>,
    pub end_name: Option<String>,
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        && !s.starts_with('_')
}
fn txt(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn e(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl PickerInstance {
    fn control_ids(&self) -> (String, String, String) {
        (
            format!("{}-start", self.id),
            format!("{}-end", self.id),
            format!("{}-calendar", self.id),
        )
    }
    pub fn derived_ids(&self) -> std::collections::BTreeSet<String> {
        let (start, end, calendar) = self.control_ids();
        let mut ids = std::collections::BTreeSet::from([
            self.id.clone(),
            format!("{}-label", self.id),
            start,
            calendar.clone(),
            format!("{calendar}-heading"),
        ]);
        if matches!(self.value, Value::Range { .. }) {
            ids.insert(end);
        }
        if self.presentation == Presentation::Dropdown {
            ids.insert(format!("{}-panel", self.id));
        }
        if self.hint.is_some() || self.error.is_some() {
            ids.insert(format!("{}-description", self.id));
        }
        ids
    }
    pub fn validate(&self) -> Result<(), String> {
        if !token(&self.id)
            || !txt(&self.label)
            || !txt(&self.start_label)
            || !txt(&self.end_label)
            || !txt(&self.calendar_label)
            || !txt(&self.open_calendar_label)
            || !txt(&self.previous_label)
            || !txt(&self.next_label)
        {
            return Err("picker ids and labels must be safe and nonblank".into());
        }
        if self.month.day() != 1
            || self.first_day > 6
            || self.month_labels.len() != 12
            || self.weekday_short_labels.len() != 7
            || self.weekday_full_labels.len() != 7
            || self
                .month_labels
                .iter()
                .chain(&self.weekday_short_labels)
                .chain(&self.weekday_full_labels)
                .any(|x| !txt(x))
        {
            return Err(
                "picker requires first-of-month, valid firstDay, and explicit localized labels"
                    .into(),
            );
        }
        if self.minimum.zip(self.maximum).is_some_and(|(a, b)| a > b) {
            return Err("minimum must not exceed maximum".into());
        }
        if self
            .unavailable_dates
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.unavailable_dates.len()
        {
            return Err("unavailableDates must be unique".into());
        }
        if self.hint.as_deref().is_some_and(|x| !txt(x))
            || self.error.as_deref().is_some_and(|x| !txt(x))
        {
            return Err("hint and error must be nonblank plain text".into());
        }
        match &self.value {
            Value::Single { date } => {
                if self.name.as_deref().is_none_or(|n| !token(n)) {
                    return Err("single date requires a safe name".into());
                }
                if self.start_name.is_some() || self.end_name.is_some() {
                    return Err("single date does not accept range names".into());
                }
                if date.is_some_and(|d| {
                    self.minimum.is_some_and(|x| d < x)
                        || self.maximum.is_some_and(|x| d > x)
                        || self.unavailable_dates.contains(&d)
                }) {
                    return Err("selected date violates picker constraints".into());
                }
            }
            Value::Range { start, end } => {
                if start.is_none() && end.is_some() {
                    return Err("range end requires a start date".into());
                }
                if self.start_name.as_deref().is_none_or(|n| !token(n))
                    || self.end_name.as_deref().is_none_or(|n| !token(n))
                    || self.name.is_some()
                {
                    return Err("range date requires separate safe startName/endName".into());
                }
                if start.zip(*end).is_some_and(|(a, b)| b < a) {
                    return Err("range end must not precede start".into());
                }
                for d in [*start, *end].into_iter().flatten() {
                    if self.minimum.is_some_and(|x| d < x)
                        || self.maximum.is_some_and(|x| d > x)
                        || self.unavailable_dates.contains(&d)
                    {
                        return Err("selected date violates picker constraints".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub fn render(&self, fragment: &str, calendar_fragment: &str) -> Result<String, String> {
        self.render_inner(fragment, calendar_fragment, None)
    }
    pub fn render_with_icons(
        &self,
        fragment: &str,
        calendar_fragment: &str,
        icons: &std::collections::BTreeMap<String, String>,
    ) -> Result<String, String> {
        self.render_inner(fragment, calendar_fragment, Some(icons))
    }
    fn render_inner(
        &self,
        fragment: &str,
        calendar_fragment: &str,
        icons: Option<&std::collections::BTreeMap<String, String>>,
    ) -> Result<String, String> {
        self.validate()?;
        if fragment.matches("[[picker]]").count() != 1 || fragment.matches("[[").count() != 1 {
            return Err("picker fragment requires exactly one [[picker]] slot".into());
        }
        let (start_id, end_id, cal_id) = self.control_ids();
        let (mode, start, end, start_name, end_name) = match &self.value {
            Value::Single { date } => (
                SelectionMode::Single,
                *date,
                None,
                self.name.clone().unwrap_or_default(),
                String::new(),
            ),
            Value::Range { start, end } => (
                SelectionMode::Range,
                *start,
                *end,
                self.start_name.clone().unwrap_or_default(),
                self.end_name.clone().unwrap_or_default(),
            ),
        };
        let selection = match mode {
            SelectionMode::Single => start
                .map(|date| Selection::Single { date })
                .unwrap_or(Selection::None),
            SelectionMode::Range => start
                .map(|start| Selection::Range { start, end })
                .unwrap_or(Selection::None),
        };
        let c = CalendarInstance {
            id: cal_id.clone(),
            month: self.month,
            today: self.today,
            label: self.calendar_label.clone(),
            month_labels: self.month_labels.clone(),
            weekday_short_labels: self.weekday_short_labels.clone(),
            weekday_full_labels: self.weekday_full_labels.clone(),
            previous_label: self.previous_label.clone(),
            next_label: self.next_label.clone(),
            first_day: self.first_day,
            minimum: self.minimum,
            maximum: self.maximum,
            unavailable_dates: self.unavailable_dates.clone(),
            selection_mode: mode,
            selection,
            interactive: true,
            disabled: self.disabled,
        };
        let calendar = match icons {
            Some(icons) => c.render_with_icons(calendar_fragment, icons)?,
            None => c.render(calendar_fragment)?,
        };
        let mut h = format!(
            "<section class=\"cui-date-picker cui-date-picker--{}{}\" id=\"{}\" data-cui-component=\"date-picker\" data-cui-date-picker data-cui-date-mode=\"{}\" data-cui-date-presentation=\"{}\" aria-labelledby=\"{}-label\"><label class=\"cui-date-picker__label\" id=\"{}-label\" for=\"{}\">{}{}</label>",
            if self.presentation == Presentation::Inline {
                "inline"
            } else {
                "dropdown"
            },
            if self.error.is_some() {
                " cui-date-picker--invalid"
            } else {
                ""
            },
            e(&self.id),
            if mode == SelectionMode::Range {
                "range"
            } else {
                "single"
            },
            if self.presentation == Presentation::Inline {
                "inline"
            } else {
                "dropdown"
            },
            e(&self.id),
            e(&self.id),
            e(&start_id),
            e(&self.label),
            if self.required {
                " <span aria-hidden=\"true\">*</span>"
            } else {
                ""
            },
        );
        let descr = if self.hint.is_some() || self.error.is_some() {
            format!("{}-description", e(&self.id))
        } else {
            String::new()
        };
        for (id, name, label, value) in [
            (
                start_id,
                start_name,
                self.start_label.clone(),
                start.map(Date::iso).unwrap_or_default(),
            ),
            (
                end_id,
                end_name,
                self.end_label.clone(),
                end.map(Date::iso).unwrap_or_default(),
            ),
        ]
        .into_iter()
        .take(if mode == SelectionMode::Single { 1 } else { 2 })
        {
            if mode == SelectionMode::Range {
                h.push_str(&format!(
                    "<label class=\"cui-date-picker__range-field\" for=\"{}\">{}",
                    e(&id),
                    e(&label)
                ))
            }
            h.push_str(&format!("<input class=\"cui-date-picker__input\" id=\"{}\" name=\"{}\" type=\"date\" value=\"{}\" data-cui-date-{}{}{}{}{}{}{}>",e(&id),e(&name),e(&value),if id.ends_with("-end"){"end"}else{"start"},self.minimum.map(|d|format!(" min=\"{d}\"")).unwrap_or_default(),self.maximum.map(|d|format!(" max=\"{d}\"")).unwrap_or_default(),if descr.is_empty(){String::new()}else{format!(" aria-describedby=\"{}\"",descr)},if self.required{" required"}else{""},if self.disabled{" disabled"}else{""},if self.error.is_some(){" aria-invalid=\"true\""}else{""}));
            if mode == SelectionMode::Range {
                h.push_str("</label>")
            }
        }
        if let Some(hint) = &self.hint {
            let _ = hint;
        }
        if self.presentation == Presentation::Dropdown {
            h.push_str(&format!("<details class=\"cui-date-picker__dropdown\" data-cui-date-dropdown><summary class=\"cui-date-picker__trigger\" aria-controls=\"{}-panel\" aria-label=\"{}\">{}</summary><div class=\"cui-date-picker__panel\" id=\"{}-panel\" data-cui-date-panel>{}</div></details>",e(&self.id),e(&self.open_calendar_label),e(&self.open_calendar_label),e(&self.id),calendar))
        } else {
            h.push_str(&format!(
                "<div class=\"cui-date-picker__calendar\">{}</div>",
                calendar
            ))
        }
        if !descr.is_empty() {
            h.push_str(&format!(
                "<div class=\"cui-date-picker__description\" id=\"{}\">",
                descr
            ));
            if let Some(error) = &self.error {
                h.push_str(&format!(
                    "<p class=\"cui-date-picker__error\">{}</p>",
                    e(error)
                ))
            }
            if let Some(hint) = &self.hint {
                h.push_str(&format!(
                    "<p class=\"cui-date-picker__hint\">{}</p>",
                    e(hint)
                ))
            }
            h.push_str("</div>")
        }
        h.push_str("</section>");
        crate::fragment::fill(fragment, &[("[[picker]]", &h)])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outputs_native_single_and_distinct_range_inputs() {
        let json = r#"{"id":"booking","label":"Dates","value":{"mode":"range","start":"2024-02-10","end":null},"month":"2024-02-01","today":"2024-02-12","monthLabels":["January","February","March","April","May","June","July","August","September","October","November","December"],"weekdayShortLabels":["Su","Mo","Tu","We","Th","Fr","Sa"],"weekdayFullLabels":["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],"firstDay":0,"previousLabel":"Previous","nextLabel":"Next","startLabel":"From","endLabel":"To","calendarLabel":"Calendar","openCalendarLabel":"Open","minimum":null,"maximum":null,"name":null,"startName":"from","endName":"to"}"#;
        let p: PickerInstance = serde_json::from_str(json).unwrap();
        let html = p.render("[[picker]]", "[[calendar]]").unwrap();
        assert!(html.contains("name=\"from\" type=\"date\""));
        assert!(html.contains("name=\"to\" type=\"date\""));
        assert!(html.contains("data-cui-date-selection-end=\"\""));
        assert!(p.derived_ids().contains("booking-start"));
        assert!(serde_json::from_str::<PickerInstance>(
            &json.replace("\"name\":null", "\"name\":null,\"oops\":true")
        )
        .is_err());
        let end_without_start = json
            .replace("\"start\":\"2024-02-10\"", "\"start\":null")
            .replace("\"end\":null", "\"end\":\"2024-02-12\"");
        assert!(serde_json::from_str::<PickerInstance>(&end_without_start)
            .unwrap()
            .validate()
            .is_err());
        let duplicate_unavailable = json.replace(
            "\"name\":null",
            "\"unavailableDates\":[\"2024-02-12\",\"2024-02-12\"],\"name\":null",
        );
        assert!(
            serde_json::from_str::<PickerInstance>(&duplicate_unavailable)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
