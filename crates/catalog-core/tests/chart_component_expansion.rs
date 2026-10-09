use catalog_core::expansion::{self, Package};
use std::{collections::BTreeMap, fs, path::Path};

fn package() -> Package {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let mut assets = BTreeMap::new();
    fn collect(root: &Path, dir: &Path, assets: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, assets);
            } else {
                assets.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    collect(&root, &root, &mut assets);
    Package::from_assets(&assets).unwrap()
}

#[test]
fn chart_expands_typed_scene_markup_and_accessible_zero_gap_table() {
    let package = package();
    let result = expansion::expand(
        r#"<cui-chart id="latency-week" data="{{ chart.chart }}" label="Latency <week>" />"#,
        &package,
    )
    .unwrap();
    assert!(result.used_components.contains("chart"));
    assert!(result.html.contains("id=\"latency-week\""));
    assert!(result.html.contains("id=\"latency-week-caption\""));
    assert!(result.html.contains("id=\"latency-week-clip\""));
    assert!(result
        .html
        .contains("aria-labelledby=\"latency-week-caption\""));
    assert!(result
        .html
        .contains("ui_scene('echarts_chart_v1', {'width': chart.chart.width"));
    assert!(result.html.contains("'samples': chart.chart.samples.items"));
    assert!(result.html.contains("ui_integer(path.role, 0, 4)"));
    assert!(result.html.contains("Missing"));
    assert!(result.html.contains("sample.value"));
    assert!(result
        .html
        .contains("stroke-width=\"{{ path.stroke_width }}\""));
    assert!(result.html.contains("font-size=\"{{ label.font_size }}\""));
    assert!(result
        .html
        .contains("font-weight=\"{{ label.font_weight }}\""));
    assert!(result.html.contains("data-cui-chart-point"));
    assert!(result.html.contains("Latency &lt;week&gt;"));
    assert!(result.browser_scripts.is_empty());
    assert!(result.bindings.is_empty());
}

#[test]
fn enhancement_selects_only_the_locked_component_installer() {
    let package = package();
    let result = expansion::expand(
        r#"<cui-chart id="daily-chart" data="{{ report.chart }}" label="Daily chart" enhance="true" />"#,
        &package,
    )
    .unwrap();
    assert_eq!(
        result.browser_scripts,
        ["components/chart/interaction.js".to_owned()].into()
    );
    assert!(result.html.contains("data-cui-chart-enhance=\"true\""));
}

#[test]
fn chart_rejects_nonliteral_or_untyped_declarations() {
    let package = package();
    for declaration in [
        r#"<cui-chart data="{{ chart.chart }}" label="Chart" />"#,
        r#"<cui-chart id="chart" data="{{ make_chart() }}" label="Chart" />"#,
        r#"<cui-chart id="chart" data="{{ 9chart.chart }}" label="Chart" />"#,
        r#"<cui-chart id="chart" data="{{ chart.chart }}" label="{% include 'other.html' %}" />"#,
        r#"<cui-chart id="chart" data="{{ chart.chart }}" label="{# hidden comment #}" />"#,
        r#"<cui-chart id="chart" data="{{ chart.samples[0] }}" label="Chart" />"#,
        r#"<cui-chart id="chart" data='{"samples":[]}' label="Chart" />"#,
        r#"<cui-chart id="chart" data="{{ chart.chart }}" label="{{ title }}" />"#,
        r#"<cui-chart id="chart" data="{{ chart.chart }}" label="Chart" enhance="yes" />"#,
        r#"<cui-chart id="chart" data="{{ chart.chart }}" label="Chart" title="extra" />"#,
        r#"<cui-chart id="chart-caption" data="{{ chart.chart }}" label="Chart"><span>bad</span></cui-chart>"#,
    ] {
        assert!(
            expansion::expand(declaration, &package).is_err(),
            "accepted {declaration}"
        );
    }
}
