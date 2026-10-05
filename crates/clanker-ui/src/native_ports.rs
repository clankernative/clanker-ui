use catalog_core::layout::AdmittedChildren;
use catalog_core::{
    activity_feed, button_group, checkbox_group, command_menu, confirm_dialog, copy_field,
    data_viewport, date_calendar, date_picker, definition_list, disclosure, drawer, file_upload,
    modal, popover, progress_steps, radio_group, segmented_control, tabs, theme_switcher, toast,
    toggle, tooltip,
};
use serde_json::Value;
use std::collections::BTreeMap;

/// Render a self-authored, strict JSON fixture for the eleven newly ported contracts.
/// Fixture `children` are explicit host-admitted samples; they are never parsed as HTML options.
pub(super) fn render(
    name: &str,
    value: &Value,
    template: &str,
    icons: &BTreeMap<String, String>,
    icon_fragment: &str,
) -> Result<(String, Vec<String>), String> {
    render_with_dependencies(name, value, template, icons, icon_fragment, None)
}

/// Dependency fragments must come from the same captured package as the fixture.
pub(super) fn render_with_dependencies(
    name: &str,
    value: &Value,
    template: &str,
    icons: &BTreeMap<String, String>,
    icon_fragment: &str,
    calendar_template: Option<&str>,
) -> Result<(String, Vec<String>), String> {
    let expected = value.get("expectedHtml").and_then(Value::as_str);
    let mut options = value.get("options").unwrap_or(value).clone();
    if value.get("options").is_none() {
        if let Some(object) = options.as_object_mut() {
            object.remove("expectedHtml");
            object.remove("children");
        }
    }
    let json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    let children = value.get("children");
    let (html, variants) = match name {
        "activity-feed" => {
            let i = activity_feed::ActivityFeedInstance::parse(&json)?;
            let v = i
                .entries
                .iter()
                .map(|entry| {
                    serde_json::to_value(entry.tone)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned()
                })
                .collect();
            (activity_feed::render(&i, template)?, v)
        }
        "definition-list" => {
            let i = definition_list::DefinitionListInstance::parse(&json)?;
            let v = vec![if i.density == definition_list::Density::Compact {
                "compact"
            } else {
                "standard"
            }
            .into()];
            (definition_list::render(&i, template)?, v)
        }
        "disclosure" => {
            let i = disclosure::DisclosureInstance::parse(&json)?;
            let body = child(children, "body")?;
            let v = vec![if i.appearance == disclosure::Appearance::Contained {
                "contained"
            } else {
                "plain"
            }
            .into()];
            (
                disclosure::render_with_body(
                    &i,
                    &AdmittedChildren::from_host_admitted(body),
                    template,
                )?,
                v,
            )
        }
        "button-group" => {
            let i = button_group::ButtonGroupInstance::parse(&json)?;
            let values = children
                .and_then(|x| x.as_object().and_then(|o| o.get("body")).or(Some(x)))
                .and_then(Value::as_array)
                .ok_or("button-group fixture requires admitted body button array")?;
            let buttons = values
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(|s| AdmittedChildren::from_host_admitted(s.to_owned()))
                        .ok_or("button child must be string")
                })
                .collect::<Result<Vec<_>, _>>()?;
            let v = vec![if i.appearance == button_group::Appearance::Joined {
                "joined"
            } else {
                "spaced"
            }
            .into()];
            (button_group::render(&i, &buttons, template)?, v)
        }
        "checkbox-group" => {
            let i = checkbox_group::CheckboxGroupInstance::parse(&json)?;
            let mut v = vec![if i.layout == checkbox_group::Layout::Inline {
                "inline"
            } else {
                "stacked"
            }
            .into()];
            if i.disabled || i.choices.iter().any(|choice| choice.disabled) {
                v.push("disabled".into());
            }
            if i.error.is_some() {
                v.push("invalid".into());
            }
            (checkbox_group::render(&i, template)?, v)
        }
        "radio-group" => {
            let i = radio_group::RadioGroupInstance::parse(&json)?;
            let mut v = vec![if i.layout == radio_group::Layout::Inline {
                "inline"
            } else {
                "stacked"
            }
            .into()];
            if i.required {
                v.push("required".into());
            }
            if i.disabled || i.choices.iter().any(|choice| choice.disabled) {
                v.push("disabled".into());
            }
            if i.error.is_some() {
                v.push("invalid".into());
            }
            (radio_group::render(&i, template)?, v)
        }
        "toggle" => {
            let i = toggle::ToggleInstance::parse(&json)?;
            let mut v = vec![if i.checked { "checked" } else { "unchecked" }.into()];
            if i.required {
                v.push("required".into());
            }
            if i.disabled {
                v.push("disabled".into());
            }
            if i.error.is_some() {
                v.push("invalid".into());
            }
            (toggle::render(&i, template)?, v)
        }
        "tabs" => {
            let i = tabs::TabsInstance::parse(&json)?;
            (
                tabs::render(&i, template)?,
                vec!["linked".into(), "current".into()],
            )
        }
        "segmented-control" => {
            let i = segmented_control::SegmentedControlInstance::parse(&json)?;
            (
                segmented_control::render(&i, template)?,
                i.items
                    .iter()
                    .map(|item| {
                        serde_json::to_value(item.kind)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_owned()
                    })
                    .collect(),
            )
        }
        "progress-steps" => {
            let i = progress_steps::ProgressStepsInstance::parse(&json)?;
            let orientation = serde_json::to_value(i.orientation)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let appearance = serde_json::to_value(i.appearance)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let mut v = vec![orientation, appearance];
            v.extend(i.items.iter().map(|item| {
                serde_json::to_value(item.state)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            }));
            (
                progress_steps::render_with_icons(&i, template, icons, icon_fragment)?,
                v,
            )
        }
        "copy-field" => {
            let i = copy_field::CopyFieldInstance::parse(&json)?;
            (
                copy_field::render(&i, template, icons)?,
                vec!["standard".into()],
            )
        }
        "modal" => {
            let i = modal::ModalInstance::parse(json.as_bytes())?;
            let slots = admitted_slots(children, &["body", "actions"])?;
            (
                modal::render(&i, &slots, template, icons)?,
                vec![serde_json::to_value(i.size)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into()],
            )
        }
        "drawer" => {
            let i = drawer::DrawerInstance::parse(json.as_bytes())?;
            let slots = admitted_slots(children, &["body", "actions"])?;
            (
                drawer::render(&i, &slots, template, icons)?,
                vec![
                    serde_json::to_value(i.placement)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .into(),
                    serde_json::to_value(i.size)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .into(),
                ],
            )
        }
        "popover" => {
            let i = popover::PopoverInstance::parse(json.as_bytes(), icons)?;
            let body = child(children, "body")?;
            (
                popover::render(
                    &i,
                    &AdmittedChildren::from_host_admitted(body.to_owned()),
                    template,
                    icons,
                )?,
                vec![
                    serde_json::to_value(i.alignment)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .into(),
                    serde_json::to_value(i.width)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .into(),
                ],
            )
        }
        "tooltip" => {
            let i = tooltip::TooltipInstance::parse(json.as_bytes(), icons)?;
            (
                tooltip::render(&i, template, icons)?,
                vec![serde_json::to_value(i.placement)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into()],
            )
        }
        "toast" => {
            let i = toast::ToastInstance::parse(json.as_bytes(), icons)?;
            let mut variants: Vec<String> = i
                .messages
                .iter()
                .map(|m| m.tone.as_str().to_owned())
                .collect();
            variants.push(
                serde_json::to_value(i.position)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
            );
            variants.push(
                serde_json::to_value(i.history_policy)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
            );
            (toast::render(&i, template, icons)?, variants)
        }
        "theme-switcher" => {
            let i = theme_switcher::ThemeSwitcherInstance::parse(json.as_bytes(), icons)?;
            (
                theme_switcher::render(&i, template, icons)?,
                vec![serde_json::to_value(i.presentation)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into()],
            )
        }
        "date-calendar" => {
            let i: date_calendar::CalendarInstance =
                serde_json::from_slice(json.as_bytes()).map_err(|e| e.to_string())?;
            i.validate()?;
            let mut variants = vec![if i.interactive {
                "interactive"
            } else {
                "readonly"
            }
            .into()];
            if i.disabled {
                variants.push("disabled".into());
            }
            (i.render_with_icons(template, icons)?, variants)
        }
        "date-picker" => {
            let i: date_picker::PickerInstance =
                serde_json::from_slice(json.as_bytes()).map_err(|e| e.to_string())?;
            i.validate()?;
            let mode = match i.value {
                date_picker::Value::Single { .. } => "single",
                date_picker::Value::Range { .. } => "range",
            };
            let presentation = serde_json::to_value(i.presentation).unwrap();
            let variants = vec![format!("{mode}-{}", presentation.as_str().unwrap())];
            let calendar = calendar_template
                .ok_or("date-picker requires its locked date-calendar fragment")?;
            (i.render_with_icons(template, calendar, icons)?, variants)
        }
        "command-menu" => {
            let i = command_menu::CommandMenuInstance::parse(json.as_bytes(), icons)?;
            (command_menu::render(&i, template, icons)?, {
                let mut variants = vec!["grouped".into()];
                if i.shortcut.is_some() {
                    variants.push("shortcut-enabled".into());
                }
                variants
            })
        }
        "confirm-dialog" => {
            let i = confirm_dialog::ConfirmDialogInstance::parse(json.as_bytes())?;
            let variants = vec![serde_json::to_value(i.tone)
                .unwrap()
                .as_str()
                .unwrap()
                .into()];
            (confirm_dialog::render(&i, template, icons)?, variants)
        }
        "file-upload" => {
            let i = file_upload::FileUploadInstance::parse(&json)?;
            let mut variants = vec![if i.multiple { "multiple" } else { "single" }.into()];
            if i.disabled {
                variants.push("disabled".into());
            }
            if i.error.is_some() {
                variants.push("error".into());
            }
            if !i.files.is_empty() {
                variants.push("server-file-states".into());
            }
            (file_upload::render(&i, template)?, variants)
        }
        "data-viewport" => {
            let i = data_viewport::ViewportInstance::parse(&json)?;
            let mut variants = vec![
                serde_json::to_value(i.height)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
                serde_json::to_value(i.mode)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
                if i.items.is_empty() { "table" } else { "list" }.into(),
            ];
            let html = match children {
                None => data_viewport::render(&i, template)?,
                Some(value) => {
                    let object = value
                        .as_object()
                        .ok_or("viewport children must be an object")?;
                    if object.len() != 1 || !object.contains_key("navigation") {
                        return Err(
                            "viewport fixture allows only one admitted navigation child".into()
                        );
                    }
                    let nav = object["navigation"]
                        .as_str()
                        .ok_or("viewport navigation must be admitted string content")?;
                    data_viewport::render_with_admitted_navigation(
                        &i,
                        &AdmittedChildren::from_host_admitted(nav.to_owned()),
                        template,
                    )?
                }
            };
            (html, std::mem::take(&mut variants))
        }
        _ => return Err(format!("unsupported native port {name}")),
    };
    if expected.is_some_and(|expected| html != expected) {
        return Err("rendered HTML differs from expected fixture".into());
    }
    Ok((html, variants))
}
fn admitted_slots(
    children: Option<&Value>,
    allowed: &[&str],
) -> Result<BTreeMap<String, AdmittedChildren>, String> {
    let object = children
        .and_then(Value::as_object)
        .ok_or("fixture requires admitted children object")?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unsupported admitted fixture slot".into());
    }
    object
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|content| {
                    (
                        key.clone(),
                        AdmittedChildren::from_host_admitted(content.to_owned()),
                    )
                })
                .ok_or_else(|| "admitted child must be string".into())
        })
        .collect()
}

fn child<'a>(children: Option<&'a Value>, slot: &str) -> Result<&'a str, String> {
    children
        .and_then(|value| {
            value.as_object().and_then(|x| x.get(slot)).or_else(|| {
                if slot == "body" {
                    Some(value)
                } else {
                    None
                }
            })
        })
        .and_then(Value::as_str)
        .ok_or_else(|| format!("fixture requires admitted {slot} child"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check(name: &str, fixture: Value) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/vanilla/components");
        let dir = root.join(name);
        let template = std::fs::read_to_string(dir.join("fragment.html")).unwrap();
        let icons: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        let icon_fragment = include_str!("../../../packages/vanilla/components/icon/fragment.html");
        render_with_dependencies(
            name,
            &fixture,
            &template,
            &icons,
            icon_fragment,
            Some(include_str!(
                "../../../packages/vanilla/components/date-calendar/fragment.html"
            )),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    #[test]
    fn renders_self_authored_component_goldens_exactly() {
        for (name, path) in [
            ("activity-feed", "activity-feed/fixtures/typical.json"),
            ("definition-list", "definition-list/fixtures/typical.json"),
            ("disclosure", "disclosure/fixtures/contained.json"),
            ("button-group", "button-group/fixtures/typical.json"),
            ("tabs", "tabs/fixtures/escaped-localized-golden.json"),
            (
                "segmented-control",
                "segmented-control/fixtures/escaped-localized-golden.json",
            ),
            (
                "progress-steps",
                "progress-steps/fixtures/typical-golden.json",
            ),
            ("copy-field", "copy-field/fixtures/typical-golden.json"),
            ("modal", "modal/fixtures/small-golden.json"),
            ("modal", "modal/fixtures/standard-golden.json"),
            ("modal", "modal/fixtures/wide-golden.json"),
            ("drawer", "drawer/fixtures/start-narrow-golden.json"),
            ("drawer", "drawer/fixtures/end-standard-golden.json"),
            ("drawer", "drawer/fixtures/start-wide-golden.json"),
            ("popover", "popover/fixtures/start-narrow-golden.json"),
            ("popover", "popover/fixtures/end-standard-golden.json"),
            ("popover", "popover/fixtures/start-standard-golden.json"),
            ("popover", "popover/fixtures/end-wide-golden.json"),
            ("popover", "popover/fixtures/start-wide-golden.json"),
            ("tooltip", "tooltip/fixtures/top-golden.json"),
            ("tooltip", "tooltip/fixtures/right-golden.json"),
            ("tooltip", "tooltip/fixtures/bottom-golden.json"),
            ("tooltip", "tooltip/fixtures/left-golden.json"),
            ("toast", "toast/fixtures/inline-golden.json"),
            ("toast", "toast/fixtures/top-center-golden.json"),
            ("toast", "toast/fixtures/top-end-golden.json"),
            ("toast", "toast/fixtures/bottom-center-golden.json"),
            ("toast", "toast/fixtures/bottom-end-golden.json"),
            ("toast", "toast/fixtures/preserve-golden.json"),
            ("toast", "toast/fixtures/remove-golden.json"),
            ("theme-switcher", "theme-switcher/fixtures/text-golden.json"),
            (
                "theme-switcher",
                "theme-switcher/fixtures/icons-golden.json",
            ),
        ] {
            let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packages/vanilla/components")
                .join(path);
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(fixture_path).unwrap()).unwrap();
            check(name, fixture);
        }
        for name in ["checkbox-group", "radio-group", "toggle"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packages/vanilla/components")
                .join(name)
                .join("fixtures/typical-golden.json");
            check(
                name,
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
            );
        }
    }
}
