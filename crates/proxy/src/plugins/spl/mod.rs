//! Port of upstream `optillm/plugins/spl` (System Prompt Learning).
//!
//! Upstream keeps its strategy database in the installed package directory
//! (`optillm/plugins/spl/data`); optillm-rs uses `$OPTILLM_SPL_DATA_DIR`,
//! defaulting to `~/.optillm/spl`. The file formats are the same.
//! Strategy merging upstream deduplicates examples and tags via `set()`, whose
//! order depends on Python's per-process hash seed; merged lists keep first-seen
//! order here.

mod prompts;

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;
use crate::py::sklearn::{cosine, Tfidf};
use prompts::*;

const DEFAULT_MAX_TOKENS: i64 = 4096;
const MAINTENANCE_INTERVAL: i64 = 40;
const STRATEGY_CREATION_THRESHOLD: f64 = 0.7;
const STRATEGY_MERGING_THRESHOLD: f64 = 0.6;
const MIN_SUCCESS_RATE_FOR_INFERENCE: f64 = 0.4;
const MAX_STRATEGIES_PER_TYPE: usize = 10;
const MAX_STRATEGIES_FOR_INFERENCE: usize = 3;
const VALID_PROBLEM_TYPES: [&str; 16] = [
    "arithmetic_calculation",
    "algebraic_equation",
    "statistical_analysis",
    "logical_reasoning",
    "word_problem",
    "coding_problem",
    "algorithm_design",
    "creative_writing",
    "text_summarization",
    "information_retrieval",
    "planning_task",
    "decision_making",
    "knowledge_question",
    "language_translation",
    "sequence_completion",
    "general_problem",
];

fn now_iso() -> String {
    chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S%.6f")
        .to_string()
}

#[derive(Clone, Debug)]
struct Strategy {
    strategy_id: String,
    problem_type: String,
    strategy_text: String,
    examples: Vec<String>,
    success_count: i64,
    total_attempts: i64,
    created_at: String,
    last_used: Option<String>,
    last_updated: String,
    confidence: f64,
    tags: Vec<String>,
    reasoning_examples: Vec<String>,
}

impl Strategy {
    fn new(
        id: String,
        problem_type: &str,
        text: String,
        examples: Vec<String>,
        reasoning: Vec<String>,
    ) -> Self {
        let created = now_iso();
        Strategy {
            strategy_id: id,
            problem_type: if VALID_PROBLEM_TYPES.contains(&problem_type) {
                problem_type.to_string()
            } else {
                "general_problem".into()
            },
            strategy_text: text,
            examples,
            success_count: 0,
            total_attempts: 0,
            created_at: created.clone(),
            last_used: None,
            last_updated: created,
            confidence: 0.5,
            tags: Vec::new(),
            reasoning_examples: reasoning,
        }
    }

    fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.success_count as f64 / self.total_attempts as f64
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "strategy_id": self.strategy_id,
            "problem_type": self.problem_type,
            "strategy_text": self.strategy_text,
            "examples": self.examples,
            "success_count": self.success_count,
            "total_attempts": self.total_attempts,
            "created_at": self.created_at,
            "last_used": self.last_used,
            "last_updated": self.last_updated,
            "confidence": self.confidence,
            "tags": self.tags,
            "reasoning_examples": self.reasoning_examples
        })
    }

    fn from_value(v: &Value) -> Option<Self> {
        let strs = |k: &str| -> Vec<String> {
            v.get(k)
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };
        let created = v
            .get("created_at")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(now_iso);
        let problem_type = v.get("problem_type")?.as_str()?.to_string();
        Some(Strategy {
            strategy_id: v.get("strategy_id")?.as_str()?.to_string(),
            problem_type: if VALID_PROBLEM_TYPES.contains(&problem_type.as_str()) {
                problem_type
            } else {
                "general_problem".into()
            },
            strategy_text: v.get("strategy_text")?.as_str()?.to_string(),
            examples: strs("examples"),
            success_count: v.get("success_count").and_then(|x| x.as_i64()).unwrap_or(0),
            total_attempts: v
                .get("total_attempts")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            last_used: v
                .get("last_used")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            last_updated: v
                .get("last_updated")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| created.clone()),
            created_at: created,
            confidence: v.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.5),
            tags: strs("tags"),
            reasoning_examples: strs("reasoning_examples"),
        })
    }

    fn record_attempt(&mut self, success: bool) {
        self.total_attempts += 1;
        if success {
            self.success_count += 1;
        }
        self.last_used = Some(now_iso());
        self.confidence = 0.9 * self.confidence + 0.1 * if success { 1.0 } else { 0.0 };
    }

    fn add_reasoning_example(&mut self, reasoning: &str) {
        if !py::strings::strip(reasoning).is_empty() {
            if self.reasoning_examples.len() >= 5 {
                self.reasoning_examples.remove(0);
            }
            self.reasoning_examples
                .push(py::strings::strip(reasoning).to_string());
        }
    }
}

struct Db {
    dir: PathBuf,
    strategies: Vec<Strategy>,
    metrics: Map<String, Value>,
}

fn metric(m: &Map<String, Value>, k: &str) -> i64 {
    m.get(k).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn bump(m: &mut Map<String, Value>, k: &str) {
    let v = metric(m, k) + 1;
    m.insert(k.into(), json!(v));
}

fn similarity_to_last(texts: &[String], query: &str) -> Option<Vec<f64>> {
    let mut docs = texts.to_vec();
    docs.push(query.to_string());
    let (_, rows) = Tfidf::fit_transform_opts(&docs, true).ok()?;
    let q = rows.last()?.clone();
    Some(
        rows[..rows.len() - 1]
            .iter()
            .map(|r| cosine(&q, r))
            .collect(),
    )
}

impl Db {
    fn load() -> Self {
        let dir = std::env::var("OPTILLM_SPL_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                    .join(".optillm")
                    .join("spl")
            });
        let _ = std::fs::create_dir_all(&dir);
        let mut metrics = Map::new();
        for k in [
            "total_queries",
            "strategy_applications",
            "strategies_created",
            "strategies_refined",
            "successful_resolutions",
            "last_strategy_id",
            "reasoning_examples_collected",
            "strategies_merged",
        ] {
            metrics.insert(k.into(), json!(0));
        }
        let mut strategies = Vec::new();
        if let Ok(text) = std::fs::read_to_string(dir.join("strategies.json")) {
            if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&text) {
                strategies = items.iter().filter_map(Strategy::from_value).collect();
            }
        }
        for s in &strategies {
            if let Some(n) = s
                .strategy_id
                .strip_prefix("strategy_")
                .and_then(|n| n.split('_').next())
                .and_then(|n| n.parse::<i64>().ok())
            {
                let v = metric(&metrics, "last_strategy_id").max(n);
                metrics.insert("last_strategy_id".into(), json!(v));
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("metrics.json")) {
            if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&text) {
                let last = metric(&metrics, "last_strategy_id");
                for (k, v) in m.iter() {
                    metrics.insert(k.clone(), v.clone());
                }
                if m.contains_key("last_strategy_id") {
                    let v = last.max(metric(&m, "last_strategy_id"));
                    metrics.insert("last_strategy_id".into(), json!(v));
                }
            }
        }
        Db {
            dir,
            strategies,
            metrics,
        }
    }

    fn save(&self) {
        let list: Vec<Value> = self.strategies.iter().map(|s| s.to_value()).collect();
        let _ = std::fs::write(
            self.dir.join("strategies.json"),
            py::json::dumps_indent(&json!(list), 2),
        );
        let _ = std::fs::write(
            self.dir.join("metrics.json"),
            py::json::dumps_indent(&Value::Object(self.metrics.clone()), 2),
        );
    }

    fn for_type(&self, t: &str) -> Vec<Strategy> {
        self.strategies
            .iter()
            .filter(|s| s.problem_type == t)
            .cloned()
            .collect()
    }

    fn by_id(&mut self, id: &str) -> Option<&mut Strategy> {
        self.strategies.iter_mut().find(|s| s.strategy_id == id)
    }

    fn add_strategy(&mut self, s: Strategy) {
        if let Some(n) = s
            .strategy_id
            .strip_prefix("strategy_")
            .and_then(|n| n.split('_').next())
            .and_then(|n| n.parse::<i64>().ok())
        {
            let v = metric(&self.metrics, "last_strategy_id").max(n);
            self.metrics.insert("last_strategy_id".into(), json!(v));
        }
        self.strategies.push(s);
        bump(&mut self.metrics, "strategies_created");
        self.save();
    }

    fn next_id(&mut self) -> String {
        bump(&mut self.metrics, "last_strategy_id");
        format!("strategy_{}", metric(&self.metrics, "last_strategy_id"))
    }

    fn find_similar_strategy(
        &self,
        problem_type: &str,
        query: &str,
        threshold: f64,
    ) -> Option<Strategy> {
        let type_strategies = self.for_type(problem_type);
        if type_strategies.is_empty() {
            return None;
        }
        let texts: Vec<String> = type_strategies
            .iter()
            .map(|s| s.strategy_text.clone())
            .collect();
        let sims = similarity_to_last(&texts, query)?;
        // numpy argmax: first maximum
        let idx = py::strings::argmax_by(&sims, |x| *x)?;
        if sims[idx] >= threshold {
            Some(type_strategies[idx].clone())
        } else {
            None
        }
    }

    fn find_similar_examples(
        &self,
        problem_type: &str,
        query: &str,
        threshold: f64,
    ) -> Option<Strategy> {
        let mut best = 0.0;
        let mut found: Option<Strategy> = None;
        for s in self.for_type(problem_type) {
            if s.examples.is_empty() {
                continue;
            }
            let sims = similarity_to_last(&s.examples, query)?;
            let m = sims.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            if m > best {
                best = m;
                found = Some(s);
            }
        }
        found.filter(|_| best >= threshold)
    }

    fn merge_similar(&mut self) -> Result<i64> {
        if self.strategies.len() <= 1 {
            return Ok(0);
        }
        let mut merged = 0;
        let mut i = 0;
        while i < self.strategies.len() {
            let mut j = i + 1;
            while j < self.strategies.len() {
                if self.strategies[i].problem_type == self.strategies[j].problem_type {
                    let docs = vec![
                        self.strategies[i].strategy_text.clone(),
                        self.strategies[j].strategy_text.clone(),
                    ];
                    let (_, rows) =
                        Tfidf::fit_transform_opts(&docs, true).map_err(|e| anyhow!(e))?;
                    if cosine(&rows[0], &rows[1]) >= STRATEGY_MERGING_THRESHOLD {
                        let (a, b) = (self.strategies[i].clone(), self.strategies[j].clone());
                        let (base, other) = if a.success_rate() >= b.success_rate() {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        let mut m = base.clone();
                        for e in other.examples {
                            if !m.examples.contains(&e) {
                                m.examples.push(e);
                            }
                        }
                        for t in other.tags {
                            if !m.tags.contains(&t) {
                                m.tags.push(t);
                            }
                        }
                        m.success_count = base.success_count + other.success_count;
                        m.total_attempts = base.total_attempts + other.total_attempts;
                        m.created_at =
                            std::cmp::min(base.created_at.clone(), other.created_at.clone());
                        m.last_used = match (&base.last_used, &other.last_used) {
                            (Some(x), Some(y)) => Some(std::cmp::max(x.clone(), y.clone())),
                            _ => base.last_used.clone(),
                        };
                        m.last_updated = now_iso();
                        m.confidence = (base.confidence + other.confidence) / 2.0;
                        m.reasoning_examples.extend(other.reasoning_examples);
                        if m.reasoning_examples.len() > 5 {
                            let n = m.reasoning_examples.len();
                            m.reasoning_examples = m.reasoning_examples[n - 5..].to_vec();
                        }
                        self.strategies[i] = m;
                        self.strategies.remove(j);
                        merged += 1;
                        bump(&mut self.metrics, "strategies_merged");
                    } else {
                        j += 1;
                    }
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
        if merged > 0 {
            self.save();
        }
        Ok(merged)
    }

    fn limit_per_type(&mut self) {
        let mut types: Vec<String> = Vec::new();
        for s in &self.strategies {
            if !types.contains(&s.problem_type) {
                types.push(s.problem_type.clone());
            }
        }
        let mut remove: Vec<String> = Vec::new();
        for t in types {
            let mut scored: Vec<(Strategy, f64)> = self
                .for_type(&t)
                .into_iter()
                .map(|s| {
                    let score = 0.7 * s.success_rate() + 0.3 * recency(&s);
                    (s, score)
                })
                .collect();
            if scored.len() <= MAX_STRATEGIES_PER_TYPE {
                continue;
            }
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            remove.extend(
                scored[MAX_STRATEGIES_PER_TYPE..]
                    .iter()
                    .map(|(s, _)| s.strategy_id.clone()),
            );
        }
        let before = self.strategies.len();
        self.strategies.retain(|s| !remove.contains(&s.strategy_id));
        if self.strategies.len() != before {
            self.save();
        }
    }

    fn prune(&mut self) {
        let before = self.strategies.len();
        self.strategies
            .retain(|s| s.total_attempts < 5 || s.success_rate() >= 0.3);
        if self.strategies.len() != before {
            self.save();
        }
    }
}

fn recency(s: &Strategy) -> f64 {
    match &s.last_used {
        Some(lu) => match chrono::NaiveDateTime::parse_from_str(lu, "%Y-%m-%dT%H:%M:%S%.f") {
            Ok(t) => {
                let days = (chrono::Local::now().naive_local() - t).num_days() as f64;
                f64::max(0.0, 1.0 - f64::min(1.0, days / 30.0))
            }
            Err(_) => 0.0,
        },
        None => 0.0,
    }
}

/// `extract_thinking` -> (final response, thinking).
fn extract_thinking(response: &str) -> (String, Option<String>) {
    let blocks: Vec<String> = Regex::new(r"(?s)<think>(.*?)</think>")
        .unwrap()
        .captures_iter(response)
        .map(|c| c[1].to_string())
        .collect();
    if blocks.is_empty() {
        return (response.to_string(), None);
    }
    let parts: Vec<&str> = response.split("</think>").collect();
    let final_response = if parts.len() > 1 {
        py::strings::strip(parts[parts.len() - 1]).to_string()
    } else {
        response.to_string()
    };
    (final_response, Some(blocks.join("\n")))
}

fn augment_system_prompt(system_prompt: &str, strategies: &[Strategy]) -> String {
    if strategies.is_empty() {
        return system_prompt.to_string();
    }
    let mut section = String::new();
    for (i, s) in strategies.iter().enumerate() {
        section += &format!(
            "Strategy {} for {} problems:\n{}\n\n",
            i + 1,
            s.problem_type,
            s.strategy_text
        );
        if let Some(r) = s.reasoning_examples.last() {
            if !r.is_empty() {
                section += &format!("Example reasoning process:\n<think>\n{}\n</think>\n\n", r);
            }
        }
    }
    let prompt = py::strings::format_named(
        STRATEGY_APPLICATION_PROMPT,
        &[("strategies_section", &section)],
    )
    .unwrap_or_default();
    format!("{}\n\n{}", system_prompt, prompt)
}

async fn content(ctx: &Ctx, body: Value) -> Result<String> {
    let r = ctx.create(body).await?;
    r.content_at(0)?
        .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))
}

async fn classify_problem(ctx: &Ctx, query: &str) -> String {
    let types = VALID_PROBLEM_TYPES[..VALID_PROBLEM_TYPES.len() - 1].join(", ");
    let system =
        py::strings::format_named(PROBLEM_CLASSIFICATION_PROMPT, &[("problem_types", &types)])
            .unwrap_or_default();
    let user = format!(
        "Classify the following problem into ONE of these types: {}\n\nProblem: {}",
        types, query
    );
    let raw = match content(
        ctx,
        json!({
            "model": ctx.model,
            "messages": [msg("system", &system), msg("user", &user)],
            "temperature": 0.1,
            "max_tokens": DEFAULT_MAX_TOKENS
        }),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return "general_problem".into(),
    };
    let (final_response, _) = extract_thinking(&raw);
    let f = py::strings::strip(&final_response).to_lowercase();
    for t in VALID_PROBLEM_TYPES {
        if t == f {
            return t.into();
        }
    }
    for t in VALID_PROBLEM_TYPES {
        if f.contains(t) {
            return t.into();
        }
    }
    "general_problem".into()
}

async fn generate_strategy(ctx: &Ctx, problem: &str, problem_type: &str, db: &mut Db) -> Strategy {
    let user = format!(
        "Create a problem-solving strategy for the following {} problem:\n\n{}\n\nThis strategy should help solve not just this specific problem, but any {} problem.",
        problem_type, problem, problem_type
    );
    match content(ctx, json!({
        "model": ctx.model,
        "messages": [msg("system", STRATEGY_GENERATION_PROMPT), msg("user", &user)],
        "temperature": 0.7,
        "max_tokens": DEFAULT_MAX_TOKENS
    }))
    .await
    {
        Ok(text) => {
            let (mut strategy_text, thinking) = extract_thinking(&text);
            if py::strings::strip(&strategy_text).is_empty() {
                strategy_text = text.clone();
            }
            let id = db.next_id();
            Strategy::new(
                id,
                problem_type,
                py::strings::strip(&strategy_text).to_string(),
                vec![problem.to_string()],
                thinking.filter(|t| !t.is_empty()).into_iter().collect(),
            )
        }
        Err(_) => Strategy::new(
            format!("fallback_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
            problem_type,
            format!(
                "When solving {} problems:\n1. Break down the problem into smaller parts\n2. Solve each part systematically\n3. Combine the solutions",
                problem_type
            ),
            vec![problem.to_string()],
            Vec::new(),
        ),
    }
}

fn select_relevant(db: &Db, problem_type: &str, learning: bool) -> Vec<Strategy> {
    let qualified: Vec<Strategy> = db
        .for_type(problem_type)
        .into_iter()
        .filter(|s| {
            (learning && s.total_attempts < 5)
                || (s.success_rate() >= MIN_SUCCESS_RATE_FOR_INFERENCE && s.total_attempts >= 5)
        })
        .collect();
    if qualified.len() <= MAX_STRATEGIES_FOR_INFERENCE {
        return qualified;
    }
    let mut scored: Vec<(Strategy, f64)> = qualified
        .into_iter()
        .map(|s| {
            let score = 0.7 * s.success_rate() + 0.3 * recency(&s);
            (s, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(MAX_STRATEGIES_FOR_INFERENCE)
        .map(|(s, _)| s)
        .collect()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let learning = ctx
        .request_config
        .get("spl_learning")
        .map(truthy)
        .unwrap_or(false);
    let mut db = Db::load();
    if learning {
        bump(&mut db.metrics, "total_queries");
        db.save();
    }
    let problem_type = classify_problem(ctx, initial_query).await;
    let existing = db.for_type(&problem_type);

    if learning {
        let (create, similar) = if existing.is_empty() {
            (true, None)
        } else if existing.len() >= MAX_STRATEGIES_PER_TYPE {
            if let Some(s) =
                db.find_similar_strategy(&problem_type, initial_query, STRATEGY_CREATION_THRESHOLD)
            {
                (false, Some(s))
            } else if let Some(s) =
                db.find_similar_examples(&problem_type, initial_query, STRATEGY_CREATION_THRESHOLD)
            {
                (false, Some(s))
            } else {
                let mut sorted = existing.clone();
                sorted.sort_by(|a, b| {
                    a.success_rate()
                        .partial_cmp(&b.success_rate())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                (false, sorted.into_iter().next())
            }
        } else {
            match db.find_similar_strategy(
                &problem_type,
                initial_query,
                STRATEGY_CREATION_THRESHOLD,
            ) {
                Some(s) => (false, Some(s)),
                None => (true, None),
            }
        };
        if create {
            let s = generate_strategy(ctx, initial_query, &problem_type, &mut db).await;
            db.add_strategy(s);
        } else if let Some(s) = similar {
            if let Some(target) = db.by_id(&s.strategy_id) {
                if !initial_query.is_empty() && !target.examples.iter().any(|e| e == initial_query)
                {
                    target.examples.push(initial_query.to_string());
                }
                db.save();
            }
        }
    }

    if learning && metric(&db.metrics, "total_queries") % MAINTENANCE_INTERVAL == 0 {
        db.merge_similar()?;
        db.limit_per_type();
        db.prune();
    }

    let selected = select_relevant(&db, &problem_type, learning);
    let augmented = if selected.is_empty() {
        system_prompt.to_string()
    } else {
        augment_system_prompt(system_prompt, &selected)
    };

    let main: Result<(String, i64)> = async {
        let mut body = Map::new();
        body.insert("model".into(), json!(ctx.model));
        body.insert("messages".into(), json!([msg("system", &augmented), msg("user", initial_query)]));
        for (k, v) in &ctx.request_config {
            if k != "spl_learning" {
                body.insert(k.clone(), v.clone());
            }
        }
        let too_small = match body.get("max_tokens") {
            None => true,
            Some(v) => match v.as_f64() {
                Some(x) => x < DEFAULT_MAX_TOKENS as f64,
                None => return Err(anyhow!("'<' not supported between instances of 'NoneType' and 'int'")),
            },
        };
        if too_small {
            body.insert("max_tokens".into(), json!(DEFAULT_MAX_TOKENS));
        }
        let response = ctx.create(Value::Object(body)).await?;
        let tokens = response.completion_tokens()?;
        let text = response
            .content_at(0)?
            .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?;
        let (final_response, thinking) = extract_thinking(&text);

        if learning && !selected.is_empty() {
            // evaluate_strategy_effectiveness
            let full = match &thinking {
                Some(t) if !t.is_empty() => format!("{}\n\n{}", t, final_response),
                _ => final_response.clone(),
            };
            let mut results: Vec<(String, bool)> = Vec::new();
            let eval: Result<()> = async {
                for s in &selected {
                    let s = db.strategies.iter().find(|x| x.strategy_id == s.strategy_id).cloned().unwrap_or_else(|| s.clone());
                    let user = format!(
                        "Strategy:\n{}\n\nResponse (including reasoning):\n{}\n\nDoes the response show clear evidence that the strategy was effectively applied? Answer with ONLY YES or NO.",
                        s.strategy_text, full
                    );
                    let out = content(ctx, json!({
                        "model": ctx.model,
                        "messages": [msg("system", STRATEGY_EVALUATION_PROMPT), msg("user", &user)],
                        "temperature": 0.1,
                        "max_tokens": DEFAULT_MAX_TOKENS
                    }))
                    .await?;
                    let (r, _) = extract_thinking(&out);
                    results.push((s.strategy_id.clone(), py::strings::strip(&r).to_uppercase().contains("YES")));
                }
                Ok(())
            }
            .await;
            if eval.is_err() {
                results = selected.iter().map(|s| (s.strategy_id.clone(), false)).collect();
            }
            for (id, effective) in &results {
                if id == "fallback_temporary" {
                    continue;
                }
                if let Some(s) = db.by_id(id) {
                    s.record_attempt(*effective);
                    bump(&mut db.metrics, "strategy_applications");
                    if *effective {
                        bump(&mut db.metrics, "successful_resolutions");
                    }
                    db.save();
                }
                if *effective {
                    if let Some(t) = thinking.as_ref().filter(|t| !t.is_empty()) {
                        if let Some(s) = db.by_id(id) {
                            s.add_reasoning_example(t);
                            bump(&mut db.metrics, "reasoning_examples_collected");
                            db.save();
                        }
                    }
                }
            }
            // Refine after every 10 uses. Upstream's selected strategies are the
            // database objects themselves, so they see the updated attempt counts.
            for s in &selected {
                let Some(current) = db.strategies.iter().find(|x| x.strategy_id == s.strategy_id).cloned() else {
                    continue;
                };
                let attempts = current.total_attempts;
                if current.strategy_id != "fallback_temporary" && attempts % 10 == 0 && attempts > 0 {
                    let user = format!(
                        "Original strategy for {} problems:\n{}\n\nNew problem:\n{}\n\nSolution process (including reasoning):\n{}\n\nProvide a refined version of the original strategy that incorporates any insights from this new example.",
                        current.problem_type, current.strategy_text, initial_query, full
                    );
                    if let Ok(out) = content(ctx, json!({
                        "model": ctx.model,
                        "messages": [msg("system", STRATEGY_REFINEMENT_PROMPT), msg("user", &user)],
                        "temperature": 0.5,
                        "max_tokens": DEFAULT_MAX_TOKENS
                    }))
                    .await
                    {
                        let (mut refined, _) = extract_thinking(&out);
                        if py::strings::strip(&refined).is_empty() {
                            refined = out.clone();
                        }
                        if let Some(target) = db.by_id(&current.strategy_id) {
                            target.strategy_text = py::strings::strip(&refined).to_string();
                            target.last_updated = now_iso();
                        }
                        bump(&mut db.metrics, "strategies_refined");
                        db.save();
                    }
                }
            }
        }
        Ok((text, tokens))
    }
    .await;

    match main {
        Ok((text, tokens)) => Ok((json!(text), tokens)),
        Err(e) => {
            let fallback: Result<(Value, i64)> = async {
                let r = ctx
                    .create(json!({
                        "model": ctx.model,
                        "messages": [msg("system", system_prompt), msg("user", initial_query)],
                        "max_tokens": DEFAULT_MAX_TOKENS
                    }))
                    .await?;
                let c = r
                    .message(0)
                    .and_then(|m| m.get("content"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok((c, r.completion_tokens()?))
            }
            .await;
            match fallback {
                Ok(v) => Ok(v),
                Err(_) => Ok((json!(format!("Error processing request: {}", e)), 0)),
            }
        }
    }
}
