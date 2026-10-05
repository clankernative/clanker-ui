use crate::ports::LoadedPackage;
use catalog_core::css_check::{ControlledProperty, PageFacts, RuleFacts, SelectorFacts};
use catalog_core::expansion::Package;
use catalog_core::tokens::{
    CssDeclaration, CssRule, CssStylesheet, CssValue, SourceLocation, TokenCatalog, ValuePart,
};
use cssparser::{Parser, ParserInput, ToCss, Token};
use scraper::{Html, Selector};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component as PathComponent, Path};

const MAX_FILE: usize = 1_048_576;
const MAX_FILES: usize = 512;
const MAX_TOTAL: usize = 32 * 1_048_576;
const MAX_RULES: usize = 20_000;
const MAX_DECLARATIONS: usize = 100_000;
const MAX_DEPTH: usize = 64;
const MAX_TREE_ENTRIES: usize = 4_096;

#[derive(Clone, Debug)]
pub struct UiInspection {
    pub files: Vec<String>,
    pub rules: Vec<RuleFacts>,
    pub pages: PageFacts,
    pub limitations: Vec<String>,
}

fn loc(path: &str, line: u32, column: u32) -> SourceLocation {
    SourceLocation {
        file: path.to_owned(),
        line: line + 1,
        column,
    }
}

fn token_text(token: &Token<'_>) -> String {
    token.to_css_string()
}
fn is_space(token: &Token<'_>) -> bool {
    matches!(token, Token::WhiteSpace(_) | Token::Comment(_))
}
fn next_token<'i>(
    p: &mut Parser<'i, '_>,
) -> Result<Option<(Token<'i>, cssparser::SourceLocation)>, String> {
    if p.is_exhausted() {
        return Ok(None);
    }
    match p.next_including_whitespace_and_comments() {
        Ok(t) => Ok(Some((t.clone(), p.current_source_location()))),
        Err(e) => Err(format!("CSS tokenizer error: {e:?}")),
    }
}

fn parse_value(raw: &str, depth: usize) -> Result<CssValue, String> {
    if depth > MAX_DEPTH {
        return Err("CSS value nesting limit exceeded".into());
    }
    let mut input = ParserInput::new(raw);
    let mut parser = Parser::new(&mut input);
    parse_value_tokens(&mut parser, depth)
}

fn parse_value_tokens(parser: &mut Parser<'_, '_>, depth: usize) -> Result<CssValue, String> {
    if depth > MAX_DEPTH {
        return Err("CSS value nesting limit exceeded".into());
    }
    let mut parts = Vec::new();
    let mut text = String::new();
    while let Some((token, _)) = next_token(parser)? {
        if let Token::Function(name) = &token {
            if name.eq_ignore_ascii_case("var") {
                let nested = parser
                    .parse_nested_block(|inner| {
                        collect_css_tokens(inner)
                            .map_err(|error| inner.new_custom_error::<_, String>(error))
                    })
                    .map_err(|e| format!("invalid var() value: {e}"))?;
                let (variable, fallback) = split_var_args(&nested)?;
                if !variable.starts_with("--") || variable.len() <= 2 {
                    return Err("invalid var() custom-property name".into());
                }
                if !text.is_empty() {
                    parts.push(ValuePart::Text(std::mem::take(&mut text)));
                }
                parts.push(ValuePart::Var {
                    name: variable,
                    fallback: fallback.map(|s| parse_value(&s, depth + 1)).transpose()?,
                });
            } else {
                text.push_str(&token_text(&token));
                flush_value_text(&mut parts, &mut text);
                let nested = parser
                    .parse_nested_block(|inner| {
                        parse_value_tokens(inner, depth + 1)
                            .map_err(|error| inner.new_custom_error::<_, String>(error))
                    })
                    .map_err(|e| format!("invalid nested CSS function: {e}"))?;
                parts.extend(nested.parts);
                text.push(')');
            }
        } else if matches!(
            token,
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock
        ) {
            let open = token_text(&token);
            text.push_str(&open);
            flush_value_text(&mut parts, &mut text);
            let nested = parser
                .parse_nested_block(|inner| {
                    parse_value_tokens(inner, depth + 1)
                        .map_err(|error| inner.new_custom_error::<_, String>(error))
                })
                .map_err(|e| format!("invalid nested CSS block: {e}"))?;
            parts.extend(nested.parts);
            text.push_str(match token {
                Token::ParenthesisBlock => ")",
                Token::SquareBracketBlock => "]",
                _ => "}",
            });
        } else {
            text.push_str(&token_text(&token));
        }
    }
    if !text.is_empty() || parts.is_empty() {
        parts.push(ValuePart::Text(text));
    }
    Ok(CssValue { parts })
}

fn flush_value_text(parts: &mut Vec<ValuePart>, text: &mut String) {
    if !text.is_empty() {
        parts.push(ValuePart::Text(std::mem::take(text)));
    }
}

fn split_var_args(raw: &str) -> Result<(String, Option<String>), String> {
    let mut input = ParserInput::new(raw);
    let mut p = Parser::new(&mut input);
    let name = loop {
        let Some((token, _)) = next_token(&mut p)? else {
            return Err("empty var() argument".into());
        };
        if is_space(&token) {
            continue;
        }
        if let Token::Ident(name) = token {
            break name.to_string();
        }
        return Err("invalid var() custom-property name".into());
    };
    let mut fallback = None;
    while let Some((token, _)) = next_token(&mut p)? {
        if is_space(&token) {
            continue;
        }
        if matches!(token, Token::Comma) {
            fallback = Some(collect_css_tokens(&mut p)?);
            break;
        }
        return Err("invalid var() argument".into());
    }
    if !name.starts_with("--") || name.len() <= 2 {
        return Err("invalid var() custom-property name".into());
    }
    Ok((name, fallback.map(|value| value.trim().to_owned())))
}

fn collect_css_tokens(parser: &mut Parser<'_, '_>) -> Result<String, String> {
    collect_css_tokens_at_depth(parser, 0)
}

fn collect_css_tokens_at_depth(
    parser: &mut Parser<'_, '_>,
    depth: usize,
) -> Result<String, String> {
    if depth > MAX_DEPTH {
        return Err("CSS value nesting limit exceeded".into());
    }
    let mut result = String::new();
    while let Some((token, _)) = next_token(parser)? {
        result.push_str(&token_text(&token));
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) {
            let nested = parser
                .parse_nested_block(|inner| {
                    collect_css_tokens_at_depth(inner, depth + 1)
                        .map_err(|error| inner.new_custom_error::<_, String>(error))
                })
                .map_err(|error| format!("malformed CSS value: {error}"))?;
            result.push_str(&nested);
            if matches!(token, Token::Function(_) | Token::ParenthesisBlock) {
                result.push(')');
            } else if matches!(token, Token::SquareBracketBlock) {
                result.push(']');
            } else {
                result.push('}');
            }
        }
    }
    Ok(result)
}

fn parse_declarations(
    parser: &mut Parser<'_, '_>,
    path: &str,
    count: &mut usize,
    source_len: usize,
) -> Result<Vec<CssDeclaration>, String> {
    let mut declarations = Vec::new();
    loop {
        let start = parser.current_source_location();
        let mut name = String::new();
        let mut value = String::new();
        let mut colon = false;
        let mut ended = false;
        while let Some((token, _)) = next_token(parser)? {
            match token {
                Token::Semicolon => {
                    ended = true;
                    break;
                }
                Token::Comment(_) => {}
                Token::Colon if !colon => colon = true,
                Token::CurlyBracketBlock => {
                    return Err(format!(
                        "unexpected nested block in declaration at {}:{}",
                        path, start.line
                    ));
                }
                other => {
                    let mut serialized = token_text(&other);
                    if colon
                        && matches!(
                            other,
                            Token::Function(_)
                                | Token::ParenthesisBlock
                                | Token::SquareBracketBlock
                                | Token::CurlyBracketBlock
                        )
                    {
                        let nested = parser
                            .parse_nested_block(|inner| {
                                collect_css_tokens(inner)
                                    .map_err(|error| inner.new_custom_error::<_, String>(error))
                            })
                            .map_err(|error| format!("malformed nested CSS value: {error}"))?;
                        serialized.push_str(&nested);
                        serialized.push_str(match other {
                            Token::Function(_) | Token::ParenthesisBlock => ")",
                            Token::SquareBracketBlock => "]",
                            _ => "}",
                        });
                    }
                    if colon {
                        value.push_str(&serialized);
                    } else {
                        name.push_str(&serialized);
                    }
                }
            }
        }
        let name = name.trim();
        let value = value.trim();
        if !colon {
            if name.is_empty() && !ended {
                break;
            }
            if !name.is_empty() {
                return Err(format!(
                    "malformed CSS declaration at {}:{}",
                    path, start.line
                ));
            }
            if !ended {
                break;
            }
            continue;
        }
        if name.is_empty() {
            return Err(format!("empty CSS property at {}:{}", path, start.line));
        }
        *count += 1;
        if *count > MAX_DECLARATIONS {
            return Err("CSS declaration limit exceeded".into());
        }
        let (value, important) = strip_important(value)?;
        declarations.push(CssDeclaration {
            name: if name.starts_with("--") {
                name.to_owned()
            } else {
                name.to_ascii_lowercase()
            },
            value: value.to_owned(),
            expression: parse_value(&value, 0)?,
            important,
            location: loc(path, start.line, start.column),
        });
        if !ended {
            break;
        }
    }
    if parser.position().byte_index() >= source_len {
        return Err(format!("unclosed CSS declaration block in {path}"));
    }
    Ok(declarations)
}

fn strip_important(value: &str) -> Result<(String, bool), String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let mut pieces = Vec::<(Token<'_>, String)>::new();
    while let Some((token, _)) = next_token(&mut parser)? {
        let mut text = token_text(&token);
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) {
            let nested = parser
                .parse_nested_block(|inner| {
                    collect_css_tokens(inner)
                        .map_err(|error| inner.new_custom_error::<_, String>(error))
                })
                .map_err(|error| format!("malformed CSS value: {error}"))?;
            text.push_str(&nested);
            text.push_str(match token {
                Token::Function(_) | Token::ParenthesisBlock => ")",
                Token::SquareBracketBlock => "]",
                _ => "}",
            });
        }
        pieces.push((token, text));
    }
    if let Some(last) = pieces.iter().rposition(|(token, _)| !is_space(token)) {
        if matches!(&pieces[last].0, Token::Ident(name) if name.eq_ignore_ascii_case("important")) {
            if let Some(bang) = (0..last).rposition(|index| !is_space(&pieces[index].0)) {
                if matches!(pieces[bang].0, Token::Delim('!')) {
                    return Ok((
                        pieces[..bang]
                            .iter()
                            .map(|(_, text)| text.as_str())
                            .collect::<String>()
                            .trim_end()
                            .to_owned(),
                        true,
                    ));
                }
            }
        }
    }
    Ok((
        pieces.iter().map(|(_, text)| text.as_str()).collect(),
        false,
    ))
}

// Recursive CSS parsing threads parser state, source metadata, nesting, and shared output limits.
#[allow(clippy::too_many_arguments)]
fn parse_rules(
    parser: &mut Parser<'_, '_>,
    path: &str,
    owner: Option<&str>,
    source_len: usize,
    nested: bool,
    conditions: &[String],
    depth: usize,
    rules: &mut Vec<CssRule>,
    count: &mut usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("CSS rule nesting limit exceeded".into());
    }
    loop {
        let start = parser.current_source_location();
        let mut prelude = String::new();
        let mut at_name: Option<String> = None;
        let mut saw_block = false;
        while let Some((token, _)) = next_token(parser)? {
            match token {
                Token::CurlyBracketBlock => {
                    saw_block = true;
                    let trimmed = prelude.trim().to_owned();
                    if trimmed.is_empty() {
                        return Err(format!("empty CSS rule at {}:{}", path, start.line));
                    }
                    if let Some(at) = at_name.as_deref() {
                        if matches!(
                            at.to_ascii_lowercase().as_str(),
                            "media" | "supports" | "layer" | "container" | "document" | "scope"
                        ) {
                            let mut nested_conditions = conditions.to_vec();
                            nested_conditions.push(trimmed.clone());
                            parser
                                .parse_nested_block(|inner| {
                                    parse_rules(
                                        inner,
                                        path,
                                        owner,
                                        source_len,
                                        true,
                                        &nested_conditions,
                                        depth + 1,
                                        rules,
                                        count,
                                    )
                                    .map_err(|e| inner.new_custom_error::<_, String>(e))
                                })
                                .map_err(|e| format!("invalid nested CSS at-rule: {e}"))?;
                        } else if at.eq_ignore_ascii_case("font-face") {
                            let declarations = parser
                                .parse_nested_block(|inner| {
                                    parse_declarations(inner, path, count, source_len)
                                        .map_err(|e| inner.new_custom_error::<_, String>(e))
                                })
                                .map_err(|e| {
                                    format!("invalid @font-face at {}:{}: {e}", path, start.line)
                                })?;
                            *count += 1;
                            rules.push(CssRule {
                                selector: "@font-face".into(),
                                conditions: conditions.to_vec(),
                                declarations,
                                location: loc(path, start.line, start.column),
                            });
                        } else if at.to_ascii_lowercase().contains("keyframes") {
                            // Keyframe selectors are animation steps, not document selectors.
                            parser
                                .parse_nested_block(|inner| {
                                    skip_nested(inner, depth + 1)
                                        .map_err(|e| inner.new_custom_error::<_, String>(e))
                                })
                                .map_err(|e| {
                                    format!("invalid keyframes at {}:{}: {e}", path, start.line)
                                })?;
                        } else {
                            parser
                                .parse_nested_block(|inner| {
                                    skip_nested(inner, depth + 1)
                                        .map_err(|e| inner.new_custom_error::<_, String>(e))
                                })
                                .map_err(|e| format!("invalid at-rule block: {e}"))?;
                        }
                    } else {
                        let declarations = parser
                            .parse_nested_block(|inner| {
                                parse_declarations(inner, path, count, source_len)
                                    .map_err(|e| inner.new_custom_error::<_, String>(e))
                            })
                            .map_err(|e| {
                                format!(
                                    "invalid CSS declaration block at {}:{}: {e}",
                                    path, start.line
                                )
                            })?;
                        *count += 1;
                        if *count > MAX_RULES {
                            return Err("CSS rule limit exceeded".into());
                        }
                        rules.push(CssRule {
                            selector: trimmed,
                            conditions: conditions.to_vec(),
                            declarations,
                            location: loc(path, start.line, start.column),
                        });
                    }
                    break;
                }
                Token::Semicolon => break,
                Token::AtKeyword(name) if prelude.trim().is_empty() => {
                    at_name = Some(name.to_string());
                    prelude.push('@');
                    prelude.push_str(&name);
                }
                other
                    if matches!(
                        other,
                        Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock
                    ) =>
                {
                    prelude.push_str(&token_text(&other));
                    let nested = parser
                        .parse_nested_block(|inner| {
                            collect_css_tokens(inner)
                                .map_err(|error| inner.new_custom_error::<_, String>(error))
                        })
                        .map_err(|error| format!("malformed CSS rule prelude: {error}"))?;
                    prelude.push_str(&nested);
                    prelude.push_str(match other {
                        Token::Function(_) | Token::ParenthesisBlock => ")",
                        Token::SquareBracketBlock => "]",
                        _ => unreachable!(),
                    });
                }
                other => prelude.push_str(&token_text(&other)),
            }
        }
        if !saw_block && prelude.trim().is_empty() {
            break;
        }
        if !saw_block && at_name.is_none() && !prelude.trim().is_empty() {
            return Err(format!("CSS rule missing block at {}:{}", path, start.line));
        }
        if !saw_block && at_name.is_some() && !prelude.trim().is_empty() { /* valid statement at-rule */
        }
        if !saw_block && parser.is_exhausted() {
            break;
        }
    }
    if nested && parser.position().byte_index() >= source_len {
        return Err(format!("unclosed nested CSS rule in {path}"));
    }
    let _ = owner;
    Ok(())
}

fn skip_nested(parser: &mut Parser<'_, '_>, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("CSS nesting limit exceeded".into());
    }
    while let Some((token, _)) = next_token(parser)? {
        if matches!(
            token,
            Token::CurlyBracketBlock | Token::ParenthesisBlock | Token::SquareBracketBlock
        ) {
            parser
                .parse_nested_block(|inner| {
                    skip_nested(inner, depth + 1)
                        .map_err(|e| inner.new_custom_error::<_, String>(e))
                })
                .map_err(|e| format!("malformed nested CSS: {e}"))?;
        }
    }
    Ok(())
}

pub fn parse_stylesheet(
    path: &str,
    owner: Option<&str>,
    source: &str,
) -> Result<CssStylesheet, String> {
    if source.len() > MAX_FILE {
        return Err(format!("CSS file exceeds {MAX_FILE} bytes: {path}"));
    }
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut rules = Vec::new();
    let mut declarations = 0;
    parse_rules(
        &mut parser,
        path,
        owner,
        source.len(),
        false,
        &[],
        0,
        &mut rules,
        &mut declarations,
    )?;
    if rules.len() > MAX_RULES {
        return Err("CSS rule limit exceeded".into());
    }
    Ok(CssStylesheet {
        path: path.to_owned(),
        owner: owner.map(str::to_owned),
        rules,
    })
}

#[derive(Default)]
struct Captured {
    files: BTreeMap<String, Vec<u8>>,
    total: usize,
}
fn capture_tree(root: &Path) -> Result<Captured, String> {
    fn walk(
        base: &Path,
        here: &Path,
        out: &mut Captured,
        depth: usize,
        entry_count: &mut usize,
    ) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("UI directory nesting limit exceeded".into());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(here).map_err(|e| format!("cannot read UI directory: {e}"))? {
            *entry_count += 1;
            if *entry_count > MAX_TREE_ENTRIES {
                return Err(format!("UI tree entry limit exceeded ({MAX_TREE_ENTRIES})"));
            }
            entries.push(entry.map_err(|e| e.to_string())?);
        }
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            let kind = meta.file_type();
            if kind.is_symlink() {
                return Err(format!(
                    "symlink is not allowed in UI tree: {}",
                    path.display()
                ));
            }
            if kind.is_dir() {
                walk(base, &path, out, depth + 1, entry_count)?;
                continue;
            }
            if !kind.is_file() {
                return Err(format!("special file is not allowed: {}", path.display()));
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "html" | "css") {
                continue;
            }
            let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
            if rel
                .components()
                .any(|c| !matches!(c, PathComponent::Normal(_)))
            {
                return Err("unsafe relative UI path".into());
            }
            if out.files.len() >= MAX_FILES {
                return Err(format!("UI file limit exceeded ({MAX_FILES})"));
            }
            let size = meta.len() as usize;
            if size > MAX_FILE {
                return Err(format!(
                    "UI file exceeds {MAX_FILE} bytes: {}",
                    rel.display()
                ));
            }
            let key = rel.to_string_lossy().replace('\\', "/");
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            if bytes.len() > MAX_FILE {
                return Err(format!(
                    "UI file exceeds {MAX_FILE} bytes: {}",
                    rel.display()
                ));
            }
            out.total = out
                .total
                .checked_add(bytes.len())
                .ok_or_else(|| "UI tree byte count overflow".to_owned())?;
            if out.total > MAX_TOTAL {
                return Err(format!("UI tree exceeds {MAX_TOTAL} bytes"));
            }
            out.files.insert(key, bytes);
        }
        Ok(())
    }
    let root_metadata =
        fs::symlink_metadata(root).map_err(|e| format!("cannot read UI directory: {e}"))?;
    if root_metadata.file_type().is_symlink() {
        return Err(format!(
            "symlink UI root is not allowed: {}",
            root.display()
        ));
    }
    if !root_metadata.is_dir() {
        return Err(format!("UI root is not a directory: {}", root.display()));
    }
    let mut captured = Captured::default();
    let mut entry_count = 0;
    walk(root, root, &mut captured, 0, &mut entry_count)?;
    Ok(captured)
}

fn split_selector_list(selector: &str) -> Vec<String> {
    let mut input = ParserInput::new(selector);
    let mut p = Parser::new(&mut input);
    let mut result = Vec::new();
    let mut current = String::new();
    while let Some((token, _)) = next_token(&mut p).ok().flatten() {
        if matches!(token, Token::Comma) {
            if !current.trim().is_empty() {
                result.push(current.trim().to_owned());
            }
            current.clear();
        } else {
            current.push_str(&token_text(&token));
            if matches!(
                token,
                Token::Function(_)
                    | Token::ParenthesisBlock
                    | Token::SquareBracketBlock
                    | Token::CurlyBracketBlock
            ) {
                if let Ok(nested) = p.parse_nested_block(|inner| {
                    collect_css_tokens(inner)
                        .map_err(|error| inner.new_custom_error::<_, String>(error))
                }) {
                    current.push_str(&nested);
                    current.push_str(match token {
                        Token::Function(_) | Token::ParenthesisBlock => ")",
                        Token::SquareBracketBlock => "]",
                        _ => "}",
                    });
                }
            }
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_owned());
    }
    result
}

fn lexical_facts(selector: &str) -> (Vec<String>, Vec<String>, bool, bool, bool) {
    let mut input = ParserInput::new(selector);
    let mut p = Parser::new(&mut input);
    let mut components = BTreeSet::new();
    let mut elements = BTreeSet::new();
    let mut scope = false;
    let mut unknown = false;
    let mut class_or_id_scope = false;
    let mut component_root = false;
    let mut internal = false;
    while let Some((t, _)) = next_token(&mut p).ok().flatten() {
        match t {
            Token::Delim('.') => {
                class_or_id_scope = true;
                if let Ok(Token::Ident(name)) = p.next_including_whitespace_and_comments() {
                    if let Some((component, _)) = name.split_once("__") {
                        if component.starts_with("cui-") {
                            internal = true;
                            components.insert(component.trim_start_matches("cui-").to_owned());
                        }
                    } else if let Some(component) = name.strip_prefix("cui-") {
                        if !component.is_empty() {
                            component_root = true;
                            components.insert(component.to_owned());
                        }
                    }
                }
            }
            Token::Delim(':') => {
                if let Ok(Token::Ident(name)) = p.next_including_whitespace_and_comments() {
                    if ["root", "host"]
                        .iter()
                        .any(|s| name.eq_ignore_ascii_case(s))
                    {
                        scope = true;
                    }
                }
            }
            Token::Delim('#') | Token::IDHash(_) => class_or_id_scope = true,
            Token::Ident(name) => {
                let n = name.to_ascii_lowercase();
                if n == "body" || n == "html" {
                    scope = true;
                }
                if [
                    "html", "body", "table", "th", "td", "caption", "h1", "h2", "h3", "h4", "h5",
                    "h6", "button", "input",
                ]
                .contains(&n.as_str())
                {
                    elements.insert(n);
                }
            }
            Token::SquareBracketBlock => {
                let (is_component, component_value) = p.parse_nested_block(|q| {
                    let mut first = None;
                    while let Ok(token) = q.next_including_whitespace_and_comments() {
                        if is_space(token) { continue; }
                        first = Some(token.clone());
                        break;
                    }
                    let is_component = matches!(first, Some(Token::Ident(ref name)) if name.eq_ignore_ascii_case("data-cui-component"));
                    if !is_component { return Ok::<_, cssparser::ParseError<'_, ()>>((false, None)); }
                    while let Ok(token) = q.next_including_whitespace_and_comments() {
                        if !is_space(token) { if !matches!(token, Token::Delim('=')) { return Ok((true, None)); } break; }
                    }
                    while let Ok(token) = q.next_including_whitespace_and_comments() {
                        if is_space(token) { continue; }
                        let value = match token { Token::Ident(name) | Token::QuotedString(name) => Some(name.to_string()), _ => None };
                        return Ok((true, value));
                    }
                    Ok((true, None))
                }).unwrap_or((false, None));
                if is_component {
                    component_root = true;
                }
                if let Some(value) = component_value {
                    components.insert(value);
                }
            }
            Token::WhiteSpace(_) if component_root => internal = true,
            Token::Delim('>' | '+' | '~') if component_root => internal = true,
            Token::ParenthesisBlock | Token::Function(_) => {
                unknown = true;
            }
            _ => {}
        }
    }
    (
        components.into_iter().collect(),
        if class_or_id_scope {
            Vec::new()
        } else {
            elements.into_iter().collect()
        },
        scope,
        unknown,
        internal,
    )
}

fn component_name(
    element: scraper::ElementRef<'_>,
    catalog: &catalog_core::Catalog,
) -> Option<String> {
    element
        .ancestors()
        .filter_map(scraper::ElementRef::wrap)
        .find_map(|node| {
            node.value()
                .attr("data-cui-component")
                .map(str::to_owned)
                .or_else(|| {
                    node.value()
                        .name()
                        .strip_prefix("cui-")
                        .and_then(|name| catalog.get(name).map(|component| component.name.clone()))
                })
                .or_else(|| {
                    node.value().classes().find_map(|class| {
                        catalog.components().find_map(|component| {
                            let root = format!("cui-{}", component.name);
                            (class == root || class.starts_with(&format!("{root}__")))
                                .then(|| component.name.clone())
                        })
                    })
                })
        })
}

fn resolve_literal_includes(
    mut source: String,
    templates: &BTreeMap<String, String>,
    limitations: &mut Vec<String>,
    depth: usize,
) -> Result<String, String> {
    if depth > 16 {
        return Err("HTML include nesting limit exceeded".into());
    }
    if source.len() > MAX_TOTAL {
        return Err(format!("expanded HTML exceeds {MAX_TOTAL} bytes"));
    }
    let mut offset = 0usize;
    loop {
        let Some(relative) = source[offset..].find("{% include") else {
            break;
        };
        let start = offset + relative;
        let Some(end_rel) = source[start..].find("%}") else {
            limitations.push("dynamic or malformed template include was not resolved".into());
            break;
        };
        let end = start + end_rel + 2;
        let directive = source[start + 2..end - 2].trim();
        let rest = directive.strip_prefix("include").unwrap_or("").trim();
        let literal = rest
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .or_else(|| rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')));
        let Some(name) = literal else {
            limitations.push("dynamic template include was not resolved".into());
            offset = end;
            continue;
        };
        let name = name.replace('\\', "/");
        let safe = !name.starts_with('/')
            && Path::new(&name)
                .components()
                .all(|c| matches!(c, PathComponent::Normal(_)));
        if !safe {
            return Err(format!("unsafe local template include: {name}"));
        }
        let Some(included) = templates.get(&name) else {
            limitations.push(format!("local template include was not captured: {name}"));
            offset = end;
            continue;
        };
        let replacement =
            resolve_literal_includes(included.clone(), templates, limitations, depth + 1)?;
        let expanded_len = source
            .len()
            .checked_sub(end - start)
            .and_then(|len| len.checked_add(replacement.len()))
            .ok_or_else(|| "expanded HTML byte count overflow".to_owned())?;
        if replacement.len() > MAX_TOTAL || expanded_len > MAX_TOTAL {
            return Err(format!("expanded HTML exceeds {MAX_TOTAL} bytes"));
        }
        source.replace_range(start..end, &replacement);
        offset = start + replacement.len();
    }
    Ok(source)
}

pub fn inspect_ui(
    ui: &Path,
    package: &LoadedPackage,
    catalog: &TokenCatalog,
    package_styles: &[CssStylesheet],
) -> Result<UiInspection, String> {
    let captured = capture_tree(ui)?;
    let files = captured.files.keys().cloned().collect::<Vec<_>>();
    let mut limitations = Vec::new();
    let mut stylesheets = Vec::new();
    for (path, bytes) in &captured.files {
        if path.ends_with(".css") {
            let source =
                std::str::from_utf8(bytes).map_err(|_| format!("CSS is not UTF-8: {path}"))?;
            stylesheets.push(parse_stylesheet(path, None, source)?);
        }
    }
    let templates = captured
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".html"))
        .map(|(path, bytes)| {
            Ok((
                path.clone(),
                std::str::from_utf8(bytes)
                    .map_err(|_| format!("HTML is not UTF-8: {path}"))?
                    .to_owned(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let template_len = templates.values().try_fold(0usize, |total, template| {
        total
            .checked_add(template.len())
            .and_then(|len| len.checked_add(1))
            .ok_or_else(|| "aggregate HTML byte count overflow".to_owned())
    })?;
    if template_len > MAX_TOTAL {
        return Err(format!("aggregate HTML exceeds {MAX_TOTAL} bytes"));
    }
    let mut template_source = templates.values().cloned().collect::<Vec<_>>().join("\n");
    template_source = resolve_literal_includes(template_source, &templates, &mut limitations, 0)?;
    let mut expanded = template_source.clone();
    let assets = package.assets.clone();
    let core_package = Package::from_assets(&assets)
        .map_err(|e| format!("cannot construct captured component package: {e}"))?;
    match catalog_core::expansion::expand(&template_source, &core_package) {
        Ok(result) if result.html.len() <= MAX_TOTAL => expanded = result.html,
        Ok(_) => return Err(format!("expanded HTML exceeds {MAX_TOTAL} bytes")),
        Err(e) => limitations.push(format!(
            "target DOM analysis is partial: component expansion failed ({e})"
        )),
    }
    let dom = Html::parse_document(&expanded);
    // Gather style blocks and style attributes from parsed HTML; preserve original file sources separately where possible.
    let mut inline_css = Vec::new();
    let selector_all = Selector::parse("style").unwrap();
    for style in dom.select(&selector_all) {
        let text = style.text().collect::<String>();
        let path = files
            .iter()
            .find(|f| f.ends_with(".html"))
            .map(String::as_str)
            .unwrap_or("<inline>");
        inline_css.push(parse_stylesheet(path, None, &text)?);
    }
    let selector_styled = Selector::parse("[style]").unwrap();
    for el in dom.select(&selector_styled) {
        if let Some(value) = el.value().attr("style") {
            let mut sheet = parse_stylesheet("<style-attribute>", None, &format!("x{{{value}}}"))?;
            for rule in &mut sheet.rules {
                rule.selector = "*".into();
            }
            inline_css.push(sheet);
        }
    }
    if !inline_css.is_empty() {
        limitations.push("inline style blocks and attributes are parsed, but HTML source line/column provenance is approximate".into());
    }
    stylesheets.extend(inline_css);
    let mut page_components = BTreeMap::<String, BTreeSet<String>>::new();
    let all = Selector::parse("*").unwrap();
    let mut elements = Vec::new();
    for el in dom.select(&all) {
        if let Some(name) = component_name(el, &package.catalog) {
            page_components
                .entry(name)
                .or_default()
                .insert(el.value().name().to_owned());
        }
        elements.push(el);
    }
    let mut rules = Vec::new();
    let token_names = catalog
        .tokens
        .iter()
        .map(|t| t.name.as_str())
        .collect::<BTreeSet<_>>();
    for sheet in &stylesheets {
        for rule in &sheet.rules {
            for selector in split_selector_list(&rule.selector) {
                let (mut components, bare, global, lexical_unknown, lexical_internal) =
                    lexical_facts(&selector);
                let parsed = Selector::parse(&selector);
                let mut matched_components = BTreeSet::new();
                let mut noncomponent = false;
                let mut target_known =
                    !lexical_unknown && parsed.is_ok() && sheet.path != "<style-attribute>";
                if let Ok(sel) = parsed {
                    for el in &elements {
                        if sel.matches(el) {
                            if let Some(name) = component_name(*el, &package.catalog) {
                                matched_components.insert(name);
                            } else {
                                noncomponent = true;
                            }
                        }
                    }
                } else {
                    target_known = false;
                    limitations.push(format!(
                        "unsupported selector cannot be matched: {selector}"
                    ));
                }
                if !target_known {
                    matched_components.clear();
                    noncomponent = false;
                }
                components.extend(matched_components.iter().cloned());
                let mut controlled = Vec::new();
                if target_known {
                    for component in &matched_components {
                        for decl in &rule.declarations {
                            let refs = decl.expression.references();
                            for pkg_sheet in package_styles
                                .iter()
                                .filter(|s| s.owner.as_deref() == Some(component))
                            {
                                for pkg_rule in &pkg_sheet.rules {
                                    let pkg_sel = Selector::parse(&pkg_rule.selector);
                                    if let Ok(pkg_sel) = pkg_sel {
                                        for el in &elements {
                                            if component_name(*el, &package.catalog).as_deref()
                                                == Some(component)
                                                && pkg_sel.matches(el)
                                                && parsed_selector_matches(&selector, *el)
                                            {
                                                for pkg_decl in &pkg_rule.declarations {
                                                    if !pkg_decl.name.starts_with("--")
                                                        && pkg_decl
                                                            .expression
                                                            .references()
                                                            .iter()
                                                            .any(|r| {
                                                                token_names.contains(r.as_str())
                                                            })
                                                    {
                                                        controlled.push(ControlledProperty {
                                                            component: component.clone(),
                                                            property: pkg_decl.name.clone(),
                                                            tokens: pkg_decl
                                                                .expression
                                                                .references()
                                                                .into_iter()
                                                                .filter(|r| {
                                                                    token_names.contains(r.as_str())
                                                                })
                                                                .collect(),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        target_known = false;
                                        limitations.push(format!(
                                            "package selector cannot be proven for {component}: {}",
                                            pkg_rule.selector
                                        ));
                                    }
                                }
                            }
                            let _ = refs;
                        }
                    }
                }
                controlled
                    .sort_by(|a, b| (&a.component, &a.property).cmp(&(&b.component, &b.property)));
                controlled.dedup_by(|a, b| {
                    a.component == b.component && a.property == b.property && a.tokens == b.tokens
                });
                rules.push(RuleFacts {
                    rule: rule.clone(),
                    selectors: vec![SelectorFacts {
                        selector: selector.clone(),
                        internal: lexical_internal,
                        unscoped_elements: bare,
                        components,
                        non_component: noncomponent,
                        target_known,
                        global_token_scope: global,
                        controlled_properties: controlled,
                    }],
                });
            }
        }
    }
    Ok(UiInspection {
        files,
        rules,
        pages: PageFacts {
            component_elements: page_components,
        },
        limitations,
    })
}
fn parsed_selector_matches(selector: &str, element: scraper::ElementRef<'_>) -> bool {
    Selector::parse(selector).is_ok_and(|s| s.matches(&element))
}
