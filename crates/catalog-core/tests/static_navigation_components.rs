#[path = "../src/button.rs"]
mod button;
#[path = "../src/fragment.rs"]
mod fragment;
#[path = "../src/icon.rs"]
mod icon;
#[path = "../src/progress_steps.rs"]
mod progress_steps;
#[path = "../src/segmented_control.rs"]
mod segmented_control;
#[path = "../src/tabs.rs"]
mod tabs;

const TAB_FRAGMENT: &str = "<nav [[attributes]] aria-label=\"[[label]]\">[[items]]</nav>";
const SEGMENT_FRAGMENT: &str = "<nav [[attributes]] aria-label=\"[[label]]\">[[items]]</nav>";
const STEP_FRAGMENT: &str = "<nav [[attributes]] aria-label=\"[[label]]\">[[items]]</nav>";

#[test]
fn tabs_golden_linked_navigation_and_accessible_current() {
    let value = tabs::TabsInstance::parse(include_str!(
        "../../../packages/vanilla/components/tabs/fixtures/typical.json"
    ))
    .unwrap();
    let html = tabs::render(&value, TAB_FRAGMENT).unwrap();
    assert!(html.starts_with(
        "<nav class=\"cui-tabs\" data-cui-component=\"tabs\" aria-label=\"Repository sections\">"
    ));
    assert_eq!(html.matches("aria-current=\"page\"").count(), 1);
    assert!(html.contains("href=\"/settings/members\" aria-current=\"page\">Members</a>"));
    assert!(html.contains("<ul class=\"cui-tabs__list\">"));
    assert_eq!(html, tabs::render(&value, TAB_FRAGMENT).unwrap());
}
#[test]
fn tabs_localized_escaping_and_contract_rejections() {
    let value = tabs::TabsInstance::parse(include_str!(
        "../../../packages/vanilla/components/tabs/fixtures/escaped-localized.json"
    ))
    .unwrap();
    let html = tabs::render(&value, TAB_FRAGMENT).unwrap();
    assert!(html.contains("Réglages &amp; accès"));
    assert!(html.contains("Équipe &lt;12&gt;"));
    assert!(tabs::TabsInstance::parse(r#"{"items":[{"label":"A","href":"/a","active":true},{"label":"B","href":"/b","active":true}]}"#).is_err());
    assert!(
        tabs::TabsInstance::parse(r#"{"items":[{"label":"A","href":"/a","active":true}]}"#)
            .is_err()
    );
    assert!(tabs::TabsInstance::parse(r#"{"items":[{"label":"A","href":"javascript:alert(1)","active":true},{"label":"B","href":"/b"}]}"#).is_err());
    assert!(tabs::TabsInstance::parse(
        r#"{"items":[{"label":"A","href":"/a","active":true},{"label":"B","href":"/b"}],"extra":1}"#
    )
    .is_err());
    assert!(tabs::render(&value, "[[attributes]][[label]]").is_err());

    let mut maximum = tabs::TabsInstance {
        label: "Many sections".into(),
        items: (0..tabs::MAX_TABS)
            .map(|index| tabs::TabItem {
                label: format!("Section {index}"),
                href: format!("?section={index}"),
                active: index == 0,
            })
            .collect(),
    };
    assert!(maximum.validate().is_ok());
    maximum.items.push(tabs::TabItem {
        label: "Too many".into(),
        href: "/extra".into(),
        active: false,
    });
    assert!(maximum.validate().is_err());
}
#[test]
fn segmented_control_golden_variants_and_localized_escaping() {
    let value = segmented_control::SegmentedControlInstance::parse(include_str!(
        "../../../packages/vanilla/components/segmented-control/fixtures/typical.json"
    ))
    .unwrap();
    let html = segmented_control::render(&value, SEGMENT_FRAGMENT).unwrap();
    assert!(html.contains("cui-segmented-control__link"));
    assert!(html.contains("aria-current=\"page\">Shared"));
    assert!(html.contains("aria-disabled=\"true\">Archived"));
    assert_eq!(html.matches("aria-current=\"page\"").count(), 1);
    let equal = segmented_control::SegmentedControlInstance::parse(include_str!(
        "../../../packages/vanilla/components/segmented-control/fixtures/equal-width.json"
    ))
    .unwrap();
    assert!(segmented_control::render(&equal, SEGMENT_FRAGMENT)
        .unwrap()
        .contains("cui-segmented-control--equal"));
    let localized = segmented_control::SegmentedControlInstance::parse(include_str!(
        "../../../packages/vanilla/components/segmented-control/fixtures/escaped-localized.json"
    ))
    .unwrap();
    let html = segmented_control::render(&localized, SEGMENT_FRAGMENT).unwrap();
    assert!(html.contains("Affichage &amp; filtres"));
    assert!(html.contains("Tout &lt;voir&gt;"));
}
#[test]
fn segmented_control_rejects_bad_counts_states_href_and_unknown_enum() {
    for json in [
        r#"{"label":"x","items":[{"kind":"current","label":"a"}]}"#,
        r#"{"label":"x","items":[{"kind":"current","label":"a"},{"kind":"current","label":"b"}]}"#,
        r#"{"label":"x","items":[{"kind":"link","href":"/a","label":"a"},{"kind":"current","label":"b"},{"kind":"link","href":"/c","label":"c"},{"kind":"link","href":"/d","label":"d"},{"kind":"link","href":"/e","label":"e"},{"kind":"link","href":"/f","label":"f"},{"kind":"link","href":"/g","label":"g"},{"kind":"link","href":"/h","label":"h"},{"kind":"link","href":"/i","label":"i"}]}"#,
        r#"{"label":"x","items":[{"kind":"link","label":"a"},{"kind":"current","label":"b"}]}"#,
        r#"{"label":"x","items":[{"kind":"current","href":"/a","label":"a"},{"kind":"link","href":"/b","label":"b"}]}"#,
        r#"{"label":"x","items":[{"kind":"disabled","href":"/a","label":"a"},{"kind":"current","label":"b"}]}"#,
        r#"{"label":"x","items":[{"kind":"current","label":"a"},{"kind":"unknown","label":"b"}]}"#,
    ] {
        assert!(
            segmented_control::SegmentedControlInstance::parse(json).is_err(),
            "{json}"
        );
    }
    assert!(segmented_control::SegmentedControlInstance::parse(r#"{"label":"x","items":[{"kind":"link","href":"//evil.test","label":"a"},{"kind":"current","label":"b"}]}"#).is_err());
}
#[test]
fn progress_steps_golden_states_modes_and_custom_state_labels() {
    let value = progress_steps::ProgressStepsInstance::parse(include_str!(
        "../../../packages/vanilla/components/progress-steps/fixtures/typical.json"
    ))
    .unwrap();
    let html = progress_steps::render(&value, STEP_FRAGMENT).unwrap();
    assert!(html.contains("cui-progress-steps--horizontal cui-progress-steps--detailed"));
    assert!(html.contains("aria-current=\"step\""));
    assert!(html.contains("Completed: "));
    assert!(html.contains("href=\"?step=request\""));
    let localized = progress_steps::ProgressStepsInstance::parse(include_str!(
        "../../../packages/vanilla/components/progress-steps/fixtures/vertical-error-localized.json"
    ))
    .unwrap();
    let html = progress_steps::render(&localized, STEP_FRAGMENT).unwrap();
    assert!(html.contains("--vertical cui-progress-steps--detailed"));
    assert!(html.contains("Action requise: "));
    assert!(html.contains("Vérification &lt;à corriger&gt;"));
    assert!(html.contains("Choisir une équipe &amp; un rôle"));
    let compact = progress_steps::ProgressStepsInstance::parse(include_str!(
        "../../../packages/vanilla/components/progress-steps/fixtures/compact.json"
    ))
    .unwrap();
    assert!(progress_steps::render(&compact, STEP_FRAGMENT)
        .unwrap()
        .contains("--compact"));

    let maximum = progress_steps::ProgressStepsInstance {
        label: "Ten step workflow".into(),
        orientation: progress_steps::Orientation::Horizontal,
        appearance: progress_steps::Appearance::Detailed,
        state_labels: progress_steps::StateLabels::default(),
        items: (0..10)
            .map(|index| progress_steps::ProgressStep {
                state: if index == 9 {
                    progress_steps::StepState::Current
                } else {
                    progress_steps::StepState::Complete
                },
                label: format!("Step {index}"),
                href: (index != 9).then(|| format!("?step={index}")),
                detail: None,
            })
            .collect(),
    };
    assert!(maximum.validate().is_ok());
    let mut too_many = maximum;
    too_many.items.insert(
        9,
        progress_steps::ProgressStep {
            state: progress_steps::StepState::Upcoming,
            label: "Extra".into(),
            href: None,
            detail: None,
        },
    );
    assert!(too_many.validate().is_err());
}
#[test]
fn progress_steps_rejects_order_count_href_unknown_fields_and_enums() {
    for json in [
        r#"{"label":"x","items":[{"state":"current","label":"a"}]}"#,
        r#"{"label":"x","items":[{"state":"current","label":"a"},{"state":"error","label":"b"}]}"#,
        r#"{"label":"x","items":[{"state":"upcoming","label":"a"},{"state":"current","label":"b"}]}"#,
        r#"{"label":"x","items":[{"state":"current","href":"/a","label":"a"},{"state":"upcoming","label":"b"}]}"#,
        r#"{"label":"x","items":[{"state":"complete","href":"javascript:bad","label":"a"},{"state":"current","label":"b"}]}"#,
        r#"{"label":"x","items":[{"state":"current","label":"a"},{"state":"complete","href":"/b","label":"b"}]}"#,
        r#"{"label":"x","orientation":"diagonal","items":[{"state":"current","label":"a"},{"state":"upcoming","label":"b"}]}"#,
        r#"{"label":"x","items":[{"state":"current","label":"a"},{"state":"upcoming","label":"b"}],"unknown":true}"#,
    ] {
        assert!(
            progress_steps::ProgressStepsInstance::parse(json).is_err(),
            "{json}"
        );
    }
}
#[test]
fn progress_steps_optional_icons_use_only_validated_closed_geometry() {
    let value = progress_steps::ProgressStepsInstance::parse(include_str!(
        "../../../packages/vanilla/components/progress-steps/fixtures/vertical-error-localized.json"
    ))
    .unwrap();
    let geometries: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
    let html = progress_steps::render_with_icons(
        &value,
        STEP_FRAGMENT,
        &geometries,
        "<svg [[attributes]]>[[geometry]]</svg>",
    )
    .unwrap();
    assert!(html.contains("data-cui-icon=\"alert\""));
    assert!(html.contains("aria-hidden=\"true\""));
    let mut unsafe_geometry = geometries;
    unsafe_geometry.insert("alert".into(), "<script>bad</script>".into());
    assert!(progress_steps::render_with_icons(
        &value,
        STEP_FRAGMENT,
        &unsafe_geometry,
        "<svg [[attributes]]>[[geometry]]</svg>"
    )
    .is_err());
}
