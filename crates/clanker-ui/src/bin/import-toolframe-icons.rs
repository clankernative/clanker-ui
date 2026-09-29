//! One-time migration tool. Normal app builds use the checked-in JSON, not Toolframe sources.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

fn parse_branch(line: &str) -> Option<String> {
    let line = line.trim();
    let start = if line.starts_with("{{ if name == \"") {
        "{{ if name == \""
    } else if line.starts_with("{{ else if name == \"") {
        "{{ else if name == \""
    } else {
        return None;
    };
    line.strip_prefix(start)?
        .split('"')
        .next()
        .map(str::to_owned)
}

fn import(contract: &str, template: &str) -> Result<BTreeMap<String, String>, String> {
    let names = contract
        .lines()
        .filter_map(|line| line.trim().strip_prefix("definition "))
        .filter_map(|line| line.split('"').nth(1))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if names.len() != 100 {
        return Err(format!(
            "expected 100 Toolframe icon names, got {}",
            names.len()
        ));
    }
    let mut branches = BTreeMap::new();
    let mut active: Option<String> = None;
    let mut body = String::new();
    for line in template.lines() {
        let next = parse_branch(line)
            .or_else(|| (line.trim() == "{{ else }}").then(|| "__fallback".into()));
        if let Some(next) = next {
            if let Some(name) = active.replace(next) {
                branches.insert(name, body.trim().to_owned());
                body.clear();
            }
        } else if line.trim() == "{{ end }}" {
            if let Some(name) = active.take() {
                branches.insert(name, body.trim().to_owned());
            }
            body.clear();
        } else if active.is_some() {
            body.push_str(line.trim());
            body.push('\n');
        }
    }
    let fallback = branches
        .remove("__fallback")
        .ok_or("icon fallback not found")?;
    let mut icons = BTreeMap::new();
    for name in names {
        let geometry = branches.remove(&name).unwrap_or_else(|| fallback.clone());
        if geometry.is_empty() || geometry.contains("{{") || !geometry.contains('<') {
            return Err(format!("invalid geometry for {name}"));
        }
        if icons.insert(name.clone(), geometry).is_some() {
            return Err(format!("duplicate icon name {name}"));
        }
    }
    if !branches.is_empty() {
        return Err(format!("unrecognized icon branches: {:?}", branches.keys()));
    }
    Ok(icons)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: import-toolframe-icons <Icon/Contract.fs> <Icon/template.html> <output.json>"
                .into(),
        );
    }
    let icons = import(
        &fs::read_to_string(&args[0])?,
        &fs::read_to_string(&args[1])?,
    )?;
    fs::write(Path::new(&args[2]), serde_json::to_vec_pretty(&icons)?)?;
    println!("Imported {} closed icon names", icons.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toolframe_closed_catalog() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = root.join("../../../../internal-tools-toolframe/src/components/icon");
        if source.exists() {
            let icons = import(
                &fs::read_to_string(source.join("Contract.fs")).unwrap(),
                &fs::read_to_string(source.join("template.html")).unwrap(),
            )
            .unwrap();
            assert_eq!(icons.len(), 100);
            assert!(icons["check"].contains("<path"));
        }
    }
}
