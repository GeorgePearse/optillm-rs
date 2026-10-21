//! Port of upstream `optillm/cepo/cepo.py` (CePO: Cerebras planning and optimization).
//!
//! Concurrency mirrors upstream's thread pools: `bestofn_n` completions run
//! in parallel, each with `planning_m` parallel plan attempts consumed in
//! completion order (`as_completed`) until `planning_n` plans exist; the
//! remaining attempts still run to completion, as upstream's executor waits
//! for them, but their results and tokens are discarded.
//!
//! The CePO config is process-global and mutable, as upstream's is: a 400
//! "think value" error permanently disables `use_reasoning`.

use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use regex::Regex;
use serde_json::{json, Map, Value};

use super::{ApproachResult, Ctx};
use crate::client::LlmError;
use crate::config::ServerConfig;
use crate::parse::msg;
use crate::py;

const DEFAULT_CONFIG: &str = include_str!("cepo_config.yaml");

/// Upstream `CepoConfig` (a dataclass), kept as JSON values so ints / floats
/// serialise exactly as the YAML / CLI produced them.
#[derive(Clone, Debug)]
pub struct CepoConfig(Map<String, Value>);

const FIELDS: [&str; 22] = [
    "bestofn_n",
    "bestofn_temperature",
    "bestofn_max_tokens",
    "bestofn_rating_type",
    "planning_n",
    "planning_m",
    "planning_temperature_step1",
    "planning_temperature_step2",
    "planning_temperature_direct_resp",
    "planning_temperature_step3",
    "planning_temperature_step4",
    "planning_max_tokens_step1",
    "planning_max_tokens_step2",
    "planning_max_tokens_direct_resp",
    "planning_max_tokens_step3",
    "planning_max_tokens_step4",
    "use_plan_diversity",
    "use_reasoning",
    "use_reasoning_fallback",
    "num_of_retries",
    "rating_model",
    "print_output",
];

impl CepoConfig {
    /// `init_cepo_config`: YAML file merged with `--cepo_*` overrides.
    pub fn load(server: &ServerConfig) -> Result<Self> {
        let text = match &server.cepo_config_file {
            Some(path) => std::fs::read_to_string(path)?,
            None => {
                let local = std::path::Path::new("optillm/cepo/configs/cepo_config.yaml");
                if local.exists() {
                    std::fs::read_to_string(local)?
                } else {
                    DEFAULT_CONFIG.to_string()
                }
            }
        };
        let yaml: serde_yaml::Value = serde_yaml::from_str(&text)?;
        let mut map = match serde_json::to_value(yaml)? {
            Value::Object(m) => m,
            Value::Null => Map::new(),
            _ => return Err(anyhow!("CePO config must be a mapping")),
        };
        for (k, v) in &server.cepo_overrides {
            map.insert(k.clone(), v.clone());
        }
        for k in map.keys() {
            if !FIELDS.contains(&k.as_str()) {
                return Err(anyhow!(
                    "CepoConfig.__init__() got an unexpected keyword argument '{}'",
                    k
                ));
            }
        }
        map.entry("rating_model").or_insert(Value::Null);
        map.entry("print_output").or_insert(Value::Bool(false));
        for k in FIELDS {
            if !map.contains_key(k) {
                return Err(anyhow!(
                    "CepoConfig.__init__() missing 1 required positional argument: '{}'",
                    k
                ));
            }
        }
        Ok(CepoConfig(map))
    }

    pub fn get(&self, k: &str) -> Value {
        self.0.get(k).cloned().unwrap_or(Value::Null)
    }

    fn int(&self, k: &str) -> i64 {
        self.0.get(k).and_then(|v| v.as_i64()).unwrap_or(0)
    }

    fn truthy(&self, k: &str) -> bool {
        match self.0.get(k) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(Value::String(s)) => !s.is_empty(),
            Some(Value::Null) | None => false,
            Some(_) => true,
        }
    }
}

static CEPO_CONFIG: OnceLock<Mutex<CepoConfig>> = OnceLock::new();

/// The process-wide CePO config (loaded on first use, like upstream's global).
pub fn global_config(server: &ServerConfig) -> Result<&'static Mutex<CepoConfig>> {
    if let Some(c) = CEPO_CONFIG.get() {
        return Ok(c);
    }
    let cfg = CepoConfig::load(server)?;
    Ok(CEPO_CONFIG.get_or_init(|| Mutex::new(cfg)))
}

struct Cepo<'a> {
    ctx: &'a Ctx,
    cfg: &'static Mutex<CepoConfig>,
    model: String,
}

fn cfg_snapshot(cfg: &Mutex<CepoConfig>) -> CepoConfig {
    cfg.lock().unwrap().clone()
}

/// `extract_question_only`.
pub fn extract_question_only(task: &str) -> String {
    #[rustfmt::skip]
    let instruction = concat!(
        "\n",
        "\n",
        "\n",
        "## Instruction \n",
        "\n",
        "Please answer this question by first reasoning and then providing your answer.\n",
        "Present your reasoning and solution in the following json format. \n",
        "Please show your final answer in the `answer` field, e.g.,`\"answer\": \"42\"`.\n",
        "\n",
        "```json\n",
        "{\n",
        "    \"reasoning\": \"___\",\n",
        "    \"answer\": \"___\"\n",
        "}\n",
        "```\n",
    );
    task.replace("\n## Question: \n\n", "")
        .replace(instruction, "")
}

/// `remove_think_section`.
pub fn remove_think_section(response: &str) -> String {
    if response.is_empty() {
        return String::new();
    }
    if !response.contains("<think>") {
        return response.to_string();
    }
    let re = Regex::new(r"(?s)</think>\s*(.*)").unwrap();
    match re.captures(response) {
        Some(c) => c[1].to_string(),
        None => response.to_string(),
    }
}

fn status_of(e: &anyhow::Error) -> Option<u16> {
    e.downcast_ref::<LlmError>().and_then(|le| le.status)
}

/// Fallback result: (response, finish_reason, completion_tokens); a `None`
/// response comes with finish_reason "error".
type CallResult = (Option<String>, Option<String>, i64);

impl<'a> Cepo<'a> {
    /// `llm_call`: one create() with `stream=False`, retried on 400 / 5xx.
    async fn llm_call(
        &self,
        request: &Map<String, Value>,
    ) -> Result<(String, Option<String>, i64)> {
        let retries = self.cfg.lock().unwrap().int("num_of_retries") + 1;
        let mut attempt = 0;
        loop {
            let mut body = Map::new();
            body.insert("stream".into(), Value::Bool(false));
            for (k, v) in request {
                body.insert(k.clone(), v.clone());
            }
            match self.ctx.create(Value::Object(body)).await {
                Ok(resp) => {
                    if resp.choices().is_empty() {
                        return Err(anyhow!("list index out of range"));
                    }
                    let content = resp.content(0).map(|c| py::strings::strip(&c).to_string());
                    let finish = resp.finish_reason(0);
                    let tokens = resp
                        .usage()
                        .and_then(|u| u.get("completion_tokens"))
                        .and_then(|t| t.as_i64())
                        .unwrap_or(0);
                    let text = remove_think_section(&content.unwrap_or_default());
                    return Ok((text, finish, tokens));
                }
                Err(e) => {
                    let s = status_of(&e);
                    let retryable = matches!(s, Some(400)) || matches!(s, Some(x) if x >= 500);
                    if retryable && attempt < retries - 1 {
                        tokio::time::sleep(std::time::Duration::from_secs_f64(
                            0.2 * (attempt + 1) as f64,
                        ))
                        .await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// `llm_call_reason_effort_fallback`.
    async fn fallback(
        &self,
        request: &mut Map<String, Value>,
        levels: &[&str],
    ) -> Result<CallResult> {
        let use_fallback = self.cfg.lock().unwrap().truthy("use_reasoning_fallback");
        let levels: Vec<&str> = if use_fallback {
            levels.to_vec()
        } else {
            vec!["high"]
        };
        for effort in &levels {
            if self.cfg.lock().unwrap().truthy("use_reasoning") {
                request.insert("reasoning_effort".into(), json!(effort));
            } else {
                request.shift_remove("reasoning_effort");
            }
            match self.llm_call(request).await {
                Ok((resp, finish, tokens)) => {
                    if finish.as_deref() != Some("length") {
                        return Ok((Some(resp), finish, tokens));
                    }
                }
                Err(e) => match status_of(&e) {
                    Some(400) => {
                        let message = e.to_string();
                        if levels.len() == 1
                            && message
                                .starts_with("Error code: 400 - {'error': {'message': 'think value")
                        {
                            tracing::info!("The think level {} was not supported by the model; Disabling thinking", effort);
                            self.cfg
                                .lock()
                                .unwrap()
                                .0
                                .insert("use_reasoning".into(), Value::Bool(false));
                        }
                    }
                    Some(s) if s >= 500 => continue,
                    _ => return Err(e),
                },
            }
        }
        Ok((None, Some("error".into()), 0))
    }

    fn request(
        &self,
        messages: &[Value],
        max_tokens: Value,
        temperature: Value,
        top_p: f64,
    ) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("model".into(), json!(self.model));
        m.insert("messages".into(), Value::Array(messages.to_vec()));
        m.insert("max_tokens".into(), max_tokens);
        m.insert("temperature".into(), temperature);
        m.insert("top_p".into(), json!(top_p));
        m
    }

    /// `generate_single_plan(i)` -> (i, plan, tokens).
    async fn generate_single_plan(
        &self,
        i: usize,
        system_prompt: &str,
        question_only: &str,
        approach: Option<&str>,
    ) -> Result<(usize, Option<String>, i64)> {
        let cfg = cfg_snapshot(self.cfg);
        let mut local_tokens = 0;
        let mut messages: Vec<Value>;
        let step1_tokens = cfg.int("planning_max_tokens_step1");
        if step1_tokens != 0 {
            let content = if cfg.truthy("use_plan_diversity") {
                let approach = approach.ok_or_else(|| anyhow!("AssertionError"))?;
                #[rustfmt::skip]
                let c = format!(
                    concat!(
                        "To answer this question, can you come up with a concise plan using to solve it step-by-step but do not provide the final answer. Here is the approach you need to follow to generate the plan: {p0}. Also, for each step, provide your confidence in the correctness of that step as well as your ability to execute it correctly. Here is the question:\n",
                        "{p1}\n",
                        "Read the question again:\n",
                        "\n",
                        "{p2}",
                    ),
                    p0 = approach, p1 = question_only, p2 = question_only,
                );
                c
            } else {
                if approach.is_some() {
                    return Err(anyhow!("AssertionError"));
                }
                #[rustfmt::skip]
                let c = format!(
                    concat!(
                        "To answer this question, can you come up with a concise plan to solve it step-by-step but do not provide the final answer. Also, for each step, provide your confidence in the correctness of that step as well as your ability to execute it correctly. Here is the question:\n",
                        "{p0}\n",
                        "Read the question again:\n",
                        "\n",
                        "{p1}",
                    ),
                    p0 = question_only, p1 = question_only,
                );
                c
            };
            messages = vec![msg("system", system_prompt), msg("user", &content)];
            let mut req = self.request(
                &messages,
                cfg.get("planning_max_tokens_step1"),
                cfg.get("planning_temperature_step1"),
                1.0,
            );
            let (response, finish, tokens) = self.fallback(&mut req, &["high", "medium"]).await?;
            local_tokens += tokens;
            if finish.as_deref() == Some("length") {
                return Ok((i, None, local_tokens));
            }
            messages.push(json!({"role": "assistant", "content": response}));
            messages.push(msg("user", "Can you execute the above plan step-by-step to produce the final answer. Be extra careful when executing steps where your confidence is lower. /think"));
        } else {
            #[rustfmt::skip]
            let c = format!(
                concat!(
                    "Can you solve this problem step-by-step to produce the final answer? Here is the question:\n",
                    "{p0}\n",
                    "Read the question again:\n",
                    "\n",
                    "{p1} /think",
                ),
                p0 = question_only, p1 = question_only,
            );
            messages = vec![msg("user", &c)];
        }
        let mut req = self.request(
            &messages,
            cfg.get("planning_max_tokens_step1"),
            cfg.get("planning_temperature_step1"),
            1.0,
        );
        let (response, finish, tokens) = self.fallback(&mut req, &["high", "medium"]).await?;
        local_tokens += tokens;
        if finish.as_deref() == Some("length") {
            return Ok((i, None, local_tokens));
        }
        Ok((i, response, local_tokens))
    }

    /// `generate_completion`.
    async fn generate_completion(
        &self,
        system_prompt: &str,
        task: &str,
        approach: Option<&str>,
    ) -> Result<(String, i64)> {
        let cfg = cfg_snapshot(self.cfg);
        let mut completion_tokens = 0;
        let question_only = extract_question_only(task);
        let planning_m = cfg.int("planning_m").max(0) as usize;
        let planning_n = cfg.int("planning_n");

        let mut pending: FuturesUnordered<_> = (0..planning_m)
            .map(|i| self.generate_single_plan(i, system_prompt, &question_only, approach))
            .collect();
        let mut plans: Vec<(usize, String)> = Vec::new();
        let mut failure: Option<anyhow::Error> = None;
        while let Some(result) = pending.next().await {
            match result {
                Ok((i, plan, tokens)) => {
                    completion_tokens += tokens;
                    if let Some(p) = plan.filter(|p| !p.is_empty()) {
                        plans.push((i, p));
                    }
                    if plans.len() as i64 == planning_n {
                        break;
                    }
                }
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        // The executor still waits for every submitted attempt.
        while pending.next().await.is_some() {}
        if let Some(e) = failure {
            return Err(e);
        }
        plans.sort_by_key(|(i, _)| *i);
        let mut plans: Vec<String> = plans.into_iter().map(|(_, p)| p).collect();

        if plans.is_empty() {
            let messages = vec![msg("user", &question_only)];
            let mut req = self.request(
                &messages,
                cfg.get("planning_max_tokens_direct_resp"),
                cfg.get("planning_temperature_direct_resp"),
                0.95,
            );
            let (response, finish, tokens) =
                self.fallback(&mut req, &["high", "medium", "low"]).await?;
            completion_tokens += tokens;
            let response = match response {
                Some(r) if finish.as_deref() != Some("length") => r,
                _ => String::new(),
            };
            plans.push(response);
        }

        let mut plans_message = String::new();
        for (i, plan) in plans.iter().enumerate() {
            plans_message += &format!("Response {}:\n{}\n\n", i + 1, plan);
        }
        let plans_message = py::strings::rstrip(&plans_message).to_string();
        #[rustfmt::skip]
        let content = format!(
            concat!(
                "Can you review your last {p0} responses and identify any inconsistency between them. After that, can you address it and present a final step-by-step solution to the problem? Here is the question:\n",
                "{p1} /think",
            ),
            p0 = plans.len(), p1 = question_only,
        );
        let user_content = format!(
            "Previous responses to review:\n\n{}\n\n{}",
            plans_message, content
        );
        let messages = vec![msg("system", system_prompt), msg("user", &user_content)];
        let mut req = self.request(
            &messages,
            cfg.get("planning_max_tokens_step1"),
            cfg.get("planning_temperature_step1"),
            1.0,
        );
        let (response, finish, tokens) = self.fallback(&mut req, &["high", "medium"]).await?;
        completion_tokens += tokens;
        let final_solution = match response {
            Some(r) if finish.as_deref() != Some("length") => r,
            _ => plans[0].clone(),
        };

        let final_output = if cfg.int("planning_max_tokens_step4") != 0 {
            #[rustfmt::skip]
            let content = format!(
                concat!(
                    "Use your final solution from above to correctly answer the question. Here is the question:\n",
                    "{p0} /think",
                ),
                p0 = task,
            );
            let messages = vec![
                msg("system", system_prompt),
                msg(
                    "user",
                    &format!(
                        "Here's my final solution: {}\n\nNow {}",
                        final_solution, content
                    ),
                ),
            ];
            let mut req = self.request(
                &messages,
                cfg.get("planning_max_tokens_step1"),
                cfg.get("planning_temperature_step1"),
                1.0,
            );
            let (response, finish, tokens) = self.fallback(&mut req, &["high", "medium"]).await?;
            completion_tokens += tokens;
            match response {
                Some(r) if finish.as_deref() != Some("length") => r,
                _ => final_solution,
            }
        } else {
            final_solution
        };
        Ok((final_output, completion_tokens))
    }

    /// `generate_approaches` (only reachable with use_plan_diversity).
    async fn generate_approaches(
        &self,
        system_prompt: &str,
        initial_query: &str,
        num_approach: i64,
    ) -> Result<(Vec<String>, i64)> {
        let question_only = extract_question_only(initial_query);
        #[rustfmt::skip]
        let content = format!(
            concat!(
                "To answer the question: \"{p0}\", please propose {p1} different high-level approaches to solve the problem. All approaches should be fundamentally different from each other and easily excecutable without too much steps. Do not include a step-by-step plan or the final answer. You must present the approaches in the following JSON format which is directly loadable:\n",
                "{{\n",
                "    \"approach_1\": \"<Description of approach 1>\",\n",
                "    \"approach_2\": \"<Description of approach 2>\",\n",
                "    \"approach_3\": \"<Description of approach 3>\",\n",
                "    ...\n",
                "}}",
            ),
            p0 = question_only, p1 = num_approach,
        );
        let _ = (system_prompt, content);
        // Upstream reads cepo_config.planning_max_tokens_step0, which the
        // CepoConfig dataclass does not define.
        Err(anyhow!(
            "'CepoConfig' object has no attribute 'planning_max_tokens_step0'"
        ))
    }

    /// `generate_n_completions`.
    async fn generate_n_completions(
        &self,
        system_prompt: &str,
        initial_query: &str,
    ) -> Result<(Vec<String>, i64)> {
        let cfg = cfg_snapshot(self.cfg);
        let n = cfg.int("bestofn_n").max(0) as usize;
        let mut completion_tokens = 0;
        let mut approaches: Option<Vec<String>> = None;
        if cfg.truthy("use_plan_diversity") {
            let (a, t) = self
                .generate_approaches(system_prompt, initial_query, n as i64)
                .await?;
            completion_tokens += t;
            approaches = Some(a);
        }
        let mut completions: Vec<Option<String>> = vec![None; n];
        let approaches_ref = approaches.as_ref().filter(|a| !a.is_empty());
        let mut pending: FuturesUnordered<_> = (0..n)
            .map(|i| async move {
                let approach = approaches_ref.and_then(|a| a.get(i)).map(|s| s.as_str());
                let r = self
                    .generate_completion(system_prompt, initial_query, approach)
                    .await;
                (i, r)
            })
            .collect();
        let mut failure = None;
        while let Some((i, r)) = pending.next().await {
            match r {
                Ok((resp, tokens)) => {
                    completions[i] = Some(resp);
                    completion_tokens += tokens;
                }
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        while pending.next().await.is_some() {}
        if let Some(e) = failure {
            return Err(e);
        }
        Ok((
            completions
                .into_iter()
                .map(|c| c.unwrap_or_default())
                .collect(),
            completion_tokens,
        ))
    }

    async fn rate_absolute(
        &self,
        system_prompt: &str,
        initial_query: &str,
        completions: &[String],
    ) -> Result<(String, i64)> {
        let cfg = cfg_snapshot(self.cfg);
        #[rustfmt::skip]
        let rating_prompt = concat!(
            "Please act as an impartial judge and evaluate the accuracy of the response provided by an AI assistant to the user question displayed below. Your evaluation should consider only correctness and accuracy as the primary factor. Evaluation Criteria:\n",
            "- Correctness: How free is it from errors or mistakes?\n",
            "- Accuracy: Are the information and explanations factually correct?\n",
            "Evaluation Process:\n",
            "1. Carefully review the user question and the AI assistant's response.\n",
            "2. Assess the response for any inaccuracies in reasoning as well as execution.\n",
            "3. Provide a detailed explanation of your step-by-step evaluation.\n",
            "4. Identify if the final answer is correct or not. \n",
            "Begin your evaluation by thinking through the given problem and response step-by-step. VERY IMPORTANT: Re-do any calculations present and check if you arrive at the same answer. Throughly check for any inaccuracies in reasoning and calculations for each step. Be as objective as possible. After providing your detailed explanation, please rate the response as 0 or 1, (0 for incorrect and 1 for correct) by strictly following this format: \"Rating: [[rating]]\", for example: \"Rating: [[0]]\"",
        );
        #[rustfmt::skip]
        let rating_format_instruction = concat!(
            "\n",
            "\n",
            "Rate the above response beginning with the detailed explanation followed by a rating of 0 or 1 by strictly following this format: \"Explanation: <reason for your rating>\n",
            "\n",
            "Rating: [[rating]]\"",
        );
        let re = Regex::new(r"Rating: \[\[(\d+)\]\]").unwrap();
        let mut tokens = 0;
        let mut ratings: Vec<f64> = Vec::new();
        for completion in completions {
            let system_content = format!(
                "USER QUESTION: {}\n\nRESPONSE: {}",
                initial_query, completion
            );
            let messages = vec![
                msg("system", &format!("{}\n\n{}", system_prompt, rating_prompt)),
                msg(
                    "user",
                    &format!("{}{}", system_content, rating_format_instruction),
                ),
            ];
            let mut req = self.request(
                &messages,
                cfg.get("bestofn_max_tokens"),
                cfg.get("bestofn_temperature"),
                1.0,
            );
            // Upstream makes a direct create() first and discards it.
            self.ctx.create(Value::Object(req.clone())).await?;
            let (rating_response, _, t) = self.fallback(&mut req, &["high", "medium"]).await?;
            tokens += t;
            let rating_response = rating_response
                .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?;
            let rating = re
                .captures(&rating_response)
                .map(|c| c[1].to_string())
                .unwrap_or_else(|| "-1".into());
            ratings.push(py::strings::parse_float(&rating).unwrap_or(-1.0));
        }
        let best = py::strings::argmax_by(&ratings, |r| *r)
            .ok_or_else(|| anyhow!("max() iterable argument is empty"))?;
        Ok((completions[best].clone(), tokens))
    }

    async fn rate_pairwise(
        &self,
        system_prompt: &str,
        initial_query: &str,
        completions: &[String],
    ) -> Result<(String, i64)> {
        let cfg = cfg_snapshot(self.cfg);
        #[rustfmt::skip]
        let rating_prompt = concat!(
            "Please act as an impartial judge and compare the quality of the two responses provided by the AI assistant to the user's question displayed below. Evaluation Criteria:\n",
            "- Helpfulness: How effectively does the response meet the user's needs?\n",
            "- Relevance: How directly does the response address the original question?\n",
            "- Accuracy: Are the information and explanations factually correct?\n",
            "- Depth: Does the response provide comprehensive and meaningful insights?\n",
            "- Creativity: Does the response offer unique or innovative perspectives?\n",
            "- Clarity: Is the response well-organized, coherent, and easy to understand?\n",
            "Evaluation Process:\n",
            "1. Carefully review the user's question and the AI assistant's responses.\n",
            "2. Compare the responses against each other for each criterion.\n",
            "3. Provide a concise explanation of your overall evaluation.\n",
            "4. Select the response that is superior based on the above criteria.\n",
            "Reply with \"Better Response: [[response id]]\".\n",
            "If the first response is better, reply with \"Better Response: [[0]]\". If the second response is better, reply with \"Better Response: [[1]]\".",
        );
        let n = cfg.int("bestofn_n").max(0) as usize;
        let mut ratings = vec![0i64; n];
        let re = Regex::new(r"Better Response: \[\[(\d+)\]\]").unwrap();
        let mut tokens = 0;
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let (a, b) = match (completions.get(i), completions.get(j)) {
                    (Some(a), Some(b)) => (a, b),
                    _ => return Err(anyhow!("list index out of range")),
                };
                #[rustfmt::skip]
                let comparison = format!(
                    concat!(
                        "User Question: {p0}\n",
                        "\n",
                        "Response 0: {p1}\n",
                        "\n",
                        "Response 1: {p2}\n",
                        "\n",
                        "Which response is better? Please provide your reasoning and then indicate your choice with \"Better Response: [[0]]\" if the first response is better, or \"Better Response: [[1]]\" if the second response is better.",
                    ),
                    p0 = initial_query, p1 = a, p2 = b,
                );
                let messages = vec![
                    msg("system", &format!("{}\n\n{}", system_prompt, rating_prompt)),
                    msg("user", &comparison),
                ];
                let response = self
                    .ctx
                    .create(json!({
                        "model": self.model,
                        "messages": messages,
                        "max_tokens": cfg.get("bestofn_max_tokens"),
                        "temperature": cfg.get("bestofn_temperature")
                    }))
                    .await?;
                tokens += response.completion_tokens()?;
                let text = response
                    .content_at(0)?
                    .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
                let text = py::strings::strip(&text).to_string();
                let pair = [i, j];
                match re.captures(&text) {
                    Some(c) => match c[1].parse::<usize>() {
                        Ok(r) if r < 2 => ratings[pair[r]] += 1,
                        Ok(_) => return Err(anyhow!("tuple index out of range")),
                        Err(_) => ratings[pair[0]] += 1,
                    },
                    None => ratings[pair[0]] += 1,
                }
            }
        }
        let fr: Vec<f64> = ratings.iter().map(|r| *r as f64).collect();
        let best = py::strings::argmax_by(&fr, |r| *r)
            .ok_or_else(|| anyhow!("max() iterable argument is empty"))?;
        let chosen = completions
            .get(best)
            .ok_or_else(|| anyhow!("list index out of range"))?;
        Ok((chosen.clone(), tokens))
    }
}

/// `MCQ_PATTERNS` (Python `re`, compiled with fancy-regex for lookarounds).
fn mcq_patterns() -> &'static Vec<fancy_regex::Regex> {
    static P: OnceLock<Vec<fancy_regex::Regex>> = OnceLock::new();
    P.get_or_init(|| {
        let raw: [&str; 13] = [
            r"(?ix)(?:\*{1,2}|_{1,2})Answer[s]?\s*[:\-–]?(?:\*{1,2}|_{1,2})\s*([ABCD])\b",
            r"(?imx)^\s*(?:\*{1,2}|_{1,2})?Answer:?(?:\*{1,2}|_{1,2})?\s*:?\s*(?:\*{1,2}|_{1,2})?([ABCD])(?:\*{1,2}|_{1,2})?\s*",
            r"(?ix)\bAnswer[s]?\b\s*[:\-–]?\s*\(\s*([ABCD])\s*\)",
            r"(?ix)\bAnswer[s]?\b\s*[:\-–]?\s*([ABCD])\b",
            r"(?ix)\b(?:Option|Choice)\b\s*[:\-–]?\s*([ABCD])\b",
            r"(?mx)\\boxed\{[^}]*?([ABCD])[^}]*\}",
            r"(?mx)\\boxed\{[^}]*?\\textbf\{[^}]*?([ABCD])[^}]*\}[^}]*\}",
            r"(?mx)\\boxed\{[^}]*?\\text\{[^}]*?([ABCD])[^}]*\}[^}]*\}",
            r"(?x)(?<![A-Za-z0-9])[\(\[]\s*([ABCD])\s*[\)\]](?![A-Za-z0-9])",
            r"(?x)(?<![A-Za-z0-9])(?:\*{1,2}|_{1,2})([ABCD])(?:\*{1,2}|_{1,2})(?![A-Za-z0-9])",
            r"(?x)\\textbf\{[^}]*?([ABCD])[^}]*\}",
            r"(?x)(?<![A-Za-z0-9])(?:\*{1,2}|_{1,2})\s*([ABCD])\)[^*_\n]+?(?:\*{1,2}|_{1,2})(?![A-Za-z0-9])",
            r"(?mx)^\s*(?:\*{1,2}|_{1,2})?([ABCD])(?:\*{1,2}|_{1,2})?\s*[\.\)\-–:]?\s*.*$",
        ];
        raw.iter()
            .map(|p| fancy_regex::Regex::new(p).expect("MCQ pattern"))
            .collect()
    })
}

/// `extract_abcd`: the first pattern (in priority order) that matches wins.
pub fn extract_abcd(text: &str) -> String {
    for pat in mcq_patterns() {
        if let Ok(Some(c)) = pat.captures(text) {
            if let Some(g) = c.get(1) {
                let letter = g.as_str().to_uppercase();
                if "ABCD".contains(letter.as_str()) {
                    return letter;
                }
            }
        }
    }
    py::strings::prefix(text.strip_prefix("**").unwrap_or(text), 1)
}

/// `rate_completions_majority` (MCQ vote; the math_verify path is only
/// reached with no completions, where it returns None).
fn rate_majority(completions: &[String]) -> Option<String> {
    let last_n = 150isize;
    let extracted: Vec<(String, String)> = completions
        .iter()
        .map(|r| {
            (
                r.clone(),
                extract_abcd(&py::strings::slice(r, Some(-last_n), None)),
            )
        })
        .collect();
    let mut counts: Vec<(String, usize)> = Vec::new();
    for (_, a) in &extracted {
        match counts.iter_mut().find(|(k, _)| k == a) {
            Some(e) => e.1 += 1,
            None => counts.push((a.clone(), 1)),
        }
    }
    let best = counts
        .iter()
        .fold(None::<&(String, usize)>, |acc, c| match acc {
            Some(a) if a.1 >= c.1 => Some(a),
            _ => Some(c),
        })?;
    extracted
        .iter()
        .find(|(_, a)| *a == best.0)
        .map(|(r, _)| r.clone())
}

pub async fn cepo(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let cfg = global_config(&ctx.config)?;
    let snapshot = cfg_snapshot(cfg);
    let rating_model = match snapshot.get("rating_model") {
        Value::String(s) if !s.is_empty() => s,
        _ => ctx.model.clone(),
    };
    let planner = Cepo {
        ctx,
        cfg,
        model: ctx.model.clone(),
    };
    let (completions, planning_tokens) = planner
        .generate_n_completions(system_prompt, initial_query)
        .await?;
    let completions: Vec<String> = completions.into_iter().filter(|c| !c.is_empty()).collect();
    let rater = Cepo {
        ctx,
        cfg,
        model: rating_model,
    };
    let rating_type = py::json::to_str(&snapshot.get("bestofn_rating_type"));
    let (best, rating_tokens) = match rating_type.as_str() {
        "absolute" => {
            rater
                .rate_absolute(system_prompt, initial_query, &completions)
                .await?
        }
        "pairwise" => {
            rater
                .rate_pairwise(system_prompt, initial_query, &completions)
                .await?
        }
        "majority" => match rate_majority(&completions) {
            Some(r) => (r, 0),
            None => return Ok((Value::Null, planning_tokens)),
        },
        _ => return Err(anyhow!("Invalid rating type in cepo_config")),
    };
    Ok((Value::String(best), planning_tokens + rating_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcq_extraction() {
        assert_eq!(extract_abcd("**Answer:** C"), "C");
        assert_eq!(extract_abcd("The answer is (B)."), "B");
        assert_eq!(extract_abcd("Answer: D"), "D");
        assert_eq!(extract_abcd("\\boxed{A}"), "A");
        assert_eq!(extract_abcd("**hello"), "h");
    }

    #[test]
    fn default_config_loads() {
        let c = CepoConfig::load(&ServerConfig::default()).unwrap();
        assert_eq!(c.get("bestofn_n"), json!(3));
        assert_eq!(c.get("planning_temperature_step1"), json!(0.55));
    }
}
