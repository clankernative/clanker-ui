//! Exact, single-pass fragment substitution. Inserted values are never templates.

pub fn fill(fragment: &str, replacements: &[(&str, &str)]) -> Result<String, String> {
    for (slot, _) in replacements {
        if fragment.matches(slot).count() != 1 {
            return Err(format!("fragment must contain exactly one {slot} slot"));
        }
    }
    let mut output = String::with_capacity(fragment.len());
    let mut remaining = fragment;
    while let Some(start) = remaining.find("[[") {
        output.push_str(&remaining[..start]);
        let tail = &remaining[start..];
        let (slot, value) = replacements
            .iter()
            .find(|(slot, _)| tail.starts_with(slot))
            .ok_or("fragment contains an unsupported slot")?;
        output.push_str(value);
        remaining = &tail[slot.len()..];
    }
    output.push_str(remaining);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_slot_looking_text_is_not_reinterpreted() {
        assert_eq!(
            fill(
                "[[label]] / [[detail]]",
                &[("[[label]]", "[[detail]]"), ("[[detail]]", "safe")]
            )
            .unwrap(),
            "[[detail]] / safe"
        );
    }

    #[test]
    fn rejects_missing_duplicate_and_unknown_template_slots() {
        for fragment in [
            "no slot",
            "[[label]][[label]]",
            "[[label]][[extra]]",
            "[[label]][[unterminated",
        ] {
            assert!(
                fill(fragment, &[("[[label]]", "safe")]).is_err(),
                "{fragment}"
            );
        }
    }
}
