//! Byte-compatible re-implementations of Python's `json.dumps` and `repr()`
//! for `serde_json::Value`, so responses and prompts that embed serialised
//! data match upstream exactly.

use serde_json::{Map, Number, Value};

/// Separators used by `json.dumps(..., separators=...)`.
#[derive(Clone, Copy, Debug)]
pub struct Separators {
    pub item: &'static str,
    pub key: &'static str,
}

/// Python's default `json.dumps` separators: `(", ", ": ")`.
pub const DEFAULT_SEPARATORS: Separators = Separators {
    item: ", ",
    key: ": ",
};
/// Flask's compact separators: `(",", ":")`.
pub const COMPACT_SEPARATORS: Separators = Separators {
    item: ",",
    key: ":",
};

/// Options mirroring the keyword arguments of `json.dumps` that matter here.
#[derive(Clone, Copy, Debug)]
pub struct DumpOptions {
    pub sort_keys: bool,
    pub ensure_ascii: bool,
    pub separators: Separators,
    /// `Some(n)` mirrors `indent=n`.
    pub indent: Option<usize>,
}

impl Default for DumpOptions {
    fn default() -> Self {
        Self {
            sort_keys: false,
            ensure_ascii: true,
            separators: DEFAULT_SEPARATORS,
            indent: None,
        }
    }
}

/// `json.dumps(value)` with Python defaults.
pub fn dumps(value: &Value) -> String {
    dumps_with(value, DumpOptions::default())
}

/// `json.dumps(value, indent=n)`; Python switches the item separator to "," when indenting.
pub fn dumps_indent(value: &Value, indent: usize) -> String {
    dumps_with(
        value,
        DumpOptions {
            indent: Some(indent),
            separators: Separators {
                item: ",",
                key: ": ",
            },
            ..Default::default()
        },
    )
}

/// `json.dumps(value, ensure_ascii=False)`.
pub fn dumps_unicode(value: &Value) -> String {
    dumps_with(
        value,
        DumpOptions {
            ensure_ascii: false,
            ..Default::default()
        },
    )
}

/// Flask `jsonify` body: sorted keys, compact separators, ASCII-escaped, trailing newline.
pub fn flask_jsonify(value: &Value) -> String {
    let mut s = dumps_with(
        value,
        DumpOptions {
            sort_keys: true,
            ensure_ascii: true,
            separators: COMPACT_SEPARATORS,
            indent: None,
        },
    );
    s.push('\n');
    s
}

pub fn dumps_with(value: &Value, opts: DumpOptions) -> String {
    let mut out = String::new();
    write_value(&mut out, value, &opts, 0);
    out
}

fn write_indent(out: &mut String, opts: &DumpOptions, level: usize) {
    if let Some(n) = opts.indent {
        out.push('\n');
        for _ in 0..(n * level) {
            out.push(' ');
        }
    }
}

fn write_value(out: &mut String, value: &Value, opts: &DumpOptions, level: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number_repr(n)),
        Value::String(s) => write_json_string(out, s, opts.ensure_ascii),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(opts.separators.item);
                }
                write_indent(out, opts, level + 1);
                write_value(out, item, opts, level + 1);
            }
            write_indent(out, opts, level);
            out.push(']');
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            if opts.sort_keys {
                entries.sort_by(|a, b| a.0.cmp(b.0));
            }
            for (i, (k, v)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(opts.separators.item);
                }
                write_indent(out, opts, level + 1);
                write_json_string(out, k, opts.ensure_ascii);
                out.push_str(opts.separators.key);
                write_value(out, v, opts, level + 1);
            }
            write_indent(out, opts, level);
            out.push('}');
        }
    }
}

fn write_json_string(out: &mut String, s: &str, ensure_ascii: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if ensure_ascii && (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Python `repr(float)`.
pub fn float_repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if f == 0.0 {
        return if f.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    // Shortest round-trip digits in scientific form, e.g. "1.2345e2".
    let sci = format!("{:e}", f);
    let (mantissa, exp) = sci.split_once('e').expect("scientific format");
    let exp: i32 = exp.parse().expect("exponent");
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if negative { "-" } else { "" };
    if (-4..16).contains(&exp) {
        // Fixed notation.
        let n = digits.len() as i32;
        let body = if exp >= 0 {
            let int_len = exp + 1;
            if n <= int_len {
                format!("{}{}.0", digits, "0".repeat((int_len - n) as usize))
            } else {
                format!(
                    "{}.{}",
                    &digits[..int_len as usize],
                    &digits[int_len as usize..]
                )
            }
        } else {
            format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        };
        format!("{}{}", sign, body)
    } else {
        let mant = if digits.len() > 1 {
            format!("{}.{}", &digits[..1], &digits[1..])
        } else {
            digits.clone()
        };
        let exp_sign = if exp < 0 { '-' } else { '+' };
        format!("{}{}e{}{:02}", sign, mant, exp_sign, exp.abs())
    }
}

fn number_repr(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        i.to_string()
    } else if let Some(u) = n.as_u64() {
        u.to_string()
    } else {
        let f = n.as_f64().unwrap_or(0.0);
        if f.is_nan() {
            "NaN".into()
        } else if f.is_infinite() {
            if f > 0.0 {
                "Infinity".into()
            } else {
                "-Infinity".into()
            }
        } else {
            float_repr(f)
        }
    }
}

/// Python `repr(str)`.
pub fn str_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if !is_printable(c) => {
                let cp = c as u32;
                if cp < 0x100 {
                    out.push_str(&format!("\\x{:02x}", cp));
                } else if cp < 0x10000 {
                    out.push_str(&format!("\\u{:04x}", cp));
                } else {
                    out.push_str(&format!("\\U{:08x}", cp));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Approximation of `str.isprintable()` for a single character.
fn is_printable(c: char) -> bool {
    let cp = c as u32;
    if cp < 0x20 || (0x7f..0xa0).contains(&cp) {
        return false;
    }
    // Separators other than ASCII space (Zl, Zp, Zs) and format characters.
    if c != ' ' && c.is_whitespace() {
        return false;
    }
    !matches!(cp, 0xad | 0x200b..=0x200f | 0x2028..=0x202e | 0x2060..=0x2064 | 0xfeff)
}

/// Python `repr()` of a JSON-like value (dict / list / str / float / bool / None).
pub fn repr(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(n) => {
            if n.is_f64() {
                float_repr(n.as_f64().unwrap_or(0.0))
            } else {
                n.to_string()
            }
        }
        Value::String(s) => str_repr(s),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(repr).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Object(map) => repr_map(map),
    }
}

fn repr_map(map: &Map<String, Value>) -> String {
    let parts: Vec<String> = map
        .iter()
        .map(|(k, v)| format!("{}: {}", str_repr(k), repr(v)))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

/// Python `str(list_of_str)`.
pub fn repr_str_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| str_repr(s)).collect();
    format!("[{}]", parts.join(", "))
}

/// Python `str(x)` for a JSON value: strings are returned as-is, everything else via `repr`.
pub fn to_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => repr(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn float_repr_matches_python() {
        assert_eq!(float_repr(1.0), "1.0");
        assert_eq!(float_repr(0.1), "0.1");
        assert_eq!(float_repr(1e16), "1e+16");
        assert_eq!(float_repr(1.5e-5), "1.5e-05");
        assert_eq!(float_repr(0.0001), "0.0001");
        assert_eq!(float_repr(123456.789), "123456.789");
        assert_eq!(float_repr(-2.5), "-2.5");
        assert_eq!(float_repr(1e15), "1000000000000000.0");
        assert_eq!(float_repr(0.7 - 0.1), "0.6");
        assert_eq!(float_repr(0.1 + 0.2), "0.30000000000000004");
    }

    #[test]
    fn dumps_matches_python() {
        let v = json!({"b": 1, "a": [1.0, "é\n", null, true]});
        assert_eq!(dumps(&v), r#"{"b": 1, "a": [1.0, "\u00e9\n", null, true]}"#);
        assert_eq!(
            flask_jsonify(&v),
            "{\"a\":[1.0,\"\\u00e9\\n\",null,true],\"b\":1}\n"
        );
        assert_eq!(dumps(&json!("\u{1F600}")), "\"\\ud83d\\ude00\"");
        assert_eq!(
            dumps_indent(&json!({"a": [1, 2]}), 2),
            "{\n  \"a\": [\n    1,\n    2\n  ]\n}"
        );
    }

    #[test]
    fn repr_matches_python() {
        assert_eq!(
            repr(&json!({"error": {"message": "it's bad", "code": null}})),
            r#"{'error': {'message': "it's bad", 'code': None}}"#
        );
        assert_eq!(str_repr("a'b\"c"), r#"'a\'b"c'"#);
        assert_eq!(
            repr_str_list(&["x".into(), "y\n".into()]),
            r#"['x', 'y\n']"#
        );
    }
}
