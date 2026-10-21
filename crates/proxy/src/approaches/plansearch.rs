//! Port of upstream `optillm/plansearch.py`.

use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

const TRUNCATED: &str = "Error: Response was truncated due to token limit. Please increase max_tokens or max_completion_tokens.";

struct PlanSearch<'a> {
    system_prompt: &'a str,
    ctx: &'a Ctx,
    max_tokens: Value,
    tokens: i64,
}

fn non_empty_lines(text: &str) -> Vec<String> {
    py::strings::strip(text)
        .split('\n')
        .map(|l| py::strings::strip(l).to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

impl<'a> PlanSearch<'a> {
    /// One call; returns `None` when the response is unusable.
    async fn ask(&mut self, prompt: String) -> anyhow::Result<Option<String>> {
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "max_tokens": self.max_tokens,
                "messages": [msg("system", self.system_prompt), msg("user", &prompt)]
            }))
            .await?;
        self.tokens += response.completion_tokens()?;
        if response.is_unusable() {
            return Ok(None);
        }
        Ok(response.content(0))
    }

    async fn generate_observations(
        &mut self,
        problem: &str,
        num_observations: i64,
    ) -> anyhow::Result<Vec<String>> {
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert Python programmer. You will be given a competitive programming question\n",
                "(problem specification). You will return several useful, non-obvious, and correct observations\n",
                "about the problem, like hints to solve the problem. You will NOT return any code. Be as\n",
                "creative as possible, going beyond what you think is intuitively correct.\n",
                "\n",
                "Here is the competitive programming problem:\n",
                "{p0}\n",
                "\n",
                "Please provide {p1} observations.",
            ),
            p0 = problem,
            p1 = num_observations,
        );
        Ok(self
            .ask(prompt)
            .await?
            .map(|c| non_empty_lines(&c))
            .unwrap_or_default())
    }

    async fn generate_derived_observations(
        &mut self,
        problem: &str,
        observations: &[String],
        num_new_observations: i64,
    ) -> anyhow::Result<Vec<String>> {
        let listed: Vec<String> = observations
            .iter()
            .enumerate()
            .map(|(i, o)| format!("{}. {}", i + 1, o))
            .collect();
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert Python programmer. You will be given a competitive programming question\n",
                "(problem specification) and several correct observations about the problem.\n",
                "You will brainstorm several new, useful, and correct observations about the problem, derived\n",
                "from the given observations. You will NOT return any code. Be as creative as possible, going\n",
                "beyond what you think is intuitively correct.\n",
                "\n",
                "Here is the competitive programming problem:\n",
                "{p0}\n",
                "\n",
                "Here are the existing observations:\n",
                "{p1}\n",
                "\n",
                "Please provide {p2} new observations derived from the existing ones.",
            ),
            p0 = problem,
            p1 = listed.join("\n"),
            p2 = num_new_observations,
        );
        Ok(self
            .ask(prompt)
            .await?
            .map(|c| non_empty_lines(&c))
            .unwrap_or_default())
    }

    async fn generate_solution(
        &mut self,
        problem: &str,
        observations: &[String],
    ) -> anyhow::Result<String> {
        let listed: Vec<String> = observations
            .iter()
            .enumerate()
            .map(|(i, o)| format!("Observation {}: {}", i + 1, o))
            .collect();
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "Here is the competitive programming problem:\n",
                "{p0}\n",
                "\n",
                "Here are the intelligent observations to help solve the problem:\n",
                "{p1}\n",
                "\n",
                "Use these observations above to brainstorm a natural language solution to the problem above.\n",
                "Note that your intuition may lead you astray, so come up with simple, creative ideas that\n",
                "go beyond what you would usually come up with and exceeds your narrow intuition.\n",
                "Quote relevant parts of the observations EXACTLY before each step of the solution. QUOTING\n",
                "IS CRUCIAL.",
            ),
            p0 = problem,
            p1 = listed.join("\n"),
        );
        Ok(match self.ask(prompt).await? {
            Some(c) => py::strings::strip(&c).to_string(),
            None => TRUNCATED.to_string(),
        })
    }

    async fn implement_solution(
        &mut self,
        problem: &str,
        solution: &str,
    ) -> anyhow::Result<String> {
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "You are an expert Python programmer. You will be given a question (problem specification)\n",
                "and a natural language solution/tutorial that describes how to solve the problem. You will\n",
                "generate a correct Python program that matches said specification and tutorial and passes\n",
                "all tests. You will NOT return anything except for the program inside markdown codeblocks.\n",
                "\n",
                "Problem:\n",
                "{p0}\n",
                "\n",
                "Solution:\n",
                "{p1}\n",
                "\n",
                "Please implement the solution in Python.",
            ),
            p0 = problem,
            p1 = solution,
        );
        Ok(match self.ask(prompt).await? {
            Some(c) => py::strings::strip(&c).to_string(),
            None => TRUNCATED.to_string(),
        })
    }

    async fn solve(&mut self, problem: &str) -> anyhow::Result<String> {
        let initial = self.generate_observations(problem, 3).await?;
        let derived = self
            .generate_derived_observations(problem, &initial, 2)
            .await?;
        let mut all = initial;
        all.extend(derived);
        let solution = self.generate_solution(problem, &all).await?;
        self.implement_solution(problem, &solution).await
    }
}

pub async fn plansearch(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
    n: i64,
) -> ApproachResult {
    let mut planner = PlanSearch {
        system_prompt,
        ctx,
        max_tokens: ctx.max_tokens(4096),
        tokens: 0,
    };
    let mut solutions = Vec::new();
    for _ in 0..n.max(0) {
        solutions.push(Value::String(planner.solve(initial_query).await?));
    }
    Ok((Value::Array(solutions), planner.tokens))
}
