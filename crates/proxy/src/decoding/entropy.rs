//! `entropy_decoding` approximation over API logprobs (one call per token);
//! mirrors the sampling rules of upstream `optillm/entropy_decoding.py` using
//! only the top-20 logprobs the API exposes. Attention metrics are 0.

use anyhow::Result;
use serde_json::json;

use super::{build_completion, usage_prompt_tokens, Backend, Request};
use crate::client::ChatCompletion;
use crate::py::random::PyRandom;

// Upstream uses this truncated literal, not f64::consts::LN_2.
#[allow(clippy::approx_constant)]
const LN_2: f64 = 0.69314718056;

/// (entropy, varentropy) in bits over the renormalised top-k distribution.
fn varentropy(logprobs: &[f64]) -> (f64, f64) {
    let max = logprobs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let z: f64 = logprobs.iter().map(|l| (l - max).exp()).sum();
    let logp: Vec<f64> = logprobs.iter().map(|l| l - max - z.ln()).collect();
    let entropy = -logp.iter().map(|l| l.exp() * l).sum::<f64>() / LN_2;
    let var = logp
        .iter()
        .map(|l| l.exp() * (l / LN_2 + entropy).powi(2))
        .sum::<f64>();
    (entropy, var)
}

/// `_sample` over the candidate list (upstream's min_p mask never affects
/// the probabilities it samples from, so it is not applied here either).
fn sample(
    cands: &[(String, f64)],
    temperature: f64,
    top_p: f64,
    top_k: usize,
    rng: &mut PyRandom,
) -> usize {
    let scaled: Vec<f64> = cands.iter().map(|(_, l)| l / temperature).collect();
    let max = scaled.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let z: f64 = scaled.iter().map(|l| (l - max).exp()).sum();
    let mut order: Vec<(usize, f64)> = scaled
        .iter()
        .enumerate()
        .map(|(i, l)| (i, (l - max).exp() / z))
        .collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    order.truncate(top_k.max(1));
    order.reverse(); // torch.flip: ascending
    let mut cum = 0.0;
    let mut kept: Vec<(usize, f64)> = Vec::new();
    for (i, p) in order {
        cum += p;
        if cum - p <= top_p {
            kept.push((i, p));
        }
    }
    let total: f64 = kept.iter().map(|k| k.1).sum();
    let mut r = rng.random() * total;
    for (i, p) in &kept {
        if r < *p {
            return *i;
        }
        r -= p;
    }
    kept.last().map(|k| k.0).unwrap_or(0)
}

pub async fn entropy_decode(backend: &Backend<'_>, req: &Request) -> Result<ChatCompletion> {
    let max_new = req.opt_i64("max_tokens").unwrap_or(512);
    let temperature = req.f64("temperature", 0.666);
    let top_p = req.f64("top_p", 0.90);
    let top_k = req.i64("top_k", 27).max(1) as usize;
    let mut generated = String::new();
    let mut prompt_tokens = 0;
    let mut steps = 0;
    for _ in 0..max_new.max(0) {
        let resp = backend
            .complete(
                &req.messages,
                &generated,
                json!({"max_tokens": 1, "temperature": 1.0, "logprobs": true, "top_logprobs": 20}),
            )
            .await?;
        if prompt_tokens == 0 {
            prompt_tokens = usage_prompt_tokens(&resp);
        }
        let cands: Vec<(String, f64)> = resp
            .choices()
            .first()
            .and_then(|c| c.get("logprobs"))
            .and_then(|l| l.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|t| t.get("top_logprobs"))
            .and_then(|t| t.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        Some((
                            x.get("token")?.as_str()?.to_string(),
                            x.get("logprob")?.as_f64()?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if cands.is_empty() {
            break; // end of sequence
        }
        let lps: Vec<f64> = cands.iter().map(|c| c.1).collect();
        let (ent, vent) = varentropy(&lps);
        let idx = {
            let mut rng = backend.rng.lock().unwrap();
            if ent < 0.1 && vent < 0.1 {
                0
            } else if ent > 3.0 && vent < 0.1 {
                sample(
                    &cands,
                    f64::min(1.5, temperature * 1.3),
                    top_p,
                    top_k,
                    &mut rng,
                )
            } else if ent < 5.0 && vent > 5.0 {
                let k = std::cmp::max(5, (top_k as f64 * 1.5) as usize);
                sample(&cands, f64::min(1.5, temperature * 1.2), top_p, k, &mut rng)
            } else if ent > 5.0 && vent > 5.0 {
                sample(
                    &cands,
                    f64::max(2.0, temperature * 2.0),
                    f64::max(0.5, top_p),
                    top_k,
                    &mut rng,
                )
            } else {
                // adaptive_sample with attention metrics = 0: 5 samples, keep the most likely.
                let u = ent + vent;
                let t = temperature * (1.0 + 0.3 * u);
                let p = top_p.clamp(0.1, 1.0);
                let k = (top_k as f64).round().clamp(1.0, 100.0) as usize;
                (0..5)
                    .map(|_| sample(&cands, t, p, k, &mut rng))
                    .max_by(|a, b| {
                        cands[*a]
                            .1
                            .partial_cmp(&cands[*b].1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(0)
            }
        };
        generated.push_str(&cands[idx].0);
        steps += 1;
        if resp.finish_reason(0).as_deref() == Some("stop")
            && resp.content(0).unwrap_or_default().is_empty()
        {
            break;
        }
    }
    Ok(build_completion(
        backend.model,
        &generated,
        None,
        prompt_tokens,
        steps,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_of_uniform_pair_is_one_bit() {
        let (e, v) = varentropy(&[0.5f64.ln(), 0.5f64.ln()]);
        assert!((e - 1.0).abs() < 1e-6);
        assert!(v.abs() < 1e-6);
    }
}
