mod svg_validation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use svg_validation::{valid_path_data, valid_transform};

pub const FRAME_LIMIT: usize = 1024 * 1024;
const SVG_LIMIT: usize = 512 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub time: i64,
    pub value: i64,
    pub missing: bool,
    pub key: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub width: u16,
    pub height: u16,
    pub start: i64,
    pub end: i64,
    pub y_min: i64,
    pub y_max: i64,
    pub title: String,
    pub kind: String,
    pub samples: Vec<Sample>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub d: String,
    pub transform: String,
    pub role: u8,
    pub clipped: bool,
    pub stroke_width: String,
    pub linecap: String,
    pub linejoin: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Label {
    pub text: String,
    pub x: String,
    pub y: String,
    pub transform: String,
    pub anchor: String,
    pub baseline: String,
    pub font_size: String,
    pub font_weight: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plot {
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub key: String,
    pub time: i64,
    pub value: i64,
    pub missing: bool,
    pub x: String,
    pub y: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub paths: Vec<Path>,
    pub labels: Vec<Label>,
    pub plot: Plot,
    pub points: Vec<Point>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    abi: u8,
    renderer: String,
    data: Input,
}
#[derive(Serialize)]
struct Success {
    abi: u8,
    renderer: &'static str,
    #[serde(rename = "inputDigest")]
    input_digest: String,
    result: Scene,
}
#[derive(Serialize)]
struct Failure {
    abi: u8,
    renderer: &'static str,
    #[serde(rename = "inputDigest")]
    input_digest: String,
    error: String,
}

fn shape() -> serde_json::Value {
    serde_json::json!({"record":{"width":"integer","height":"integer","start":"integer","end":"integer","y_min":"integer","y_max":"integer","title":"string","kind":"string","samples":{"list":{"record":{"time":"integer","value":"integer","missing":"boolean","key":"string"}}}}})
}
pub fn contracts() -> serde_json::Value {
    let output = serde_json::json!({"record":{
        "paths":{"list":{"record":{"d":"string","transform":"string","role":"integer","clipped":"boolean","stroke_width":"string","linecap":"string","linejoin":"string"}}},
        "labels":{"list":{"record":{"text":"string","x":"string","y":"string","transform":"string","anchor":"string","baseline":"string","font_size":"string","font_weight":"string"}}},
        "plot":{"record":{"x":"string","y":"string","width":"string","height":"string"}},
        "points":{"list":{"record":{"key":"string","time":"integer","value":"integer","missing":"boolean","x":"string","y":"string"}}}
    }});
    serde_json::json!({"schemaVersion":1,"renderers":{"echarts_chart_v1":{"input":shape(),"output":output}}})
}
fn digest(b: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(b))
}
fn safe_text(s: &str, max: usize) -> bool {
    s.len() <= max
        && !s
            .chars()
            .any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}')
}
pub fn validate(i: &Input) -> Result<(), &'static str> {
    if !(320..=2048).contains(&i.width) || !(120..=1024).contains(&i.height) {
        return Err("dimension_range");
    }
    if i.start >= i.end
        || i.y_min >= i.y_max
        || i.start.unsigned_abs() > 8_640_000_000_000_000
        || i.end.unsigned_abs() > 8_640_000_000_000_000
        || i.y_min.unsigned_abs() > 9_007_199_254_740_991
        || i.y_max.unsigned_abs() > 9_007_199_254_740_991
    {
        return Err("domain_range");
    }
    if !matches!(i.kind.as_str(), "line" | "bar") {
        return Err("kind");
    }
    if !safe_text(&i.title, 160) || i.title.trim().is_empty() {
        return Err("title");
    }
    if i.samples.len() > 128 {
        return Err("sample_count");
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut prev = None;
    for s in &i.samples {
        if s.time < i.start
            || s.time >= i.end
            || s.time.unsigned_abs() > 8_640_000_000_000_000
            || prev.is_some_and(|p| s.time <= p)
        {
            return Err("sample_order");
        }
        if s.value.unsigned_abs() > 9_007_199_254_740_991
            || (!s.missing && (s.value < i.y_min || s.value > i.y_max))
        {
            return Err("value_range");
        }
        if !safe_text(&s.key, 80) || s.key.is_empty() || !keys.insert(&s.key) {
            return Err("sample_key");
        }
        prev = Some(s.time);
    }
    Ok(())
}
fn num(v: f64) -> Result<String, &'static str> {
    if !v.is_finite() || v.abs() > 100_000.0 {
        return Err("svg_coordinate");
    }
    let s = format!("{v:.3}");
    Ok(s.trim_end_matches('0').trim_end_matches('.').to_owned())
}
fn attr_style<'a>(node: &'a roxmltree::Node<'a, 'a>, name: &str) -> Option<String> {
    if let Some(v) = node.attribute(name) {
        return Some(v.to_owned());
    }
    node.attribute("style")?.split(';').find_map(|part| {
        let (k, v) = part.split_once(':')?;
        (k.trim() == name).then(|| v.trim().to_owned())
    })
}
fn inherited_style(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    let mut n = Some(node);
    while let Some(current) = n {
        if let Some(v) = attr_style(&current, name) {
            return Some(v);
        }
        n = current.parent();
    }
    None
}
fn preserves_xml_space(node: roxmltree::Node<'_, '_>) -> bool {
    node.ancestors().chain(std::iter::once(node)).any(|n| {
        n.attributes().any(|a| {
            a.name() == "space"
                && a.namespace() == Some("http://www.w3.org/XML/1998/namespace")
                && a.value() == "preserve"
        })
    })
}
fn inherited(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    let mut n = Some(node);
    while let Some(x) = n {
        if let Some(v) = x.attribute(name) {
            return Some(v.to_owned());
        }
        if name == "clip-path" {
            if let Some(v) = x.attribute("style").and_then(|s| {
                s.split(';')
                    .find_map(|p| p.trim().strip_prefix("clip-path:").map(str::trim))
            }) {
                return Some(v.to_owned());
            }
        }
        n = x.parent();
    }
    None
}
fn transform(node: roxmltree::Node<'_, '_>) -> String {
    let mut chain = Vec::new();
    let mut n = Some(node);
    while let Some(x) = n {
        if let Some(t) = x.attribute("transform") {
            chain.push(t.to_owned());
        }
        n = x.parent();
    }
    chain.reverse();
    chain.join(" ")
}
// Preserve source SVG stroke metrics, with SVG's butt/miter defaults when attributes are absent.
fn stroke_geometry(
    node: roxmltree::Node<'_, '_>,
) -> Result<(String, String, String), &'static str> {
    let width = inherited_style(node, "stroke-width")
        .unwrap_or_else(|| "1".into())
        .parse::<f64>()
        .map_err(|_| "svg_stroke_width")?;
    if !width.is_finite() || !(0.0..=16.0).contains(&width) {
        return Err("svg_stroke_width");
    }
    let linecap = inherited_style(node, "stroke-linecap").unwrap_or_else(|| "butt".into());
    if !matches!(linecap.as_str(), "butt" | "round" | "square") {
        return Err("svg_linecap");
    }
    let linejoin = inherited_style(node, "stroke-linejoin").unwrap_or_else(|| "miter".into());
    if !matches!(
        linejoin.as_str(),
        "miter" | "round" | "bevel" | "arcs" | "miter-clip"
    ) {
        return Err("svg_linejoin");
    }
    Ok((width.to_string(), linecap, linejoin))
}
fn role_for(node: roxmltree::Node<'_, '_>) -> Result<Option<u8>, &'static str> {
    for attr in ["opacity", "fill-opacity", "stroke-opacity"] {
        if let Some(v) = inherited_style(node, attr) {
            if v != "0" && v != "1" {
                return Err("svg_opacity");
            }
        }
    }
    if inherited_style(node, "opacity").as_deref() == Some("0") {
        return Ok(None);
    }
    let mut fill = inherited_style(node, "fill")
        .unwrap_or_default()
        .to_lowercase();
    let mut stroke = inherited_style(node, "stroke")
        .unwrap_or_default()
        .to_lowercase();
    if fill.contains("url(") || stroke.contains("url(") {
        return Err("svg_paint");
    }
    if inherited_style(node, "fill-opacity").as_deref() == Some("0") {
        fill = "none".into();
    }
    if inherited_style(node, "stroke-opacity").as_deref() == Some("0")
        || inherited_style(node, "stroke-width").and_then(|v| v.parse::<f64>().ok()) == Some(0.0)
    {
        stroke = "none".into();
    }
    let white = matches!(fill.as_str(), "#fff" | "white" | "#ffffff");
    if white && (stroke == "#a84716" || stroke == "rgb(168, 71, 22)") {
        return Ok(Some(4));
    }
    if fill == "#a84716" || fill == "rgb(168, 71, 22)" {
        return Ok(Some(3));
    }
    if stroke == "#a84716" || stroke == "rgb(168, 71, 22)" {
        return Ok(Some(2));
    }
    if stroke == "#555d63" || stroke == "#555" || stroke == "rgb(85, 93, 99)" {
        return Ok(Some(0));
    }
    if stroke == "#d7dcdf" || stroke == "rgb(215, 220, 223)" {
        return Ok(Some(1));
    }
    if white {
        return Ok(Some(4));
    }
    if matches!(fill.as_str(), "" | "none" | "transparent")
        && matches!(stroke.as_str(), "" | "none")
    {
        return Ok(None);
    }
    Err("svg_paint")
}
fn remove_default_hover_styles(svg: &str) -> Result<String, &'static str> {
    let Some(begin) = svg.find("<style") else {
        return Ok(svg.to_owned());
    };
    let open_end = svg[begin..].find('>').ok_or("svg_style")? + begin;
    let close = svg[open_end..].find("</style>").ok_or("svg_style")? + open_end;
    let mut css = &svg[open_end + 1..close];
    css = css.trim().strip_prefix("<![CDATA[").unwrap_or(css).trim();
    css = css.strip_suffix("]]>").unwrap_or(css).trim();
    for rule in css.split('}').filter(|r| !r.trim().is_empty()) {
        let (selector, body) = rule.split_once('{').ok_or("svg_style")?;
        let selector = selector.trim();
        if !selector
            .split(',')
            .map(str::trim)
            .all(|s| s.starts_with(".zr") && s.contains("-cls-") && s.ends_with(":hover"))
        {
            return Err("svg_style");
        }
        for decl in body.split(';').map(str::trim).filter(|d| !d.is_empty()) {
            if !matches!(
                decl,
                "pointer-events:none"
                    | "cursor:pointer"
                    | "fill:rgba(0,0,0,1)"
                    | "fill:rgba(184,78,24,1)"
                    | "fill:rgba(255,255,255,1)"
            ) {
                return Err("svg_style");
            }
        }
    }
    let end = close + "</style>".len();
    Ok(format!("{}{}", &svg[..begin], &svg[end..]))
}
fn parse_svg(svg: &str) -> Result<(Vec<Path>, Vec<Label>), &'static str> {
    if svg.len() > SVG_LIMIT {
        return Err("svg_budget");
    }
    let svg = remove_default_hover_styles(svg)?;
    if svg.contains("<!") {
        return Err("svg_budget");
    }
    let doc = roxmltree::Document::parse(&svg).map_err(|_| "svg_parse")?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" || root.attribute("width").is_none() {
        return Err("svg_root");
    }
    let clip_ids: std::collections::BTreeSet<&str> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "clipPath")
        .filter_map(|n| n.attribute("id"))
        .collect();
    let mut paths = Vec::new();
    let mut labels = Vec::new();
    for n in root.descendants().filter(|n| n.is_element()) {
        let tag = n.tag_name().name();
        // ECharts class/ecmeta attributes are renderer-owned hover/hit-test bookkeeping; the worker
        // validates them but does not expose that unsupported interaction ABI. All other attributes
        // fail closed so visible SVG semantics cannot disappear during scene conversion.
        const ATTRS: &[&str] = &[
            "xmlns",
            "width",
            "height",
            "viewBox",
            "version",
            "baseProfile",
            "xlink",
            "id",
            "class",
            "transform",
            "clip-path",
            "d",
            "fill",
            "fill-opacity",
            "stroke",
            "stroke-width",
            "stroke-opacity",
            "stroke-linejoin",
            "stroke-linecap",
            "opacity",
            "pointer-events",
            "style",
            "x",
            "y",
            "dominant-baseline",
            "text-anchor",
            "ecmeta_series_index",
            "ecmeta_data_index",
            "ecmeta_data_name",
            "ecmeta_ssr_type",
        ];
        if n.attributes().any(|a| {
            !ATTRS.contains(&a.name())
                && !(matches!(tag, "text" | "tspan")
                    && a.name() == "space"
                    && a.namespace() == Some("http://www.w3.org/XML/1998/namespace")
                    && a.value() == "preserve")
        }) {
            return Err("svg_attribute");
        }
        if n.ancestors()
            .chain(std::iter::once(n))
            .any(|a| a.attribute("opacity").is_some_and(|v| v != "0" && v != "1"))
        {
            return Err("svg_opacity");
        }
        if n.attribute("style").is_some() && !matches!(tag, "text" | "tspan") {
            return Err("svg_style");
        }
        if n.attribute("pointer-events")
            .is_some_and(|v| !matches!(v, "none" | "visible"))
        {
            return Err("svg_pointer_events");
        }
        if let Some(clip) = n.attribute("clip-path") {
            let id = clip
                .strip_prefix("url(#")
                .and_then(|s| s.strip_suffix(')'))
                .ok_or("svg_clip")?;
            if id.is_empty() || !clip_ids.contains(id) {
                return Err("svg_clip");
            }
        }
        if !matches!(
            tag,
            "svg" | "g" | "defs" | "clipPath" | "path" | "rect" | "text" | "tspan"
        ) {
            return Err("svg_element");
        }
        if tag == "path" || tag == "rect" {
            if n.ancestors()
                .chain(std::iter::once(n))
                .any(|a| a.attribute("opacity") == Some("0"))
            {
                continue;
            }
            if n.ancestors()
                .any(|a| matches!(a.tag_name().name(), "defs" | "clipPath"))
            {
                continue;
            }
            let d = if tag == "path" {
                n.attribute("d").ok_or("svg_path")?.to_owned()
            } else {
                let x = n
                    .attribute("x")
                    .unwrap_or("0")
                    .parse::<f64>()
                    .map_err(|_| "svg_rect")?;
                let y = n
                    .attribute("y")
                    .unwrap_or("0")
                    .parse::<f64>()
                    .map_err(|_| "svg_rect")?;
                let w = n
                    .attribute("width")
                    .ok_or("svg_rect")?
                    .parse::<f64>()
                    .map_err(|_| "svg_rect")?;
                let h = n
                    .attribute("height")
                    .ok_or("svg_rect")?
                    .parse::<f64>()
                    .map_err(|_| "svg_rect")?;
                if w < 0.0 || h < 0.0 {
                    return Err("svg_rect");
                }
                format!(
                    "M{} {}h{}v{}h-{}Z",
                    num(x)?,
                    num(y)?,
                    num(w)?,
                    num(h)?,
                    num(w)?
                )
            };
            if d.trim().is_empty() {
                continue;
            }
            if d.len() > 8192 || !valid_path_data(&d) {
                return Err("svg_path");
            }
            let trans = transform(n);
            if trans.len() > 512 || !valid_transform(&trans) {
                return Err("svg_transform");
            }
            let (stroke_width, linecap, linejoin) = stroke_geometry(n)?;
            if let Some(role) = role_for(n)? {
                paths.push(Path {
                    d,
                    transform: trans,
                    role,
                    clipped: inherited(n, "clip-path").is_some(),
                    stroke_width,
                    linecap,
                    linejoin,
                });
            }
        } else if tag == "text" || tag == "tspan" {
            if n.ancestors()
                .chain(std::iter::once(n))
                .any(|a| a.attribute("opacity") == Some("0"))
            {
                continue;
            }
            let raw_text = n.text().unwrap_or("");
            let text = if preserves_xml_space(n) {
                raw_text
            } else {
                raw_text.trim()
            };
            if !text.is_empty() {
                let raw_x = n
                    .attribute("x")
                    .and_then(|x| x.split_whitespace().next())
                    .unwrap_or("0");
                let raw_y = n
                    .attribute("y")
                    .and_then(|x| x.split_whitespace().next())
                    .unwrap_or("0");
                let style = inherited_style(n, "style").ok_or("svg_text_style")?;
                let (font_size, font_weight) = match style.as_str() {
                    "font-size:11px;font-family:Arial;" | "font: normal normal 11px Arial" => {
                        ("11px", "normal")
                    }
                    "font-size:14px;font-family:Arial;font-weight:bold;" => ("14px", "bold"),
                    _ => return Err("svg_text_style"),
                };
                if inherited_style(n, "fill").is_some_and(|v| v != "#555d63") {
                    return Err("svg_text_paint");
                }
                let text_transform = transform(n);
                if text_transform.len() > 512 || !valid_transform(&text_transform) {
                    return Err("svg_transform");
                }
                if !safe_text(text, 160) {
                    return Err("svg_text");
                }
                let x = raw_x.parse::<f64>().map_err(|_| "svg_text")?;
                let y = raw_y.parse::<f64>().map_err(|_| "svg_text")?;
                let anchor = inherited_style(n, "text-anchor").unwrap_or_else(|| "start".into());
                if !matches!(anchor.as_str(), "start" | "middle" | "end") {
                    return Err("svg_text_anchor");
                }
                let baseline =
                    inherited_style(n, "dominant-baseline").unwrap_or_else(|| "alphabetic".into());
                if !matches!(
                    baseline.as_str(),
                    "auto"
                        | "text-bottom"
                        | "alphabetic"
                        | "ideographic"
                        | "middle"
                        | "central"
                        | "mathematical"
                        | "hanging"
                        | "text-top"
                ) {
                    return Err("svg_text_baseline");
                }
                labels.push(Label {
                    text: text.to_owned(),
                    x: num(x)?,
                    y: num(y)?,
                    transform: text_transform,
                    anchor,
                    baseline,
                    font_size: font_size.to_owned(),
                    font_weight: font_weight.to_owned(),
                });
            }
        }
    }
    if paths.len() > 1024 || labels.len() > 1024 {
        return Err("scene_budget");
    }
    Ok((paths, labels))
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EngineOutput {
    pub svg: String,
    pub rect: [f64; 4],
    pub pix: Vec<[f64; 2]>,
}

pub trait Engine {
    fn render(&self, input: &Input) -> Result<EngineOutput, &'static str>;
}

pub fn render_with(input: &Input, engine: &impl Engine) -> Result<Scene, &'static str> {
    validate(input)?;
    scene_from_output(input, engine.render(input)?)
}

pub fn scene_from_output(input: &Input, output: EngineOutput) -> Result<Scene, &'static str> {
    validate(input)?;
    if output.pix.len() != input.samples.len() {
        return Err("engine_result");
    }
    let (paths, labels) = parse_svg(&output.svg)?;
    let [x, y, w, h] = output.rect;
    if ![x, y, w, h].iter().all(|n| n.is_finite())
        || x < 0.0
        || y < 0.0
        || w <= 0.0
        || h <= 0.0
        || x + w > f64::from(input.width)
        || y + h > f64::from(input.height)
    {
        return Err("plot_rect");
    }
    let points = input
        .samples
        .iter()
        .zip(&output.pix)
        .map(|(s, [x, y])| {
            if !x.is_finite()
                || !y.is_finite()
                || *x < 0.0
                || *y < 0.0
                || *x > f64::from(input.width)
                || *y > f64::from(input.height)
            {
                return Err("svg_coordinate");
            }
            Ok(Point {
                key: s.key.clone(),
                time: s.time,
                value: s.value,
                missing: s.missing,
                x: num(*x)?,
                y: num(*y)?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let scene = Scene {
        paths,
        labels,
        plot: Plot {
            x: num(x)?,
            y: num(y)?,
            width: num(w)?,
            height: num(h)?,
        },
        points,
    };
    let encoded = serde_json::to_vec(&scene).map_err(|_| "scene")?;
    let strings_fit = scene.paths.iter().all(|p| {
        p.d.len() <= 16_384
            && p.transform.len() <= 16_384
            && p.stroke_width.len() <= 16_384
            && p.linecap.len() <= 16_384
            && p.linejoin.len() <= 16_384
    }) && scene.labels.iter().all(|l| {
        l.text.len() <= 16_384
            && l.x.len() <= 16_384
            && l.y.len() <= 16_384
            && l.transform.len() <= 16_384
            && l.anchor.len() <= 16_384
            && l.baseline.len() <= 16_384
            && l.font_size.len() <= 16_384
            && l.font_weight.len() <= 16_384
    }) && scene
        .points
        .iter()
        .all(|p| p.key.len() <= 16_384 && p.x.len() <= 16_384 && p.y.len() <= 16_384);
    if encoded.len() > FRAME_LIMIT
        || !strings_fit
        || scene.paths.len() + scene.labels.len() + scene.points.len() > 1024
    {
        return Err("scene_budget");
    }
    Ok(scene)
}
pub fn process_error(id: String, error: &str) -> Vec<u8> {
    serde_json::to_vec(&Failure {
        abi: 1,
        renderer: "echarts_chart_v1",
        input_digest: id,
        error: error.into(),
    })
    .unwrap()
}
pub fn process_with(bytes: &[u8], engine: &impl Engine) -> Vec<u8> {
    let id = digest(bytes);
    match serde_json::from_slice::<Request>(bytes) {
        Err(_) => process_error(id, "invalid_request"),
        Ok(r) if r.abi != 1 || r.renderer != "echarts_chart_v1" => process_error(id, "identity"),
        Ok(r) => match render_with(&r.data, engine) {
            Ok(scene) => serde_json::to_vec(&Success {
                abi: 1,
                renderer: "echarts_chart_v1",
                input_digest: id,
                result: scene,
            })
            .unwrap(),
            Err(e) => process_error(id, e),
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(kind: &str) -> Input {
        Input {
            width: 640,
            height: 320,
            start: 0,
            end: 10_001,
            y_min: 0,
            y_max: 100,
            title: "Requests".into(),
            kind: kind.into(),
            samples: vec![
                Sample {
                    time: 0,
                    value: 0,
                    missing: false,
                    key: "zero".into(),
                },
                Sample {
                    time: 5_000,
                    value: 40,
                    missing: true,
                    key: "gap".into(),
                },
                Sample {
                    time: 10_000,
                    value: 100,
                    missing: false,
                    key: "peak".into(),
                },
            ],
        }
    }

    struct NeverEngine;
    impl Engine for NeverEngine {
        fn render(&self, _: &Input) -> Result<EngineOutput, &'static str> {
            panic!("invalid input must not reach engine");
        }
    }
    #[test]
    fn svg_adapter_rejects_adversarial_features_and_budgets() {
        fn wrap(body: &str) -> String {
            format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="320">{body}</svg>"#)
        }
        assert_eq!(parse_svg(&wrap("<script/>")), Err("svg_element"));
        assert_eq!(
            parse_svg(&wrap(r#"<path d="M0 0L1 1" onload="x"/>"#)),
            Err("svg_attribute")
        );
        assert_eq!(
            parse_svg(&wrap(r#"<image href="https://example.test/a.svg"/>"#)),
            Err("svg_attribute")
        );
        assert_eq!(
            parse_svg(&wrap(
                r##"<path d="M0 0L1 1" clip-path="url(https://example.test/c.svg#c)"/>"##
            )),
            Err("svg_clip")
        );
        assert_eq!(
            parse_svg(&wrap(
                r#"<path d="M0 0L1 1" transform="translate(1)evil()"/>"#
            )),
            Err("svg_transform")
        );
        assert_eq!(parse_svg(&wrap(r#"<path d="M0 0L"/>"#)), Err("svg_path"));
        assert_eq!(
            parse_svg(&wrap(r#"<path d="M100001 0L1 1"/>"#)),
            Err("svg_path")
        );
        assert_eq!(parse_svg(&" ".repeat(SVG_LIMIT + 1)), Err("svg_budget"));
        let mut many = String::new();
        for _ in 0..=1024 {
            many.push_str(r##"<path d="M0 0L1 1" fill="none" stroke="#a84716"/>"##);
        }
        assert_eq!(parse_svg(&wrap(&many)), Err("scene_budget"));
    }
    #[test]
    fn paths_preserve_transformed_source_stroke_metrics() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="320"><path d="M0 0L1 1" transform="scale(3)" fill="none" stroke="#a84716" stroke-width="0.3333333333333333" stroke-linecap="square" stroke-linejoin="bevel"/><path d="M1 1L2 2" fill="none" stroke="#555d63" stroke-width="3"/></svg>"##;
        let (paths, _) = parse_svg(svg).unwrap();
        assert_eq!(paths[0].role, 2);
        assert_eq!(paths[0].transform, "scale(3)");
        assert_eq!(paths[0].stroke_width, "0.3333333333333333");
        assert!((paths[0].stroke_width.parse::<f64>().unwrap() * 3.0 - 1.0).abs() < 1e-12);
        assert_eq!(paths[0].linecap, "square");
        assert_eq!(paths[0].linejoin, "bevel");
        assert_eq!(paths[1].role, 0);
        assert_eq!(paths[1].stroke_width, "3");
        assert_eq!(paths[1].linecap, "butt");
        assert_eq!(paths[1].linejoin, "miter");
        let bad = svg.replace("stroke-width=\"3\"", "stroke-width=\"17\"");
        assert_eq!(parse_svg(&bad), Err("svg_stroke_width"));
    }
    #[test]
    fn label_alignment_is_typed_and_closed() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="320"><text x="3" y="4" style="font-size:11px;font-family:Arial;" text-anchor="end" dominant-baseline="hanging" fill="#555d63">label</text></svg>"##;
        let (_, labels) = parse_svg(svg).unwrap();
        assert_eq!(labels[0].anchor, "end");
        assert_eq!(labels[0].baseline, "hanging");
        assert_eq!(labels[0].font_size, "11px");
        assert_eq!(labels[0].font_weight, "normal");
        let bad = svg.replace("hanging", "unsupported");
        assert_eq!(parse_svg(&bad), Err("svg_text_baseline"));
    }
    #[test]
    fn xml_space_and_font_source_values_are_preserved_and_closed() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="320"><text xml:space="preserve" x="3" y="4" style="font-size:14px;font-family:Arial;font-weight:bold;" text-anchor="middle" dominant-baseline="central" fill="#555d63">  Current samples  </text></svg>"##;
        let (_, labels) = parse_svg(svg).unwrap();
        assert_eq!(labels[0].text, "  Current samples  ");
        assert_eq!(labels[0].font_size, "14px");
        assert_eq!(labels[0].font_weight, "bold");
        let bad_font = svg.replace("14px", "12px");
        assert_eq!(parse_svg(&bad_font), Err("svg_text_style"));
        let bad_weight = svg.replace("font-weight:bold", "font-weight:500");
        assert_eq!(parse_svg(&bad_weight), Err("svg_text_style"));
        let bad_space = svg.replace("xml:space=\"preserve\"", "xml:space=\"default\"");
        assert_eq!(parse_svg(&bad_space), Err("svg_attribute"));
        let unnamespaced = svg.replace("xml:space=", "space=");
        assert_eq!(parse_svg(&unnamespaced), Err("svg_attribute"));
    }
    #[test]
    fn blank_titles_and_unsafe_missing_integers_are_rejected() {
        let mut i = fixture("line");
        i.title = "  ".into();
        assert_eq!(validate(&i), Err("title"));
        i.title = "Chart".into();
        i.samples[1].missing = true;
        i.samples[1].value = i64::MAX;
        assert_eq!(validate(&i), Err("value_range"));
    }
    #[test]
    fn closed_validation_and_protocol_identity() {
        let valid = fixture("line");
        assert!(validate(&valid).is_ok());
        let mut bad = valid.clone();
        bad.samples[1].time = 0;
        assert_eq!(validate(&bad), Err("sample_order"));
        let mut at_end = valid.clone();
        at_end.samples[2].time = at_end.end;
        assert_eq!(validate(&at_end), Err("sample_order"));
        let mut bad = valid.clone();
        bad.samples[0].value = 101;
        assert_eq!(validate(&bad), Err("value_range"));
        let raw = br#"{"abi":1,"renderer":"echarts_chart_v1","data":{"bad":1}}"#;
        let r: serde_json::Value =
            serde_json::from_slice(&process_with(raw, &NeverEngine)).unwrap();
        assert_eq!(r["inputDigest"], digest(raw));
        assert_eq!(r["error"], "invalid_request");
        let mut unknown = serde_json::to_value(Request {
            abi: 1,
            renderer: "echarts_chart_v1".into(),
            data: valid,
        })
        .unwrap();
        unknown["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Request>(unknown).is_err());
    }
    #[test]
    fn contracts_have_closed_expected_identity() {
        let c = contracts();
        assert_eq!(c["schemaVersion"], 1);
        assert!(
            c["renderers"]["echarts_chart_v1"]["input"]["record"]["samples"]["list"]["record"]
                ["missing"]
                == "boolean"
        );
        assert_eq!(
            c["renderers"]["echarts_chart_v1"]["output"]["record"]["labels"]["list"]["record"]
                ["anchor"],
            "string"
        );
        assert_eq!(
            c["renderers"]["echarts_chart_v1"]["output"]["record"]["labels"]["list"]["record"]
                ["baseline"],
            "string"
        );
        assert_eq!(
            c["renderers"]["echarts_chart_v1"]["output"]["record"]["labels"]["list"]["record"]
                ["font_size"],
            "string"
        );
        assert_eq!(
            c["renderers"]["echarts_chart_v1"]["output"]["record"]["labels"]["list"]["record"]
                ["font_weight"],
            "string"
        );
        for key in ["stroke_width", "linecap", "linejoin"] {
            assert_eq!(
                c["renderers"]["echarts_chart_v1"]["output"]["record"]["paths"]["list"]["record"]
                    [key],
                "string"
            );
        }
    }
}
