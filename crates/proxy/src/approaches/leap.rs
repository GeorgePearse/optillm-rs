//! Port of upstream `optillm/leap.py` (LEAP: learning principles from mistakes).

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Map, Value};

use super::{ApproachResult, Ctx};
use crate::client::ChatCompletion;
use crate::parse::msg;
use crate::py;

/// `LEAP.extract_output`.
pub fn extract_output(text: &str) -> String {
    let re = Regex::new(r"(?s)<output>(.*?)(?:</output>|$)").unwrap();
    match re.captures(text) {
        Some(c) => py::strings::strip(c.get(1).unwrap().as_str()).to_string(),
        None => String::new(),
    }
}

fn extract_content(response: &ChatCompletion, context: &str) -> Result<String> {
    if response.is_unusable() {
        return Err(anyhow!(
            "LEAP: provider returned an empty, None, or truncated response while {}",
            context
        ));
    }
    Ok(response.content(0).unwrap_or_default())
}

/// Python iteration over a parsed JSON value followed by `example['question']`.
fn examples_from_json(parsed: Value) -> Result<Option<Vec<(Value, Value)>>> {
    let items: Vec<Value> = match parsed {
        Value::Array(items) => items,
        Value::Object(map) => map.keys().map(|k| Value::String(k.clone())).collect(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Value::Number(n) => {
            let ty = if n.is_f64() { "float" } else { "int" };
            return Err(anyhow!("'{}' object is not iterable", ty));
        }
        Value::Bool(_) => return Err(anyhow!("'bool' object is not iterable")),
        Value::Null => return Err(anyhow!("'NoneType' object is not iterable")),
    };
    let mut out = Vec::new();
    for item in items {
        match item {
            Value::Object(m) => {
                let (Some(q), Some(a)) = (m.get("question"), m.get("answer")) else {
                    // KeyError -> caught upstream, examples stay empty.
                    return Ok(None);
                };
                out.push((q.clone(), a.clone()));
            }
            Value::String(_) => {
                return Err(anyhow!("string indices must be integers, not 'str'"));
            }
            Value::Array(_) => {
                return Err(anyhow!("list indices must be integers or slices, not str"));
            }
            Value::Number(n) => {
                let ty = if n.is_f64() { "float" } else { "int" };
                return Err(anyhow!("'{}' object is not subscriptable", ty));
            }
            Value::Bool(_) => return Err(anyhow!("'bool' object is not subscriptable")),
            Value::Null => return Err(anyhow!("'NoneType' object is not subscriptable")),
        }
    }
    Ok(Some(out))
}

struct Leap<'a> {
    system_prompt: &'a str,
    ctx: &'a Ctx,
    max_tokens: Value,
    low_level_principles: Vec<String>,
    high_level_principles: Vec<String>,
    tokens: i64,
}

impl<'a> Leap<'a> {
    async fn call(&mut self, user: String, temperature: Option<f64>) -> Result<ChatCompletion> {
        let mut req = Map::new();
        req.insert("model".into(), json!(self.ctx.model));
        req.insert("max_tokens".into(), self.max_tokens.clone());
        req.insert(
            "messages".into(),
            json!([msg("system", self.system_prompt), msg("user", &user)]),
        );
        if let Some(t) = temperature {
            req.insert("temperature".into(), json!(t));
        }
        let response = self.ctx.create(Value::Object(req)).await?;
        self.tokens += response.completion_tokens()?;
        Ok(response)
    }

    async fn extract_examples_from_query(
        &mut self,
        initial_query: &str,
    ) -> Result<Vec<(Value, Value)>> {
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "\n",
                "                Analyze the following query and determine if it contains few-shot examples.\n",
                "                If it does, extract the examples and their corresponding answers.\n",
                "                Format the examples as a JSON array of objects, where each object has \"question\" and \"answer\" fields.\n",
                "                If there are no examples, return an empty array.\n",
                "                Enclose your response within <output></output> tags.\n",
                "                Do not put any explanation or any other reponse other than the JSON array within the <output></output> tags.\n",
                "\n",
                "                Example output format:\n",
                "                <output>\n",
                "                [\n",
                "                    {{\"question\": \"What is 2+2?\", \"answer\": \"4\"}},\n",
                "                    {{\"question\": \"What is the capital of France?\", \"answer\": \"Paris\"}}\n",
                "                ]\n",
                "                </output>\n",
                "\n",
                "                Query: {p0}\n",
                "                ",
            ),
            p0 = initial_query,
        );
        let response = self.call(prompt, None).await?;
        let examples_str = extract_output(&extract_content(
            &response,
            "extracting examples from the query",
        )?);
        if examples_str.is_empty() {
            return Ok(Vec::new());
        }
        match serde_json::from_str::<Value>(&examples_str) {
            Ok(parsed) => Ok(examples_from_json(parsed)?.unwrap_or_default()),
            Err(_) => Ok(Vec::new()),
        }
    }

    async fn generate_mistakes(
        &mut self,
        examples: &[(Value, Value)],
    ) -> Result<Vec<(String, String, String, Value)>> {
        let mut mistakes = Vec::new();
        for (question, correct_answer) in examples {
            let q = py::json::to_str(question);
            #[rustfmt::skip]
            let prompt = format!(
                concat!(
                    "\n",
                    "                    Instruction: Answer the following question step by step. To induce a mistake, \n",
                    "                    deliberately introduce an error in your reasoning or calculation.\n",
                    "                    Question: {p0}\n",
                    "                    Provide your step-by-step reasoning, then enclose your final answer within <output></output> tags.\n",
                    "                    Think step by step, but make sure to include a mistake.\n",
                    "                    ",
                ),
                p0 = q,
            );
            let response = self.call(prompt, Some(0.7)).await?;
            let generated_reasoning =
                extract_content(&response, "generating reasoning for a mistake example")?;
            let generated_answer = extract_output(&generated_reasoning);
            if Value::String(generated_answer.clone()) != *correct_answer {
                mistakes.push((
                    q,
                    generated_reasoning,
                    generated_answer,
                    correct_answer.clone(),
                ));
            }
        }
        Ok(mistakes)
    }

    async fn generate_low_level_principles(
        &mut self,
        mistakes: &[(String, String, String, Value)],
    ) -> Result<()> {
        for (question, generated_reasoning, generated_answer, correct_answer) in mistakes {
            #[rustfmt::skip]
            let prompt = format!(
                concat!(
                    "\n",
                    "                    Question: {p0}\n",
                    "                    Generated Reasoning: {p1}\n",
                    "                    Generated Answer: {p2}\n",
                    "                    Correct Answer: {p3}\n",
                    "                    Instruction: Conduct a thorough analysis of the generated answer in comparison to the\n",
                    "                    correct answer. Also observe how the generated reasoning differs from the correct\n",
                    "                    reasoning. Identify any discrepancies, misunderstandings, or errors. Provide clear\n",
                    "                    insights, principles, or guidelines that can be derived from this analysis to improve\n",
                    "                    future responses. We are not focused on this one data point, but rather on the general\n",
                    "                    principle.\n",
                    "                    Reasoning: <discuss why the generated answer is wrong>\n",
                    "                    Insights: Enclose ONLY the principles or insights within <output></output> tags.\n",
                    "                    ",
                ),
                p0 = question,
                p1 = generated_reasoning,
                p2 = generated_answer,
                p3 = py::json::to_str(correct_answer),
            );
            let response = self.call(prompt, None).await?;
            let principle = extract_output(&extract_content(
                &response,
                "generating low-level principles",
            )?);
            self.low_level_principles.push(principle);
        }
        Ok(())
    }

    async fn generate_high_level_principles(&mut self) -> Result<()> {
        let principles_text = self.low_level_principles.join("\n");
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "\n",
                "                Low-level principles: {p0}\n",
                "                Create a list of *unique* and insightful principles to improve future responses based\n",
                "                on the analysis above.\n",
                "                Focus on capturing the essence of the feedback while eliminating redundancies.\n",
                "                Ensure that each point is clear, concise, and directly derived from the introspection\n",
                "                results.\n",
                "                Create a numbered list of principles. Leave specific details in place.\n",
                "                Limit to at most 8 principles.\n",
                "                Enclose your list of principles within <output></output> tags.\n",
                "                ",
            ),
            p0 = principles_text,
        );
        let response = self.call(prompt, None).await?;
        let text = extract_output(&extract_content(
            &response,
            "generating high-level principles",
        )?);
        self.high_level_principles = text.split('\n').map(|s| s.to_string()).collect();
        Ok(())
    }

    async fn apply_principles(&mut self, query: &str) -> Result<String> {
        let principles_text = self.high_level_principles.join("\n");
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "\n",
                "                Please answer the following query. Keep in mind these principles:\n",
                "\n",
                "                {p0}\n",
                "\n",
                "                Query: {p1}\n",
                "                ",
            ),
            p0 = principles_text,
            p1 = query,
        );
        let response = self.call(prompt, None).await?;
        extract_content(&response, "applying principles to the query")
    }
}

pub async fn leap(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let mut solver = Leap {
        system_prompt,
        ctx,
        max_tokens: ctx.max_tokens(4096),
        low_level_principles: Vec::new(),
        high_level_principles: Vec::new(),
        tokens: 0,
    };
    let examples = solver.extract_examples_from_query(initial_query).await?;
    let answer = if examples.is_empty() {
        solver.apply_principles(initial_query).await?
    } else {
        let mistakes = solver.generate_mistakes(&examples).await?;
        solver.generate_low_level_principles(&mistakes).await?;
        solver.generate_high_level_principles().await?;
        solver.apply_principles(initial_query).await?
    };
    Ok((Value::String(answer), solver.tokens))
}
