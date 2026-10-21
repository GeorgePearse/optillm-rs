//! Python `str` semantics that differ subtly from Rust's: whitespace set,
//! `float()` / `int()` parsing, code-point slicing.

/// `str.isspace()` for a single character.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{0b}'
            | '\u{0c}'
            | '\r'
            | '\u{1c}'
            | '\u{1d}'
            | '\u{1e}'
            | '\u{1f}'
            | ' '
            | '\u{85}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

/// `s.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `s.lstrip()`.
pub fn lstrip(s: &str) -> &str {
    s.trim_start_matches(is_space)
}

/// `s.rstrip()`.
pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(is_space)
}

/// `s.split()` (no separator).
pub fn split_whitespace(s: &str) -> Vec<&str> {
    s.split(is_space).filter(|p| !p.is_empty()).collect()
}

/// `s.splitlines()`.
pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < bytes.len() {
        let (idx, c) = bytes[i];
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if is_break {
            out.push(&s[start..idx]);
            let mut next = idx + c.len_utf8();
            if c == '\r' && i + 1 < bytes.len() && bytes[i + 1].1 == '\n' {
                next += 1;
                i += 1;
            }
            start = next;
        }
        i += 1;
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Number of code points (`len(s)`).
pub fn len(s: &str) -> usize {
    s.chars().count()
}

/// `s[start:end]` with code-point indices; negative indices count from the end.
pub fn slice(s: &str, start: Option<isize>, end: Option<isize>) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len() as isize;
    let norm = |i: isize| -> usize {
        let j = if i < 0 { n + i } else { i };
        j.clamp(0, n) as usize
    };
    let a = start.map(norm).unwrap_or(0);
    let b = end.map(norm).unwrap_or(n as usize);
    if a >= b {
        return String::new();
    }
    chars[a..b].iter().collect()
}

/// `s[:n]`.
pub fn prefix(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn valid_digit_group(s: &str) -> bool {
    // Digits with single underscores between them.
    !s.is_empty()
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
        && s.chars().all(|c| c.is_ascii_digit() || c == '_')
}

/// `float(s)`; `None` where Python raises `ValueError`.
pub fn parse_float(s: &str) -> Option<f64> {
    let t = strip(s);
    if t.is_empty() {
        return None;
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'+' => (1.0, &t[1..]),
        b'-' => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    let lower = body.to_ascii_lowercase();
    match lower.as_str() {
        "inf" | "infinity" => return Some(sign * f64::INFINITY),
        "nan" => return Some(f64::NAN),
        _ => {}
    }
    let (mantissa, exponent) = match lower.find('e') {
        Some(i) => (&lower[..i], Some(&lower[i + 1..])),
        None => (lower.as_str(), None),
    };
    let (int_part, frac_part) = match mantissa.find('.') {
        Some(i) => (&mantissa[..i], Some(&mantissa[i + 1..])),
        None => (mantissa, None),
    };
    let int_ok = int_part.is_empty() || valid_digit_group(int_part);
    let frac_ok = match frac_part {
        None => true,
        Some(f) => f.is_empty() || valid_digit_group(f),
    };
    let has_digits = !int_part.is_empty() || frac_part.map(|f| !f.is_empty()).unwrap_or(false);
    if !(int_ok && frac_ok && has_digits) {
        return None;
    }
    if let Some(e) = exponent {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if !valid_digit_group(e) {
            return None;
        }
    }
    let cleaned: String = body.chars().filter(|c| *c != '_').collect();
    cleaned.parse::<f64>().ok().map(|v| sign * v)
}

/// `int(s)` (base 10); `None` where Python raises `ValueError`.
pub fn parse_int(s: &str) -> Option<i64> {
    let t = strip(s);
    let (neg, body) = match t.as_bytes().first() {
        Some(b'+') => (false, &t[1..]),
        Some(b'-') => (true, &t[1..]),
        _ => (false, t),
    };
    if !valid_digit_group(body) {
        return None;
    }
    let cleaned: String = body.chars().filter(|c| *c != '_').collect();
    cleaned
        .parse::<i64>()
        .ok()
        .map(|v| if neg { -v } else { v })
}

/// Python's `max(iterable, key=...)`: the first element whose key is maximal
/// (ties keep the earliest; NaN keys never replace the running maximum).
pub fn argmax_by<T, F: Fn(&T) -> f64>(items: &[T], key: F) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, item) in items.iter().enumerate() {
        let k = key(item);
        match best {
            None => best = Some((i, k)),
            Some((_, bk)) if k > bk => best = Some((i, k)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// CPython 3.12+ `sum(floats)` (start 0): Neumaier-compensated, exactly as
/// `builtin_sum_impl` does it, so averages print the same digits.
pub fn py_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut iter = values.into_iter();
    let Some(first) = iter.next() else { return 0.0 };
    // 0 (int) + first float
    let mut f_result = 0.0 + first;
    let mut c = 0.0f64;
    for x in iter {
        let t = f_result + x;
        if f_result.abs() >= x.abs() {
            c += (f_result - t) + x;
        } else {
            c += (x - t) + f_result;
        }
        f_result = t;
    }
    if c != 0.0 && c.is_finite() {
        f_result += c;
    }
    f_result
}

/// `template.format(**args)` for plain `{name}` fields and `{{` / `}}` escapes.
pub fn format_named(template: &str, args: &[(&str, &str)]) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut name = String::new();
                for n in chars.by_ref() {
                    if n == '}' {
                        break;
                    }
                    name.push(n);
                }
                match args.iter().find(|(k, _)| *k == name) {
                    Some((_, v)) => out.push_str(v),
                    None => return Err(format!("'{}'", name)),
                }
            }
            '}' => return Err("Single '}' encountered in format string".into()),
            c => out.push(c),
        }
    }
    Ok(out)
}

/// Python's `str.title()` approximation used by a few prompts (ASCII words).
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_parsing_matches_python() {
        assert_eq!(parse_float(" 7 "), Some(7.0));
        assert_eq!(parse_float("8.5"), Some(8.5));
        assert_eq!(parse_float("1_0"), Some(10.0));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("5."), Some(5.0));
        assert_eq!(parse_float("1e3"), Some(1000.0));
        assert!(parse_float("nan").unwrap().is_nan());
        assert_eq!(parse_float("-Infinity"), Some(f64::NEG_INFINITY));
        assert_eq!(parse_float("8/10"), None);
        assert_eq!(parse_float("Score: 8"), None);
        assert_eq!(parse_float("."), None);
        assert_eq!(parse_float("1__0"), None);
        assert_eq!(parse_float(""), None);
    }

    #[test]
    fn strip_uses_python_whitespace() {
        assert_eq!(strip("\u{1c} a \u{85}"), "a");
        assert_eq!(split_whitespace(" a  b\tc "), vec!["a", "b", "c"]);
        assert_eq!(splitlines("a\r\nb\nc"), vec!["a", "b", "c"]);
        assert_eq!(splitlines("a\n"), vec!["a"]);
    }

    #[test]
    fn sum_matches_cpython_312() {
        // python3.12: sum([0.8] * 54) / 54 == 0.8 and sum([0.1, 0.2, 0.3]) == 0.6 (naive: 0.6000000000000001)
        let v: Vec<f64> = std::iter::repeat(0.8).take(54).collect();
        assert_eq!(py_sum(v.iter().copied()) / 54.0, 0.8);
        assert_eq!(py_sum([0.1, 0.2, 0.3]), 0.6);
    }

    #[test]
    fn argmax_keeps_first() {
        assert_eq!(argmax_by(&[1.0, 3.0, 3.0], |x| *x), Some(1));
        // max([nan, 3.0]) is nan in Python: NaN never compares greater.
        assert_eq!(argmax_by(&[f64::NAN, 3.0], |x| *x), Some(0));
        assert_eq!(argmax_by(&[3.0, f64::NAN], |x| *x), Some(0));
    }
}
