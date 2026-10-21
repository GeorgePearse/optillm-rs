//! Port of upstream `optillm/plugins/privacy_plugin.py`. PII detection is
//! spaCy NER + Presidio, so anonymisation runs upstream's own code in
//! `py_helper.py`; de-anonymisation of the reply is native.

use anyhow::anyhow;
use regex::Regex;
use serde_json::{json, Value};

use super::pyhelper;
use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

/// `replace_entities(entity_map, text)`.
fn replace_entities(mapping: &Value, text: &str) -> String {
    let mut reverse: Vec<(String, String)> = Vec::new();
    if let Some(types) = mapping.as_object() {
        for entities in types.values() {
            if let Some(e) = entities.as_object() {
                for (name, placeholder) in e {
                    if let Some(p) = placeholder.as_str() {
                        // dict assignment: later duplicates overwrite earlier ones
                        reverse.retain(|(k, _)| k != p);
                        reverse.push((p.to_string(), name.clone()));
                    }
                }
            }
        }
    }
    Regex::new(r"<[A-Z_]+_\d+>")
        .unwrap()
        .replace_all(text, |c: &regex::Captures| {
            let p = &c[0];
            reverse
                .iter()
                .find(|(k, _)| k == p)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| p.to_string())
        })
        .to_string()
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let r = pyhelper::call(json!({"op": "privacy", "text": initial_query}), 1800).await?;
    if let Some(e) = r.get("error").and_then(|v| v.as_str()) {
        return Err(anyhow!(e.to_string()));
    }
    let anonymized = r.get("text").and_then(|v| v.as_str()).unwrap_or_default();
    let mapping = r.get("mapping").cloned().unwrap_or(json!({}));
    let response = ctx
        .create(json!({"model": ctx.model, "messages": [msg("system", system_prompt), msg("user", anonymized)]}))
        .await?;
    let content = response
        .content_at(0)?
        .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
    let final_response = replace_entities(&mapping, py::strings::strip(&content));
    Ok((json!(final_response), response.completion_tokens()?))
}
