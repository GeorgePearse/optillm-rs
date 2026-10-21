//! Port of upstream `optillm/plugins/web_search_plugin.py`.
//!
//! Query extraction, result formatting and the enhanced-query layout are
//! verbatim. Upstream drives a real Chrome against google.com through
//! Selenium (with CAPTCHA waits); optillm-rs fetches results over HTTP
//! instead: `$OPTILLM_SEARCH_BACKEND` = `duckduckgo` (default, the
//! html.duckduckgo.com endpoint) or `searxng` (`$OPTILLM_SEARXNG_URL`, JSON API).

use anyhow::{anyhow, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::py;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// `extract_search_queries`.
pub fn extract_search_queries(text: &str) -> Vec<String> {
    let mut text = py::strings::strip(text).to_string();
    for prefix in [
        "User:",
        "user:",
        "User ",
        "user ",
        "Assistant:",
        "assistant:",
        "System:",
        "system:",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = py::strings::strip(rest).to_string();
        }
    }
    let patterns = [
        r"(?i)search for[:\s]+(\S[^\n]*?)(?:\s*\n|$)",
        r"(?i)find information about[:\s]+(\S[^\n]*?)(?:\s*\n|$)",
        r"(?i)look up[:\s]+(\S[^\n]*?)(?:\s*\n|$)",
        r"(?i)research[:\s]+(\S[^\n]*?)(?:\s*\n|$)",
    ];
    let mut queries = Vec::new();
    for p in patterns {
        for c in Regex::new(p).unwrap().captures_iter(&text) {
            let cleaned = py::strings::strip(&c[1])
                .trim_end_matches(['"', '\''])
                .trim_start_matches(['"', '\''])
                .to_string();
            if !cleaned.is_empty() {
                queries.push(cleaned);
            }
        }
    }
    if queries.is_empty() {
        let lower = py::strings::strip(&text.to_lowercase()).to_string();
        let is_empty_search = [
            "search for",
            "find information about",
            "look up",
            "research",
        ]
        .iter()
        .any(|p| {
            lower.starts_with(p)
                && py::strings::len(
                    py::strings::strip(&lower.replace(p, "")).trim_matches(['"', '\'']),
                ) < 2
        });
        if !is_empty_search && !py::strings::strip(&text).is_empty() {
            let cleaned = py::strings::split_whitespace(py::strings::strip(&text)).join(" ");
            if py::strings::len(&cleaned) >= 3 {
                queries.push(cleaned);
            }
        }
    }
    queries
}

/// `format_search_results`.
pub fn format_search_results(query: &str, results: &[SearchResult]) -> String {
    if results.is_empty() {
        return format!("No search results found for: {}", query);
    }
    let mut out = format!("Search results for '{}':\n\n", query);
    for (i, r) in results.iter().enumerate() {
        out += &format!("{}. **{}**\n", i + 1, r.title);
        out += &format!("   URL: {}\n", r.url);
        if !r.snippet.is_empty() {
            out += &format!("   Summary: {}\n", r.snippet);
        }
        out += "\n";
    }
    out
}

/// Fetch results from the configured HTTP backend (`Err` when the results
/// page could not be loaded, where upstream's searcher returns `[]` early).
pub async fn search(query: &str, num_results: usize) -> Result<Vec<SearchResult>> {
    let backend = std::env::var("OPTILLM_SEARCH_BACKEND").unwrap_or_else(|_| "duckduckgo".into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36")
        .build()?;
    match backend.as_str() {
        "searxng" => {
            let base = std::env::var("OPTILLM_SEARXNG_URL")
                .map_err(|_| anyhow!("OPTILLM_SEARXNG_URL is not set"))?;
            let v: Value = client
                .get(format!("{}/search", base.trim_end_matches('/')))
                .query(&[("q", query), ("format", "json")])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            Ok(v.get("results")
                .and_then(|r| r.as_array())
                .map(|a| {
                    a.iter()
                        .take(num_results)
                        .map(|r| SearchResult {
                            title: r
                                .get("title")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            url: r
                                .get("url")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            snippet: r
                                .get("content")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default())
        }
        _ => {
            let html = client
                .post("https://html.duckduckgo.com/html/")
                .form(&[("q", query)])
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let doc = Html::parse_document(&html);
            let result = Selector::parse("div.result").unwrap();
            let link = Selector::parse("a.result__a").unwrap();
            let snippet = Selector::parse(".result__snippet").unwrap();
            let mut out = Vec::new();
            for r in doc.select(&result) {
                let Some(a) = r.select(&link).next() else {
                    continue;
                };
                let href = a.value().attr("href").unwrap_or_default().to_string();
                let url = url::Url::parse(&format!("https:{}", href))
                    .ok()
                    .and_then(|u| {
                        u.query_pairs()
                            .find(|(k, _)| k == "uddg")
                            .map(|(_, v)| v.to_string())
                    })
                    .unwrap_or(href);
                out.push(SearchResult {
                    title: py::strings::strip(&a.text().collect::<String>()).to_string(),
                    url,
                    snippet: r
                        .select(&snippet)
                        .next()
                        .map(|s| py::strings::strip(&s.text().collect::<String>()).to_string())
                        .unwrap_or_default(),
                });
                if out.len() >= num_results {
                    break;
                }
            }
            Ok(out)
        }
    }
}

/// Upstream's post-search pause: `delay_seconds`, else `random.randint(4, 32)`,
/// applied after every search whose results page loaded.
/// `$OPTILLM_SEARCH_DELAY_SCALE` multiplies the sleep (the draw is unchanged).
async fn post_search_delay(ctx: &Ctx, delay: Option<f64>) {
    let secs = match delay {
        Some(d) => d,
        None => ctx.rng().randint(4, 32) as f64,
    };
    if secs > 0.0 {
        let scale = std::env::var("OPTILLM_SEARCH_DELAY_SCALE")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);
        if secs * scale > 0.0 {
            tokio::time::sleep(std::time::Duration::from_secs_f64(secs * scale)).await;
        }
    }
}

/// `web_search_plugin.run` with its request-config knobs resolved.
pub async fn run_with(
    initial_query: &str,
    ctx: &Ctx,
    num_results: usize,
    delay: Option<f64>,
) -> String {
    let queries = extract_search_queries(initial_query);
    if queries.is_empty() {
        return initial_query.to_string();
    }
    let mut enhanced = initial_query.to_string();
    for q in &queries {
        let results = match search(q, num_results).await {
            Ok(r) => {
                post_search_delay(ctx, delay).await;
                r
            }
            Err(_) => Vec::new(),
        };
        if results.is_empty() {
            enhanced = format!(
                "{}\n\n[Web Search Results]:\nNo results found for '{}'. This may be due to network issues or search restrictions.",
                enhanced, q
            );
        } else {
            enhanced = format!(
                "{}\n\n[Web Search Results]:\n{}",
                enhanced,
                format_search_results(q, &results)
            );
        }
    }
    enhanced
}

pub async fn run(_system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let rc = &ctx.request_config;
    let num_results = rc.get("num_results").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let delay = rc.get("delay_seconds").and_then(|v| v.as_f64());
    Ok((
        json!(run_with(initial_query, ctx, num_results, delay).await),
        0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_extraction_matches_upstream_rules() {
        assert_eq!(
            extract_search_queries("User: search for: rust async\nthanks"),
            vec!["rust async"]
        );
        assert_eq!(
            extract_search_queries("Look up \"Python 3.12\" release"),
            vec!["Python 3.12\" release"]
        );
        assert_eq!(extract_search_queries("search for"), Vec::<String>::new());
        assert_eq!(
            extract_search_queries("weather in\nParis"),
            vec!["weather in Paris"]
        );
    }
}
