use super::*;

const RENDERER: &str = "echarts_chart_v1";
const SCRIPT: &str = "components/chart/interaction.js";

pub(super) fn render(declaration: &Button, package: &Package) -> Result<(String, bool)> {
    checked_attributes(declaration, &["id", "data", "label", "enhance"], "chart")?;
    let id = declaration
        .attrs
        .get("id")
        .context("cui-chart requires id")?;
    ensure!(
        !id.is_empty()
            && id.len() <= 64
            && id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b':'
            })
            && id.as_bytes()[0].is_ascii_alphabetic(),
        "cui-chart id must be a stable HTML identifier"
    );
    let label = declaration
        .attrs
        .get("label")
        .context("cui-chart requires label")?;
    ensure!(
        !label.trim().is_empty()
            && !label.chars().any(char::is_control)
            && !["{{", "}}", "{%", "%}", "{#", "#}"]
                .iter()
                .any(|delimiter| label.contains(delimiter)),
        "cui-chart label must be literal nonblank text"
    );
    let data = declaration
        .attrs
        .get("data")
        .context("cui-chart requires data")?;
    let data_path =
        interpolation(data).context("cui-chart data must be a whole dotted identifier field")?;
    ensure!(
        field_path(data_path).is_some()
            && data_path
                .split('.')
                .all(|part| { part.as_bytes()[0].is_ascii_lowercase() || part.starts_with('_') }),
        "cui-chart data must be a whole dotted identifier field"
    );
    ensure!(
        package.presentation_contracts.contains_key(RENDERER),
        "cui-chart requires its locked echarts_chart_v1 renderer contract"
    );
    let enhance = declaration
        .attrs
        .get("enhance")
        .map(String::as_str)
        .unwrap_or("false");
    ensure!(
        enhance == "true" || enhance == "false",
        "cui-chart enhance must be true or false"
    );
    let caption_id = format!("{id}-caption");
    let clip_id = format!("{id}-clip");
    let attributes = format!(
        "class=\"cui-chart\" id=\"{}\" aria-labelledby=\"{}\" data-cui-component=\"chart\" data-cui-chart data-cui-chart-enhance=\"{}\" data-start=\"{{{{ {data_path}.start }}}}\" data-end=\"{{{{ {data_path}.end }}}}\"",
        escape(id),
        escape(&caption_id),
        enhance
    );
    let caption_attributes = format!("id=\"{}\"", escape(&caption_id));
    let label = escape(label);
    // Native API collections remain bounded CollectionPage envelopes. Only their
    // checked items are projected into the component-owned worker contract.
    let projection = [
        "width", "height", "start", "end", "title", "kind", "y_min", "y_max",
    ]
    .iter()
    .map(|field| format!("'{field}': {data_path}.{field}"))
    .chain(std::iter::once(format!(
        "'samples': {data_path}.samples.items"
    )))
    .collect::<Vec<_>>()
    .join(", ");
    let scene = format!("ui_scene('{RENDERER}', {{{projection}}})");
    let svg = format!(
        "<svg class=\"cui-chart__scene\" viewBox=\"0 0 {{{{ {data_path}.width }}}} {{{{ {data_path}.height }}}}\" width=\"{{{{ {data_path}.width }}}}\" height=\"{{{{ {data_path}.height }}}}\" aria-hidden=\"true\"><defs><clipPath id=\"{}\"><rect x=\"{{{{ {scene}.plot.x }}}}\" y=\"{{{{ {scene}.plot.y }}}}\" width=\"{{{{ {scene}.plot.width }}}}\" height=\"{{{{ {scene}.plot.height }}}}\"></rect></clipPath></defs><g class=\"cui-chart__paths\">{{% for path in {scene}.paths %}}<path class=\"cui-chart__path cui-chart__path--{{{{ ui_integer(path.role, 0, 4) }}}}\" d=\"{{{{ path.d }}}}\" transform=\"{{{{ path.transform }}}}\" stroke-width=\"{{{{ path.stroke_width }}}}\" stroke-linecap=\"{{{{ path.linecap }}}}\" stroke-linejoin=\"{{{{ path.linejoin }}}}\"{{% if path.clipped %}} clip-path=\"url(#{})\"{{% endif %}}></path>{{% endfor %}}</g><g class=\"cui-chart__labels\">{{% for label in {scene}.labels %}}<text x=\"{{{{ label.x }}}}\" y=\"{{{{ label.y }}}}\" transform=\"{{{{ label.transform }}}}\" text-anchor=\"{{{{ label.anchor }}}}\" dominant-baseline=\"{{{{ label.baseline }}}}\" font-size=\"{{{{ label.font_size }}}}\" font-weight=\"{{{{ label.font_weight }}}}\">{{{{ label.text }}}}</text>{{% endfor %}}</g></svg>",
        escape(&clip_id),
        escape(&clip_id)
    );
    let table = format!(
        "<table class=\"cui-chart__data\"><caption>Values for {label}</caption><thead><tr><th scope=\"col\">UTC time (milliseconds)</th><th scope=\"col\">Value</th></tr></thead><tbody>{{% for sample in {data_path}.samples.items %}}<tr><td>{{{{ sample.time }}}}</td><td>{{% if sample.missing %}}Missing{{% else %}}{{{{ sample.value }}}}{{% endif %}}</td></tr>{{% else %}}<tr><td colspan=\"2\">No samples in this range.</td></tr>{{% endfor %}}</tbody></table>"
    );
    let mapping = format!(
        "<span class=\"cui-chart__point-map\" hidden aria-hidden=\"true\">{{% for point in {scene}.points %}}<span data-cui-chart-point data-key=\"{{{{ point.key }}}}\" data-time=\"{{{{ point.time }}}}\" data-value=\"{{{{ point.value }}}}\" data-missing=\"{{{{ point.missing }}}}\" data-x=\"{{{{ point.x }}}}\" data-y=\"{{{{ point.y }}}}\"></span>{{% endfor %}}</span>"
    );
    let fragment = static_fragment(package, "chart")?;
    let html = fill_slots(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[caption_attributes]]", &caption_attributes),
            ("[[label]]", &label),
            ("[[svg]]", &svg),
            ("[[table]]", &table),
            ("[[mapping]]", &mapping),
        ],
    )?;
    Ok((html, enhance == "true"))
}

pub(super) fn script_path() -> &'static str {
    SCRIPT
}
