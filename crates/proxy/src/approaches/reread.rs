//! Port of upstream `optillm/reread.py` (RE2: re-reading).

use anyhow::anyhow;
use serde_json::{json, Map, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

pub async fn re2_approach(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
    n: i64,
) -> ApproachResult {
    let max_tokens = ctx
        .request_config
        .get("max_tokens")
        .filter(|v| !v.is_null())
        .cloned();
    let re2_prompt = format!(
        "{}\nRead the question again: {}",
        initial_query, initial_query
    );

    let mut provider_request = Map::new();
    provider_request.insert("model".into(), json!(ctx.model));
    provider_request.insert(
        "messages".into(),
        json!([msg("system", system_prompt), msg("user", &re2_prompt)]),
    );
    provider_request.insert("n".into(), json!(n));
    if let Some(mt) = max_tokens {
        provider_request.insert("max_tokens".into(), mt);
    }
    let response = ctx.create(Value::Object(provider_request)).await?;
    if response.choices().is_empty() {
        return Err(anyhow!(
            "re2_approach: provider returned an empty or None response"
        ));
    }
    let tokens = response.completion_tokens()?;
    if n == 1 {
        if response.content(0).is_none() || response.finish_reason(0).as_deref() == Some("length") {
            return Err(anyhow!(
                "re2_approach: provider returned a None or truncated response"
            ));
        }
        let content = response.content(0).unwrap_or_default();
        return Ok((json!(py::strings::strip(&content)), tokens));
    }
    let items: Vec<Value> = (0..response.choices().len())
        .filter(|&i| {
            response.content(i).is_some() && response.finish_reason(i).as_deref() != Some("length")
        })
        .map(|i| json!(py::strings::strip(&response.content(i).unwrap_or_default())))
        .collect();
    Ok((Value::Array(items), tokens))
}
