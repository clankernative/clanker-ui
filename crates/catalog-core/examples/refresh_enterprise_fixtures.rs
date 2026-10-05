//! Refresh self-authored golden output after reviewing a renderer change.
use catalog_core::{
    breadcrumbs, data_table, filter_bar, layout::AdmittedChildren, pagination, select_field,
};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn render(name: &str, fixture: &Value, fragment: &str) -> Result<String, String> {
    let mut options = fixture.get("options").unwrap_or(fixture).clone();
    options
        .as_object_mut()
        .ok_or("fixture must be an object")?
        .remove("expectedHtml");
    let json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    match name {
        "select-field" => {
            select_field::render(&select_field::SelectFieldInstance::parse(&json)?, fragment)
        }
        "breadcrumbs" => {
            breadcrumbs::render(&breadcrumbs::BreadcrumbsInstance::parse(&json)?, fragment)
        }
        "pagination" => {
            pagination::render(&pagination::PaginationInstance::parse(&json)?, fragment)
        }
        "data-table" if fixture.get("children").is_none() => {
            data_table::render(&data_table::DataTableInstance::parse(&json)?, fragment)
        }
        "data-table" => data_table::render_with_admitted_body(
            &data_table::DataTableInstance::parse(&json)?,
            fragment,
            AdmittedChildren::from_host_admitted(
                fixture["children"]["body"]
                    .as_str()
                    .ok_or("missing table body")?,
            ),
        ),
        "filter-bar" => {
            let children: BTreeMap<String, String> =
                serde_json::from_value(fixture["children"].clone()).map_err(|e| e.to_string())?;
            let slots = filter_bar::FilterBarSlots {
                controls: AdmittedChildren::from_host_admitted(
                    children.get("controls").ok_or("missing controls")?.clone(),
                ),
                actions: children
                    .get("actions")
                    .cloned()
                    .map(AdmittedChildren::from_host_admitted),
                applied: children
                    .get("applied")
                    .cloned()
                    .map(AdmittedChildren::from_host_admitted),
            };
            filter_bar::render_with_slots(
                &filter_bar::FilterBarInstance::parse(&json)?,
                fragment,
                &slots,
            )
        }
        _ => Err("unknown component".into()),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla/components");
    for name in [
        "select-field",
        "filter-bar",
        "breadcrumbs",
        "pagination",
        "data-table",
    ] {
        let dir = root.join(name);
        let fragment = fs::read_to_string(dir.join("fragment.html"))?;
        for entry in fs::read_dir(dir.join("fixtures"))? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let mut fixture: Value = serde_json::from_slice(&fs::read(&path)?)?;
            fixture["expectedHtml"] = Value::String(render(name, &fixture, &fragment)?);
            fs::write(
                &path,
                format!("{}\n", serde_json::to_string_pretty(&fixture)?),
            )?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
