//! App-owned JSON in an inert Markdown file, ignored by Native UI admission.

pub fn decode(bytes: &[u8]) -> Result<&[u8], String> {
    if bytes.first() == Some(&b'{') {
        return Ok(bytes);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "component document is not UTF-8")?;
    let marker = "```clanker-ui-json\n";
    let start = text.find(marker).ok_or("missing clanker-ui-json block")? + marker.len();
    let suffix = &text[start..];
    let end = suffix
        .find("\n```")
        .ok_or("unclosed clanker-ui-json block")?;
    if suffix[end + 4..].contains(marker) {
        return Err("multiple clanker-ui-json blocks".into());
    }
    Ok(&suffix.as_bytes()[..end])
}

pub fn encode(title: &str, json: &[u8]) -> Result<Vec<u8>, String> {
    let value = std::str::from_utf8(json).map_err(|_| "lock is not UTF-8")?;
    Ok(format!("# {title}\n\n```clanker-ui-json\n{value}\n```\n").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_roundtrips_json() {
        let bytes = encode("Lock", br#"{"schemaVersion":1}"#).unwrap();
        assert_eq!(decode(&bytes).unwrap(), br#"{"schemaVersion":1}"#);
    }
}
