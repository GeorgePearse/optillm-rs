//! Port of upstream `optillm/z3_solver.py` (Z3 / SymPy solver).
//!
//! The generated solver code is Python, so it is executed the same way
//! upstream does it: by CPython with z3 and sympy, here in a subprocess
//! (`$OPTILLM_Z3_PYTHON`, default `python3`) running `z3_runner.py`, which
//! carries upstream's execution globals verbatim. Upstream's 30 s timeout is
//! enforced by killing the subprocess.

use std::time::Duration;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

const RUNNER: &str = include_str!("z3_runner.py");
const TIMEOUT: Duration = Duration::from_secs(30);

struct Solver<'a> {
    system_prompt: &'a str,
    ctx: &'a Ctx,
    max_tokens: Value,
    /// Upstream assigns (not adds) the last call's completion tokens.
    tokens: i64,
}

fn runner_path() -> std::io::Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!("optillm_rs_z3_runner_{}.py", std::process::id()));
    if !path.exists() {
        std::fs::write(&path, RUNNER)?;
    }
    Ok(path)
}

/// `Z3SymPySolverSystem.execute_solver_code`.
pub async fn execute_solver_code(code: &str) -> String {
    let python = std::env::var("OPTILLM_Z3_PYTHON").unwrap_or_else(|_| "python3".into());
    let runner = match runner_path() {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };
    let out_path = std::env::temp_dir().join(format!(
        "optillm_rs_z3_{}_{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let child = tokio::process::Command::new(&python)
        .arg(&runner)
        .arg(&out_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return format!("Error: could not start {}: {}", python, e),
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(code.as_bytes()).await;
    }
    let status = match tokio::time::timeout(TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return format!("Error: {}", e),
        Err(_) => return "Error: Execution timed out".to_string(),
    };
    let result = std::fs::read_to_string(&out_path);
    let _ = std::fs::remove_file(&out_path);
    let result: Value = match result.ok().and_then(|s| serde_json::from_str(&s).ok()) {
        Some(v) => v,
        None => {
            return format!(
                "Error: {}",
                String::from_utf8_lossy(&status.stderr).trim_end()
            )
        }
    };
    match result["kind"].as_str() {
        Some("syntax") => format!(
            "Error: Syntax error: {}",
            result["message"].as_str().unwrap_or_default()
        ),
        Some("error") => format!("Error: {}", result["output"].as_str().unwrap_or_default()),
        _ => result["output"].as_str().unwrap_or_default().to_string(),
    }
}

/// `Z3SymPySolverSystem.extract_and_validate_expressions`.
fn extract_and_validate_expressions(analysis: &str) -> Result<String> {
    let re = Regex::new(r"```python\n([\s\S]+?)```").unwrap();
    match re.captures(analysis) {
        Some(c) => Ok(py::strings::strip(&c[1]).to_string()),
        None => Err(anyhow!(
            "No valid Z3 or SymPy formulation found in the analysis."
        )),
    }
}

impl<'a> Solver<'a> {
    async fn call(&mut self, user: &str, max_tokens: Value) -> Result<Value> {
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "messages": [msg("system", self.system_prompt), msg("user", user)],
                "max_tokens": max_tokens,
                "n": 1,
                "temperature": 0.1
            }))
            .await?;
        self.tokens = response.completion_tokens()?;
        if response.choices().is_empty() {
            return Err(anyhow!("list index out of range"));
        }
        Ok(response
            .message(0)
            .and_then(|m| m.get("content"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    async fn standard_llm_inference(&mut self, query: &str) -> Result<Value> {
        let mt = self.max_tokens.clone();
        self.call(query, mt).await
    }

    async fn analyze_query(&mut self, query: &str) -> Result<Value> {
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "Analyze the given query and determine if it can be solved using Z3 or SymPy:\n",
                "\n",
                "1. Identify variables, constraints, and objectives.\n",
                "2. Determine the problem type (e.g., SAT, optimization, symbolic manipulation).\n",
                "3. Decide if Z3, SymPy, or a combination of both is suitable.\n",
                "\n",
                "If Z3 or SymPy can be applied, provide Python code using the appropriate library (or both) to solve the problem. Make sure you define any additional methods you need for solving the problem.\n",
                "The code will be executed in an environment with Z3 and SymPy available, so do not include any other libraries or modules.\n",
                "\n",
                "Query: {p0}\n",
                "\n",
                "Respond with:\n",
                "SOLVER_CAN_BE_APPLIED: [True/False]\n",
                "\n",
                "SOLVER_FORMULATION:\n",
                "```python\n",
                "# Z3 and/or SymPy code here\n",
                "```\n",
                "\n",
                "Analysis:\n",
                "[Your step-by-step analysis]\n",
            ),
            p0 = query,
        );
        self.call(&prompt, json!(1024)).await
    }

    async fn solve_with_z3_sympy(&mut self, mut formulation: String) -> Result<(bool, String)> {
        for _ in 0..3 {
            let output = execute_solver_code(&formulation).await;
            if !output.contains("Error:") {
                return Ok((true, output));
            }
            #[rustfmt::skip]
            let error_prompt = format!(
                concat!(
                    "Fix the Z3 or SymPy code that resulted in an error. Follow these steps:\n",
                    "\n",
                    "    1. Review the original code and the error message carefully.\n",
                    "    2. Analyze the error and identify its root cause.\n",
                    "    3. Think through the necessary changes to fix the error.\n",
                    "    4. Generate a corrected version of the code.\n",
                    "\n",
                    "    Original Code:\n",
                    "    {p0}\n",
                    "\n",
                    "    Error Message:\n",
                    "    {p1}\n",
                    "\n",
                    "    Step-by-Step Analysis:\n",
                    "    [Provide your step-by-step analysis here]\n",
                    "\n",
                    "    Corrected Z3 or SymPy Code:\n",
                    "    ```python\n",
                    "    # Corrected code here\n",
                    "    ```\n",
                    "    ",
                ),
                p0 = formulation,
                p1 = output,
            );
            let content = self.call(&error_prompt, json!(1024)).await?;
            let text = content
                .as_str()
                .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?;
            formulation = extract_and_validate_expressions(text)?;
        }
        Ok((
            false,
            "Failed to solve after multiple attempts.".to_string(),
        ))
    }

    async fn generate_response(
        &mut self,
        query: &str,
        analysis: &str,
        solved: (bool, String),
    ) -> Result<Value> {
        if !solved.0 {
            return self.standard_llm_inference(query).await;
        }
        #[rustfmt::skip]
        let prompt = format!(
            concat!(
                "Provide a clear answer to the query using the analysis and solver result:\n",
                "\n",
                "Query: {p0}\n",
                "\n",
                "Analysis: {p1}\n",
                "\n",
                "Solver Result: {p2}\n",
                "\n",
                "Response:\n",
            ),
            p0 = query,
            p1 = analysis,
            p2 = solved.1,
        );
        let mt = self.max_tokens.clone();
        self.call(&prompt, mt).await
    }

    async fn try_process(&mut self, query: &str) -> Result<Value> {
        let analysis = self.analyze_query(query).await?;
        let analysis = analysis
            .as_str()
            .ok_or_else(|| anyhow!("argument of type 'NoneType' is not iterable"))?
            .to_string();
        if !analysis.contains("SOLVER_CAN_BE_APPLIED: True") {
            return self.standard_llm_inference(query).await;
        }
        let formulation = extract_and_validate_expressions(&analysis)?;
        let solved = self.solve_with_z3_sympy(formulation).await?;
        self.generate_response(query, &analysis, solved).await
    }
}

pub async fn z3_solver(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let mut solver = Solver {
        system_prompt,
        ctx,
        max_tokens: ctx.max_tokens(4096),
        tokens: 0,
    };
    let response = match solver.try_process(initial_query).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "An error occurred while processing the query with Z3 and SymPy, returning standard llm inference results: {}",
                e
            );
            solver.standard_llm_inference(initial_query).await?
        }
    };
    Ok((response, solver.tokens))
}
