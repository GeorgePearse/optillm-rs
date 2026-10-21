//! Request parsing helpers ported from upstream `server.py`: conversation
//! flattening, `<optillm_approach>` tags, slug / combinator parsing, and the
//! post-processing applied to approach output.

use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

use crate::py;

/// Pipeline operation selected by the model slug.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Single,
    /// `a&b`: run approaches in sequence, feeding each output into the next.
    And,
    /// `a|b`: run approaches in parallel and return every result.
    Or,
}

impl Operation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Operation::Single => "SINGLE",
            Operation::And => "AND",
            Operation::Or => "OR",
        }
    }
}

fn approach_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<optillm_approach>(.*?)</optillm_approach>").unwrap())
}

/// Text of a message `content` that may be a string or a list of parts.
fn text_of(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter(|i| i.get("type").and_then(|t| t.as_str()) == Some("text"))
            .map(|i| i.get("text").map(py::json::to_str).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" "),
        Value::Null => String::new(),
        Value::Bool(false) => String::new(),
        other => py::json::to_str(other),
    }
}

/// `normalize_message_content`: list contents become space-joined text.
pub fn normalize_message_content(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| {
            let mut m = m.clone();
            if let Some(obj) = m.as_object_mut() {
                if let Some(Value::Array(_)) = obj.get("content") {
                    let text = text_of(&obj["content"]);
                    obj.insert("content".into(), Value::String(text));
                }
            }
            m
        })
        .collect()
}

/// `extract_optillm_approach(content)`.
pub fn extract_optillm_approach(content: &str) -> (String, Option<String>) {
    let re = approach_tag_re();
    if let Some(caps) = re.captures(content) {
        let approach = caps.get(1).map(|m| m.as_str().to_string());
        let stripped = re.replace_all(content, "");
        (py::strings::strip(&stripped).to_string(), approach)
    } else {
        (content.to_string(), None)
    }
}

/// `parse_conversation(messages)` -> (system_prompt, initial_query, optillm_approach).
pub fn parse_conversation(messages: &[Value]) -> (String, String, Option<String>) {
    let mut system_prompt = String::new();
    let mut conversation: Vec<String> = Vec::new();
    let mut optillm_approach: Option<String> = None;

    for message in messages {
        let role = message.get("role").and_then(|r| r.as_str());
        let content = message.get("content").cloned().unwrap_or(Value::Null);
        let falsy = match &content {
            Value::Null => true,
            Value::String(s) => s.is_empty(),
            Value::Array(a) => a.is_empty(),
            Value::Bool(b) => !b,
            _ => false,
        };
        let mut text_content = if falsy {
            String::new()
        } else {
            text_of(&content)
        };

        if role == Some("assistant") && text_content.is_empty() {
            continue;
        }
        match role {
            Some("system") => {
                let (t, a) = extract_optillm_approach(&text_content);
                system_prompt = t;
                optillm_approach = a;
            }
            Some("user") => {
                if optillm_approach.is_none() {
                    let (t, a) = extract_optillm_approach(&text_content);
                    text_content = t;
                    optillm_approach = a;
                }
                conversation.push(format!("User: {}", text_content));
            }
            Some("assistant") => conversation.push(format!("Assistant: {}", text_content)),
            _ => {}
        }
    }
    (system_prompt, conversation.join("\n"), optillm_approach)
}

/// `strip_optillm_approach_tags(messages)`: passthrough copy without tags.
pub fn strip_optillm_approach_tags(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| {
            let mut m = m.clone();
            if let Some(obj) = m.as_object_mut() {
                match obj.get("content").cloned() {
                    Some(Value::String(s)) => {
                        obj.insert(
                            "content".into(),
                            Value::String(extract_optillm_approach(&s).0),
                        );
                    }
                    Some(Value::Array(items)) => {
                        let items: Vec<Value> = items
                            .into_iter()
                            .map(|item| match item {
                                Value::Object(mut io) => {
                                    if let Some(Value::String(t)) = io.get("text").cloned() {
                                        io.insert(
                                            "text".into(),
                                            Value::String(extract_optillm_approach(&t).0),
                                        );
                                    }
                                    Value::Object(io)
                                }
                                other => other,
                            })
                            .collect();
                        obj.insert("content".into(), Value::Array(items));
                    }
                    _ => {}
                }
            }
            m
        })
        .collect()
}

/// `parse_combined_approach(model, known_approaches, plugin_approaches)`.
pub fn parse_combined_approach(
    model: &str,
    is_approach: impl Fn(&str) -> bool,
) -> (Operation, Vec<String>, String) {
    if model == "auto" {
        return (Operation::Single, vec!["none".into()], model.to_string());
    }
    let mut approaches: Vec<String> = Vec::new();
    let mut operation = Operation::Single;
    let mut model_parts: Vec<&str> = Vec::new();
    let mut parsing_approaches = true;

    for part in model.split('-') {
        if parsing_approaches {
            if is_approach(part) {
                approaches.push(part.to_string());
            } else if part.contains('&') {
                operation = Operation::And;
                approaches.extend(part.split('&').map(|s| s.to_string()));
            } else if part.contains('|') {
                operation = Operation::Or;
                approaches.extend(part.split('|').map(|s| s.to_string()));
            } else {
                parsing_approaches = false;
                model_parts.push(part);
            }
        } else {
            model_parts.push(part);
        }
    }
    if approaches.is_empty() {
        approaches = vec!["none".into()];
        operation = Operation::Single;
    }
    (operation, approaches, model_parts.join("-"))
}

/// `tagged_conversation_to_messages` followed by "take the last message's
/// content", as `proxy()` applies it to string results.
pub fn untag_response(text: &str) -> String {
    if !(text.contains("User:") || text.contains("Assistant:")) {
        return text.to_string();
    }
    let stripped = py::strings::strip(text);
    // re.split(r'(?=(User:|Assistant:))') keeps every tagged segment; the last
    // message is the text after the right-most tag.
    let last_user = stripped.rfind("User:").map(|i| (i, "User:".len()));
    let last_asst = stripped
        .rfind("Assistant:")
        .map(|i| (i, "Assistant:".len()));
    let (pos, tag_len) = match (last_user, last_asst) {
        (Some(u), Some(a)) => {
            if u.0 > a.0 {
                u
            } else {
                a
            }
        }
        (Some(u), None) => u,
        (None, Some(a)) => a,
        (None, None) => return text.to_string(),
    };
    py::strings::strip(&stripped[pos + tag_len..]).to_string()
}

/// `count_reasoning_tokens(text)` without a tokenizer.
pub fn count_reasoning_tokens(text: &str) -> i64 {
    if text.is_empty() {
        return 0;
    }
    static COMPLETE: OnceLock<Regex> = OnceLock::new();
    let complete = COMPLETE.get_or_init(|| Regex::new(r"(?s)<think>(.*?)</think>").unwrap());
    let mut thinking: String = complete
        .captures_iter(text)
        .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or("").to_string())
        .collect();
    // r'<think>(?!.*</think>)(.*)$' with DOTALL: first <think> with no </think> after it.
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find("<think>") {
        let start = search_from + rel;
        let after = start + "<think>".len();
        if !text[after..].contains("</think>") {
            thinking.push_str(&text[after..]);
            break;
        }
        search_from = start + 1;
    }
    if thinking.is_empty() {
        return 0;
    }
    let content_length = py::strings::len(py::strings::strip(&thinking)) as i64;
    if content_length > 0 {
        (content_length / 4).max(1)
    } else {
        0
    }
}

/// Build the passthrough `messages` list when none were supplied.
pub fn messages_from_prompt(system_prompt: &str, initial_query: &str) -> Vec<Value> {
    let mut messages = Vec::new();
    if !system_prompt.is_empty() {
        messages.push(msg("system", system_prompt));
    }
    if !initial_query.is_empty() {
        messages.push(msg("user", initial_query));
    }
    messages
}

/// `{"role": role, "content": content}`.
pub fn msg(role: &str, content: &str) -> Value {
    let mut m = Map::new();
    m.insert("role".into(), Value::String(role.into()));
    m.insert("content".into(), Value::String(content.into()));
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn known(s: &str) -> bool {
        ["moa", "bon", "mcts", "none", "re2", "memory"].contains(&s)
    }

    #[test]
    fn slug_parsing_matches_upstream() {
        assert_eq!(
            parse_combined_approach("moa-gpt-4o-mini", known),
            (Operation::Single, vec!["moa".into()], "gpt-4o-mini".into())
        );
        assert_eq!(
            parse_combined_approach("bon&moa-gpt-4o", known),
            (
                Operation::And,
                vec!["bon".into(), "moa".into()],
                "gpt-4o".into()
            )
        );
        assert_eq!(
            parse_combined_approach("bon|moa|mcts-gpt-4o", known),
            (
                Operation::Or,
                vec!["bon".into(), "moa".into(), "mcts".into()],
                "gpt-4o".into()
            )
        );
        assert_eq!(
            parse_combined_approach("gpt-4o-mini", known),
            (Operation::Single, vec!["none".into()], "gpt-4o-mini".into())
        );
        assert_eq!(
            parse_combined_approach("auto", known),
            (Operation::Single, vec!["none".into()], "auto".into())
        );
        // Parsing continues after a combinator part and stops at the first non-approach.
        assert_eq!(
            parse_combined_approach("memory-bon&moa-re2-gpt-re2", known),
            (
                Operation::And,
                vec!["memory".into(), "bon".into(), "moa".into(), "re2".into()],
                "gpt-re2".into()
            )
        );
    }

    #[test]
    fn conversation_parsing() {
        let msgs = vec![
            json!({"role": "system", "content": "Be brief <optillm_approach>moa</optillm_approach>"}),
            json!({"role": "user", "content": [{"type": "text", "text": "hi"}, {"type": "image_url"}, {"type": "text", "text": "there"}]}),
            json!({"role": "assistant", "content": null, "tool_calls": []}),
            json!({"role": "assistant", "content": "ok"}),
            json!({"role": "user", "content": "<optillm_approach>bon</optillm_approach> again"}),
        ];
        let (sys, q, a) = parse_conversation(&msgs);
        assert_eq!(sys, "Be brief");
        assert_eq!(a.as_deref(), Some("moa"));
        assert_eq!(
            q,
            "User: hi there\nAssistant: ok\nUser: <optillm_approach>bon</optillm_approach> again"
        );
    }

    #[test]
    fn untagging() {
        assert_eq!(untag_response("plain"), "plain");
        assert_eq!(untag_response("x User: a\nAssistant: b "), "b");
        assert_eq!(untag_response("Assistant: only"), "only");
    }

    #[test]
    fn reasoning_tokens() {
        assert_eq!(count_reasoning_tokens("<think>abcdefgh</think>out"), 2);
        assert_eq!(count_reasoning_tokens("<think>ab</think>"), 1);
        assert_eq!(count_reasoning_tokens("<think>abcd</think> <think>efgh"), 2);
        assert_eq!(count_reasoning_tokens("none"), 0);
    }
}
