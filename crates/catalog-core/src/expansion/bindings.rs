//! Checked Native UI bindings for values whose types are enforced by template admission.
use super::*;
use serde_json::to_string as json_string;

fn progress_argument(value: &str) -> Result<String> {
    if let Some(path) = interpolation(value) {
        return Ok(path.to_owned());
    }
    ensure!(
        !value.contains("{{") && !value.contains("}}"),
        "progress binding must be a whole checked page field"
    );
    let valid =
        value.parse::<u64>().is_ok() || value.parse::<f64>().is_ok_and(|number| number.is_finite());
    ensure!(valid, "progress numeric literal must be finite");
    Ok(json_string(value)?)
}

/// Render determinate progress values that may come from checked numeric fields.
pub(super) fn render_progress(progress: &Button, package: &Package) -> Result<String> {
    let state = progress
        .attrs
        .get("state")
        .context("cui-progress requires state")?;
    if state != "determinate" {
        // The literal renderer owns the indeterminate rules and error messages.
        return render_progress_literal(progress, package);
    }

    let value_text = progress
        .attrs
        .get("value")
        .context("determinate progress requires value and maximum")?;
    let maximum_text = progress
        .attrs
        .get("maximum")
        .context("determinate progress requires value and maximum")?;
    let value_arg = progress_argument(value_text)?;
    let maximum_arg = progress_argument(maximum_text)?;

    // This static placeholder is never used as runtime state; ordinary template
    // expressions below validate and render the original exact operands.
    let mut literal = progress.clone();
    literal.attrs.insert("value".into(), "0".into());
    literal.attrs.insert("maximum".into(), "1".into());
    let mut html = render_progress_literal(&literal, package)?;

    let track = "<progress class=\"cui-progress__track\" value=\"0\" max=\"1\" aria-label=\"";
    let dynamic_track = format!(
        "<progress class=\"cui-progress__track\" value=\"{{{{ ui_number({value_arg}, \"0\", {maximum_arg}, false) }}}}\" max=\"{{{{ ui_number({maximum_arg}, \"0\", none, true) }}}}\" aria-label=\""
    );
    ensure!(
        html.contains(track),
        "progress renderer produced unexpected track markup"
    );
    html = html.replacen(track, &dynamic_track, 1);

    // Literal completion is only a placeholder artifact; completion is decided
    // by the runtime helper over the original checked arguments.
    let mut class_end = html
        .find("\" data-cui-component=\"progress\"")
        .context("progress renderer produced unexpected class markup")?;
    let completion = " cui-progress--complete";
    if html[..class_end].ends_with(completion) {
        html.replace_range(class_end - completion.len()..class_end, "");
        class_end -= completion.len();
    }
    html.insert_str(
        class_end,
        &format!("{{% if ui_compare({value_arg}, {maximum_arg}) == 0 %}} cui-progress--complete{{% endif %}}"),
    );
    Ok(html)
}

/// Render an avatar with optional admitted local or checked remote image source.
pub(super) fn render_avatar(avatar: &Button, package: &Package) -> Result<String> {
    checked_attributes(
        avatar,
        &["initials", "label", "size", "tone", "src", "image-asset"],
        "avatar",
    )?;
    ensure!(
        !(avatar.attrs.contains_key("src") && avatar.attrs.contains_key("image-asset")),
        "avatar src and image-asset are mutually exclusive"
    );
    let initials = avatar
        .attrs
        .get("initials")
        .context("cui-avatar requires initials")?;
    let bound_initials = interpolation(initials);
    let rendered_initials = if let Some(path) = bound_initials {
        Some(format!("{{{{ ui_text({path}, \"nonblank\", 1, 3) }}}}"))
    } else {
        ensure!(
            !initials.contains("{{") && !initials.contains("}}"),
            "avatar initials binding must be a whole checked page field"
        );
        None
    };

    let image_src = if let Some(key) = avatar.attrs.get("image-asset") {
        ensure!(
            !key.trim().is_empty()
                && !key.chars().any(char::is_control)
                && !key.contains("{{")
                && !key.contains("}}"),
            "avatar image-asset must be a literal admitted asset key"
        );
        Some(format!("{{{{ asset({}) }}}}", json_string(key)?))
    } else if let Some(src) = avatar.attrs.get("src") {
        if let Some(path) = interpolation(src) {
            Some(format!("{{{{ ui_image({path}) }}}}"))
        } else {
            ensure!(
                !src.contains("{{") && !src.contains("}}"),
                "avatar src must be an HTTPS literal or whole checked page field"
            );
            crate::template_values::validate_remote_image_source(src)
                .context("avatar src must be a valid HTTPS image source")?;
            Some(format!("{{{{ ui_image({}) }}}}", json_string(src)?))
        }
    } else {
        None
    };

    let mut literal = avatar.clone();
    literal.attrs.remove("src");
    literal.attrs.remove("image-asset");
    if bound_initials.is_some() {
        literal.attrs.insert("initials".into(), "X".into());
    }
    let mut html = render_avatar_literal(&literal, package)?;
    if let Some(dynamic_initials) = rendered_initials {
        let initials_markup = format!(
            "<span class=\"cui-avatar__initials\" aria-hidden=\"true\">{}</span>",
            escape("X")
        );
        ensure!(
            html.contains(&initials_markup),
            "avatar renderer produced unexpected initials markup"
        );
        html = html.replacen(
            &initials_markup,
            &format!("<span class=\"cui-avatar__initials\" aria-hidden=\"true\">{dynamic_initials}</span>"),
            1,
        );
    }
    if let Some(src) = image_src {
        let close = html
            .rfind("</span>")
            .context("avatar renderer produced unexpected outer markup")?;
        html.insert_str(
            close,
            &format!("<img class=\"cui-avatar__image\" src=\"{src}\" alt=\"\" aria-hidden=\"true\" referrerpolicy=\"no-referrer\" loading=\"lazy\" decoding=\"async\">"),
        );
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_arguments_only_allow_literals_or_whole_fields() {
        assert!(progress_argument("page.count").is_err());
        assert!(progress_argument("{{ page.count }}").is_ok());
        assert!(progress_argument("{{ page.count + 1 }}").is_err());
        assert!(progress_argument("NaN").is_err());
        assert!(progress_argument("Infinity").is_err());
    }

    #[test]
    fn json_literal_arguments_are_quoted() {
        assert_eq!(progress_argument("12.5").unwrap(), "\"12.5\"");
        assert_eq!(
            progress_argument("18446744073709551615").unwrap(),
            "\"18446744073709551615\""
        );
    }
}
