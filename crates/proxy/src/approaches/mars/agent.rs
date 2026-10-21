//! Port of upstream `optillm/mars/agent.py`.

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};

use super::prompts::{AGENT_EXPLORATION_PROMPT, MATHEMATICAL_SYSTEM_PROMPT, VERIFICATION_PROMPT};
use super::workspace::{AgentSolution, VerificationResult};
use super::{reasoning_and_total, Config};
use crate::approaches::Ctx;
use crate::parse::msg;
use crate::py;

#[derive(Clone)]
pub struct MarsAgent {
    pub agent_id: i64,
    pub temperature: f64,
    pub max_tokens: Value,
}

impl MarsAgent {
    pub fn new(agent_id: i64, config: &Config) -> Self {
        let temperatures = [0.3, 0.6, 1.0];
        Self {
            agent_id,
            temperature: temperatures[(agent_id % 3) as usize],
            max_tokens: config.get("max_tokens"),
        }
    }

    pub fn reasoning_effort(&self) -> &'static str {
        if self.temperature <= 0.4 {
            "low"
        } else if self.temperature <= 0.8 {
            "medium"
        } else {
            "high"
        }
    }

    async fn call(
        &self,
        ctx: &Ctx,
        user: &str,
        temperature: f64,
        effort: &str,
    ) -> Result<crate::client::ChatCompletion> {
        ctx.create(json!({
            "model": ctx.model,
            "messages": [msg("system", MATHEMATICAL_SYSTEM_PROMPT), msg("user", user)],
            "max_tokens": self.max_tokens,
            "temperature": temperature,
            "extra_body": {"reasoning": {"effort": effort}}
        }))
        .await
    }

    async fn try_generate(&self, ctx: &Ctx, problem: &str) -> Result<(AgentSolution, Option<i64>)> {
        let prompt = py::strings::format_named(
            AGENT_EXPLORATION_PROMPT,
            &[
                ("agent_id", &self.agent_id.to_string()),
                ("temperature", &py::json::float_repr(self.temperature)),
                ("problem", problem),
            ],
        )
        .map_err(|e| anyhow!(e))?;
        let response = self
            .call(ctx, &prompt, self.temperature, self.reasoning_effort())
            .await?;
        let content = response
            .content_at(0)?
            .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
        let solution_text = py::strings::strip(&content).to_string();
        let (reasoning, total) = reasoning_and_total(&response);
        // reasoning_ratio = reasoning / total * 100 if total > 0 else 0
        match total {
            Some(t) if t > 0 => {
                if reasoning.is_none() {
                    return Err(anyhow!(
                        "unsupported operand type(s) for /: 'NoneType' and 'int'"
                    ));
                }
            }
            Some(_) => {}
            None => {
                return Err(anyhow!(
                    "'>' not supported between instances of 'NoneType' and 'int'"
                ))
            }
        }
        let confidence = estimate_confidence(&solution_text);
        Ok((
            AgentSolution {
                agent_id: self.agent_id.to_string(),
                solution: solution_text,
                confidence,
                reasoning_tokens: reasoning,
                is_verified: false,
                verification_score: 0.0,
                temperature: self.temperature,
                verification_results: Vec::new(),
            },
            reasoning,
        ))
    }

    /// `generate_solution`: never fails; errors become an error solution.
    pub async fn generate_solution(
        &self,
        ctx: &Ctx,
        problem: &str,
    ) -> (AgentSolution, Option<i64>) {
        match self.try_generate(ctx, problem).await {
            Ok(r) => r,
            Err(e) => {
                let error_message = format!("Error generating solution: {}", e);
                (
                    AgentSolution {
                        agent_id: self.agent_id.to_string(),
                        solution: error_message,
                        confidence: 0.0,
                        reasoning_tokens: Some(0),
                        is_verified: false,
                        verification_score: 0.0,
                        temperature: self.temperature,
                        verification_results: Vec::new(),
                    },
                    Some(0),
                )
            }
        }
    }

    async fn try_verify(
        &self,
        ctx: &Ctx,
        problem: &str,
        solution: &str,
        solution_agent_id: &str,
    ) -> Result<VerificationResult> {
        let prompt = py::strings::format_named(
            VERIFICATION_PROMPT,
            &[("problem", problem), ("solution", solution)],
        )
        .map_err(|e| anyhow!(e))?;
        let response = self.call(ctx, &prompt, 0.1, "low").await?;
        let content = response
            .content_at(0)?
            .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
        let verification_text = py::strings::strip(&content).to_string();
        let (assessment, confidence, _issues) = parse_verification(&verification_text)?;
        Ok(VerificationResult {
            solution_id: format!("agent_{}_iter_0", solution_agent_id),
            assessment,
            confidence,
        })
    }

    /// `verify_solution`: errors become an INCOMPLETE result.
    pub async fn verify_solution(
        &self,
        ctx: &Ctx,
        problem: &str,
        solution: &str,
        solution_agent_id: &str,
    ) -> VerificationResult {
        match self
            .try_verify(ctx, problem, solution, solution_agent_id)
            .await
        {
            Ok(r) => r,
            Err(e) => VerificationResult {
                solution_id: format!("agent_{}_iter_0", solution_agent_id),
                assessment: "INCOMPLETE".into(),
                confidence: {
                    tracing::warn!("MARS verification error: {}", e);
                    0.0
                },
            },
        }
    }
}

/// `MARSAgent._estimate_confidence`.
pub fn estimate_confidence(solution: &str) -> f64 {
    let lower = solution.to_lowercase();
    let mut confidence = 0.5;
    if solution.contains("\\boxed{") {
        confidence += 0.2;
    }
    if lower.contains("therefore") || lower.contains("thus") {
        confidence += 0.1;
    }
    if lower.contains("proof") {
        confidence += 0.1;
    }
    if py::strings::split_whitespace(solution).len() > 200 {
        confidence += 0.1;
    }
    if lower.contains("let") && lower.contains("assume") {
        confidence += 0.1;
    }
    if lower.contains("might") || lower.contains("possibly") {
        confidence -= 0.1;
    }
    if lower.contains("unsure") || lower.contains("not sure") {
        confidence -= 0.2;
    }
    crate::approaches::py_max(0.1, crate::approaches::py_min(1.0, confidence))
}

/// `MARSAgent._parse_verification` -> (assessment, confidence, issues).
pub fn parse_verification(text: &str) -> Result<(String, f64, Vec<String>)> {
    let lower = text.to_lowercase();
    let (mut assessment, mut confidence) = ("INCOMPLETE".to_string(), 0.5);
    if lower.contains("correct") && !lower.contains("incorrect") {
        assessment = "CORRECT".into();
        confidence = 0.8;
    } else if lower.contains("incorrect") {
        assessment = "INCORRECT".into();
        confidence = 0.8;
    } else if lower.contains("incomplete") {
        assessment = "INCOMPLETE".into();
        confidence = 0.6;
    }
    let re = Regex::new(r"confidence.*?(\d+).*?(?:out of|/)\s*(\d+)").unwrap();
    if let Some(c) = re.captures(&lower) {
        let score = py::strings::parse_float(&c[1]).unwrap_or(0.0);
        let total = py::strings::parse_float(&c[2]).unwrap_or(0.0);
        if total == 0.0 {
            return Err(anyhow!("float division by zero"));
        }
        confidence = score / total;
    }
    let keywords = ["error", "mistake", "incorrect", "wrong", "issue"];
    let issues = text
        .split('\n')
        .filter(|l| {
            let ll = l.to_lowercase();
            keywords.iter().any(|k| ll.contains(k))
        })
        .map(|l| py::strings::strip(l).to_string())
        .collect();
    Ok((assessment, confidence, issues))
}
