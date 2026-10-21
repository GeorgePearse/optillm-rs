//! Port of upstream `optillm/mars/answer_extraction.py` and of the answer
//! extraction the synthesis vote uses (`optillm/utils/answer_extraction.py`).

use regex::Regex;

use crate::py;

/// `extract_clean_answer(text, mode)`.
pub fn extract_clean_answer(text: &str, mode: &str) -> String {
    if mode == "none" {
        return text.to_string();
    }
    let mode = if mode == "auto" {
        detect_answer_type(text)
    } else {
        mode
    };
    match mode {
        "code" => extract_code_answer(text),
        "math" => extract_math_answer(text),
        _ => extract_generic_answer(text),
    }
}

fn detect_answer_type(text: &str) -> &'static str {
    let code = [
        "```", "def ", "import ", "class ", "return ", "for ", "while ",
    ];
    let math = ["\\boxed", "\\frac", "\\sum", "\\int", "$$", "$\\"];
    if code.iter().any(|i| text.contains(i)) {
        "code"
    } else if math.iter().any(|i| text.contains(i)) {
        "math"
    } else {
        "generic"
    }
}

fn extract_code_answer(text: &str) -> String {
    let re = Regex::new(r"(?s)```(?:python|cpp|java|javascript|go|rust)?\n(.*?)\n```").unwrap();
    let blocks: Vec<String> = re.captures_iter(text).map(|c| c[1].to_string()).collect();
    if let Some(last) = blocks.last() {
        return format!("```python\n{}\n```", py::strings::strip(last));
    }
    let sections =
        Regex::new(r"(?i)\n#+\s+(?:Final Solution|Solution|Implementation|Code)\s*\n").unwrap();
    let parts: Vec<&str> = sections.split(text).collect();
    if parts.len() > 1 {
        return py::strings::strip(parts[parts.len() - 1]).to_string();
    }
    let parts: Vec<&str> = text.split("###").collect();
    if parts.len() > 1 {
        return py::strings::strip(parts[parts.len() - 1]).to_string();
    }
    text.to_string()
}

fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| py::strings::strip(p).to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn extract_math_answer(text: &str) -> String {
    let boxed = Regex::new(r"\\boxed\{([^}]+)\}").unwrap();
    let answers: Vec<String> = boxed
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .collect();
    if let Some(last) = answers.last() {
        return format!("The final answer is $\\boxed{{{}}}$", last);
    }
    let patterns = [
        r"[Ff]inal answer[:\s]+(.+?)(?:\n|$)",
        r"[Tt]he answer is[:\s]+(.+?)(?:\n|$)",
        r"[Tt]herefore[,\s]+(.+?)(?:\n|$)",
    ];
    for p in patterns {
        let re = Regex::new(p).unwrap();
        let matches: Vec<String> = re.captures_iter(text).map(|c| c[1].to_string()).collect();
        if let Some(last) = matches.last() {
            return py::strings::strip(last).to_string();
        }
    }
    match paragraphs(text).last() {
        Some(p) => p.clone(),
        None => text.to_string(),
    }
}

fn extract_generic_answer(text: &str) -> String {
    let lower = text.to_lowercase();
    let is_proof = [
        "proof",
        "QED",
        "proven",
        "demonstrate",
        "show that",
        "prove that",
    ]
    .iter()
    .any(|i| lower.contains(&i.to_lowercase()));
    let markers = [
        "In conclusion",
        "Therefore",
        "Thus",
        "Hence",
        "Finally",
        "The answer is",
        "The final answer",
    ];
    for marker in markers {
        if let Some((_, after)) = text.rsplit_once(marker) {
            let answer = py::strings::strip(after);
            let first_para =
                py::strings::strip(answer.split("\n\n").next().unwrap_or("")).to_string();
            if py::strings::len(&first_para) > 20 {
                return first_para;
            }
        }
    }
    let paras = paragraphs(text);
    if is_proof && !paras.is_empty() {
        if paras.len() >= 3 {
            return paras[paras.len() - 3..].join("\n\n");
        }
        return text.to_string();
    }
    if let Some(p) = paras.last() {
        return p.clone();
    }
    let sentences: Vec<String> = text
        .split('.')
        .map(|s| py::strings::strip(s).to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(s) = sentences.last() {
        return format!("{}.", s);
    }
    text.to_string()
}

/// `wrap_with_thinking_tags`.
pub fn wrap_with_thinking_tags(reasoning: &str, final_answer: &str) -> String {
    format!("<think>\n{}\n</think>\n\n{}", reasoning, final_answer)
}

/// One voting entry: (answer text, kind, integer value for numeric answers).
#[derive(Clone, Debug)]
pub struct VoteEntry {
    pub text: String,
    pub kind: &'static str,
    pub int: Option<i64>,
}

/// A value `extract_answer` can return.
enum Answer {
    Int(i64),
    Float(f64),
    Set(Vec<i64>),
    Str(String),
}

/// `str.isdigit()` for the ASCII digits that matter here.
fn is_py_digit(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// `_parse_boxed_content`.
fn parse_boxed_content(content: &str) -> anyhow::Result<Answer> {
    let content = py::strings::strip(content);
    if is_py_digit(content) {
        if let Ok(i) = content.parse::<i64>() {
            return Ok(Answer::Int(i));
        }
    }
    if let Some(f) = py::strings::parse_float(content) {
        if f.is_infinite() {
            return Err(anyhow::anyhow!("cannot convert float infinity to integer"));
        }
        if !f.is_nan() {
            return Ok(if f == f.trunc() {
                Answer::Int(f as i64)
            } else {
                Answer::Float(f)
            });
        }
    }
    if content.starts_with('{') && content.ends_with('}') && content.len() >= 2 {
        let inner = &content[1..content.len() - 1];
        if inner.contains(',') {
            let nums: Vec<i64> = inner
                .split(',')
                .map(py::strings::strip)
                .filter(|x| is_py_digit(x))
                .filter_map(|x| x.parse().ok())
                .collect();
            let mut set = nums;
            set.sort();
            set.dedup();
            return Ok(Answer::Set(set));
        }
    }
    Ok(Answer::Str(content.to_string()))
}

/// `AnswerExtractor.extract_answer(text, problem_type="imo", problem_id=None)` as
/// upstream's server actually runs it: `math_verify.parse` raises inside the
/// Flask worker thread (its timeout uses `signal.alarm`), so only the general
/// regex fallback ever produces an answer.
fn extract_answer(solution: &str) -> anyhow::Result<Option<Answer>> {
    if solution.is_empty() {
        return Ok(None);
    }
    let patterns: [(&str, bool); 7] = [
        (r"(?i)\\boxed\{([^}]+)\}", true),
        (r"(?i)boxed\{([^}]+)\}", true),
        (r"(?i)(?:the\s+)?answer\s+is\s+([^\n.!?]+)", false),
        (r"(?i)(?:final\s+)?answer:\s*([^\n.!?]+)", false),
        (r"(?i)therefore,?\s+([^\n.!?]+)", false),
        (r"(?i)thus,?\s+([^\n.!?]+)", false),
        // Python's `$` also matches before a trailing newline.
        (r"(?i)=\s*([^\n.!?]+)\n?\z", false),
    ];
    for (pattern, boxed) in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(last) = re.captures_iter(solution).last() {
            let content = py::strings::strip(&last[1]).to_string();
            if !content.is_empty() {
                return Ok(Some(if boxed {
                    parse_boxed_content(&content)?
                } else {
                    Answer::Str(content)
                }));
            }
        }
    }
    Ok(None)
}

/// The per-solution body of the voting loop in `_synthesize_final_solution`.
pub fn voting_entries(text: &str) -> anyhow::Result<Vec<VoteEntry>> {
    let Some(answer) = extract_answer(text)? else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    let has_nonempty_str = matches!(&answer, Answer::Str(s) if !py::strings::strip(s).is_empty());
    match &answer {
        Answer::Int(i) => entries.push(VoteEntry {
            text: i.to_string(),
            kind: "unified_numeric",
            int: Some(*i),
        }),
        Answer::Float(f) => {
            let i = f.trunc() as i64;
            entries.push(VoteEntry {
                text: i.to_string(),
                kind: "unified_numeric",
                int: Some(i),
            });
        }
        Answer::Str(s) if !py::strings::strip(s).is_empty() => entries.push(VoteEntry {
            text: s.clone(),
            kind: "unified_formula",
            int: None,
        }),
        Answer::Set(v) => {
            let inner: Vec<String> = v.iter().map(|x| x.to_string()).collect();
            entries.push(VoteEntry {
                text: format!("{{{}}}", inner.join(", ")),
                kind: "unified_set",
                int: None,
            });
        }
        Answer::Str(_) => {}
    }
    if !has_nonempty_str {
        // str(extracted_answer)
        let text = match &answer {
            Answer::Int(i) => i.to_string(),
            Answer::Float(f) => py::json::float_repr(*f),
            Answer::Str(s) => s.clone(),
            Answer::Set(v) => {
                if v.is_empty() {
                    "set()".to_string()
                } else {
                    format!(
                        "{{{}}}",
                        v.iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        };
        entries.push(VoteEntry {
            text,
            kind: "unified_other",
            int: None,
        });
    }
    Ok(entries)
}
