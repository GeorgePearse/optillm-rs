//! `deepconf` (online mode) over API logprobs; mirrors upstream
//! `optillm/deepconf/{processor,confidence}.py`.

use anyhow::Result;
use serde_json::json;

use super::{build_completion, token_logprobs, usage_prompt_tokens, Backend, Request};
use crate::client::ChatCompletion;
use crate::py;

struct Trace {
    text: String,
    average_confidence: f64,
    stats: Stats,
    tokens: i64,
    terminated_early: bool,
}

#[derive(Clone, Copy, Default)]
struct Stats {
    average_confidence: f64,
    bottom_10_percent: f64,
    lowest_group: f64,
    current_group: f64,
}

/// numpy mean (pairwise summation is not reproduced; differences are in the last ulp).
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// `np.percentile(values, q)` with linear interpolation.
fn percentile(values: &[f64], q: f64) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if v.is_empty() {
        return 0.0;
    }
    let pos = (v.len() - 1) as f64 * q / 100.0;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    v[lo] + (v[hi] - v[lo]) * (pos - lo as f64)
}

fn answer_key(text: &str) -> String {
    let t = py::strings::strip(text);
    let last = t.split('.').next_back().unwrap_or("");
    let ans = py::strings::strip(last).to_string();
    if ans.is_empty() {
        py::strings::strip(&py::strings::slice(t, Some(-50), None)).to_string()
    } else {
        ans
    }
}

struct Config {
    variant: String,
    warmup: i64,
    consensus: f64,
    max_traces: i64,
    window: usize,
    top_k: i64,
    min_len: usize,
    max_per_trace: i64,
    temperature: f64,
    metric: String,
    include_stats: bool,
}

async fn generate_trace(
    backend: &Backend<'_>,
    req: &Request,
    cfg: &Config,
    threshold: Option<f64>,
) -> Result<(Trace, i64)> {
    let resp = backend
        .complete(
            &req.messages,
            "",
            json!({
                "max_tokens": cfg.max_per_trace,
                "temperature": cfg.temperature,
                "logprobs": true,
                "top_logprobs": cfg.top_k.clamp(1, 20)
            }),
        )
        .await?;
    let steps = token_logprobs(&resp);
    let mut token_conf: Vec<f64> = Vec::new();
    let mut groups: Vec<f64> = Vec::new();
    let mut text = String::new();
    let mut terminated_early = false;
    for (count, (token, _, tops)) in steps.iter().enumerate() {
        // C = -(1/k) * sum of the top-k logprobs.
        token_conf.push(-mean(tops));
        if token_conf.len() >= cfg.window {
            groups.push(mean(&token_conf[token_conf.len() - cfg.window..]));
        }
        if let Some(th) = threshold {
            if count >= cfg.min_len {
                if let Some(g) = groups.last() {
                    if *g < th {
                        terminated_early = true;
                        token_conf.pop();
                        break;
                    }
                }
            }
        }
        text.push_str(token);
    }
    if steps.is_empty() {
        text = resp.content(0).unwrap_or_default();
    }
    let bottom = if groups.is_empty() {
        0.0
    } else {
        let mut g = groups.clone();
        g.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        mean(&g[..std::cmp::max(1, g.len() / 10)])
    };
    let stats = Stats {
        average_confidence: mean(&token_conf),
        bottom_10_percent: bottom,
        lowest_group: if groups.is_empty() {
            0.0
        } else {
            groups.iter().cloned().fold(f64::INFINITY, f64::min)
        },
        current_group: groups.last().copied().unwrap_or(0.0),
    };
    let n = token_conf.len() as i64;
    Ok((
        Trace {
            text,
            average_confidence: stats.average_confidence,
            stats,
            tokens: n,
            terminated_early,
        },
        usage_prompt_tokens(&resp),
    ))
}

pub async fn deepconf_decode(backend: &Backend<'_>, req: &Request) -> Result<ChatCompletion> {
    let cfg = Config {
        variant: req.str("variant", "low"),
        warmup: req.i64("warmup_samples", 16),
        consensus: req.f64("consensus_threshold", 0.95),
        max_traces: req.i64("max_traces", 128),
        window: req.i64("window_size", 2048).max(1) as usize,
        top_k: req.i64("top_k", 5),
        min_len: req.i64("min_trace_length", 100).max(0) as usize,
        max_per_trace: req.i64("max_tokens_per_trace", 4096),
        temperature: req.f64("temperature", 1.0),
        metric: req.str("confidence_metric", "average_confidence"),
        include_stats: req.bool("include_stats", false),
    };
    let mut traces: Vec<Trace> = Vec::new();
    let mut prompt_tokens = 0;
    for _ in 0..cfg.warmup.max(0) {
        let (t, p) = generate_trace(backend, req, &cfg, None).await?;
        prompt_tokens = p;
        traces.push(t);
    }
    let metric_values: Vec<f64> = traces
        .iter()
        .map(|t| match cfg.metric.as_str() {
            "bottom_10_percent" => t.stats.bottom_10_percent,
            "lowest_group" => t.stats.lowest_group,
            "current_group" => t.stats.current_group,
            _ => t.stats.average_confidence,
        })
        .collect();
    let threshold = if metric_values.is_empty() {
        0.0
    } else if cfg.variant == "low" {
        percentile(&metric_values, 90.0)
    } else {
        percentile(&metric_values, 10.0)
    };
    for _ in 0..(cfg.max_traces - cfg.warmup).max(0) {
        let (t, _) = generate_trace(backend, req, &cfg, Some(threshold)).await?;
        traces.push(t);
        // check_consensus
        let keys: Vec<String> = traces.iter().map(|t| answer_key(&t.text)).collect();
        let mut counts: Vec<(String, usize)> = Vec::new();
        for k in &keys {
            match counts.iter_mut().find(|(x, _)| x == k) {
                Some(c) => c.1 += 1,
                None => counts.push((k.clone(), 1)),
            }
        }
        let best = counts.iter().fold(0usize, |m, c| m.max(c.1));
        if best as f64 / keys.len() as f64 >= cfg.consensus {
            break;
        }
    }
    // weighted_majority_vote
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    for t in &traces {
        let k = answer_key(&t.text);
        match groups.iter_mut().find(|(x, _)| *x == k) {
            Some(g) => g.1.push(t.average_confidence),
            None => groups.push((k, vec![t.average_confidence])),
        }
    }
    let total = traces.len().max(1) as f64;
    let scores: Vec<f64> = groups
        .iter()
        .map(|(_, c)| {
            py::strings::py_sum(c.iter().copied()) / c.len() as f64 * 0.7
                + c.len() as f64 / total * 0.3
        })
        .collect();
    let best = py::strings::argmax_by(&scores, |s| *s).unwrap_or(0);
    let answer = groups.get(best).map(|g| g.0.clone()).unwrap_or_default();
    let tokens: i64 = traces.iter().map(|t| t.tokens).sum();
    let mut response = py::strings::strip(&answer).to_string();
    if cfg.include_stats {
        let warmup = (cfg.warmup.max(0) as usize).min(traces.len());
        #[rustfmt::skip]
        let stats_text = format!(
            concat!(
                "\n",
                "\n",
                "DeepConf Statistics:\n",
                "- Variant: {p0}\n",
                "- Total traces: {p1} (warmup: {p2}, online: {p3})\n",
                "- Early terminations: {p4}\n",
                "- Total tokens: {p5}\n",
                "- Confidence threshold: {p6:.4}\n",
                "- Unique answers: {p7}",
            ),
            p0 = cfg.variant, p1 = traces.len(), p2 = warmup, p3 = traces.len() - warmup,
            p4 = traces.iter().filter(|t| t.terminated_early).count(), p5 = tokens,
            p6 = threshold, p7 = groups.len(),
        );
        response.push_str(&stats_text);
    }
    Ok(build_completion(
        backend.model,
        &response,
        None,
        prompt_tokens,
        tokens,
    ))
}
