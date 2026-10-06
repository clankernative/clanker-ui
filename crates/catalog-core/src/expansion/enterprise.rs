//! Narrow expansion rules for the five enterprise components and their private helpers.
use super::*;

const MAX_ITEMS: usize = 100;

pub(super) fn take_body(input: &str, cursor: &mut usize, name: &str) -> Result<String> {
    let close = format!("</{name}");
    let open = format!("<{name}");
    let start = *cursor;
    let mut depth = 1usize;
    while *cursor < input.len() {
        let Some(offset) = input[*cursor..].find('<') else {
            break;
        };
        let at = *cursor + offset;
        if input[at..].starts_with("<!--") {
            *cursor = input[at + 4..]
                .find("-->")
                .map(|end| at + end + 7)
                .context("unterminated comment in Native UI component")?;
            continue;
        }
        if input[at..].starts_with(&close) {
            let end = composition::tag_end(input, at)?;
            depth -= 1;
            if depth == 0 {
                let body = input[start..at].to_owned();
                *cursor = end;
                return Ok(body);
            }
            *cursor = end;
            continue;
        }
        if input[at..].starts_with(&open) {
            let (end, _, self_closing) = parse_opening(input, at, &open)?;
            if !self_closing {
                depth += 1;
            }
            *cursor = end;
            continue;
        }
        *cursor = at + 1;
    }
    anyhow::bail!("unterminated cui-{name}")
}

pub(super) fn render<const N: usize>(
    index: usize,
    declaration: &Button,
    body: Option<&str>,
    package: &Package,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<String> {
    let html = match index {
        26 => select_field(declaration, body.unwrap_or(""), package),
        27 => filter_bar(
            declaration,
            body.context("cui-filter-bar requires paired slots")?,
            package,
            counts,
            bindings,
        ),
        28 => data_table(
            declaration,
            body.context("cui-data-table requires paired rows")?,
            package,
            counts,
            bindings,
        ),
        29 => breadcrumbs(
            declaration,
            body.context("cui-breadcrumbs requires paired crumbs")?,
            package,
        ),
        30 => pagination(
            declaration,
            body.context("cui-pagination requires paired pages")?,
            package,
        ),
        _ => anyhow::bail!("unknown enterprise component index"),
    }?;
    // These item grammars are already fully validated by their renderer. Record
    // their symbolic fields as well as root/recursively composed declarations.
    if let Some(body) = body {
        let item = match index {
            26 => Some("<cui-option"),
            29 => Some("<cui-crumb"),
            30 => Some("<cui-page"),
            _ => None,
        };
        if let Some(item) = item {
            let mut cursor = 0;
            while let Some((tag, end)) = next_tag(body, &mut cursor)? {
                cursor = end;
                if !tag.strip_prefix(item).is_some_and(|tail| {
                    tail.starts_with(char::is_whitespace) || tail.starts_with('/')
                }) {
                    continue;
                }
                let (declaration, _) = attrs_for_tag(tag, item)?;
                for (attribute, value) in declaration.attrs {
                    if let Some(path) = interpolation(&value) {
                        bindings.push(Binding {
                            field_path: path.to_owned(),
                            expected_kind: binding_kind(
                                item.trim_start_matches("<cui-"),
                                &attribute,
                            )
                            .context("unsupported item binding")?
                            .into(),
                            component: item.trim_start_matches("<cui-").into(),
                            attribute,
                        });
                    }
                }
            }
        }
    }
    Ok(html)
}

fn attrs<'a>(
    decl: &'a Button,
    allowed: &[&str],
    component: &str,
) -> Result<&'a BTreeMap<String, String>> {
    checked_attributes(decl, allowed, component)?;
    Ok(&decl.attrs)
}
fn token(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':')),
        "invalid {field} token"
    );
    Ok(())
}
fn plain(value: &str, field: &str) -> Result<String> {
    checked_template_text(value, field)
}
fn field_or_plain(value: &str, field: &str, allow_blank: bool) -> Result<String> {
    field_text(value, field, false, allow_blank)
}
fn attrs_for_tag(source: &str, name: &str) -> Result<(Button, bool)> {
    let (end, decl, closed) = parse_opening(source, 0, name)?;
    ensure!(end == source.len(), "malformed {name} declaration");
    Ok((decl, closed))
}
fn next_tag<'a>(source: &'a str, cursor: &mut usize) -> Result<Option<(&'a str, usize)>> {
    let Some(offset) = source[*cursor..].find('<') else {
        *cursor = source.len();
        return Ok(None);
    };
    let start = *cursor + offset;
    *cursor = start;
    let end = composition::tag_end(source, start)?;
    Ok(Some((&source[start..end], end)))
}
fn control_flow(text: &str) -> Option<String> {
    let mut output = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let whitespace = rest.len() - rest.trim_start().len();
        output.push_str(&rest[..whitespace]);
        rest = &rest[whitespace..];
        if rest.is_empty() {
            break;
        }
        if !rest.starts_with("{%") {
            return None;
        }
        let end = rest.find("%}")? + 2;
        let statement = &rest[..end];
        let body = statement[2..statement.len() - 2].trim();
        if !(body.starts_with("for ")
            || body.starts_with("if ")
            || body.starts_with("elif ")
            || body == "else"
            || body == "endif"
            || body == "endfor")
        {
            return None;
        }
        output.push_str(statement);
        rest = &rest[end..];
    }
    Some(output)
}
fn interp_text(value: &str, context: &str) -> Result<String> {
    if let Some(path) = interpolation(value) {
        return Ok(format!("{{{{ ui_text({path}, \"nonblank\", 1, 0) }}}}"));
    }
    ensure!(
        !value.contains("{{") && !value.contains("}}"),
        "{context} must be a literal or whole checked field"
    );
    ensure!(
        !value.chars().any(char::is_control),
        "{context} contains control characters"
    );
    Ok(escape(value))
}
fn selected_attribute(value: &str) -> Result<String> {
    if let Some(path) = interpolation(value) {
        Ok(format!(
            " data-ui-choice-value=\"{{{{ ui_text({path}, \"plain\", 0, 0) }}}}\""
        ))
    } else {
        ensure!(
            !value.contains("{{") && !value.contains("}}"),
            "selected must be literal or a whole checked field"
        );
        Ok(format!(" data-ui-choice-value=\"{}\"", escape(value)))
    }
}
fn comparison(value: &str) -> Result<String> {
    if let Some(path) = interpolation(value) {
        Ok(format!("ui_text({path}, \"plain\", 0, 0)"))
    } else {
        ensure!(
            !value.contains("{{") && !value.contains("}}"),
            "selection must be a literal or whole checked field"
        );
        Ok(format!(
            "ui_text({}, \"plain\", 0, 0)",
            serde_json::to_string(value)?
        ))
    }
}

fn select_field(decl: &Button, body: &str, package: &Package) -> Result<String> {
    let a = attrs(
        decl,
        &[
            "id",
            "name",
            "label",
            "selected",
            "placeholder",
            "hint",
            "error",
            "required",
            "disabled",
            "choices",
        ],
        "select-field",
    )?;
    let id = a.get("id").context("cui-select-field requires id")?;
    let name = a.get("name").context("cui-select-field requires name")?;
    let id = &control_id(id, "select-field")?;
    token(name, "select name")?;
    ensure!(!name.starts_with('_'), "reserved select name");
    let label = plain(
        a.get("label").context("cui-select-field requires label")?,
        "select label",
    )?;
    let required = a.get("required").map(String::as_str).unwrap_or("false");
    let disabled = a.get("disabled").map(String::as_str).unwrap_or("false");
    ensure!(
        ["true", "false"].contains(&required) && ["true", "false"].contains(&disabled),
        "required and disabled must be literal booleans"
    );
    let selected = a.get("selected").map(String::as_str);
    let selected_attr = selected
        .map(selected_attribute)
        .transpose()?
        .unwrap_or_default();
    let placeholder = a
        .get("placeholder")
        .map(|p| interp_text(p, "select placeholder"))
        .transpose()?;
    let choices_json = a.get("choices");
    ensure!(
        choices_json.is_none() || body.trim().is_empty(),
        "select choices JSON and paired choices cannot be combined"
    );
    let mut options = String::new();
    if let Some(json) = choices_json {
        let values: Vec<serde_json::Value> =
            serde_json::from_str(json).context("choices must be a typed JSON array")?;
        ensure!(
            !values.is_empty() && values.len() <= MAX_ITEMS,
            "select choices must contain 1..=100 choices"
        );
        for v in values {
            let obj = v.as_object().context("select choice must be an object")?;
            ensure!(
                obj.keys()
                    .all(|k| ["value", "label", "disabled"].contains(&k.as_str())),
                "unknown select choice property"
            );
            let value = obj
                .get("value")
                .and_then(|v| v.as_str())
                .context("choice value must be a string")?;
            let label = obj
                .get("label")
                .and_then(|v| v.as_str())
                .context("choice label must be a string")?;
            let disabled = match obj.get("disabled") {
                Some(value) => value
                    .as_bool()
                    .context("choice disabled must be a boolean")?,
                None => false,
            };
            options.push_str(&option_markup(
                value,
                label,
                if disabled { "true" } else { "false" },
                selected,
            )?);
        }
    } else {
        let mut static_count = 0;
        let mut cursor = 0;
        while cursor < body.len() {
            let segment_start = cursor;
            let Some((tag, end)) = next_tag(body, &mut cursor)? else {
                let tail = control_flow(&body[segment_start..])
                    .context("select accepts only choices and template control flow")?;
                options.push_str(&tail);
                break;
            };
            let between = control_flow(&body[segment_start..cursor])
                .context("select accepts only choices and template control flow")?;
            options.push_str(&between);
            cursor = end;
            if tag.starts_with("</") {
                anyhow::bail!("unexpected close tag in select")
            }
            let (d, self_closing) = attrs_for_tag(tag, "<cui-choice")?;
            ensure!(self_closing, "cui-choice must be self-closing");
            checked_attributes(&d, &["value", "label", "disabled"], "choice")?;
            let value = d.attrs.get("value").context("choice requires value")?;
            let label = d.attrs.get("label").context("choice requires label")?;
            let disabled = d
                .attrs
                .get("disabled")
                .map(String::as_str)
                .unwrap_or("false");
            ensure!(
                ["true", "false"].contains(&disabled),
                "choice disabled must be literal boolean"
            );
            options.push_str(&option_markup(value, label, disabled, selected)?);
            static_count += 1;
            ensure!(
                static_count <= MAX_ITEMS,
                "select static choices exceed 100"
            );
        }
        let tail = control_flow(&body[cursor..]).context("invalid select choice body")?;
        options.push_str(&tail);
        ensure!(
            static_count > 0 || body.contains("{% for "),
            "select requires at least one choice"
        );
    }
    let mut desc = String::new();
    let mut described = Vec::new();
    for (key, suffix) in [("hint", "hint"), ("error", "error")] {
        if let Some(text) = a.get(key) {
            desc.push_str(&format!(
                "<p class=\"cui-select-field__{suffix}\" id=\"{}-{suffix}\">{}</p>",
                escape(id),
                field_text(text, key, true, false)?
            ));
            described.push(format!("{id}-{suffix}"));
        }
    }
    if let Some(placeholder) = placeholder {
        let selected_placeholder = if selected.is_none() { " selected" } else { "" };
        options.insert_str(0, &format!("<option data-ui-placeholder value=\"\" disabled{selected_placeholder}>{placeholder}</option>"));
    }
    let control = format!(
        "<select class=\"cui-select-field__control\" id=\"{}\" name=\"{}\" data-ui-choice-set=\"true\"{selected_attr}{}{}{}{}>{options}</select>",
        escape(id),
        escape(name),
        if described.is_empty() {
            String::new()
        } else {
            format!(" aria-describedby=\"{}\"", escape(&described.join(" ")))
        },
        if a.contains_key("error") {
            " aria-invalid=\"true\""
        } else {
            ""
        },
        if required == "true" { " required" } else { "" },
        if disabled == "true" { " disabled" } else { "" }
    );
    let mark = if required == "true" {
        " <span class=\"cui-select-field__required\" aria-hidden=\"true\">*</span>"
    } else {
        ""
    };
    let label_html = format!(
        "<label class=\"cui-select-field__label\" for=\"{}\">{}{mark}</label>",
        escape(id),
        label
    );
    let attributes = format!(
        "class=\"cui-select-field{}\" data-cui-component=\"select-field\"",
        if a.contains_key("error") {
            " cui-select-field--invalid"
        } else {
            ""
        }
    );
    fill_static(
        static_fragment(package, "select-field")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label_html),
            ("[[control]]", &control),
            ("[[description]]", &desc),
        ],
    )
}
fn option_markup(
    value: &str,
    label: &str,
    disabled: &str,
    selected: Option<&str>,
) -> Result<String> {
    let value_html = interp_text(value, "choice value")?;
    let label_html = interp_text(label, "choice label")?;
    let selection = if let Some(selected) = selected {
        let compared = comparison(value)?;
        let selected_cmp = comparison(selected)?;
        format!(
            " data-ui-selected-flag=\"{{{{ {compared} == {selected_cmp} and not ({disabled}) }}}}\""
        )
    } else {
        String::new()
    };
    Ok(format!(
        "<option value=\"{value_html}\"{}{selection}>{label_html}</option>",
        if disabled == "true" { " disabled" } else { "" }
    ))
}

fn paired_slots(body: &str, accepted: &[&str]) -> Result<BTreeMap<String, String>> {
    let mut slots = BTreeMap::new();
    let mut cursor = 0;
    while cursor < body.len() {
        let segment_start = cursor;
        let Some((tag, end)) = next_tag(body, &mut cursor)? else {
            break;
        };
        ensure!(
            body[segment_start..cursor].trim().is_empty(),
            "only direct named helper slots are allowed here"
        );
        cursor = end;
        let (d, closed) = attrs_for_tag(tag, "<cui-slot")?;
        ensure!(!closed, "cui-slot requires paired content");
        checked_attributes(&d, &["name"], "slot")?;
        let name = d.attrs.get("name").context("slot requires name")?;
        ensure!(accepted.contains(&name.as_str()), "unsupported slot {name}");
        let close = "</cui-slot>";
        let end_body = body[cursor..]
            .find(close)
            .map(|i| cursor + i)
            .context("unterminated slot")?;
        let content = body[cursor..end_body].to_owned();
        ensure!(
            slots.insert(name.clone(), content).is_none(),
            "duplicate slot {name}"
        );
        cursor = end_body + close.len();
    }
    ensure!(
        body[cursor..].trim().is_empty(),
        "invalid named slot content"
    );
    Ok(slots)
}
fn filter_bar<const N: usize>(
    decl: &Button,
    body: &str,
    package: &Package,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<String> {
    let a = attrs(decl, &["label", "summary"], "filter-bar")?;
    let label = plain(
        a.get("label").context("filter-bar requires label")?,
        "filter-bar label",
    )?;
    let slots = paired_slots(body, &["controls", "actions", "applied"])?;
    let controls = slots
        .get("controls")
        .context("filter-bar requires controls slot")?;
    ensure!(
        !controls.trim().is_empty(),
        "filter-bar controls cannot be blank"
    );
    let transformed_controls =
        transform_html_with_bindings(controls, Some(package), counts, bindings)?;
    let actions = slots
        .get("actions")
        .map(|s| transform_html_with_bindings(s, Some(package), counts, bindings))
        .transpose()?
        .unwrap_or_default();
    let applied = slots
        .get("applied")
        .map(|s| transform_html_with_bindings(s, Some(package), counts, bindings))
        .transpose()?
        .unwrap_or_default();
    let summary = a
        .get("summary")
        .map(|v| field_or_plain(v, "filter summary", false))
        .transpose()?
        .map(|v| format!("<p class=\"cui-filter-bar__summary\" role=\"status\">{v}</p>"))
        .unwrap_or_default();
    fill_static(
        static_fragment(package, "filter-bar")?,
        &[
            (
                "[[attributes]]",
                "class=\"cui-filter-bar\" role=\"group\" data-cui-component=\"filter-bar\"",
            ),
            ("[[label]]", &label),
            ("[[controls]]", &transformed_controls),
            ("[[actions]]", &actions),
            ("[[applied]]", &applied),
            ("[[summary]]", &summary),
        ],
    )
}

fn data_table<const N: usize>(
    decl: &Button,
    body: &str,
    package: &Package,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<String> {
    let a = attrs(
        decl,
        &["caption", "caption-visible", "columns"],
        "data-table",
    )?;
    let caption = plain(
        a.get("caption").context("data-table requires caption")?,
        "table caption",
    )?;
    let visible = a
        .get("caption-visible")
        .map(String::as_str)
        .unwrap_or("false");
    ensure!(
        ["true", "false"].contains(&visible),
        "caption-visible must be literal boolean"
    );
    let cols: Vec<String> = serde_json::from_str(
        a.get("columns")
            .context("data-table requires columns JSON")?,
    )
    .context("columns must be a JSON string array")?;
    ensure!(
        !cols.is_empty()
            && cols.len() <= 16
            && cols
                .iter()
                .all(|c| !c.trim().is_empty() && !c.chars().any(char::is_control)),
        "data-table needs 1..=16 nonblank safe columns"
    );
    let caption_html = format!(
        "<caption class=\"cui-data-table__caption{}\">{caption}</caption>",
        if visible == "true" {
            ""
        } else {
            " cui-visually-hidden"
        }
    );
    let head = format!(
        "<thead class=\"cui-data-table__head\"><tr>{}</tr></thead>",
        cols.iter()
            .map(|c| format!("<th scope=\"col\">{}</th>", escape(c)))
            .collect::<String>()
    );
    let mut rows = String::new();
    let mut ids = BTreeSet::new();
    let mut cursor = 0;
    let mut row_count = 0;
    while cursor < body.len() {
        let segment_start = cursor;
        let Some((tag, end)) = next_tag(body, &mut cursor)? else {
            rows.push_str(
                &control_flow(&body[segment_start..]).context("invalid data-table body")?,
            );
            break;
        };
        let between = control_flow(&body[segment_start..cursor])
            .context("data-table permits only rows and template control flow")?;
        rows.push_str(&between);
        cursor = end;
        let (row, closed) = attrs_for_tag(tag, "<cui-table-row")?;
        ensure!(!closed, "table-row must be paired");
        checked_attributes(&row, &["id", "id-prefix", "key"], "table-row")?;
        let id = if let Some(id) = row.attrs.get("id") {
            ensure!(
                !row.attrs.contains_key("id-prefix") && !row.attrs.contains_key("key"),
                "row accepts literal id or id-prefix/key"
            );
            token(id, "row id")?;
            id.clone()
        } else {
            let prefix = row
                .attrs
                .get("id-prefix")
                .context("table row requires id or id-prefix/key")?;
            token(prefix, "row id prefix")?;
            let key = row.attrs.get("key").context("id-prefix requires key")?;
            let path = interpolation(key).context("row key must be one whole checked field")?;
            format!("{prefix}-{{{{ ui_key({path}) }}}}")
        };
        if !id.contains("{{") {
            ensure!(ids.insert(id.clone()), "duplicate table row id");
        }
        let close = "</cui-table-row>";
        let body_end = body[cursor..]
            .find(close)
            .map(|p| cursor + p)
            .context("unterminated table row")?;
        let row_body = &body[cursor..body_end];
        let mut row_cursor = 0;
        let mut cells = Vec::new();
        while row_cursor < row_body.len() {
            let Some(offset) = row_body[row_cursor..].find("<cui-table-cell") else {
                break;
            };
            ensure!(
                row_body[row_cursor..row_cursor + offset].trim().is_empty(),
                "table row contains non-cell markup"
            );
            let cell_start = row_cursor + offset;
            let cell_end_tag = composition::tag_end(row_body, cell_start)?;
            let (cd, cell_closed) =
                attrs_for_tag(&row_body[cell_start..cell_end_tag], "<cui-table-cell")?;
            ensure!(!cell_closed, "table-cell must be paired");
            checked_attributes(&cd, &[], "table-cell")?;
            let content_start = cell_end_tag;
            let close_cell = "</cui-table-cell>";
            let content_end = row_body[content_start..]
                .find(close_cell)
                .map(|p| content_start + p)
                .context("unterminated table cell")?;
            let html = transform_html_with_bindings(
                &row_body[content_start..content_end],
                Some(package),
                counts,
                bindings,
            )?;
            cells.push(html);
            row_cursor = content_end + close_cell.len();
        }
        ensure!(
            row_body[row_cursor..].trim().is_empty() && cells.len() == cols.len(),
            "table row cell count must match columns"
        );
        rows.push_str(&format!(
            "<tr id=\"{}\">{}</tr>",
            id,
            cells
                .iter()
                .zip(&cols)
                .map(|(html, label)| format!("<td data-label=\"{}\">{html}</td>", escape(label)))
                .collect::<String>()
        ));
        cursor = body_end + close.len();
        row_count += 1;
        ensure!(row_count <= MAX_ITEMS, "data-table row count exceeds 100");
    }
    rows.push_str(&control_flow(&body[cursor..]).context("invalid data-table body")?);
    let attributes = format!(
        "class=\"cui-data-table\" role=\"region\" tabindex=\"0\" aria-label=\"{}\" data-cui-component=\"data-table\"",
        escape(&caption)
    );
    let body = format!("<tbody class=\"cui-data-table__body\">{rows}</tbody>");
    fill_static(
        static_fragment(package, "data-table")?,
        &[
            ("[[attributes]]", &attributes),
            ("[[caption]]", &caption_html),
            ("[[head]]", &head),
            ("[[body]]", &body),
        ],
    )
}

pub(super) fn destination(value: &str) -> Result<String> {
    if let Some(inner) = value.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
        let call = inner
            .trim()
            .strip_prefix("routes.")
            .context("destination must be a whole named route call")?;
        let (name, rest) = call
            .split_once('(')
            .context("destination requires a route call")?;
        let args = rest
            .strip_suffix(')')
            .context("malformed destination route call")?;
        // Route existence, argument types, reference codecs, and emission provenance
        // are checked by ordinary Native template admission. Keep this declaration
        // grammar narrower: named arguments with whole fields or scalar literals.
        route_destination(name, "destination")?;
        let mut names = BTreeSet::new();
        ensure!(
            args.trim().is_empty() || args.split(',').all(|part| !part.trim().is_empty()),
            "empty destination route argument"
        );
        for argument in args.split(',').filter(|s| !s.trim().is_empty()) {
            let (key, raw) = argument
                .split_once('=')
                .context("route arguments must be named")?;
            let key = key.trim();
            ensure!(
                key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && key
                        .as_bytes()
                        .first()
                        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                    && names.insert(key)
                    && names.len() <= 32,
                "invalid or duplicate destination route argument"
            );
            let raw = raw.trim();
            ensure!(
                interpolation(&format!("{{{{ {raw} }}}}")).is_some()
                    || raw.parse::<u64>().is_ok()
                    || matches!(raw, "true" | "false"),
                "destination argument must be a whole checked field or scalar literal"
            );
        }
        ensure!(
            !args.trim_end().ends_with(','),
            "trailing destination argument separator"
        );
        return Ok(format!("{{{{ routes.{name}({args}) }}}}"));
    }
    ensure!(safe_href(value), "unsafe internal destination");
    Ok(escape(value))
}
fn breadcrumbs(decl: &Button, body: &str, package: &Package) -> Result<String> {
    let a = attrs(decl, &["label"], "breadcrumbs")?;
    let label = plain(
        a.get("label").context("breadcrumbs requires label")?,
        "breadcrumbs label",
    )?;
    let mut items = String::from("<ol class=\"cui-breadcrumbs__list\">");
    let mut cursor = 0;
    let mut current = false;
    let mut ancestors = 0;
    let mut count = 0;
    while cursor < body.len() {
        let segment_start = cursor;
        let Some((tag, end)) = next_tag(body, &mut cursor)? else {
            break;
        };
        ensure!(
            body[segment_start..cursor].trim().is_empty(),
            "breadcrumbs accepts only crumb helpers"
        );
        cursor = end;
        let (d, closed) = attrs_for_tag(tag, "<cui-crumb")?;
        ensure!(closed, "crumb must be self-closing");
        checked_attributes(&d, &["label", "href", "current"], "crumb")?;
        let text = interp_text(
            d.attrs.get("label").context("crumb requires label")?,
            "crumb label",
        )?;
        let is_current = d.attrs.get("current").map(String::as_str) == Some("true");
        ensure!(
            !d.attrs.contains_key("current") || is_current,
            "current must be literal true"
        );
        if is_current {
            ensure!(
                !current && !d.attrs.contains_key("href"),
                "current crumb cannot link and must occur once"
            );
            current = true;
            items.push_str(&format!("<li class=\"cui-breadcrumbs__item\"><span class=\"cui-breadcrumbs__current\" aria-current=\"page\">{text}</span></li>"));
        } else {
            ensure!(!current, "current crumb must be last");
            let href = d
                .attrs
                .get("href")
                .context("ancestor crumb requires a safe href or named route call")?;
            items.push_str(&format!("<li class=\"cui-breadcrumbs__item\"><a class=\"cui-breadcrumbs__link\" href=\"{}\">{text}</a></li>", destination(href)?));
            ancestors += 1;
        }
        count += 1;
    }
    ensure!(
        body[cursor..].trim().is_empty() && ancestors > 0 && current && count >= 2,
        "breadcrumbs require one or more ancestors and exactly one current-last crumb"
    );
    items.push_str("</ol>");
    fill_static(
        static_fragment(package, "breadcrumbs")?,
        &[
            (
                "[[attributes]]",
                "class=\"cui-breadcrumbs\" data-cui-component=\"breadcrumbs\" data-ui-navigation=\"true\" data-ui-navigation-minimum-items=\"2\" data-ui-navigation-current-last=\"true\" data-ui-navigation-ancestor-links=\"true\"",
            ),
            ("[[label]]", &label),
            ("[[items]]", &items),
        ],
    )
}
fn pagination(decl: &Button, body: &str, package: &Package) -> Result<String> {
    let a = attrs(decl, &["label", "variant"], "pagination")?;
    let label = plain(
        a.get("label").context("pagination requires label")?,
        "pagination label",
    )?;
    let variant = a.get("variant").map(String::as_str).unwrap_or("standard");
    ensure!(
        ["standard", "outlined", "compact"].contains(&variant),
        "invalid pagination variant"
    );
    let mut items = String::from("<ol class=\"cui-pagination__list\">");
    let mut cursor = 0;
    let mut currents = 0;
    let mut count = 0;
    while cursor < body.len() {
        let segment_start = cursor;
        let Some((tag, end)) = next_tag(body, &mut cursor)? else {
            items.push_str(
                &control_flow(&body[segment_start..]).context("invalid pagination body")?,
            );
            break;
        };
        let between = control_flow(&body[segment_start..cursor])
            .context("pagination accepts only page helpers and template control flow")?;
        items.push_str(&between);
        cursor = end;
        let (d, closed) = attrs_for_tag(tag, "<cui-page")?;
        ensure!(closed, "cui-page must be self-closing");
        checked_attributes(
            &d,
            &["kind", "label", "href", "disabled", "current"],
            "page",
        )?;
        let kind = d.attrs.get("kind").context("page requires kind")?;
        ensure!(
            ["previous", "page", "current", "next", "gap"].contains(&kind.as_str()),
            "invalid page kind"
        );
        let text = interp_text(
            d.attrs.get("label").context("page requires label")?,
            "page label",
        )?;
        let disabled = d
            .attrs
            .get("disabled")
            .map(String::as_str)
            .unwrap_or("false");
        ensure!(
            ["true", "false"].contains(&disabled),
            "page disabled must be literal boolean"
        );
        let current = d.attrs.get("current").map(String::as_str);
        ensure!(
            kind == "current" || current.is_none(),
            "current is only valid on kind=current"
        );
        items.push_str(&format!(
            "<li class=\"cui-pagination__item cui-pagination__item--{kind}\">"
        ));
        if kind == "current" {
            currents += 1;
            ensure!(
                !d.attrs.contains_key("href") && disabled == "false",
                "current page cannot link or disable"
            );
            let current_attr = match current {
                Some("true") => " aria-current=\"page\"",
                Some("false") => "",
                Some(v) if interpolation(v).is_some() => {
                    let path = interpolation(v).unwrap();
                    items.push_str(&format!("{{% if {path} %}}<span class=\"cui-pagination__current\" aria-current=\"page\">{text}</span>{{% else %}}<span>{text}</span>{{% endif %}}</li>"));
                    count += 1;
                    continue;
                }
                Some(_) => anyhow::bail!("current must be literal or whole checked boolean field"),
                None => " aria-current=\"page\"",
            };
            items.push_str(&format!(
                "<span class=\"cui-pagination__current\"{current_attr}>{text}</span>"
            ));
        } else if kind == "gap" {
            ensure!(!d.attrs.contains_key("href"), "gap cannot link");
            items.push_str(&format!(
                "<span class=\"cui-pagination__gap\" aria-hidden=\"true\">{text}</span>"
            ));
        } else if disabled == "true" {
            ensure!(
                !d.attrs.contains_key("href"),
                "disabled previous/next must not have destination"
            );
            items.push_str(&format!(
                "<span class=\"cui-pagination__disabled\" aria-disabled=\"true\">{text}</span>"
            ));
        } else {
            let href = d
                .attrs
                .get("href")
                .context("enabled pagination item requires href or named route call")?;
            let rel = if kind == "previous" {
                " rel=\"prev\""
            } else if kind == "next" {
                " rel=\"next\""
            } else {
                ""
            };
            items.push_str(&format!(
                "<a class=\"cui-pagination__link\" href=\"{}\"{rel}>{text}</a>",
                destination(href)?
            ));
        }
        items.push_str("</li>");
        count += 1;
        ensure!(count <= MAX_ITEMS, "pagination item count exceeds 100");
    }
    items.push_str(&control_flow(&body[cursor..]).context("invalid pagination body")?);
    ensure!(currents > 0, "pagination requires a current page item");
    items.push_str("</ol>");
    fill_static(
        static_fragment(package, "pagination")?,
        &[
            (
                "[[attributes]]",
                &format!(
                    "class=\"cui-pagination cui-pagination--{variant}\" data-cui-component=\"pagination\" data-ui-navigation=\"true\" data-ui-navigation-minimum-items=\"1\" data-ui-navigation-current-last=\"false\""
                ),
            ),
            ("[[label]]", &label),
            ("[[items]]", &items),
        ],
    )
}
