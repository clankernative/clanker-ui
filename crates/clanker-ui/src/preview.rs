//! Script-free, fake-data preview. This is not Native admission or app execution.
use crate::{application::Application, directory_sink::DirectorySink, local::LocalPackage};
use base64::{engine::general_purpose::STANDARD, Engine};
use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value as TemplateValue};
use scraper::{ElementRef, Html, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fs, io::Write, path::Path, sync::Arc};

const MAX_SCENE: u64 = 1_048_576;
const MAX_HTML: usize = 524_288;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scene {
    #[serde(default)]
    pub page: Option<String>,
    pub data: Value,
    #[serde(default)]
    pub routes: BTreeMap<String, String>,
    #[serde(default = "default_width")]
    pub width: u32,
}
fn default_width() -> u32 {
    1280
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub schema_version: u32,
    pub package_digest: String,
    pub template: String,
    /// Studio controls the actual iframe viewport; CSS is never faked server-side.
    pub width: u32,
    pub html: String,
    pub scope: &'static str,
}
fn error(message: impl std::fmt::Display) -> minijinja::Error {
    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, message.to_string())
}
fn template_path(path: &str) -> Result<(), String> {
    if !(path.starts_with("pages/") || path.starts_with("components/"))
        || !path.ends_with(".html")
        || path.len() > 512
        || path.split('/').count() > 8
        || path.split('/').any(|part| {
            part.is_empty()
                || part.starts_with('.')
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_- .".contains(&byte))
                || part.contains(' ')
        })
    {
        return Err(
            "preview template must be a relative pages/*.html or components/*.html path within ui/"
                .into(),
        );
    }
    Ok(())
}
fn route_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && path.len() <= 4096
        && !path.chars().any(|ch| ch.is_control() || ch == '\\')
}
#[derive(Debug)]
struct Routes(BTreeMap<String, String>);
impl minijinja::value::Object for Routes {
    fn call_method(
        self: &Arc<Self>,
        _state: &minijinja::State<'_, '_>,
        name: &str,
        args: &[TemplateValue],
    ) -> Result<TemplateValue, minijinja::Error> {
        let path = self
            .0
            .get(name)
            .ok_or_else(|| error(format!("scene route is missing: {name}")))?;
        let (kwargs,): (minijinja::value::Kwargs,) = minijinja::value::from_args(args)?;
        // This is an explicit fake link, not a substitute for Roc route binding.
        // Named arguments are reflected as URL-encoded query values in previews.
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        for key in kwargs.args() {
            let value: TemplateValue = kwargs.get(key)?;
            if !matches!(
                value.kind(),
                minijinja::value::ValueKind::String
                    | minijinja::value::ValueKind::Number
                    | minijinja::value::ValueKind::Bool
            ) {
                return Err(error("scene route argument must be scalar"));
            }
            query.append_pair(key, &value.to_string());
        }
        kwargs.assert_all_used()?;
        let query = query.finish();
        let result = if query.is_empty() {
            path.clone()
        } else {
            let (base, fragment) = path.split_once('#').unwrap_or((path, ""));
            format!(
                "{base}{}{query}{}",
                if base.contains('?') { "&" } else { "?" },
                if fragment.is_empty() {
                    String::new()
                } else {
                    format!("#{fragment}")
                }
            )
        };
        Ok(TemplateValue::from(result))
    }
}
#[derive(Debug)]
struct Platform;
impl minijinja::value::Object for Platform {
    fn call_method(
        self: &Arc<Self>,
        _state: &minijinja::State<'_, '_>,
        name: &str,
        args: &[TemplateValue],
    ) -> Result<TemplateValue, minijinja::Error> {
        if !args.is_empty() {
            return Err(error("preview platform links take no arguments"));
        }
        let path = match name {
            "audit" => "/audit",
            "docs" => "/docs",
            "openapi" => "/openapi.json",
            _ => return Err(error("unknown preview platform link")),
        };
        Ok(TemplateValue::from(path))
    }
}
struct BoundedOutput(Vec<u8>);
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_HTML {
            return Err(std::io::Error::other("preview HTML budget"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn static_markup(markup: &str) -> Result<String, String> {
    let mut document = Html::parse_fragment(markup);
    for node in document.tree.nodes().filter_map(ElementRef::wrap) {
        let element = node.value();
        if matches!(
            element.name(),
            "script" | "iframe" | "object" | "embed" | "base" | "style" | "link"
        ) || element
            .attrs
            .iter()
            .any(|(name, _)| name.local.as_ref().starts_with("on"))
        {
            return Err("static preview rejects scripts, embedded browsing contexts, inline event handlers and authored style/link tags".into());
        }
    }
    for (marker, attribute) in [
        ("data-cui-selected-flag", "selected"),
        ("data-cui-checked-flag", "checked"),
        ("data-cui-hidden-flag", "hidden"),
    ] {
        let nodes = document
            .tree
            .nodes()
            .filter_map(ElementRef::wrap)
            .filter_map(|node| {
                node.value().attr(marker).map(|value| {
                    (
                        node.id(),
                        node.value().name().to_owned(),
                        node.value().attr("type").unwrap_or("").to_owned(),
                        value.to_owned(),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (id, tag, kind, value) in nodes {
            let allowed = match attribute {
                "selected" => tag == "option",
                "checked" => tag == "input" && matches!(kind.as_str(), "checkbox" | "radio"),
                "hidden" => matches!(tag.as_str(), "fieldset" | "div"),
                _ => false,
            };
            if !allowed || !matches!(value.as_str(), "true" | "false") {
                return Err("invalid preview boolean presentation flag".into());
            }
            let mut node = document.tree.get_mut(id).ok_or("preview node missing")?;
            let Node::Element(element) = node.value() else {
                return Err("preview element missing".into());
            };
            let mut name = element
                .attrs
                .iter()
                .find(|(name, _)| name.local.as_ref() == marker)
                .ok_or("preview flag missing")?
                .0
                .clone();
            element.attrs.retain(|(name, _)| {
                name.local.as_ref() != marker && name.local.as_ref() != attribute
            });
            if value == "true" {
                name.local = attribute.into();
                element.attrs.push((name, "".into()));
            }
            element.attrs.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        }
    }
    Ok(document.root_element().inner_html())
}

pub fn render(
    lock: &Path,
    ui: &Path,
    scene_path: &Path,
    fragment: Option<&Path>,
) -> Result<Preview, String> {
    let meta = fs::symlink_metadata(scene_path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_SCENE {
        return Err("scene must be a regular bounded JSON file".into());
    }
    let bytes = fs::read(scene_path).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_SCENE {
        return Err("scene byte budget".into());
    }
    let scene: Scene = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let data = scene
        .data
        .as_object()
        .ok_or("scene.data must be a record")?;
    if !(64..=8192).contains(&scene.width) || scene.routes.len() > 256 {
        return Err("scene viewport or route budget".into());
    }
    for (key, path) in &scene.routes {
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !route_path(path)
        {
            return Err("scene routes must be named safe local paths".into());
        }
    }
    if ["routes", "platform", "asset"]
        .iter()
        .any(|name| data.contains_key(*name))
    {
        return Err("scene.data cannot replace preview navigation/helpers".into());
    }
    let selected = match fragment {
        Some(path) => {
            let path = path.to_str().ok_or("fragment path must be UTF-8")?;
            path.strip_prefix("ui/").unwrap_or(path).to_owned()
        }
        None => scene
            .page
            .clone()
            .ok_or("scene.page is required without --fragment")?,
    };
    template_path(&selected)?;
    let bundle = crate::expand::expand(lock, ui, None)?;
    let package = Application {
        source: LocalPackage,
        output: DirectorySink,
    }
    .load(lock)?;
    if package.digest != bundle.package_digest {
        return Err("package changed during preview capture".into());
    }
    let mut css = String::new();
    for resource in &bundle.resources {
        if resource.kind == "stylesheet" {
            if let Some(content) = &resource.content {
                css.push_str(content);
                css.push('\n');
            } else if let Some(source) = &resource.source {
                css.push_str(
                    std::str::from_utf8(
                        package
                            .assets
                            .get(source)
                            .ok_or("missing captured stylesheet")?,
                    )
                    .map_err(|e| e.to_string())?,
                );
                css.push('\n');
            }
        }
    }
    for resource in &bundle.resources {
        if resource.kind == "font" {
            let source = resource
                .source
                .as_ref()
                .ok_or("font requires a captured source")?;
            let bytes = package.assets.get(source).ok_or("missing captured font")?;
            if bytes.len() < 48 || &bytes[..4] != b"wOF2" {
                return Err("preview requires WOFF2 fonts".into());
            }
            let url = format!("data:font/woff2;base64,{}", STANDARD.encode(bytes));
            let path = resource
                .path
                .strip_prefix("ui/")
                .ok_or("font resource is outside ui/")?;
            css = css.replace(&format!("./{path}"), &url).replace(path, &url);
        }
    }
    let mut environment = Environment::empty();
    environment.set_auto_escape_callback(|_| AutoEscape::Html);
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment.set_fuel(Some(200_000));
    environment.set_recursion_limit(32);
    environment.add_filter("length", |value: TemplateValue| {
        value
            .len()
            .ok_or_else(|| error("length requires a list or string"))
    });
    catalog_core::template_values::install(&mut environment);
    environment.add_global(
        "routes",
        TemplateValue::from_object(Routes(scene.routes.clone())),
    );
    environment.add_global("platform", TemplateValue::from_object(Platform));
    environment.add_function(
        "asset",
        |_key: String| -> Result<String, minijinja::Error> {
            Err(error("scene has no admitted asset catalog"))
        },
    );
    environment.set_formatter(|output, state, value| {
        if let Some(image) =
            value.downcast_object_ref::<catalog_core::template_values::ImageSource>()
        {
            minijinja::escape_formatter(output, state, &TemplateValue::from(image.0.as_str()))
        } else {
            minijinja::escape_formatter(output, state, value)
        }
    });
    for (name, source) in &bundle.templates {
        template_path(name)?;
        environment
            .add_template(name, source)
            .map_err(|e| e.to_string())?;
    }
    let mut output = BoundedOutput(Vec::new());
    environment
        .get_template(&selected)
        .map_err(|e| e.to_string())?
        .render_to_write(&scene.data, &mut output)
        .map_err(|e| e.to_string())?;
    let markup = static_markup(&String::from_utf8(output.0).map_err(|e| e.to_string())?)?;
    let css = css.replace('<', "\\3c ");
    let title = selected
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    let html=format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'none'; style-src 'unsafe-inline'; font-src data:; img-src data: https:; form-action 'none'; base-uri 'none'\"><title>{title}</title><style>{css}</style></head><body>{markup}</body></html>");
    if html.len() > 2_097_152 {
        return Err("preview document byte budget".into());
    }
    Ok(Preview{schema_version:1,package_digest:bundle.package_digest,template:selected,width:scene.width,html,scope:"Static fake-data preview only; no scripts, command execution, Roc-context admission or browser readiness proof."})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_flags_and_executable_markup_are_checked() {
        let output=static_markup("<select><option value=\"a\" selected data-cui-selected-flag=\"false\">A</option><option value=\"b\" data-cui-selected-flag=\"true\">B</option></select>").unwrap();
        assert!(!output.contains("data-cui-selected-flag"));
        assert_eq!(output.matches(" selected").count(), 1);
        for markup in [
            "<script>alert(1)</script>",
            "<img onerror=\"alert(1)\" src=\"x\">",
            "<iframe src=\"/\"></iframe>",
            "<div data-cui-selected-flag=\"true\"></div>",
            "<fieldset data-cui-hidden-flag=\"yes\"></fieldset>",
        ] {
            assert!(static_markup(markup).is_err(), "{markup}");
        }
    }
    #[test]
    fn scene_schema_is_closed_and_paths_cannot_escape() {
        assert!(serde_json::from_str::<Scene>(
            r#"{"page":"pages/index.html","data":{},"routes":{"workspace":"/"},"width":1280}"#
        )
        .is_ok());
        assert!(serde_json::from_str::<Scene>(r#"{"data":{},"execute":true}"#).is_err());
        for path in [
            "../outside.html",
            "pages/../outside.html",
            "/pages/index.html",
            "pages/.hidden.html",
            "pages/a.html/../../x.html",
        ] {
            assert!(template_path(path).is_err());
        }
        assert!(
            !route_path("//evil.test")
                && !route_path("javascript:alert(1)")
                && !route_path("/\\evil")
        );
    }
}
