//! Refresh only self-authored goldens for the new native contracts. Run from the repository root.
use catalog_core::{copy_field, progress_steps, segmented_control, tabs};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};
#[path = "../src/native_ports.rs"]
mod native_ports;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("packages/vanilla/components");
    let icons: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read("packages/vanilla/icons.json")?)?;
    let jobs = [
        ("tabs", "escaped-localized"),
        ("segmented-control", "escaped-localized"),
        ("progress-steps", "typical"),
        ("copy-field", "typical"),
    ];
    for (name, variant) in jobs {
        let dir = root.join(name);
        let input = dir.join("fixtures").join(format!("{variant}.json"));
        let mut options: serde_json::Value = serde_json::from_slice(&fs::read(&input)?)?;
        options
            .as_object_mut()
            .ok_or("fixture must be object")?
            .remove("expectedHtml");
        let json = serde_json::to_string(&options)?;
        let fragment = fs::read_to_string(dir.join("fragment.html"))?;
        let html = match name {
            "tabs" => tabs::render(&tabs::TabsInstance::parse(&json)?, &fragment)?,
            "segmented-control" => segmented_control::render(
                &segmented_control::SegmentedControlInstance::parse(&json)?,
                &fragment,
            )?,
            "progress-steps" => progress_steps::render_with_icons(
                &progress_steps::ProgressStepsInstance::parse(&json)?,
                &fragment,
                &icons,
                &fs::read_to_string(root.join("icon/fragment.html"))?,
            )?,
            "copy-field" => copy_field::render(
                &copy_field::CopyFieldInstance::parse(&json)?,
                &fragment,
                &icons,
            )?,
            _ => unreachable!(),
        };
        let destination = dir.join("fixtures").join(format!("{variant}-golden.json"));
        fs::create_dir_all(destination.parent().unwrap())?;
        fs::write(
            destination,
            format!(
                "{}\n",
                serde_json::to_string_pretty(
                    &serde_json::json!({"options":options,"expectedHtml":html})
                )?
            ),
        )?;
    }
    // Choice-control input fixtures stay untouched; publish separate combined options/golden files.
    for name in ["checkbox-group", "radio-group", "toggle"] {
        let dir = root.join(name);
        for variant in ["typical", "inline", "validation", "disabled"] {
            let input = dir.join("fixtures").join(format!("{variant}.json"));
            if !input.exists() {
                continue;
            }
            let options: serde_json::Value = serde_json::from_slice(&fs::read(input)?)?;
            let golden: serde_json::Value = serde_json::from_slice(&fs::read(
                dir.join("fixtures")
                    .join("goldens")
                    .join(format!("{variant}.json")),
            )?)?;
            let output = dir.join("fixtures").join(format!("{variant}-golden.json"));
            fs::write(
                output,
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"options":options,"expectedHtml":golden["expectedHtml"]})
                    )?
                ),
            )?;
        }
    }
    let icon_fragment = fs::read_to_string(root.join("icon/fragment.html"))?;
    let mut jobs: Vec<(&str, String, Value, Option<Value>)> = Vec::new();
    for size in ["small", "standard", "wide"] {
        jobs.push(("modal", size.to_owned(), json!({"id":"modal-example","title":"Review changes","triggerLabel":"Review","fallbackHref":"/review","size":size}), Some(json!({"body":"<p>Review the pending changes.</p>"}))));
    }
    for (placement, size) in [("start", "narrow"), ("end", "standard"), ("start", "wide")] {
        jobs.push(("drawer", format!("{placement}-{size}"), json!({"id":"drawer-example","title":"Filters","description":"Narrow the results.","triggerLabel":"Open filters","fallbackHref":"/filters","placement":placement,"size":size}), Some(json!({"body":"<p>Filter controls.</p>","actions":"<button type=\"button\">Apply</button>"}))));
    }
    for (alignment, width) in [
        ("start", "narrow"),
        ("end", "standard"),
        ("start", "standard"),
        ("end", "wide"),
        ("start", "wide"),
    ] {
        jobs.push(("popover", format!("{alignment}-{width}"), json!({"id":"popover-example","label":"More options","alignment":alignment,"width":width,"leadingIcon":"info"}), Some(json!({"body":"<p>Additional actions.</p>"}))));
    }
    let mut upload: Value =
        serde_json::from_slice(&fs::read(root.join("file-upload/fixtures/typical.json"))?)?;
    upload["multiple"] = json!(false);
    upload["disabled"] = json!(true);
    upload["files"] = json!([]);
    jobs.push(("file-upload", "disabled".into(), upload, None));
    let paged: Value =
        serde_json::from_slice(&fs::read(root.join("data-viewport/fixtures/paged.json"))?)?;
    let navigation = json!({"navigation":"<nav aria-label=\"Illustrative pagination\"><a href=\"/simulator?page=2\" data-cui-viewport-navigation>Next illustrative page</a></nav>"});
    jobs.push((
        "data-viewport",
        "paged-navigation".into(),
        paged.clone(),
        Some(navigation.clone()),
    ));
    let mut incremental = paged;
    incremental["mode"] = json!("incremental");
    incremental["height"] = json!("standard");
    jobs.push((
        "data-viewport",
        "incremental-standard".into(),
        incremental,
        Some(navigation),
    ));
    let mut windowed: Value = serde_json::from_slice(&fs::read(
        root.join("data-viewport/fixtures/windowed.json"),
    )?)?;
    windowed["height"] = json!("viewport");
    jobs.push(("data-viewport", "windowed-viewport".into(), windowed, None));
    for placement in ["top", "right", "bottom", "left"] {
        jobs.push(("tooltip", placement.to_owned(), json!({"id":"tooltip-example","triggerLabel":"Help","title":"More information","description":"Additional context.","icon":"help","placement":placement,"arrow":true}), None));
    }
    let tones = ["info", "success", "warning", "danger", "progress"];
    let messages: Vec<Value> = tones.iter().map(|tone| json!({"title":format!("{tone} notice"),"body":"A representative notification.","tone":tone})).collect();
    for position in [
        "inline",
        "top-center",
        "top-end",
        "bottom-center",
        "bottom-end",
    ] {
        jobs.push(("toast", position.to_owned(), json!({"label":"Notifications","messages":messages,"position":position,"historyPolicy":"reset"}), None));
    }
    for policy in ["preserve", "remove"] {
        jobs.push(("toast", policy.to_owned(), json!({"label":"Notifications","messages":messages,"position":"bottom-end","historyPolicy":policy}), None));
    }
    for presentation in ["text", "icons"] {
        jobs.push(("theme-switcher", presentation.to_owned(), json!({"label":"Theme","choices":[{"value":"light","label":"Light","icon":"sun"},{"value":"dark","label":"Dark","icon":"moon"}],"presentation":presentation,"defaultChoice":"light","systemLightTheme":"light","systemDarkTheme":"dark"}), None));
    }
    for (name, variant, options, children) in jobs {
        let dir = root.join(name);
        let template = fs::read_to_string(dir.join("fragment.html"))?;
        let fixture = match children {
            Some(ref children) => json!({"options":options,"children":children}),
            None => json!({"options":options}),
        };
        let calendar_template = fs::read_to_string(root.join("date-calendar/fragment.html"))?;
        let (html, _) = if name == "date-picker" {
            native_ports::render_with_dependencies(
                name,
                &fixture,
                &template,
                &icons,
                &icon_fragment,
                Some(&calendar_template),
            )?
        } else {
            native_ports::render(name, &fixture, &template, &icons, &icon_fragment)?
        };
        let output = dir.join("fixtures").join(format!("{variant}-golden.json"));
        fs::write(
            output,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&{
                    let mut golden = fixture;
                    golden["expectedHtml"] = json!(html);
                    golden
                })?
            ),
        )?;
    }
    // Every original self-authored input gets its own exact golden. Keep raw
    // inputs intact, ignore generated files, and derive output through the same
    // explicit typed renderer that verification uses.
    for name in [
        "activity-feed",
        "definition-list",
        "disclosure",
        "button-group",
        "tabs",
        "segmented-control",
        "progress-steps",
        "checkbox-group",
        "radio-group",
        "toggle",
        "copy-field",
        "modal",
        "drawer",
        "popover",
        "tooltip",
        "toast",
        "theme-switcher",
        "date-calendar",
        "date-picker",
        "command-menu",
        "confirm-dialog",
        "file-upload",
        "data-viewport",
    ] {
        let dir = root.join(name);
        let mut files = fs::read_dir(dir.join("fixtures"))?.collect::<Result<Vec<_>, _>>()?;
        files.sort_by_key(|entry| entry.file_name());
        for entry in files {
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            if path.extension().and_then(|value| value.to_str()) != Some("json")
                || stem.ends_with("-golden")
            {
                continue;
            }
            let mut fixture: Value = serde_json::from_slice(&fs::read(&path)?)?;
            fixture
                .as_object_mut()
                .ok_or("fixture must be an object")?
                .remove("expectedHtml");
            if name == "popover" {
                if let Some(children) = fixture.get_mut("children").and_then(Value::as_object_mut) {
                    if let Some(content) = children.remove("content") {
                        if children.insert("body".into(), content).is_some() {
                            return Err(
                                "popover fixture cannot supply both body and content".into()
                            );
                        }
                    }
                }
            }
            let template = fs::read_to_string(dir.join("fragment.html"))?;
            let calendar_template = fs::read_to_string(root.join("date-calendar/fragment.html"))?;
            let (html, _) = if name == "date-picker" {
                native_ports::render_with_dependencies(
                    name,
                    &fixture,
                    &template,
                    &icons,
                    &icon_fragment,
                    Some(&calendar_template),
                )
            } else {
                native_ports::render(name, &fixture, &template, &icons, &icon_fragment)
            }
            .map_err(|error| format!("{}: {error}", path.display()))?;
            let object = fixture.as_object_mut().unwrap();
            if !object.contains_key("options") {
                let children = object.remove("children");
                fixture = json!({"options": object.clone()});
                if let Some(children) = children {
                    fixture
                        .as_object_mut()
                        .unwrap()
                        .insert("children".into(), children);
                }
            }
            fixture
                .as_object_mut()
                .unwrap()
                .insert("expectedHtml".into(), json!(html));
            fs::write(
                dir.join("fixtures").join(format!("{stem}-golden.json")),
                format!("{}\n", serde_json::to_string_pretty(&fixture)?),
            )?;
        }
    }
    Ok(())
}
