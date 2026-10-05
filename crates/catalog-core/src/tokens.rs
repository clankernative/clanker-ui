use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenRole {
    Text,
    Background,
    Border,
    Spacing,
    Sizing,
    Typography,
    Radius,
    Shadow,
    Motion,
    Focus,
    Color,
    Opacity,
    Position,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenDefinition {
    pub name: String,
    pub role: TokenRole,
    pub purpose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<String>,
}

impl TokenDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_token_name(&self.name) {
            return Err(format!("invalid token name: {}", self.name));
        }
        let purpose = self.purpose.trim();
        if purpose.is_empty() || purpose.len() > 512 || purpose.contains(['\r', '\n']) {
            return Err(format!("invalid purpose for {}", self.name));
        }
        if let Some(semantic) = &self.semantic {
            if semantic.is_empty()
                || semantic.len() > 64
                || semantic.starts_with('-')
                || semantic.ends_with('-')
                || !semantic
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err(format!("invalid semantic label for {}", self.name));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenMetadata {
    pub schema_version: u32,
    pub tokens: Vec<TokenDefinition>,
}

impl TokenMetadata {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let metadata: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if metadata.schema_version != 1 {
            return Err("token metadata schemaVersion must be 1".into());
        }
        if metadata.tokens.is_empty() || metadata.tokens.len() > 4096 {
            return Err("token metadata must contain 1 to 4096 definitions".into());
        }
        let mut names = BTreeSet::new();
        for token in &metadata.tokens {
            token.validate()?;
            if !names.insert(&token.name) {
                return Err(format!("duplicate token: {}", token.name));
            }
        }
        Ok(metadata)
    }
}

pub(crate) fn valid_token_name(name: &str) -> bool {
    name.len() <= 128
        && name.strip_prefix("--cui-").is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                && !suffix.starts_with('-')
                && !suffix.ends_with('-')
                && !suffix.contains("--")
        })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssValue {
    pub parts: Vec<ValuePart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValuePart {
    Text(String),
    Var {
        name: String,
        fallback: Option<CssValue>,
    },
}

impl CssValue {
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            parts: vec![ValuePart::Text(text.into())],
        }
    }

    pub fn references(&self) -> BTreeSet<String> {
        fn visit(value: &CssValue, refs: &mut BTreeSet<String>) {
            for part in &value.parts {
                if let ValuePart::Var { name, fallback } = part {
                    refs.insert(name.clone());
                    if let Some(fallback) = fallback {
                        visit(fallback, refs);
                    }
                }
            }
        }
        let mut refs = BTreeSet::new();
        visit(self, &mut refs);
        refs
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssDeclaration {
    pub name: String,
    pub value: String,
    pub expression: CssValue,
    pub important: bool,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssRule {
    pub selector: String,
    pub conditions: Vec<String>,
    pub declarations: Vec<CssDeclaration>,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssStylesheet {
    pub path: String,
    pub owner: Option<String>,
    pub rules: Vec<CssRule>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSpec {
    pub definition: TokenDefinition,
    pub owner: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUse {
    pub component: String,
    pub selector: String,
    pub property: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDetails {
    pub name: String,
    pub owner: String,
    pub status: String,
    pub role: TokenRole,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic: Option<String>,
    pub default_value: Option<String>,
    pub default_expression: Option<String>,
    pub resolution: String,
    pub sources: Vec<TokenDefault>,
    pub readers: Vec<String>,
    pub uses: Vec<TokenUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDefault {
    pub expression: String,
    pub value: Option<String>,
    pub resolution: String,
    pub selector: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCatalog {
    pub tokens: Vec<TokenDetails>,
}

impl TokenCatalog {
    pub fn get(&self, name: &str) -> Option<&TokenDetails> {
        self.tokens
            .binary_search_by(|item| item.name.as_str().cmp(name))
            .ok()
            .map(|index| &self.tokens[index])
    }
}

fn render_value(value: &CssValue) -> String {
    let mut out = String::new();
    for part in &value.parts {
        match part {
            ValuePart::Text(text) => out.push_str(text),
            ValuePart::Var { name, fallback } => {
                out.push_str("var(");
                out.push_str(name);
                if let Some(fallback) = fallback {
                    out.push_str(", ");
                    out.push_str(&render_value(fallback));
                }
                out.push(')');
            }
        }
    }
    out
}

fn resolve_value(
    value: &CssValue,
    baselines: &BTreeMap<String, CssValue>,
    stack: &mut BTreeSet<String>,
) -> Result<Option<String>, String> {
    // CSS-wide keywords are not literal custom-property values. At a root
    // baseline they provide no value, allowing var()'s local fallback to run.
    if value
        .parts
        .iter()
        .all(|part| matches!(part, ValuePart::Text(_)))
        && ["initial", "inherit", "unset", "revert", "revert-layer"]
            .contains(&render_value(value).trim().to_ascii_lowercase().as_str())
    {
        return Ok(None);
    }
    if stack.len() > 128 {
        return Err("token dependency depth exceeds 128".into());
    }
    let mut out = String::new();
    for part in &value.parts {
        match part {
            ValuePart::Text(text) => out.push_str(text),
            ValuePart::Var { name, fallback } => {
                let resolved = if let Some(target) = baselines.get(name) {
                    if !stack.insert(name.clone()) {
                        return Err(format!("token dependency cycle at {name}"));
                    }
                    let result = resolve_value(target, baselines, stack)?;
                    stack.remove(name);
                    result
                } else {
                    None
                };
                let resolved = match resolved {
                    Some(value) => Some(value),
                    None => fallback
                        .as_ref()
                        .map(|value| resolve_value(value, baselines, stack))
                        .transpose()?
                        .flatten(),
                };
                match resolved {
                    Some(text) => out.push_str(&text),
                    None => return Ok(None),
                }
            }
        }
    }
    Ok(Some(out))
}

pub fn resolve_tokens(
    specs: &[TokenSpec],
    styles: &[CssStylesheet],
) -> Result<TokenCatalog, String> {
    let mut by_name = BTreeMap::new();
    for spec in specs {
        spec.definition.validate()?;
        if !["shared", "ready", "draft"].contains(&spec.status.as_str()) {
            return Err(format!("invalid token status for {}", spec.definition.name));
        }
        if by_name.insert(spec.definition.name.clone(), spec).is_some() {
            return Err(format!("duplicate token spec: {}", spec.definition.name));
        }
    }
    let mut baselines = BTreeMap::<String, CssValue>::new();
    for sheet in styles {
        for rule in &sheet.rules {
            let root = rule.selector.trim() == ":root"
                && rule.conditions.is_empty()
                && sheet.owner.is_none();
            for declaration in &rule.declarations {
                if declaration.name.starts_with("--cui-")
                    && !by_name.contains_key(&declaration.name)
                {
                    return Err(format!(
                        "unknown token reference {} at {}:{}",
                        declaration.name, declaration.location.file, declaration.location.line
                    ));
                }
                for reference in declaration.expression.references() {
                    if reference.starts_with("--cui-") && !by_name.contains_key(&reference) {
                        return Err(format!(
                            "unknown token reference {reference} at {}:{}",
                            declaration.location.file, declaration.location.line
                        ));
                    }
                }
                if root && declaration.name.starts_with("--cui-") {
                    baselines.insert(declaration.name.clone(), declaration.expression.clone());
                }
            }
        }
    }
    // Validate cycles independently of whether the token is ultimately used.
    for (name, expression) in &baselines {
        let _ = resolve_value(expression, &baselines, &mut BTreeSet::from([name.clone()]))?;
    }
    let mut candidates = BTreeMap::<String, Vec<TokenDefault>>::new();
    for sheet in styles {
        for rule in &sheet.rules {
            let root = rule.selector.trim() == ":root"
                && rule.conditions.is_empty()
                && sheet.owner.is_none();
            for declaration in &rule.declarations {
                if declaration.name.starts_with("--cui-") {
                    let resolved = resolve_value(
                        &declaration.expression,
                        &baselines,
                        &mut BTreeSet::from([declaration.name.clone()]),
                    )?;
                    candidates
                        .entry(declaration.name.clone())
                        .or_default()
                        .push(TokenDefault {
                            expression: render_value(&declaration.expression),
                            value: resolved,
                            resolution: if root {
                                "baseline".into()
                            } else if rule.conditions.is_empty() {
                                "contextual".into()
                            } else {
                                "conditional".into()
                            },
                            selector: rule.selector.clone(),
                            location: declaration.location.clone(),
                        });
                }
                add_fallback_sources(
                    &declaration.expression,
                    &mut candidates,
                    &baselines,
                    &rule.selector,
                    &declaration.location,
                )?;
            }
        }
    }
    let baseline_graph: BTreeMap<String, BTreeSet<String>> = baselines
        .iter()
        .map(|(name, value)| (name.clone(), value.references()))
        .collect();
    let mut reader_graphs = BTreeMap::<String, BTreeMap<String, BTreeSet<String>>>::new();
    for sheet in styles.iter().filter(|sheet| sheet.owner.is_some()) {
        let graph = reader_graphs
            .entry(sheet.owner.clone().unwrap())
            .or_insert_with(|| baseline_graph.clone());
        for declaration in sheet.rules.iter().flat_map(|rule| &rule.declarations) {
            if declaration.name.starts_with("--") {
                graph
                    .entry(declaration.name.clone())
                    .or_default()
                    .extend(declaration.expression.references());
            }
        }
    }
    let mut uses_by_token = BTreeMap::<String, BTreeSet<TokenUse>>::new();
    for sheet in styles.iter().filter(|sheet| sheet.owner.is_some()) {
        let component = sheet.owner.as_deref().unwrap();
        let graph = &reader_graphs[component];
        for rule in &sheet.rules {
            for declaration in &rule.declarations {
                if declaration.name.starts_with("--") {
                    continue;
                }
                let mut pending = declaration
                    .expression
                    .references()
                    .into_iter()
                    .collect::<Vec<_>>();
                let mut seen = BTreeSet::new();
                while let Some(reference) = pending.pop() {
                    if !seen.insert(reference.clone()) {
                        continue;
                    }
                    if by_name.contains_key(&reference) {
                        uses_by_token
                            .entry(reference.clone())
                            .or_default()
                            .insert(TokenUse {
                                component: component.into(),
                                selector: rule.selector.clone(),
                                property: declaration.name.clone(),
                                location: declaration.location.clone(),
                            });
                    }
                    if let Some(next) = graph.get(&reference) {
                        pending.extend(next.iter().cloned());
                    }
                }
            }
        }
    }
    let mut details = Vec::new();
    for (name, spec) in by_name {
        let source_list = candidates.remove(&name).unwrap_or_default();
        let baseline = source_list
            .iter()
            .rev()
            .find(|s| s.resolution == "baseline");
        let uses = uses_by_token.remove(&name).unwrap_or_default();
        let readers = uses
            .iter()
            .map(|usage| usage.component.clone())
            .collect::<BTreeSet<_>>();
        let fallbacks = source_list
            .iter()
            .filter(|source| source.resolution == "fallback")
            .collect::<Vec<_>>();
        let consistent_fallback = fallbacks.first().copied().filter(|first| {
            first.value.is_some()
                && fallbacks.iter().all(|source| source.value == first.value)
                && source_list.iter().all(|source| {
                    source.resolution == "fallback" || source.resolution == "baseline"
                })
        });
        let (default_value, default_expression, resolution) =
            if let Some(baseline) = baseline.filter(|source| source.value.is_some()) {
                (
                    baseline.value.clone(),
                    Some(baseline.expression.clone()),
                    "resolved".into(),
                )
            } else if let Some(fallback) = consistent_fallback {
                (
                    fallback.value.clone(),
                    Some(fallback.expression.clone()),
                    "resolved-fallback".into(),
                )
            } else {
                (
                    None,
                    baseline.map(|source| source.expression.clone()),
                    if source_list.is_empty() {
                        "unavailable"
                    } else {
                        "contextual"
                    }
                    .into(),
                )
            };
        details.push(TokenDetails {
            name,
            owner: spec.owner.clone(),
            status: spec.status.clone(),
            role: spec.definition.role.clone(),
            purpose: spec.definition.purpose.clone(),
            semantic: spec.definition.semantic.clone(),
            default_value,
            default_expression,
            resolution,
            sources: source_list,
            readers: readers.into_iter().collect(),
            uses: uses.into_iter().collect(),
        });
    }
    details.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(TokenCatalog { tokens: details })
}

fn add_fallback_sources(
    value: &CssValue,
    candidates: &mut BTreeMap<String, Vec<TokenDefault>>,
    baselines: &BTreeMap<String, CssValue>,
    selector: &str,
    location: &SourceLocation,
) -> Result<(), String> {
    for part in &value.parts {
        if let ValuePart::Var {
            name,
            fallback: Some(fallback),
        } = part
        {
            let resolved = resolve_value(fallback, baselines, &mut BTreeSet::new())?;
            candidates
                .entry(name.clone())
                .or_default()
                .push(TokenDefault {
                    expression: render_value(fallback),
                    value: resolved,
                    resolution: "fallback".into(),
                    selector: selector.into(),
                    location: location.clone(),
                });
            add_fallback_sources(fallback, candidates, baselines, selector, location)?;
        }
    }
    Ok(())
}
