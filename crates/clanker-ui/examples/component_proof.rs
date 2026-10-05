//! Render locked component fixtures for browser conformance with explicit test adapters.
//! This artifact is not a Native app and does not establish backend/host admission.
use clanker_ui::{
    application::Application, directory_sink::DirectorySink, local::LocalPackage,
    native::NativeAdapter,
};
use std::{env, fs, path::PathBuf};

const COMPONENTS: [&str; 9] = [
    "modal",
    "drawer",
    "popover",
    "command-menu",
    "confirm-dialog",
    "date-calendar",
    "date-picker",
    "file-upload",
    "data-viewport",
];
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: component_proof <lock> <separate-proof-directory>".into());
    }
    let lock = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("proof output must be a new directory".into());
    }
    let app = Application {
        source: LocalPackage,
        output: DirectorySink,
    };
    let package = app.load(&lock)?;
    let mut body = String::new();
    for name in COMPONENTS {
        let component = package
            .catalog
            .get(name)
            .ok_or_else(|| format!("{name} is not component-complete"))?;
        let fixture = component
            .fixtures
            .iter()
            .find(|path| path.contains("golden"))
            .ok_or_else(|| format!("{name} has no golden fixture"))?;
        let html = NativeAdapter::fixture_html(&LocalPackage, &package, name, fixture)?;
        if name == "confirm-dialog" {
            let value: serde_json::Value = serde_json::from_slice(&package.assets[fixture])?;
            let form = value["options"]["formId"]
                .as_str()
                .ok_or("confirmation fixture has no original form")?;
            // The typed renderer already validated this bounded ID. This is an
            // app-owned browser form, not Native signed command transport.
            body.push_str(&format!("<form id=\"{form}\" class=\"simulator\"><label>Simulator confirmation reason (required)<input name=\"reason\" required value=\"Illustrative fixture\"></label></form>"));
        }
        body.push_str(&format!("<section class=\"proof-panel\" data-proof-component=\"{name}\"><h2>{name}</h2><p>Locked component fixture. Native integration is adapter-required.</p>{html}</section>"));
        if name == "data-viewport" {
            let window = component
                .fixtures
                .iter()
                .find(|path| path.ends_with("windowed-golden.json"))
                .ok_or("viewport has no windowed golden")?;
            let window_html = NativeAdapter::fixture_html(&LocalPackage, &package, name, window)?;
            body.push_str(&format!("<section class=\"proof-panel\" data-proof-component=\"windowed-viewport\"><h2>Windowed viewport — simulator requests only</h2>{window_html}</section>"));
        }
    }
    fs::create_dir_all(&out)?;
    let mut css = String::from_utf8(package.assets[&package.catalog.package.theme].clone())?;
    if let Some(reference) = package.assets.get("theme/reference.css") {
        css.push_str(std::str::from_utf8(reference)?);
    }
    for component in package.catalog.components() {
        css.push_str(std::str::from_utf8(
            &package.assets[&component.assets.styles],
        )?);
    }
    css.push_str("body{font-family:var(--cui-font-sans,system-ui);color:var(--cui-text);background:var(--cui-surface-subtle);margin:0;padding:24px}main{max-width:1100px;margin:auto}.proof-panel{padding:24px;margin-block:24px;background:var(--cui-surface);border:1px solid var(--cui-border);border-radius:12px;min-width:0}h1,h2{color:var(--cui-heading-text)}.simulator{padding:16px;border:1px dashed var(--cui-border);margin-block:16px}output{display:block;white-space:pre-wrap}input,button{font:inherit}.proof-panel>*{max-inline-size:100%}");
    fs::write(out.join("proof.css"), css)?;
    for (path, bytes) in &package.assets {
        if path.ends_with(".js") {
            let target = out.join(path);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::write(target, bytes)?;
        }
    }
    let html = format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Component conformance — simulator adapters</title><link rel=\"stylesheet\" href=\"proof.css\"></head><body><main><h1>Component conformance</h1><p>This is a browser test artifact with illustrative fixture data and explicitly simulated adapters. It is not a Native app, production upload, or backend integration proof.</p><section class=\"simulator\" aria-label=\"Explicit simulator adapter state\"><h2>Simulator state</h2><output id=\"simulator-state\">Adapters have not been installed.</output></section>{body}</main><script type=\"module\" src=\"adapters.mjs\"></script></body></html>");
    fs::write(out.join("index.html"), html)?;
    println!("{}", out.display());
    Ok(())
}
