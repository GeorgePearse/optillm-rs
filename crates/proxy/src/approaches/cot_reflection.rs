//! Port of upstream `optillm/cot_reflection.py`.

use regex::Regex;
use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

pub async fn cot_reflection(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
    return_full_response: bool,
) -> ApproachResult {
    let temperature = ctx
        .request_config
        .get("temperature")
        .cloned()
        .unwrap_or(json!(0.6));
    let max_tokens = ctx.max_tokens(4096);
    #[rustfmt::skip]
    let cot_prompt = format!(
    concat!(
        "\n",
        "        {p0}\n",
        "\n",
        "        You are an AI assistant that uses a Chain of Thought (CoT) approach with reflection to answer queries. Follow these steps:\n",
        "\n",
        "        1. Think through the problem step by step within the <thinking> tags.\n",
        "        2. Reflect on your thinking to check for any errors or improvements within the <reflection> tags.\n",
        "        3. Make any necessary adjustments based on your reflection.\n",
        "        4. Provide your final, concise answer within the <output> tags.\n",
        "\n",
        "        Important: The <thinking> and <reflection> sections are for your internal reasoning process only. \n",
        "        Do not include any part of the final answer in these sections. \n",
        "        The actual response to the query must be entirely contained within the <output> tags.\n",
        "\n",
        "        Use the following format for your response:\n",
        "        <thinking>\n",
        "        [Your step-by-step reasoning goes here. This is your internal thought process, not the final answer.]\n",
        "        <reflection>\n",
        "        [Your reflection on your reasoning, checking for errors or improvements]\n",
        "        </reflection>\n",
        "        [Any adjustments to your thinking based on your reflection]\n",
        "        </thinking>\n",
        "        <output>\n",
        "        [Your final, concise answer to the query. This is the only part that will be shown to the user.]\n",
        "        </output>\n",
        "        ",
    ),
        p0 = system_prompt
    );

    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &cot_prompt), msg("user", initial_query)],
            "temperature": temperature,
            "max_tokens": max_tokens
        }))
        .await?;

    let full_response = response.content_at(0)?;
    let tokens = response.completion_tokens()?;
    let full_response = full_response
        .ok_or_else(|| anyhow::anyhow!("expected string or bytes-like object, got 'NoneType'"))?;

    let output_re = Regex::new(r"(?s)<output>(.*?)(?:</output>|$)").unwrap();
    let output = match output_re.captures(&full_response) {
        Some(c) => py::strings::strip(c.get(1).unwrap().as_str()).to_string(),
        None => full_response.clone(),
    };

    if return_full_response {
        Ok((Value::String(full_response), tokens))
    } else {
        Ok((Value::String(output), tokens))
    }
}
