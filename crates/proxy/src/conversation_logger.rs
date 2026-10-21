//! Port of upstream `optillm/conversation_logger.py`: JSONL logs of every
//! client request with all provider calls, rotated daily.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{json, Map, Value};

use crate::py;

struct Entry {
    request_id: String,
    timestamp: String,
    approach: String,
    model: String,
    client_request: Value,
    provider_calls: Vec<Value>,
    final_response: Option<Value>,
    error: Option<String>,
    start: Instant,
}

pub struct ConversationLogger {
    pub enabled: bool,
    pub log_dir: PathBuf,
    active: Mutex<HashMap<String, Entry>>,
}

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6f+00:00")
        .to_string()
}

impl ConversationLogger {
    pub fn new(log_dir: PathBuf, enabled: bool) -> Self {
        if enabled {
            let _ = std::fs::create_dir_all(&log_dir);
            tracing::info!(
                "Conversation logging enabled. Logs will be saved to: {}",
                log_dir.display()
            );
        }
        Self {
            enabled,
            log_dir,
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_conversation(&self, client_request: Value, approach: &str, model: &str) -> String {
        if !self.enabled {
            return String::new();
        }
        let id = format!("req_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let entry = Entry {
            request_id: id.clone(),
            timestamp: now_iso(),
            approach: approach.to_string(),
            model: model.to_string(),
            client_request,
            provider_calls: Vec::new(),
            final_response: None,
            error: None,
            start: Instant::now(),
        };
        self.active.lock().unwrap().insert(id.clone(), entry);
        id
    }

    pub fn log_provider_call(&self, request_id: &str, request: &Value, response: &Value) {
        if !self.enabled || request_id.is_empty() {
            return;
        }
        let mut active = self.active.lock().unwrap();
        if let Some(entry) = active.get_mut(request_id) {
            let falsy = |v: &Value| match v {
                Value::Null => true,
                Value::Object(m) => m.is_empty(),
                Value::String(s) => s.is_empty(),
                _ => false,
            };
            let call = json!({
                "call_number": entry.provider_calls.len() + 1,
                "timestamp": now_iso(),
                "request": if falsy(request) { Value::Null } else { request.clone() },
                "response": if falsy(response) { Value::Null } else { response.clone() },
            });
            entry.provider_calls.push(call);
        } else {
            tracing::warn!("No active conversation found for request {}", request_id);
        }
    }

    pub fn log_final_response(&self, request_id: &str, final_response: &Value) {
        if !self.enabled || request_id.is_empty() {
            return;
        }
        let mut active = self.active.lock().unwrap();
        if let Some(entry) = active.get_mut(request_id) {
            let mut fr = final_response.as_object().cloned().unwrap_or_else(Map::new);
            fr.insert("timestamp".into(), json!(now_iso()));
            entry.final_response = Some(Value::Object(fr));
        }
    }

    pub fn log_error(&self, request_id: &str, error: &str) {
        if !self.enabled || request_id.is_empty() {
            return;
        }
        if let Some(entry) = self.active.lock().unwrap().get_mut(request_id) {
            entry.error = Some(error.to_string());
        }
    }

    pub fn finalize_conversation(&self, request_id: &str) {
        if !self.enabled || request_id.is_empty() {
            return;
        }
        let entry = self.active.lock().unwrap().remove(request_id);
        let Some(entry) = entry else {
            tracing::warn!("No active conversation found for request {}", request_id);
            return;
        };
        let log_entry = json!({
            "timestamp": entry.timestamp,
            "request_id": entry.request_id,
            "approach": entry.approach,
            "model": entry.model,
            "client_request": entry.client_request,
            "provider_calls": entry.provider_calls,
            "final_response": entry.final_response,
            "total_duration_ms": entry.start.elapsed().as_millis() as i64,
            "error": entry.error,
        });
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let path = self.log_dir.join(format!("conversations_{}.jsonl", date));
        let line = py::json::dumps_with(
            &log_entry,
            py::json::DumpOptions {
                separators: py::json::COMPACT_SEPARATORS,
                ..Default::default()
            },
        );
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", line);
            }
            Err(e) => tracing::error!("Failed to write log entry: {}", e),
        }
    }
}
