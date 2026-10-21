//! Port of upstream `optillm/mars/verifier.py`.

use std::sync::Mutex;

use super::agent::MarsAgent;
use super::workspace::Workspace;
use super::Executor;
use crate::approaches::Ctx;

/// `_select_verifier_agent`: upstream compares each agent's int id with the
/// solution's str id, which never matches, so every agent is "available",
/// no temperature sort happens and the first agent always verifies.
fn select_verifier(agents: &[MarsAgent]) -> Option<&MarsAgent> {
    agents.first()
}

/// `_verify_single_solution` (runs as one executor job upstream).
async fn verify_single_solution(
    ctx: &Ctx,
    agents: &[MarsAgent],
    ws: &Mutex<Workspace>,
    index: usize,
) {
    let (problem, solution_text, solution_agent_id, threshold, attempts) = {
        let w = ws.lock().unwrap();
        let s = &w.solutions[index];
        (
            w.problem.clone(),
            s.solution.clone(),
            s.agent_id.clone(),
            w.config.int("verification_passes_required", 5),
            w.config.int("max_verification_attempts", 10),
        )
    };
    let mut consecutive = 0;
    for _ in 0..attempts.max(0) {
        let Some(verifier) = select_verifier(agents) else {
            break;
        };
        let verification = verifier
            .verify_solution(ctx, &problem, &solution_text, &solution_agent_id)
            .await;
        let correct = verification.assessment == "CORRECT";
        ws.lock().unwrap().add_verification(verification);
        if correct {
            consecutive += 1;
            if consecutive >= threshold {
                break;
            }
        } else {
            consecutive = 0;
        }
    }
}

/// `verify_solutions_parallel`.
pub async fn verify_solutions_parallel(
    ctx: &Ctx,
    agents: &[MarsAgent],
    ws: &Mutex<Workspace>,
    executor: &Executor,
) {
    let n = ws.lock().unwrap().solutions.len();
    let jobs = (0..n).map(|i| executor.run(verify_single_solution(ctx, agents, ws, i)));
    futures::future::join_all(jobs).await;
}

/// `iterative_improvement_parallel`: no solution can be matched to an agent
/// upstream (int vs str ids), so nothing is ever improved.
pub async fn iterative_improvement_parallel() -> i64 {
    0
}
