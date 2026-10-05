use catalog_core::layout::{render, AdmittedChildren};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const CONTAINER: &str =
    include_str!("../../../packages/vanilla/components/container/fragment.html");
const STACK: &str = include_str!("../../../packages/vanilla/components/stack/fragment.html");
const CLUSTER: &str = include_str!("../../../packages/vanilla/components/cluster/fragment.html");
const GRID: &str = include_str!("../../../packages/vanilla/components/grid/fragment.html");
const SPLIT: &str = include_str!("../../../packages/vanilla/components/split/fragment.html");
const CARD: &str = include_str!("../../../packages/vanilla/components/card/fragment.html");

fn run(name: &str, fragment: &str, fixture: &str) {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let options = &fixture["options"];
    // Fixture child markup is declared separately from component options and is
    // admitted explicitly here; it is never deserialized into a production API.
    let children = fixture["children"].as_object().unwrap();
    let slots = children
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                AdmittedChildren::from_host_admitted(value.as_str().unwrap()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        render(name, options, &slots, fragment).unwrap(),
        fixture["expectedHtml"].as_str().unwrap()
    );
}

#[test]
fn container_fixture_goldens_cover_defaults_and_full_width() {
    run(
        "container",
        CONTAINER,
        include_str!("../../../packages/vanilla/components/container/fixtures/typical.json"),
    );
    run(
        "container",
        CONTAINER,
        include_str!("../../../packages/vanilla/components/container/fixtures/full-width.json"),
    );
}
#[test]
fn stack_fixture_goldens_cover_default_and_alignment_values() {
    run(
        "stack",
        STACK,
        include_str!("../../../packages/vanilla/components/stack/fixtures/typical.json"),
    );
    run(
        "stack",
        STACK,
        include_str!("../../../packages/vanilla/components/stack/fixtures/alignment.json"),
    );
}
#[test]
fn cluster_fixture_goldens_cover_defaults_and_no_wrap_between() {
    run(
        "cluster",
        CLUSTER,
        include_str!("../../../packages/vanilla/components/cluster/fixtures/typical.json"),
    );
    run(
        "cluster",
        CLUSTER,
        include_str!("../../../packages/vanilla/components/cluster/fixtures/nowrap.json"),
    );
}
#[test]
fn grid_fixture_goldens_cover_intrinsic_and_fixed_columns() {
    run(
        "grid",
        GRID,
        include_str!("../../../packages/vanilla/components/grid/fixtures/typical.json"),
    );
    run(
        "grid",
        GRID,
        include_str!("../../../packages/vanilla/components/grid/fixtures/fixed-columns.json"),
    );
}
#[test]
fn split_fixture_goldens_preserve_named_start_then_end_slots() {
    run(
        "split",
        SPLIT,
        include_str!("../../../packages/vanilla/components/split/fixtures/typical.json"),
    );
    run(
        "split",
        SPLIT,
        include_str!("../../../packages/vanilla/components/split/fixtures/start-wide.json"),
    );
}
#[test]
fn card_fixture_goldens_cover_optional_heading_and_actions() {
    run(
        "card",
        CARD,
        include_str!("../../../packages/vanilla/components/card/fixtures/typical.json"),
    );
    run(
        "card",
        CARD,
        include_str!("../../../packages/vanilla/components/card/fixtures/heading-level.json"),
    );
    run(
        "card",
        CARD,
        include_str!("../../../packages/vanilla/components/card/fixtures/escaped.json"),
    );
}

#[test]
fn rejects_unknown_components_options_slots_and_malformed_fragments() {
    let body = BTreeMap::from([(
        "body".into(),
        AdmittedChildren::from_host_admitted("<p>admitted</p>"),
    )]);
    for (name, options) in [
        ("stack", json!({"gap":"huge"})),
        ("stack", json!({"rawHtml":"<b>no</b>"})),
        ("stack", json!({"alignment":"stretch","mystery":true})),
    ] {
        assert!(render(
            name,
            &options,
            &body,
            "<div class=\"[[class]]\">[[body]]</div>"
        )
        .is_err());
    }
    assert!(render("masonry", &json!({}), &body, "[[class]][[body]]").is_err());
    assert!(render("stack", &json!({}), &body, "").is_err());
    assert!(render("stack", &json!({}), &body, "[[class]][[class]][[body]]").is_err());
    let extraneous = BTreeMap::from([
        ("body".into(), AdmittedChildren::from_host_admitted("")),
        ("other".into(), AdmittedChildren::from_host_admitted("")),
    ]);
    assert!(render("stack", &json!({}), &extraneous, "[[class]][[body]]").is_err());
}

#[test]
fn rejects_card_blank_or_control_text_and_forbidden_slots() {
    let slots = BTreeMap::from([("body".into(), AdmittedChildren::from_host_admitted("body"))]);
    for title in ["", " \t", "line\nbreak"] {
        assert!(render("card", &json!({"title":title}), &slots, CARD).is_err());
    }
    for subtitle in ["", "\r"] {
        assert!(render("card", &json!({"subtitle":subtitle}), &slots, CARD).is_err());
    }
    assert!(render("card", &json!({}), &BTreeMap::new(), CARD).is_err());
    let raw = BTreeMap::from([
        ("body".into(), AdmittedChildren::from_host_admitted("")),
        ("content".into(), AdmittedChildren::from_host_admitted("")),
    ]);
    assert!(render("card", &json!({}), &raw, CARD).is_err());
}

#[test]
fn toolframe_layout_fixture_goldens_cover_every_primary_variant() {
    for (name, fragment, fixtures) in [
        (
            "cover",
            include_str!("../../../packages/vanilla/components/cover/fragment.html"),
            vec!["compact", "standard", "fill", "viewport"],
        ),
        (
            "layer",
            include_str!("../../../packages/vanilla/components/layer/fragment.html"),
            vec![
                "center",
                "top-start",
                "top-end",
                "bottom-start",
                "bottom-end",
                "stretch",
            ],
        ),
        (
            "pane",
            include_str!("../../../packages/vanilla/components/pane/fragment.html"),
            vec![
                "content",
                "compact",
                "standard",
                "fill",
                "viewport",
                "content-scroll",
                "bounded-no-scroll",
            ],
        ),
        (
            "reel",
            include_str!("../../../packages/vanilla/components/reel/fragment.html"),
            vec!["narrow", "standard", "wide", "content"],
        ),
        (
            "sidebar",
            include_str!("../../../packages/vanilla/components/sidebar/fragment.html"),
            vec!["start", "end"],
        ),
        (
            "switch",
            include_str!("../../../packages/vanilla/components/switch/fragment.html"),
            vec!["natural", "equal"],
        ),
    ] {
        for variant in fixtures {
            let path =
                format!("../../../packages/vanilla/components/{name}/fixtures/{variant}.json");
            let fixture = match (name, variant) {
                ("cover", "compact") => {
                    include_str!("../../../packages/vanilla/components/cover/fixtures/compact.json")
                }
                ("cover", "standard") => include_str!(
                    "../../../packages/vanilla/components/cover/fixtures/standard.json"
                ),
                ("cover", "fill") => {
                    include_str!("../../../packages/vanilla/components/cover/fixtures/fill.json")
                }
                ("cover", "viewport") => include_str!(
                    "../../../packages/vanilla/components/cover/fixtures/viewport.json"
                ),
                ("layer", "center") => {
                    include_str!("../../../packages/vanilla/components/layer/fixtures/center.json")
                }
                ("layer", "top-start") => include_str!(
                    "../../../packages/vanilla/components/layer/fixtures/top-start.json"
                ),
                ("layer", "top-end") => {
                    include_str!("../../../packages/vanilla/components/layer/fixtures/top-end.json")
                }
                ("layer", "bottom-start") => include_str!(
                    "../../../packages/vanilla/components/layer/fixtures/bottom-start.json"
                ),
                ("layer", "bottom-end") => include_str!(
                    "../../../packages/vanilla/components/layer/fixtures/bottom-end.json"
                ),
                ("layer", "stretch") => {
                    include_str!("../../../packages/vanilla/components/layer/fixtures/stretch.json")
                }
                ("pane", "content") => {
                    include_str!("../../../packages/vanilla/components/pane/fixtures/content.json")
                }
                ("pane", "compact") => {
                    include_str!("../../../packages/vanilla/components/pane/fixtures/compact.json")
                }
                ("pane", "standard") => {
                    include_str!("../../../packages/vanilla/components/pane/fixtures/standard.json")
                }
                ("pane", "fill") => {
                    include_str!("../../../packages/vanilla/components/pane/fixtures/fill.json")
                }
                ("pane", "viewport") => {
                    include_str!("../../../packages/vanilla/components/pane/fixtures/viewport.json")
                }
                ("pane", "content-scroll") => include_str!(
                    "../../../packages/vanilla/components/pane/fixtures/content-scroll.json"
                ),
                ("pane", "bounded-no-scroll") => include_str!(
                    "../../../packages/vanilla/components/pane/fixtures/bounded-no-scroll.json"
                ),
                ("reel", "narrow") => {
                    include_str!("../../../packages/vanilla/components/reel/fixtures/narrow.json")
                }
                ("reel", "standard") => {
                    include_str!("../../../packages/vanilla/components/reel/fixtures/standard.json")
                }
                ("reel", "wide") => {
                    include_str!("../../../packages/vanilla/components/reel/fixtures/wide.json")
                }
                ("reel", "content") => {
                    include_str!("../../../packages/vanilla/components/reel/fixtures/content.json")
                }
                ("sidebar", "start") => {
                    include_str!("../../../packages/vanilla/components/sidebar/fixtures/start.json")
                }
                ("sidebar", "end") => {
                    include_str!("../../../packages/vanilla/components/sidebar/fixtures/end.json")
                }
                ("switch", "natural") => include_str!(
                    "../../../packages/vanilla/components/switch/fixtures/natural.json"
                ),
                ("switch", "equal") => {
                    include_str!("../../../packages/vanilla/components/switch/fixtures/equal.json")
                }
                _ => panic!("fixture not wired: {path}"),
            };
            run(name, fragment, fixture);
        }
    }
}

#[test]
fn rejects_invalid_toolframe_layout_options_slots_and_scroll_intent() {
    let primary = BTreeMap::from([("primary".into(), AdmittedChildren::from_host_admitted(""))]);
    for options in [
        json!({"height":"giant"}),
        json!({"height":"compact","unknown":true}),
    ] {
        assert!(render(
            "cover",
            &options,
            &primary,
            include_str!("../../../packages/vanilla/components/cover/fragment.html")
        )
        .is_err());
    }
    let pane = BTreeMap::from([("body".into(), AdmittedChildren::from_host_admitted(""))]);
    assert!(render(
        "pane",
        &json!({"bodyScrolling":"yes"}),
        &pane,
        include_str!("../../../packages/vanilla/components/pane/fragment.html")
    )
    .is_err());
    let body = BTreeMap::from([("body".into(), AdmittedChildren::from_host_admitted(""))]);
    for label in ["", "  ", "line\nbreak"] {
        assert!(render(
            "reel",
            &json!({"label":label}),
            &body,
            include_str!("../../../packages/vanilla/components/reel/fragment.html")
        )
        .is_err());
    }
    let slots = BTreeMap::from([
        ("base".into(), AdmittedChildren::from_host_admitted("")),
        (
            "foreground".into(),
            AdmittedChildren::from_host_admitted(""),
        ),
        ("extra".into(), AdmittedChildren::from_host_admitted("")),
    ]);
    assert!(render(
        "layer",
        &json!({}),
        &slots,
        include_str!("../../../packages/vanilla/components/layer/fragment.html")
    )
    .is_err());

    let base_foreground = BTreeMap::from([
        ("base".into(), AdmittedChildren::from_host_admitted("")),
        (
            "foreground".into(),
            AdmittedChildren::from_host_admitted(""),
        ),
    ]);
    let two_regions = BTreeMap::from([
        ("main".into(), AdmittedChildren::from_host_admitted("")),
        ("aside".into(), AdmittedChildren::from_host_admitted("")),
    ]);
    for (name, options, slots, fragment) in [
        (
            "layer",
            json!({"rawHtml":"<b>no</b>"}),
            &base_foreground,
            include_str!("../../../packages/vanilla/components/layer/fragment.html"),
        ),
        (
            "sidebar",
            json!({"width":"huge"}),
            &two_regions,
            include_str!("../../../packages/vanilla/components/sidebar/fragment.html"),
        ),
    ] {
        assert!(render(name, &options, slots, fragment).is_err());
    }
    assert!(render(
        "switch",
        &json!({"extra":true}),
        &body,
        include_str!("../../../packages/vanilla/components/switch/fragment.html")
    )
    .is_err());
    let escaped = render(
        "reel",
        &json!({"label":"A & <B> \"C\""}),
        &body,
        include_str!("../../../packages/vanilla/components/reel/fragment.html"),
    )
    .unwrap();
    assert!(escaped.contains("aria-label=\"A &amp; &lt;B&gt; &quot;C&quot;\""));
}

#[test]
fn split_requires_start_and_end_and_never_accepts_arbitrary_slot_names() {
    let slots = BTreeMap::from([
        ("first".into(), AdmittedChildren::from_host_admitted("A")),
        ("second".into(), AdmittedChildren::from_host_admitted("B")),
    ]);
    assert!(render("split", &json!({}), &slots, SPLIT).is_err());
}
