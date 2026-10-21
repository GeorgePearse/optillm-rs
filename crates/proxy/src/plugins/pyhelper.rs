//! Runs `py_helper.py` (upstream's own code-execution functions) in a CPython
//! subprocess: `$OPTILLM_PYTHON`, else `python3`. Needed for plugins whose
//! behaviour is defined by Python libraries (nbconvert, ast.literal_eval).

use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

const HELPER: &str = include_str!("py_helper.py");

pub fn python_bin() -> String {
    std::env::var("OPTILLM_PYTHON")
        .or_else(|_| std::env::var("OPTILLM_Z3_PYTHON"))
        .unwrap_or_else(|_| "python3".into())
}

/// Run one helper operation and return its JSON result.
pub async fn call(request: Value, timeout_secs: u64) -> Result<Value> {
    let dir = std::env::temp_dir();
    let script = dir.join(format!("optillm_rs_py_helper_{}.py", std::process::id()));
    if !script.exists() {
        std::fs::write(&script, HELPER)?;
    }
    let out = dir.join(format!(
        "optillm_rs_helper_{}.json",
        uuid::Uuid::new_v4().simple()
    ));
    let mut child = tokio::process::Command::new(python_bin())
        .arg(&script)
        .arg(&out)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow!("could not start {}: {}", python_bin(), e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request.to_string().as_bytes()).await?;
    }
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| anyhow!("python helper timed out"))??;
    let text = std::fs::read_to_string(&out);
    let _ = std::fs::remove_file(&out);
    match text {
        Ok(t) => Ok(serde_json::from_str(&t)?),
        Err(_) => Err(anyhow!(
            "python helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        )),
    }
}
