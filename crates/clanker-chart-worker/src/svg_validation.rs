use std::str::FromStr;

fn skip_separators(bytes: &[u8], at: &mut usize) {
    while *at < bytes.len() && (bytes[*at].is_ascii_whitespace() || bytes[*at] == b',') {
        *at += 1;
    }
}
fn valid_numeric_separators(bytes: &[u8]) -> bool {
    for (i, _) in bytes.iter().enumerate().filter(|(_, b)| **b == b',') {
        let mut left = i;
        while left > 0 && bytes[left - 1].is_ascii_whitespace() {
            left -= 1;
        }
        let mut right = i + 1;
        while right < bytes.len() && bytes[right].is_ascii_whitespace() {
            right += 1;
        }
        if left == 0 || right == bytes.len() {
            return false;
        }
        let prev = bytes[left - 1];
        let next = bytes[right];
        if !(prev.is_ascii_digit() || prev == b'.')
            || !(next.is_ascii_digit() || matches!(next, b'+' | b'-' | b'.'))
        {
            return false;
        }
    }
    true
}
fn read_number(bytes: &[u8], at: &mut usize) -> Option<f64> {
    let start = *at;
    if *at < bytes.len() && matches!(bytes[*at], b'+' | b'-') {
        *at += 1;
    }
    let mut digits = 0usize;
    while *at < bytes.len() && bytes[*at].is_ascii_digit() {
        *at += 1;
        digits += 1;
    }
    if *at < bytes.len() && bytes[*at] == b'.' {
        *at += 1;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if *at < bytes.len() && matches!(bytes[*at], b'e' | b'E') {
        *at += 1;
        if *at < bytes.len() && matches!(bytes[*at], b'+' | b'-') {
            *at += 1;
        }
        let exponent_start = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        if exponent_start == *at {
            return None;
        }
    }
    let value = f64::from_str(std::str::from_utf8(&bytes[start..*at]).ok()?).ok()?;
    (value.is_finite() && value.abs() <= 100_000.0).then_some(value)
}
pub fn valid_path_data(path: &str) -> bool {
    let b = path.as_bytes();
    if !valid_numeric_separators(b) {
        return false;
    }
    let mut at = 0;
    let mut first = true;
    while at < b.len() {
        skip_separators(b, &mut at);
        if at == b.len() {
            break;
        }
        let cmd = b[at] as char;
        if !cmd.is_ascii_alphabetic() {
            return false;
        }
        let upper = cmd.to_ascii_uppercase();
        let arity = match upper {
            'M' | 'L' | 'T' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            'A' => 7,
            'Z' => 0,
            _ => return false,
        };
        if first && upper != 'M' {
            return false;
        }
        first = false;
        at += 1;
        if upper == 'Z' {
            continue;
        }
        let start = at;
        let mut nums = Vec::new();
        loop {
            skip_separators(b, &mut at);
            if at >= b.len() || (b[at] as char).is_ascii_alphabetic() {
                break;
            }
            let Some(n) = read_number(b, &mut at) else {
                return false;
            };
            nums.push(n);
            if nums.len() > 8192 {
                return false;
            }
        }
        if at == start || nums.is_empty() || nums.len() % arity != 0 {
            return false;
        }
        if upper == 'A'
            && nums
                .as_chunks::<7>()
                .0
                .iter()
                .any(|a| !matches!(a[3], 0.0 | 1.0) || !matches!(a[4], 0.0 | 1.0))
        {
            return false;
        }
    }
    !first
}
pub fn valid_transform(transform: &str) -> bool {
    if transform.trim().is_empty() {
        return true;
    }
    let b = transform.as_bytes();
    if !valid_numeric_separators(b) {
        return false;
    }
    let mut at = 0;
    while at < b.len() {
        while at < b.len() && b[at].is_ascii_whitespace() {
            at += 1;
        }
        if at == b.len() {
            break;
        }
        let name_start = at;
        while at < b.len() && b[at].is_ascii_alphabetic() {
            at += 1;
        }
        let Ok(name) = std::str::from_utf8(&b[name_start..at]) else {
            return false;
        };
        while at < b.len() && b[at].is_ascii_whitespace() {
            at += 1;
        }
        if at >= b.len() || b[at] != b'(' {
            return false;
        }
        at += 1;
        let mut args = Vec::new();
        loop {
            while at < b.len() && b[at].is_ascii_whitespace() {
                at += 1;
            }
            if at >= b.len() {
                return false;
            }
            if b[at] == b')' {
                at += 1;
                break;
            }
            if b[at] == b',' {
                at += 1;
                continue;
            }
            let Some(v) = read_number(b, &mut at) else {
                return false;
            };
            args.push(v);
            if args.len() > 6 {
                return false;
            }
        }
        let count_ok = match name {
            "matrix" => args.len() == 6,
            "translate" | "scale" => args.len() == 1 || args.len() == 2,
            "rotate" => args.len() == 1 || args.len() == 3,
            "skewX" | "skewY" => args.len() == 1,
            _ => false,
        };
        if !count_ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_parser_accepts_echarts_commands_and_rejects_malformed_data() {
        for p in [
            "M0 0L1 1",
            "M1 0A1 1 0 1 1 1 -0.1A1 1 0 0 1 1 0",
            "m-18.6 -5l37.3 0l0 24l-37.3 0Z",
        ] {
            assert!(valid_path_data(p), "{p}");
        }
        for p in [
            "L0 0",
            "M0",
            "M0 0X1 1",
            "M0 0A1 1 0 2 0 1 1",
            "M1e999 0",
            "M0,,0L1 1",
        ] {
            assert!(!valid_path_data(p), "{p}");
        }
    }
    #[test]
    fn transform_grammar_is_closed_and_bounded() {
        for t in [
            "translate(48 280)",
            "matrix(3,0,0,3,56,280)",
            "scale(2) rotate(45)",
        ] {
            assert!(valid_transform(t), "{t}");
        }
        for t in [
            "translate(1)evil()",
            "url(https://example.test)",
            "matrix(1,2)",
            "scale(1e999)",
            "translate(1,,2)",
        ] {
            assert!(!valid_transform(t), "{t}");
        }
    }
}
