//! Port of upstream `optillm/plugins/deepthink_plugin.py` and the
//! `deepthink/` package (SELF-DISCOVER + uncertainty-routed CoT).

mod reasoning_modules;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::approaches::{py_max, py_min, ApproachResult, Ctx};
use crate::client::ChatCompletion;
use crate::py;
use reasoning_modules::REASONING_MODULES;

const TRUNCATED: &str = "Error: Response was truncated due to token limit. Please increase max_tokens or max_completion_tokens.";

fn fallback_structure() -> Value {
    json!({
        "problem_understanding": "Analyze and understand the problem requirements",
        "solution_approach": "Determine the best approach based on problem characteristics",
        "step_by_step_reasoning": "Work through the problem systematically",
        "verification": "Verify the solution is correct and complete",
        "final_answer": "State the final answer clearly"
    })
}

struct Config {
    samples: i64,
    confidence_threshold: f64,
    max_tokens: Value,
    temperature: Value,
    top_p: Value,
    enable_self_discover: bool,
}

/// `_parse_config`.
fn parse_config(rc: &Map<String, Value>) -> Config {
    let num = |k: &str, d: f64| rc.get(k).and_then(|v| v.as_f64()).unwrap_or(d);
    let max_tokens = if let Some(v) = rc.get("max_completion_tokens") {
        v.clone()
    } else if let Some(v) = rc.get("max_tokens") {
        v.clone()
    } else {
        json!(16382)
    };
    let clamp_value = |k: &str, d: f64, lo: f64, hi: f64, int: bool| -> Value {
        let raw = rc
            .get(k)
            .cloned()
            .unwrap_or(if int { json!(d as i64) } else { json!(d) });
        match raw.as_f64() {
            Some(x) => {
                let c = py_max(lo, py_min(hi, x));
                if c == x {
                    raw
                } else if int {
                    json!(c as i64)
                } else {
                    json!(c)
                }
            }
            None => raw,
        }
    };
    Config {
        samples: py_max(1.0, py_min(10.0, num("deepthink_samples", 3.0))) as i64,
        confidence_threshold: py_max(0.0, py_min(1.0, num("confidence_threshold", 0.7))),
        max_tokens,
        temperature: clamp_value("temperature", 0.7, 0.0, 2.0, false),
        top_p: clamp_value("top_p", 0.95, 0.0, 1.0, false),
        enable_self_discover: rc
            .get("enable_self_discover")
            .map(|v| !matches!(v, Value::Bool(false) | Value::Null) && v.as_f64() != Some(0.0))
            .unwrap_or(true),
    }
}

/// `_extract_task_description`.
fn extract_task_description(initial_query: &str, system_prompt: &str) -> String {
    let combined = format!("{}\n\n{}", system_prompt, initial_query).to_lowercase();
    let task_keywords: [(&str, [&str; 6]); 6] = [
        (
            "mathematical",
            [
                "solve",
                "calculate",
                "equation",
                "math",
                "number",
                "formula",
            ],
        ),
        (
            "analytical",
            [
                "analyze", "evaluate", "assess", "examine", "compare", "\u{0}",
            ],
        ),
        (
            "creative",
            [
                "create",
                "design",
                "generate",
                "brainstorm",
                "invent",
                "\u{0}",
            ],
        ),
        (
            "logical",
            ["reason", "logic", "prove", "deduce", "conclude", "\u{0}"],
        ),
        (
            "planning",
            ["plan", "strategy", "approach", "method", "steps", "\u{0}"],
        ),
        (
            "problem_solving",
            ["problem", "solution", "solve", "fix", "resolve", "\u{0}"],
        ),
    ];
    let detected: Vec<&str> = task_keywords
        .iter()
        .filter(|(_, kws)| kws.iter().any(|k| *k != "\u{0}" && combined.contains(k)))
        .map(|(t, _)| *t)
        .collect();
    let mut description = if let Some(first) = detected.first() {
        format!(
            "This is primarily a {} task that requires {} thinking.",
            first,
            detected.join(", ")
        )
    } else {
        "This is a general reasoning task that requires systematic thinking and analysis."
            .to_string()
    };
    if py::strings::len(initial_query) > 50 {
        description += &format!(
            " The specific task involves: {}...",
            py::strings::prefix(initial_query, 200)
        );
    } else {
        description += &format!(" The specific task is: {}", initial_query);
    }
    description
}

fn unusable(r: &ChatCompletion) -> bool {
    r.is_unusable()
}

struct DeepThink<'a> {
    ctx: &'a Ctx,
    tokens: i64,
}

impl<'a> DeepThink<'a> {
    async fn call(
        &mut self,
        prompt: &str,
        max_tokens: Value,
        temperature: Value,
        top_p: Option<Value>,
    ) -> Result<ChatCompletion> {
        let mut body = json!({
            "model": self.ctx.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": temperature
        });
        if let Some(t) = top_p {
            body["top_p"] = t;
        }
        let r = self.ctx.create(body).await?;
        self.tokens += r.completion_tokens()?;
        Ok(r)
    }

    async fn select_modules(&mut self, task: &str) -> Result<Vec<usize>> {
        let modules_text = REASONING_MODULES
            .iter()
            .enumerate()
            .map(|(i, (n, d))| format!("{}. {}: {}", i + 1, n, d))
            .collect::<Vec<_>>()
            .join("\n");
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert in problem-solving and reasoning. Given a task description and available reasoning modules, select the most relevant modules that would be useful for solving this type of task.\n",
                "\n",
                "Task description: {p0}{p1}\n",
                "\n",
                "Available reasoning modules:\n",
                "{p2}\n",
                "\n",
                "Instructions:\n",
                "1. Analyze the task and identify what types of reasoning would be most helpful\n",
                "2. Select 3-7 reasoning modules that are most relevant for this task\n",
                "3. Consider both the complexity of the task and the complementary nature of different modules\n",
                "4. Avoid selecting too many similar modules\n",
                "5. IMPORTANT: Respond ONLY with a valid JSON array of numbers\n",
                "\n",
                "Example response format: [1, 5, 9, 15, 23]\n",
                "\n",
                "Selected modules (JSON array only):",
            ),
            p0 = task, p1 = "", p2 = modules_text,
        );
        let r = self.call(&prompt, json!(1024), json!(0.3), None).await?;
        let fallback: Vec<usize> = (0..5).collect();
        if unusable(&r) {
            return Ok(fallback);
        }
        let text = py::strings::strip(&r.content(0).unwrap_or_default()).to_string();
        let indices: Vec<i64> = match Regex::new(r"\[[\d,\s]+\]").unwrap().find(&text) {
            Some(m) => match serde_json::from_str::<Vec<i64>>(m.as_str()) {
                Ok(v) => v,
                Err(_) => return Ok(fallback),
            },
            None => Regex::new(r"\b(\d+)\b")
                .unwrap()
                .captures_iter(&text)
                .take(7)
                .filter_map(|c| c[1].parse().ok())
                .collect(),
        };
        let selected: Vec<usize> = indices
            .into_iter()
            .filter(|i| *i >= 1 && (*i as usize) <= REASONING_MODULES.len())
            .map(|i| (i - 1) as usize)
            .take(7)
            .collect();
        Ok(selected)
    }

    async fn adapt_modules(&mut self, selected: &[usize], task: &str) -> Result<Vec<String>> {
        let modules_text = selected
            .iter()
            .map(|i| format!("- {}", REASONING_MODULES[*i].1))
            .collect::<Vec<_>>()
            .join("\n");
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert in adapting general reasoning strategies to specific tasks. Given the selected reasoning modules and task description, rephrase each module to be more specific and tailored to this particular type of task.\n",
                "\n",
                "Task description: {p0}{p1}\n",
                "\n",
                "Selected reasoning modules:\n",
                "{p2}\n",
                "\n",
                "Instructions:\n",
                "1. For each module, rephrase the description to be more specific to this task\n",
                "2. Keep the core reasoning approach but make it more actionable for this specific type of problem\n",
                "3. Use terminology and concepts relevant to the task domain\n",
                "4. Make the adapted descriptions more concrete and specific\n",
                "\n",
                "Provide the adapted modules as a numbered list:",
            ),
            p0 = task, p1 = "", p2 = modules_text,
        );
        let r = self.call(&prompt, json!(2048), json!(0.3), None).await?;
        if unusable(&r) {
            return Ok(selected
                .iter()
                .map(|i| REASONING_MODULES[*i].1.to_string())
                .collect());
        }
        let text = py::strings::strip(&r.content(0).unwrap_or_default()).to_string();
        let numbered = Regex::new(r"^\d+\.").unwrap();
        let prefix = Regex::new(r"^\d+\.\s*").unwrap();
        Ok(text
            .split('\n')
            .map(py::strings::strip)
            .filter(|l| numbered.is_match(l))
            .map(|l| prefix.replace(l, "").to_string())
            .collect())
    }

    async fn implement_structure(&mut self, adapted: &[String], task: &str) -> Result<Value> {
        let modules_text = adapted
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. {}", i + 1, m))
            .collect::<Vec<_>>()
            .join("\n");
        #[rustfmt::skip]
        let demo = concat!(
            "{\n",
            "    \"problem_analysis\": \"Analyze the core components and requirements\",\n",
            "    \"approach_selection\": \"Choose the most appropriate solution method\",\n",
            "    \"step_by_step_solution\": {\n",
            "        \"step_1\": \"First logical step with clear reasoning\",\n",
            "        \"step_2\": \"Second step building on previous results\", \n",
            "        \"step_3\": \"Continue logical progression\"\n",
            "    },\n",
            "    \"verification\": \"Check the solution for accuracy and completeness\",\n",
            "    \"final_answer\": \"Present the final result clearly\"\n",
            "}",
        );
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert in creating structured reasoning plans. Given the adapted reasoning modules for a specific task, create a detailed JSON reasoning structure that can be followed step-by-step to solve instances of this task.\n",
                "\n",
                "Task description: {p0}{p1}\n",
                "\n",
                "Adapted reasoning modules:\n",
                "{p2}\n",
                "\n",
                "Example of a reasoning structure format:\n",
                "{p3}\n",
                "\n",
                "Instructions:\n",
                "1. Create a JSON structure that operationalizes the adapted reasoning modules\n",
                "2. The structure should be specific enough to guide step-by-step reasoning\n",
                "3. Include clear field names that indicate what should be filled in each step\n",
                "4. Make it actionable - each field should represent a concrete reasoning step\n",
                "5. Ensure the structure flows logically from problem understanding to final answer\n",
                "6. The structure should be comprehensive enough to handle the complexity of the task\n",
                "\n",
                "7. IMPORTANT: Return ONLY valid JSON with double quotes around all property names and string values\n",
                "8. Do not include any text before or after the JSON structure\n",
                "\n",
                "Valid JSON reasoning structure:",
            ),
            p0 = task, p1 = "", p2 = modules_text, p3 = demo,
        );
        let r = self.call(&prompt, json!(2048), json!(0.3), None).await?;
        if unusable(&r) {
            return Ok(fallback_structure());
        }
        let text = py::strings::strip(&r.content(0).unwrap_or_default()).to_string();
        Ok(parse_json_structure(&text))
    }
}

fn non_empty_dict(v: Result<Value>) -> Option<Value> {
    match v {
        Ok(Value::Object(m)) if !m.is_empty() => Some(Value::Object(m)),
        _ => None,
    }
}

fn loads(s: &str) -> Result<Value> {
    serde_json::from_str(s).map_err(|e| anyhow!("{} line {} column {}", e, e.line(), e.column()))
}

/// `_parse_json_structure`: the four extraction strategies in order.
fn parse_json_structure(text: &str) -> Value {
    let strategies: [fn(&str) -> Result<Value>; 4] =
        [strategy_1, strategy_2, strategy_3, strategy_4];
    for s in strategies {
        if let Some(v) = non_empty_dict(s(text)) {
            return v;
        }
    }
    fallback_structure()
}

fn strategy_1(text: &str) -> Result<Value> {
    let start = text
        .find('{')
        .ok_or_else(|| anyhow!("No opening brace found"))?;
    let mut depth = 0i64;
    let mut end = start;
    for (i, c) in text[start..].char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                end = start + i + 1;
                break;
            }
        }
    }
    if depth != 0 {
        return Err(anyhow!("Unbalanced braces"));
    }
    loads(&text[start..end])
}

fn strategy_2(text: &str) -> Result<Value> {
    let m = Regex::new(r"\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}")
        .unwrap()
        .find(text)
        .ok_or_else(|| anyhow!("No JSON object found with regex"))?;
    loads(m.as_str())
}

fn strategy_3(text: &str) -> Result<Value> {
    for p in [
        r"(?s)```json\s*([^`]+)```",
        r"(?s)```\s*([^`]+)```",
        r"(?s)`([^`]+)`",
    ] {
        if let Some(c) = Regex::new(p).unwrap().captures(text) {
            if let Ok(v) = loads(py::strings::strip(&c[1])) {
                return Ok(v);
            }
        }
    }
    Err(anyhow!("No valid JSON found in code blocks"))
}

fn strategy_4(text: &str) -> Result<Value> {
    let m = Regex::new(r"(?s)\{.*\}")
        .unwrap()
        .find(text)
        .ok_or_else(|| anyhow!("No JSON-like content found"))?;
    let mut s = m.as_str().to_string();
    let cleanups: [(&str, &str); 4] = [
        (r#"(?<!\\)'([^']*)'(?=\s*[,}])"#, "\"${1}\""),
        (r"([{,]\s*)([a-zA-Z_][a-zA-Z0-9_]*)\s*:", "${1}\"${2}\":"),
        (r",\s*([}\]])", "${1}"),
        (r",,+", ","),
    ];
    for (p, rep) in cleanups {
        s = fancy_regex::Regex::new(p)
            .unwrap()
            .replace_all(&s, rep)
            .to_string();
    }
    match loads(&s) {
        Ok(v) => Ok(v),
        Err(e) if e.to_string().contains("line 1 column 2") => {
            let s2 = Regex::new(r"^[^{]*").unwrap().replace(&s, "").to_string();
            loads(&s2)
        }
        Err(e) => Err(e),
    }
}

fn extract_thinking(response: &str) -> String {
    Regex::new(r"(?s)<think>(.*?)</think>")
        .unwrap()
        .captures(response)
        .map(|c| py::strings::strip(&c[1]).to_string())
        .unwrap_or_default()
}

fn extract_answer(response: &str) -> String {
    let part = match response.find("</think>") {
        Some(i) => py::strings::strip(&response[i + 8..]).to_string(),
        None => py::strings::strip(response).to_string(),
    };
    for p in [
        r"(?i)(?:the )?(?:final )?answer is:?\s*(.+?)(?:\n|$)",
        r"(?i)(?:therefore|thus|so),?\s*(?:the )?(?:answer is:?\s*)?(.+?)(?:\n|$)",
        r"(?i)(?:conclusion|result):?\s*(.+?)(?:\n|$)",
    ] {
        if let Some(c) = Regex::new(p).unwrap().captures(&part) {
            return py::strings::strip(&c[1]).to_string();
        }
    }
    for line in part.split('\n') {
        let line = py::strings::strip(line);
        if !line.is_empty() && py::strings::len(line) > 10 {
            return line.to_string();
        }
    }
    py::strings::prefix(&part, 200)
}

fn normalize_answer(a: &str) -> String {
    let n = Regex::new(r"[^\w\s]")
        .unwrap()
        .replace_all(py::strings::strip(&a.to_lowercase()), "")
        .to_string();
    Regex::new(r"\s+").unwrap().replace_all(&n, " ").to_string()
}

struct Sample {
    full: String,
    thinking: String,
    answer: String,
}

fn most_common(items: &[String]) -> (String, usize) {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for i in items {
        match counts.iter_mut().find(|(k, _)| k == i) {
            Some(c) => c.1 += 1,
            None => counts.push((i.clone(), 1)),
        }
    }
    counts
        .into_iter()
        .fold(None::<(String, usize)>, |acc, c| match acc {
            Some(a) if a.1 >= c.1 => Some(a),
            _ => Some(c),
        })
        .unwrap_or_default()
}

fn evaluate_confidence(samples: &[Sample]) -> f64 {
    if samples.len() < 2 {
        return 0.5;
    }
    let answers: Vec<&str> = samples
        .iter()
        .map(|s| s.answer.as_str())
        .filter(|a| !a.is_empty())
        .collect();
    let thinking: Vec<&str> = samples
        .iter()
        .map(|s| s.thinking.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    if answers.is_empty() {
        return 0.1;
    }
    let answer_consistency = if answers.len() < 2 {
        0.5
    } else {
        let normalized: Vec<String> = answers.iter().map(|a| normalize_answer(a)).collect();
        let (_, count) = most_common(&normalized);
        let agreement = count as f64 / answers.len() as f64;
        let mut max_sim = 0.0;
        for i in 0..normalized.len() {
            for j in i + 1..normalized.len() {
                max_sim = py_max(max_sim, py::difflib::ratio(&normalized[i], &normalized[j]));
            }
        }
        py_min(py_max(agreement, max_sim), 1.0)
    };
    let reasoning_consistency = if thinking.len() < 2 {
        0.5
    } else {
        let mut sims = Vec::new();
        for i in 0..thinking.len() {
            for j in i + 1..thinking.len() {
                sims.push(py::difflib::ratio(
                    &thinking[i].to_lowercase(),
                    &thinking[j].to_lowercase(),
                ));
            }
        }
        if sims.is_empty() {
            0.5
        } else {
            py_min(
                py::strings::py_sum(sims.iter().copied()) / sims.len() as f64,
                1.0,
            )
        }
    };
    0.6 * answer_consistency + 0.4 * reasoning_consistency
}

fn majority_vote_response(samples: &[Sample]) -> String {
    let answers: Vec<String> = samples
        .iter()
        .filter(|s| !s.answer.is_empty())
        .map(|s| normalize_answer(&s.answer))
        .collect();
    if answers.is_empty() {
        return samples[0].full.clone();
    }
    let (best_answer, _) = most_common(&answers);
    let mut best: Option<&Sample> = None;
    let mut best_len = 0;
    for s in samples.iter().filter(|s| !s.answer.is_empty()) {
        if normalize_answer(&s.answer) == best_answer {
            let l = py::strings::len(&s.thinking);
            if l > best_len {
                best_len = l;
                best = Some(s);
            }
        }
    }
    if let Some(b) = best {
        return b.full.clone();
    }
    for s in samples.iter().filter(|s| !s.answer.is_empty()) {
        if normalize_answer(&s.answer) == best_answer {
            return s.full.clone();
        }
    }
    samples[0].full.clone()
}

fn clean_response(response: &str) -> String {
    let mut r = py::strings::strip(response).to_string();
    if !r.is_empty() && !r.ends_with(['.', '!', '?', ':', '"', '\'']) {
        let compact = r.replace([' ', ',', '.'], "");
        let is_digit = !compact.is_empty() && compact.chars().all(|c| c.is_ascii_digit());
        if !(is_digit || py::strings::split_whitespace(&r).len() <= 3) {
            r.push('.');
        }
    }
    r
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let config = parse_config(&ctx.request_config);
    let rc = &ctx.request_config;
    // UncertaintyRoutedCoT.max_tokens: max_completion_tokens or max_tokens or default
    let gen_max_tokens = match rc.get("max_completion_tokens") {
        Some(v) if !(v.is_null() || v.as_f64() == Some(0.0)) => v.clone(),
        _ => rc
            .get("max_tokens")
            .cloned()
            .unwrap_or(config.max_tokens.clone()),
    };
    let mut dt = DeepThink { ctx, tokens: 0 };

    let mut structure: Option<Value> = None;
    if config.enable_self_discover {
        let task = extract_task_description(initial_query, system_prompt);
        let selected = dt.select_modules(&task).await?;
        let adapted = dt.adapt_modules(&selected, &task).await?;
        structure = Some(dt.implement_structure(&adapted, &task).await?);
    }

    let base = format!("System: {}\n\nTask: {}", system_prompt, initial_query);
    let truthy_structure = matches!(&structure, Some(Value::Object(m)) if !m.is_empty());
    let prompt = if truthy_structure {
        #[rustfmt::skip]
        let p = format!(
            concat!(
                "{p0}\n",
                "\n",
                "REASONING STRUCTURE:\n",
                "Please follow this discovered reasoning structure to solve the problem systematically:\n",
                "\n",
                "{p1}\n",
                "\n",
                "INSTRUCTIONS:\n",
                "1. Use the reasoning structure above to guide your thinking process\n",
                "2. Work through each component of the structure systematically  \n",
                "3. Wrap your detailed reasoning process in <think> tags\n",
                "4. After your reasoning, provide a clear and concise final answer\n",
                "5. Be thorough in your analysis but also aim for clarity and accuracy\n",
                "\n",
                "<think>\n",
                "[Follow the reasoning structure step-by-step to analyze and solve the problem]\n",
                "</think>\n",
                "\n",
                "Based on my systematic analysis, the answer is:",
            ),
            p0 = base, p1 = py::json::dumps_indent(structure.as_ref().unwrap(), 2),
        );
        p
    } else {
        #[rustfmt::skip]
        let p = format!(
            concat!(
                "{p0}\n",
                "\n",
                "INSTRUCTIONS:\n",
                "Please solve this problem using careful step-by-step reasoning.\n",
                "\n",
                "1. Wrap your detailed reasoning process in <think> tags\n",
                "2. Consider the problem from multiple angles\n",
                "3. Work through the solution systematically\n",
                "4. Provide a clear and well-supported final answer\n",
                "\n",
                "<think>\n",
                "[Provide your detailed step-by-step reasoning here]\n",
                "</think>\n",
                "\n",
                "Based on my analysis, the answer is:",
            ),
            p0 = base,
        );
        p
    };

    // Uncertainty-routed generation.
    let mut samples = Vec::new();
    for _ in 0..config.samples {
        let r = dt
            .call(
                &prompt,
                gen_max_tokens.clone(),
                config.temperature.clone(),
                Some(config.top_p.clone()),
            )
            .await?;
        let full = if unusable(&r) {
            String::new()
        } else {
            py::strings::strip(&r.content(0).unwrap_or_default()).to_string()
        };
        samples.push(Sample {
            thinking: extract_thinking(&full),
            answer: extract_answer(&full),
            full,
        });
    }
    let g = dt
        .call(&prompt, gen_max_tokens.clone(), json!(0.0), None)
        .await?;
    let greedy = if unusable(&g) {
        TRUNCATED.to_string()
    } else {
        py::strings::strip(&g.content(0).unwrap_or_default()).to_string()
    };
    let confidence = evaluate_confidence(&samples);
    let mut final_response = if confidence >= config.confidence_threshold {
        majority_vote_response(&samples)
    } else {
        greedy
    };
    if final_response.is_empty() {
        final_response =
            "Error: Failed to generate a response. The model may have exceeded token limits."
                .into();
    }
    Ok((json!(clean_response(&final_response)), dt.tokens))
}
