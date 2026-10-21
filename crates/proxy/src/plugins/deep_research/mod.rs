//! Port of upstream `optillm/plugins/deep_research` (TTD-DR, Test-Time
//! Diffusion Deep Researcher): preliminary draft, query decomposition and
//! initial search, then gap analysis -> targeted search -> URL fetching ->
//! denoising -> quality scoring per iteration, and a finalised report with
//! references and metadata.
//!
//! Searches go through the web_search port (an HTTP backend instead of
//! upstream's shared Selenium browser session) and pages through readurls.
//! Upstream wraps OpenAI-type clients with `max_retries=0` and falls back to
//! the original client when a call fails; that is kept.

mod prompts;

use std::collections::HashSet;

use anyhow::{anyhow, Result};
use fancy_regex::Regex as FancyRegex;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::{json, Value};

use super::{readurls, web_search};
use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

type Dict = IndexMap<String, String>;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid regex")
}

/// Python's `$` without MULTILINE: end of text or before a final newline.
const PY_END: &str = r"(?=\n\z|\z)";

/// `clean_reasoning_tags`.
pub fn clean_reasoning_tags(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut t = text.to_string();
    for tag in [
        "think",
        "thinking",
        "reasoning",
        "thought",
        "reflect",
        "reflection",
    ] {
        t = re(&format!(r"(?si)<{tag}>.*?</{tag}>"))
            .replace_all(&t, "")
            .into_owned();
    }
    t = re(r"\n\s*\n\s*\n+").replace_all(&t, "\n\n").into_owned();
    t = re(r"  +").replace_all(&t, " ").into_owned();
    py::strings::strip(&t).to_string()
}

const PLACEHOLDER_PATTERNS: [&str; 21] = [
    r"\[NEEDS RESEARCH[^\]]*\]",
    r"\[SOURCE NEEDED[^\]]*\]",
    r"\[RESEARCH NEEDED[^\]]*\]",
    r"\[CITATION NEEDED[^\]]*\]",
    r"\[MORE RESEARCH NEEDED[^\]]*\]",
    r"\[REQUIRES INVESTIGATION[^\]]*\]",
    r"\[TO BE RESEARCHED[^\]]*\]",
    r"\[VERIFY[^\]]*\]",
    r"\[CHECK[^\]]*\]",
    r"\[Placeholder for[^\]]+\]",
    r"\[\d+\]\s*\[Placeholder[^\]]+\]",
    r"\[Insert citation[^\]]*\]",
    r"\[Add reference[^\]]*\]",
    r"\[Reference needed[^\]]*\]",
    r"\[To be completed[^\]]*\]",
    r"\[Under development[^\]]*\]",
    r"\[Coming soon[^\]]*\]",
    r"\[TBD[^\]]*\]",
    r"\[TODO[^\]]*\]",
    r"\[Question \d+[^\]]*\]",
    r"\[Research question[^\]]*\]",
];

/// `cleanup_placeholder_tags`.
pub fn cleanup_placeholder_tags(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut t = text.to_string();
    for p in PLACEHOLDER_PATTERNS {
        t = re(&format!("(?i){p}")).replace_all(&t, "").into_owned();
    }
    let punct = re(r"^[\s\-\*\.\,\;\:]*$");
    let kept: Vec<&str> = t
        .split('\n')
        .filter(|line| {
            let stripped = py::strings::strip(line);
            stripped.is_empty() || !punct.is_match(stripped)
        })
        .collect();
    let result = re(r"\n\s*\n\s*\n+")
        .replace_all(&kept.join("\n"), "\n\n")
        .into_owned();
    py::strings::strip(&result).to_string()
}

/// Lines of `text` that, stripped, are non-empty.
fn content_lines(text: &str) -> usize {
    text.split('\n')
        .filter(|l| !py::strings::strip(l).is_empty())
        .count()
}

/// `validate_report_completeness(text)["is_complete"]`.
fn report_is_complete(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    for p in [
        r"\[Placeholder for[^\]]+\]",
        r"\[\d+\]\s*\[Placeholder[^\]]+\]",
        r"\[Insert citation[^\]]*\]",
        r"\[Reference needed[^\]]*\]",
    ] {
        if re(&format!("(?i){p}")).is_match(text) {
            return false;
        }
    }
    if text.contains("Research Questions for Investigation") {
        let section = FancyRegex::new(&format!(
            r"(?s)## Research Questions for Investigation.*?(?=##|{PY_END})"
        ))
        .unwrap();
        if let Ok(Some(m)) = section.find(text) {
            let questions = m
                .as_str()
                .split('\n')
                .filter(|l| {
                    let s = py::strings::strip(l);
                    s.starts_with('*') || s.starts_with('-')
                })
                .count();
            if questions > 3 {
                return false;
            }
        }
    }
    let sections = FancyRegex::new(&format!(r"(?s)##\s+([^#\n]+)\n(.*?)(?=##|{PY_END})")).unwrap();
    let placeholder = re(r"(?i)\[[^\]]*(?:placeholder|needed|research|todo|tbd)[^\]]*\]");
    for c in sections.captures_iter(text).flatten() {
        let body = c.get(2).map(|m| m.as_str()).unwrap_or("");
        let count = placeholder.find_iter(body).count();
        if count as f64 > content_lines(body) as f64 / 3.0 {
            return false;
        }
    }
    if text.matches('[').count() != text.matches(']').count() {
        return false;
    }
    py::strings::split_whitespace(text).len() >= 500
}

/// Citation numbers `[n]` in the body (before `## References`), as normalised digit strings.
fn citations_in_body(text: &str) -> HashSet<String> {
    let body = text.split("## References").next().unwrap_or("");
    re(r"\[(\d+)\]")
        .captures_iter(body)
        .map(|c| {
            let digits: String = c[1]
                .chars()
                .map(|ch| {
                    ch.to_digit(10)
                        .and_then(|d| char::from_digit(d, 10))
                        .unwrap_or(ch)
                })
                .collect();
            let trimmed = digits.trim_start_matches('0');
            if trimmed.is_empty() {
                "0".to_string()
            } else {
                trimmed.to_string()
            }
        })
        .collect()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// `max(1, a // b)` with Python floor division.
fn per_query(max_sources: i64, n: usize) -> usize {
    max_sources.div_euclid(n.max(1) as i64).max(1) as usize
}

struct Researcher<'a> {
    ctx: &'a Ctx,
    /// The wrapper's rebuilt `max_retries=0` client (OpenAI-type clients only).
    no_retry: Option<Ctx>,
    max_iterations: Value,
    max_sources: i64,
    total_tokens: i64,
    citations: IndexMap<i64, Dict>,
    citation_counter: i64,
    current_draft: String,
    draft_history: Vec<String>,
    gap_analysis_history: Vec<usize>,
}

impl<'a> Researcher<'a> {
    /// One completion; returns the stripped, tag-cleaned content and its completion tokens.
    async fn complete(
        &self,
        system: &str,
        user: &str,
        temperature: f64,
        max_tokens: i64,
    ) -> Result<(String, i64)> {
        let body = json!({
            "model": self.ctx.model,
            "messages": [msg("system", system), msg("user", user)],
            "temperature": temperature,
            "max_tokens": max_tokens
        });
        // DeepResearchClientWrapper: any failure of the rebuilt client
        // (including the request itself) retries on the original client.
        let r = match &self.no_retry {
            Some(c) => match c.create(body.clone()).await {
                Ok(r) => r,
                Err(_) => self.ctx.create(body).await?,
            },
            None => self.ctx.create(body).await?,
        };
        let content = r
            .content_at(0)?
            .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
        let content = clean_reasoning_tags(py::strings::strip(&content));
        Ok((content, r.completion_tokens()?))
    }

    async fn generate_preliminary_draft(
        &mut self,
        system_prompt: &str,
        initial_query: &str,
    ) -> String {
        match self
            .complete(
                system_prompt,
                &prompts::preliminary_draft(initial_query),
                0.7,
                2000,
            )
            .await
        {
            Ok((draft, tokens)) => {
                self.total_tokens += tokens;
                draft
            }
            Err(e) => format!("Failed to generate preliminary draft: {}", e),
        }
    }

    async fn decompose_query(&mut self, system_prompt: &str, initial_query: &str) -> Vec<String> {
        match self
            .complete(
                system_prompt,
                &prompts::decomposition(initial_query),
                0.7,
                1000,
            )
            .await
        {
            Ok((content, tokens)) => {
                self.total_tokens += tokens;
                let numbered = re(r"^\d+\.");
                let strip_marker = re(r"^\d+\.\s*\[?(.*?)\]?$");
                let mut queries = Vec::new();
                for line in content.split('\n') {
                    let line = py::strings::strip(line);
                    if numbered.is_match(line) {
                        let q = strip_marker.replace(line, "$1");
                        let q = py::strings::strip(&q);
                        if !q.is_empty() {
                            queries.push(q.to_string());
                        }
                    }
                }
                queries.truncate(5);
                queries
            }
            Err(_) => vec![initial_query.to_string()],
        }
    }

    async fn perform_web_search(&self, queries: &[String]) -> String {
        let mut all = Vec::new();
        for q in queries {
            let search_query = format!("search for {}", py::strings::strip(q));
            let enhanced = web_search::run_with(
                &search_query,
                self.ctx,
                per_query(self.max_sources, queries.len()),
                None,
            )
            .await;
            if enhanced.contains("Web Search Results") {
                all.push(enhanced);
            }
        }
        if all.is_empty() {
            return "Web search failed: No results obtained from any query".into();
        }
        all.join("\n\n")
    }

    /// `extract_and_fetch_urls`: sources parsed from the results, plus readurls on them.
    async fn extract_and_fetch_urls(&self, search_results: &str) -> (String, Vec<Dict>) {
        let mut sources = Vec::new();
        for c in re(r"(\d+)\.\s*\*\*(.+?)\*\*\s*\n\s*URL:\s*(.+?)\n").captures_iter(search_results)
        {
            let mut s = Dict::new();
            s.insert("number".into(), c[1].to_string());
            s.insert("title".into(), py::strings::strip(&c[2]).to_string());
            s.insert("url".into(), py::strings::strip(&c[3]).to_string());
            s.insert("access_date".into(), today());
            sources.push(s);
        }
        if sources.is_empty() {
            let title_re = re(r"^(\d+)\.\s*\*\*(.+?)\*\*");
            let mut current = Dict::new();
            for line in search_results.split('\n') {
                let stripped = py::strings::strip(line);
                if let Some(m) = title_re.captures(stripped) {
                    if current.contains_key("url") {
                        sources.push(std::mem::take(&mut current));
                    }
                    current = Dict::new();
                    current.insert("number".into(), m[1].to_string());
                    current.insert("title".into(), py::strings::strip(&m[2]).to_string());
                } else if stripped.starts_with("URL:") && !current.is_empty() {
                    let url = py::strings::strip(&stripped[4..]).to_string();
                    current.insert("url".into(), url);
                    current.insert("access_date".into(), today());
                }
            }
            if current.contains_key("url") {
                sources.push(current);
            }
        }
        let content = match readurls::run("", search_results, self.ctx).await {
            Ok((v, _)) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
            Err(e) => return (format!("URL fetching failed: {}", e), Vec::new()),
        };
        (content, sources)
    }

    fn register(&mut self, sources: Vec<Dict>) {
        for s in sources {
            if s.contains_key("url") {
                self.citation_counter += 1;
                self.citations.insert(self.citation_counter, s);
            }
        }
    }

    async fn analyze_draft_gaps(&mut self, current_draft: &str, original_query: &str) -> Vec<Dict> {
        match self
            .complete(
                "You are an expert research analyst.",
                &prompts::gap_analysis(original_query, current_draft),
                0.3,
                1000,
            )
            .await
        {
            Ok((content, tokens)) => {
                self.total_tokens += tokens;
                let mut gaps = Vec::new();
                let mut current = Dict::new();
                for line in content.split('\n') {
                    let line = py::strings::strip(line);
                    let value = || {
                        py::strings::strip(line.split_once(':').map(|x| x.1).unwrap_or(""))
                            .to_string()
                    };
                    if line.starts_with("GAP_ID:") {
                        if !current.is_empty() {
                            gaps.push(std::mem::take(&mut current));
                        }
                        current = Dict::new();
                        current.insert("id".into(), value());
                    } else {
                        for (prefix, key) in [
                            ("SECTION:", "section"),
                            ("GAP_TYPE:", "gap_type"),
                            ("SPECIFIC_NEED:", "specific_need"),
                            ("SEARCH_QUERY:", "search_query"),
                            ("PRIORITY:", "priority"),
                        ] {
                            if line.starts_with(prefix) {
                                current.insert(key.into(), value());
                                break;
                            }
                        }
                    }
                }
                if !current.is_empty() {
                    gaps.push(current);
                }
                gaps
            }
            Err(_) => {
                let mut g = Dict::new();
                g.insert("id".into(), "1".into());
                g.insert("section".into(), "General".into());
                g.insert("gap_type".into(), "MISSING_INFO".into());
                g.insert(
                    "specific_need".into(),
                    "More detailed information needed".into(),
                );
                g.insert("search_query".into(), original_query.into());
                vec![g]
            }
        }
    }

    async fn perform_gap_targeted_search(&self, gaps: &[Dict]) -> String {
        let rank = |g: &Dict| match g.get("priority").map(|p| p.to_uppercase()).as_deref() {
            Some("HIGH") => 0,
            Some("MEDIUM") => 1,
            _ => 2,
        };
        let mut sorted: Vec<&Dict> = gaps.iter().collect();
        sorted.sort_by_key(|g| rank(g));
        let mut all = Vec::new();
        for gap in sorted {
            let q = gap.get("search_query").map(|s| s.as_str()).unwrap_or("");
            if q.is_empty() {
                continue;
            }
            let search_query = format!("search for {}", py::strings::strip(q));
            let enhanced = web_search::run_with(
                &search_query,
                self.ctx,
                per_query(self.max_sources, gaps.len()),
                None,
            )
            .await;
            if enhanced.contains("Web Search Results") {
                let context = format!(
                    "[ADDRESSING GAP: {} - {}]\n",
                    gap.get("section").map(|s| s.as_str()).unwrap_or("Unknown"),
                    gap.get("specific_need")
                        .map(|s| s.as_str())
                        .unwrap_or("General research")
                );
                all.push(context + &enhanced);
            }
        }
        if all.is_empty() {
            "No gap-targeted search results obtained".into()
        } else {
            all.join("\n\n")
        }
    }

    fn title(source: &Dict) -> &str {
        source
            .get("title")
            .map(|s| s.as_str())
            .unwrap_or("Untitled")
    }

    async fn denoise(
        &mut self,
        current_draft: &str,
        retrieval_content: &str,
        original_query: &str,
    ) -> String {
        let mut citation_context = String::from("\n\n**AVAILABLE CITATIONS (USE THESE!):**\n");
        for (num, source) in &self.citations {
            citation_context += &format!(
                "[{}] {} - {}\n",
                num,
                Self::title(source),
                source.get("url").map(|s| s.as_str()).unwrap_or("No URL")
            );
        }
        match self
            .complete(
                "You are an expert research synthesizer performing draft denoising.",
                &prompts::denoising(
                    original_query,
                    current_draft,
                    retrieval_content,
                    &citation_context,
                ),
                0.6,
                3000,
            )
            .await
        {
            Ok((draft, tokens)) => {
                self.total_tokens += tokens;
                draft
            }
            Err(e) => format!(
                "Denoising failed: {}\n\nFalling back to current draft:\n{}",
                e, current_draft
            ),
        }
    }

    async fn evaluate_draft_quality(
        &mut self,
        draft: &str,
        previous_draft: &str,
        original_query: &str,
    ) -> IndexMap<String, f64> {
        match self
            .complete(
                "You are an expert research quality evaluator.",
                &prompts::quality_evaluation(original_query, previous_draft, draft),
                0.2,
                500,
            )
            .await
        {
            Ok((content, tokens)) => {
                self.total_tokens += tokens;
                let mut scores = IndexMap::new();
                for line in content.split('\n') {
                    if let Some((key, value)) = line.split_once(':') {
                        let key = py::strings::strip(key).to_lowercase();
                        let score =
                            py::strings::parse_float(py::strings::strip(value)).unwrap_or(0.5);
                        scores.insert(key, score);
                    }
                }
                scores
            }
            Err(_) => [
                ("completeness", 0.5),
                ("accuracy", 0.5),
                ("depth", 0.5),
                ("coherence", 0.5),
                ("citations", 0.5),
                ("improvement", 0.1),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        }
    }

    /// `fix_incomplete_report`.
    fn fix_incomplete_report(report: &str, original_query: &str) -> String {
        let mut fixed = cleanup_placeholder_tags(report);
        if fixed.contains("Research Questions for Investigation") {
            fixed = FancyRegex::new(&format!(
                r"(?s)## Research Questions for Investigation.*?(?=##|{PY_END})"
            ))
            .unwrap()
            .replace_all(&fixed, "")
            .into_owned();
        }
        fixed = re(r"\[\d+\]\s*\[Placeholder[^\]]+\]\n?")
            .replace_all(&fixed, "")
            .into_owned();
        fixed = FancyRegex::new(r"##\s+([^#\n]+)\n\s*(?=##)")
            .unwrap()
            .replace_all(&fixed, "")
            .into_owned();
        if py::strings::split_whitespace(&fixed).len() < 300 {
            let note = prompts::completion_note(original_query);
            if fixed.contains("## References") {
                fixed = fixed.replace("## References", &(note + "\n## References"));
            } else {
                fixed += &note;
            }
        }
        fixed = re(r"\n\s*\n\s*\n+")
            .replace_all(&fixed, "\n\n")
            .into_owned();
        py::strings::strip(&fixed).to_string()
    }

    async fn finalize(
        &mut self,
        system_prompt: &str,
        original_query: &str,
        final_draft: &str,
    ) -> String {
        let mut citation_context = String::from("\n\n**AVAILABLE CITATIONS:**\n");
        for (num, source) in &self.citations {
            citation_context += &format!("[{}] {}\n", num, Self::title(source));
        }
        let (content, tokens) = match self
            .complete(
                system_prompt,
                &prompts::finalization(original_query, final_draft, &citation_context),
                0.5,
                3000,
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return format!(
                    "Finalization failed: {}\n\nReturning current draft:\n{}",
                    e, final_draft
                )
            }
        };
        let mut report = cleanup_placeholder_tags(&content);
        if !report_is_complete(&report) {
            report = Self::fix_incomplete_report(&report, original_query);
        }
        self.total_tokens += tokens;
        report = FancyRegex::new(r"(?s)##\s*References.*?(?=##|\z)")
            .unwrap()
            .replace_all(&report, "")
            .into_owned();
        report = re(r"(?m)^References\s*\n\s*(?:\[\d+\]\s*\n)+")
            .replace_all(&report, "")
            .into_owned();
        report = re(r"\n\s*\n\s*\n+")
            .replace_all(&report, "\n\n")
            .into_owned();

        let total = self.citations.len();
        let used = if report.is_empty() {
            // validate_citation_usage returns no "used_citations" for empty text.
            return format!(
                "Finalization failed: 'used_citations'\n\nReturning current draft:\n{}",
                final_draft
            );
        } else {
            citations_in_body(&report)
        };
        let usage_percentage = if total > 0 {
            used.len() as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let mut references = String::from("\n\n## References\n\n");
        let mut nums: Vec<&i64> = self.citations.keys().collect();
        nums.sort();
        for num in nums {
            if used.contains(&num.to_string()) {
                let source = &self.citations[num];
                references += &format!(
                    "[{}] {}. Available at: <{}> [Accessed: {}]\n\n",
                    num,
                    Self::title(source),
                    source.get("url").map(|s| s.as_str()).unwrap_or_default(),
                    source.get("access_date").cloned().unwrap_or_else(today)
                );
            }
        }
        let mut metadata = String::from("\n---\n\n**TTD-DR Research Metadata:**\n");
        metadata += "- Algorithm: Test-Time Diffusion Deep Researcher\n";
        metadata += &format!(
            "- Denoising iterations: {}\n",
            self.draft_history.len() as i64 - 1
        );
        metadata += &format!(
            "- Total gaps addressed: {}\n",
            self.gap_analysis_history.iter().sum::<usize>()
        );
        metadata += &format!("- Total sources consulted: {}\n", total);
        metadata += &format!(
            "- Citations used in text: {} ({:.1}%)\n",
            used.len(),
            usage_percentage
        );
        metadata += &format!(
            "- Report generated: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        metadata += &format!("- Total tokens used: {}\n", self.total_tokens);
        report + &references + &metadata
    }

    async fn research(
        &mut self,
        system_prompt: &str,
        initial_query: &str,
    ) -> Result<(String, i64)> {
        self.current_draft = self
            .generate_preliminary_draft(system_prompt, initial_query)
            .await;
        self.draft_history.push(self.current_draft.clone());

        let initial_queries = self.decompose_query(system_prompt, initial_query).await;
        let queries = if initial_queries.is_empty() {
            vec![initial_query.to_string()]
        } else {
            initial_queries
        };
        let results = self.perform_web_search(&queries).await;
        if results.contains("Web Search Results") {
            let (_, sources) = self.extract_and_fetch_urls(&results).await;
            self.register(sources);
        }

        let iterations = match &self.max_iterations {
            Value::Number(n) if n.is_i64() || n.is_u64() => n.as_i64().unwrap_or(i64::MAX),
            Value::Bool(b) => *b as i64,
            Value::Number(_) => {
                return Err(anyhow!(
                    "'float' object cannot be interpreted as an integer"
                ))
            }
            Value::String(_) => {
                return Err(anyhow!("'str' object cannot be interpreted as an integer"))
            }
            Value::Null => {
                return Err(anyhow!(
                    "'NoneType' object cannot be interpreted as an integer"
                ))
            }
            Value::Array(_) => {
                return Err(anyhow!("'list' object cannot be interpreted as an integer"))
            }
            Value::Object(_) => {
                return Err(anyhow!("'dict' object cannot be interpreted as an integer"))
            }
        };
        for _ in 0..iterations.max(0) {
            let draft = self.current_draft.clone();
            let gaps = self.analyze_draft_gaps(&draft, initial_query).await;
            self.gap_analysis_history.push(gaps.len());
            if gaps.is_empty() {
                break;
            }
            let retrieval = self.perform_gap_targeted_search(&gaps).await;
            let (content, sources) = self.extract_and_fetch_urls(&retrieval).await;
            self.register(sources);
            let previous = self.current_draft.clone();
            self.current_draft = self.denoise(&previous, &content, initial_query).await;
            self.draft_history.push(self.current_draft.clone());
            let scores = self
                .evaluate_draft_quality(&self.current_draft.clone(), &previous, initial_query)
                .await;
            let completeness = scores.get("completeness").copied().unwrap_or(0.0);
            let improvement = scores.get("improvement").copied().unwrap_or(0.0);
            if completeness > 0.9 || (improvement < 0.03 && completeness > 0.7) {
                break;
            }
        }
        let draft = self.current_draft.clone();
        let report = self.finalize(system_prompt, initial_query, &draft).await;
        Ok((report, self.total_tokens))
    }
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let rc = &ctx.request_config;
    if py::strings::strip(initial_query).is_empty() {
        return Ok((json!("Error: No research query provided"), 0));
    }
    // DeepResearchClientWrapper: OpenAI-type clients are rebuilt with max_retries=0.
    let no_retry = ctx.client.with_retries(0).map(|c| {
        let mut sub = ctx.clone();
        sub.client = c;
        sub
    });
    let mut researcher = Researcher {
        ctx,
        no_retry,
        max_iterations: rc.get("max_iterations").cloned().unwrap_or(json!(5)),
        max_sources: rc.get("max_sources").and_then(|v| v.as_i64()).unwrap_or(30),
        total_tokens: 0,
        citations: IndexMap::new(),
        citation_counter: 0,
        current_draft: String::new(),
        draft_history: Vec::new(),
        gap_analysis_history: Vec::new(),
    };
    match researcher.research(system_prompt, initial_query).await {
        Ok((report, tokens)) => Ok((json!(report), tokens)),
        Err(e) => Ok((json!(format!("Deep research failed: {}", e)), 0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_tags_and_placeholders_are_cleaned() {
        assert_eq!(
            clean_reasoning_tags("<THINK>x\ny</think>A  b\n\n\n\nc "),
            "A b\n\nc"
        );
        assert_eq!(
            cleanup_placeholder_tags("Intro [SOURCE NEEDED].\n - [TODO: x]\n\nEnd"),
            "Intro .\n\nEnd"
        );
    }

    #[test]
    fn short_reports_are_incomplete() {
        assert!(!report_is_complete("## A\nshort"));
        assert_eq!(per_query(30, 4), 7);
        assert_eq!(per_query(-3, 2), 1);
    }
}
