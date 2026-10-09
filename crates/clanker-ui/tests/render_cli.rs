use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
fn request(ui: &Path, scene: &Path, fragment: Option<&str>) -> std::process::Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new(env!("CARGO_BIN_EXE_clanker-ui"));
    command
        .args(["render", "--lock"])
        .arg(root.join("examples/button-app/ui.lock.json"))
        .arg("--ui")
        .arg(ui)
        .arg("--scene")
        .arg(scene);
    if let Some(fragment) = fragment {
        command.arg("--fragment").arg(fragment);
    }
    command.output().unwrap()
}
fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let ui = tmp.path().join("ui");
    fs::create_dir_all(ui.join("pages")).unwrap();
    fs::create_dir_all(ui.join("components")).unwrap();
    fs::write(ui.join("app.css"), ".preview{color:var(--cui-text)}").unwrap();
    fs::write(ui.join("pages/index.html"),"<main class=\"preview\"><cui-page-header title=\"{{ title }}\" />{% include 'components/message.html' %}<a href=\"{{ routes.workspace() }}\">Workspace</a><cui-button label=\"Continue\" variant=\"{{ variant }}\" /></main>").unwrap();
    fs::write(ui.join("components/message.html"), "<p>{{ message }}</p>").unwrap();
    let scene = tmp.path().join("scene.json");
    fs::write(&scene,serde_json::to_vec(&json!({"page":"pages/index.html","data":{"title":"Preview","message":"<script>alert(1)</script>","variant":1},"routes":{"workspace":"/workspace"},"width":1280})).unwrap()).unwrap();
    (tmp, ui, scene)
}
#[test]
fn renders_expansion_includes_guarded_values_routes_and_embedded_fonts_without_scripts() {
    let (_tmp, ui, scene) = fixture();
    let output = request(&ui, &scene, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let data = &value["data"];
    let html = data["html"].as_str().unwrap();
    assert_eq!(value["command"], "render");
    assert_eq!(data["width"], 1280);
    assert!(data["packageDigest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<h1"));
    assert!(html.contains("Preview"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("<cui-"));
    assert!(html.contains("href=\"/workspace\""));
    assert!(html.contains("cui-button--secondary"));
    assert!(html.contains("data:font/woff2;base64,"));
    assert!(html.contains("form-action 'none'"));
    assert_eq!(
        fs::read_to_string(ui.join("components/message.html")).unwrap(),
        "<p>{{ message }}</p>"
    );
}
#[test]
fn standalone_preview_rejects_live_worker_backed_charts() {
    let (_tmp, ui, scene) = fixture();
    fs::write(
        ui.join("pages/index.html"),
        r#"<cui-chart id="latency" data="{{ chart.chart }}" label="Latency" />"#,
    )
    .unwrap();
    fs::write(
        &scene,
        r#"{"page":"pages/index.html","data":{"chart":{"chart":{}}}}"#,
    )
    .unwrap();
    let output = request(&ui, &scene, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("cannot preview live worker-backed charts"));
}

#[test]
fn fragment_uses_the_same_capture_and_scene_but_no_page_wrapper() {
    let (_tmp, ui, scene) = fixture();
    let output = request(&ui, &scene, Some("ui/components/message.html"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["template"], "components/message.html");
    assert!(!value["data"]["html"].as_str().unwrap().contains("<h1"));
}
#[test]
fn missing_data_unsafe_markup_and_invalid_runtime_values_fail_closed() {
    let (_tmp, ui, scene) = fixture();
    fs::write(
        &scene,
        r#"{"page":"pages/index.html","data":{},"routes":{"workspace":"/"}}"#,
    )
    .unwrap();
    assert!(!request(&ui, &scene, None).status.success());
    fs::write(ui.join("components/message.html"), "<script>bad()</script>").unwrap();
    fs::write(&scene, r#"{"page":"components/message.html","data":{}}"#).unwrap();
    assert!(!request(&ui, &scene, None).status.success());
    fs::write(
        ui.join("components/message.html"),
        "{{ ui_integer(variant, 0, 3) }}",
    )
    .unwrap();
    fs::write(
        &scene,
        r#"{"page":"components/message.html","data":{"variant":99}}"#,
    )
    .unwrap();
    assert!(!request(&ui, &scene, None).status.success());
    assert!(!request(&ui, &scene, Some("../outside.html"))
        .status
        .success());
}
