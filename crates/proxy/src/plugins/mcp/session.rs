//! A minimal MCP client session (JSON-RPC 2.0) over the three transports
//! upstream's plugin uses: stdio, legacy SSE and Streamable HTTP. It speaks
//! the `initialize` handshake the way the mcp 2.x Python SDK's
//! `ClientSession.initialize()` does.

use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::mpsc;

/// `mcp_types.version.HANDSHAKE_PROTOCOL_VERSIONS` (mcp 2.3.0).
const HANDSHAKE_PROTOCOL_VERSIONS: [&str; 4] =
    ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];
const LATEST_HANDSHAKE_VERSION: &str = "2025-11-25";

/// An error response from the server; displays as `MCPError.__str__` (the message).
#[derive(Debug)]
pub struct McpError {
    pub message: String,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for McpError {}

enum Transport {
    Stdio {
        child: Child,
        stdin: Option<ChildStdin>,
        stdout: Lines<BufReader<ChildStdout>>,
    },
    Http {
        client: reqwest::Client,
        url: String,
        headers: HashMap<String, String>,
        session_id: Option<String>,
        protocol_version: Option<String>,
        read_timeout: Duration,
        pending: Vec<Value>,
    },
    Sse {
        client: reqwest::Client,
        endpoint: String,
        headers: HashMap<String, String>,
        inbox: mpsc::UnboundedReceiver<Result<Value>>,
        reader: tokio::task::JoinHandle<()>,
        read_timeout: Duration,
    },
}

pub struct Session {
    transport: Transport,
    next_id: i64,
    initialized: Option<Value>,
}

/// Parse a `text/event-stream` body incrementally: returns complete
/// `(event, data)` pairs and keeps any partial event in `buf`.
fn drain_events(buf: &mut String) -> Vec<(String, String)> {
    let mut out = Vec::new();
    loop {
        let normalized = buf.replace("\r\n", "\n");
        let Some(end) = normalized.find("\n\n") else {
            *buf = normalized;
            break;
        };
        let block = normalized[..end].to_string();
        *buf = normalized[end + 2..].to_string();
        let mut event = String::from("message");
        let mut data: Vec<String> = Vec::new();
        for line in block.lines() {
            if line.starts_with(':') {
                continue;
            }
            let (field, value) = match line.split_once(':') {
                Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
                None => (line, ""),
            };
            match field {
                "event" => event = value.to_string(),
                "data" => data.push(value.to_string()),
                _ => {}
            }
        }
        if !data.is_empty() {
            out.push((event, data.join("\n")));
        }
    }
    out
}

fn header_map(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (k, v) in headers {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            map.insert(name, value);
        }
    }
    map
}

impl Session {
    /// Spawn `command args` with `env` and talk newline-delimited JSON over its stdio.
    pub fn stdio(command: &str, args: &[String], env: &HashMap<String, String>) -> Result<Self> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .env_clear()
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child.stdin.take();
        let stdout =
            BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?).lines();
        Ok(Session {
            transport: Transport::Stdio {
                child,
                stdin,
                stdout,
            },
            next_id: 0,
            initialized: None,
        })
    }

    pub fn streamable_http(
        url: &str,
        headers: HashMap<String, String>,
        timeout: f64,
        read_timeout: f64,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs_f64(timeout.max(0.001)))
            .build()?;
        Ok(Session {
            transport: Transport::Http {
                client,
                url: url.to_string(),
                headers,
                session_id: None,
                protocol_version: None,
                read_timeout: Duration::from_secs_f64(read_timeout.max(0.001)),
                pending: Vec::new(),
            },
            next_id: 0,
            initialized: None,
        })
    }

    /// Open the legacy SSE stream and wait for its `endpoint` event.
    pub async fn sse(
        url: &str,
        headers: HashMap<String, String>,
        timeout: f64,
        read_timeout: f64,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs_f64(timeout.max(0.001)))
            .build()?;
        let resp = tokio::time::timeout(
            Duration::from_secs_f64(timeout.max(0.001)),
            client
                .get(url)
                .headers(header_map(&headers))
                .header("accept", "text/event-stream")
                .send(),
        )
        .await
        .map_err(|_| anyhow!("timed out connecting to {}", url))??
        .error_for_status()?;
        let (tx, rx) = mpsc::unbounded_channel::<Result<Value>>();
        let (etx, erx) = tokio::sync::oneshot::channel::<String>();
        let reader = tokio::spawn(async move {
            let mut etx = Some(etx);
            let mut stream = resp.bytes_stream();
            let mut buf = String::new();
            while let Some(chunk) = stream.next().await {
                let Ok(chunk) = chunk else { break };
                buf.push_str(&String::from_utf8_lossy(&chunk));
                for (event, data) in drain_events(&mut buf) {
                    if event == "endpoint" {
                        if let Some(e) = etx.take() {
                            let _ = e.send(data);
                        }
                    } else if event == "message" {
                        let _ = tx.send(serde_json::from_str(&data).map_err(|e| anyhow!(e)));
                    }
                }
            }
        });
        let endpoint = tokio::time::timeout(Duration::from_secs_f64(timeout.max(0.001)), erx)
            .await
            .map_err(|_| anyhow!("timed out waiting for the SSE endpoint event"))?
            .map_err(|_| anyhow!("SSE stream closed before the endpoint event"))?;
        let base = url::Url::parse(url)?;
        let joined = base.join(&endpoint)?;
        if joined.origin() != base.origin() {
            bail!(
                "Endpoint origin does not match connection origin: {}",
                joined
            );
        }
        Ok(Session {
            transport: Transport::Sse {
                client,
                endpoint: joined.to_string(),
                headers,
                inbox: rx,
                reader,
                read_timeout: Duration::from_secs_f64(read_timeout.max(0.001)),
            },
            next_id: 0,
            initialized: None,
        })
    }

    async fn send(&mut self, message: &Value) -> Result<()> {
        match &mut self.transport {
            Transport::Stdio { stdin, .. } => {
                let stdin = stdin.as_mut().ok_or_else(|| anyhow!("stdin closed"))?;
                let mut line = serde_json::to_string(message)?;
                line.push('\n');
                stdin.write_all(line.as_bytes()).await?;
                stdin.flush().await?;
            }
            Transport::Http {
                client,
                url,
                headers,
                session_id,
                protocol_version,
                read_timeout,
                pending,
            } => {
                let mut req = client
                    .post(url.as_str())
                    .headers(header_map(headers))
                    .header("accept", "application/json, text/event-stream")
                    .header("content-type", "application/json")
                    .json(message);
                if let Some(id) = session_id {
                    req = req.header("mcp-session-id", id.as_str());
                }
                if let Some(v) = protocol_version {
                    req = req.header("mcp-protocol-version", v.as_str());
                }
                let resp = tokio::time::timeout(*read_timeout, req.send())
                    .await
                    .map_err(|_| anyhow!("timed out"))??;
                if let Some(id) = resp.headers().get("mcp-session-id") {
                    *session_id = id.to_str().ok().map(|s| s.to_string());
                }
                if resp.status().as_u16() == 202 {
                    return Ok(());
                }
                if resp.status().as_u16() == 404 && session_id.is_some() {
                    bail!("Session terminated");
                }
                let resp = resp.error_for_status()?;
                let ctype = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();
                if ctype.starts_with("text/event-stream") {
                    // Read events until the response to this request arrives.
                    let want = message.get("id").cloned();
                    let mut stream = resp.bytes_stream();
                    let mut buf = String::new();
                    'outer: loop {
                        let chunk = tokio::time::timeout(*read_timeout, stream.next())
                            .await
                            .map_err(|_| anyhow!("timed out"))?;
                        let Some(chunk) = chunk else { break };
                        buf.push_str(&String::from_utf8_lossy(&chunk?));
                        for (event, data) in drain_events(&mut buf) {
                            if event != "message" || data.is_empty() {
                                continue;
                            }
                            let v: Value = serde_json::from_str(&data)?;
                            let done = want.is_some()
                                && v.get("id") == want.as_ref()
                                && (v.get("result").is_some() || v.get("error").is_some());
                            pending.push(v);
                            if done {
                                break 'outer;
                            }
                        }
                    }
                } else if ctype.starts_with("application/json") {
                    match resp.json::<Value>().await? {
                        Value::Array(items) => pending.extend(items),
                        v => pending.push(v),
                    }
                } else {
                    bail!("Unexpected content type: {}", ctype);
                }
            }
            Transport::Sse {
                client,
                endpoint,
                headers,
                read_timeout,
                ..
            } => {
                tokio::time::timeout(
                    *read_timeout,
                    client
                        .post(endpoint.as_str())
                        .headers(header_map(headers))
                        .json(message)
                        .send(),
                )
                .await
                .map_err(|_| anyhow!("timed out"))??
                .error_for_status()?;
            }
        }
        Ok(())
    }

    async fn receive(&mut self) -> Result<Value> {
        match &mut self.transport {
            Transport::Stdio { stdout, .. } => loop {
                let line = stdout
                    .next_line()
                    .await?
                    .ok_or_else(|| anyhow!("Connection closed"))?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str(&line) {
                    Ok(v) => return Ok(v),
                    // The SDK logs and skips lines that are not JSON-RPC.
                    Err(_) => continue,
                }
            },
            Transport::Http { pending, .. } => {
                if pending.is_empty() {
                    bail!("Connection closed");
                }
                Ok(pending.remove(0))
            }
            Transport::Sse {
                inbox,
                read_timeout,
                ..
            } => tokio::time::timeout(*read_timeout, inbox.recv())
                .await
                .map_err(|_| anyhow!("timed out"))?
                .ok_or_else(|| anyhow!("Connection closed"))?,
        }
    }

    /// Send a request and wait for its result, answering server pings on the way.
    pub async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut msg = json!({"jsonrpc": "2.0", "id": id, "method": method});
        if let Some(p) = params {
            msg["params"] = p;
        }
        self.send(&msg).await?;
        loop {
            let v = self.receive().await?;
            if v.get("method").is_some() {
                if let Some(rid) = v.get("id").cloned() {
                    let reply = if v["method"] == "ping" {
                        json!({"jsonrpc": "2.0", "id": rid, "result": {}})
                    } else {
                        json!({"jsonrpc": "2.0", "id": rid, "error": {"code": -32601, "message": "Method not found"}})
                    };
                    self.send(&reply).await?;
                }
                continue;
            }
            if v.get("id").and_then(|i| i.as_i64()) != Some(id) {
                continue;
            }
            if let Some(err) = v.get("error") {
                return Err(McpError {
                    message: err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or_default()
                        .to_string(),
                }
                .into());
            }
            return Ok(v.get("result").cloned().unwrap_or(json!({})));
        }
    }

    pub async fn notify(&mut self, method: &str) -> Result<()> {
        self.send(&json!({"jsonrpc": "2.0", "method": method}))
            .await
    }

    /// `ClientSession.initialize()`: cached after the first call.
    pub async fn initialize(&mut self) -> Result<Value> {
        if let Some(r) = &self.initialized {
            return Ok(r.clone());
        }
        let result = self
            .request(
                "initialize",
                Some(json!({
                    "protocolVersion": LATEST_HANDSHAKE_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "mcp", "version": "0.1.0"}
                })),
            )
            .await?;
        let version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !HANDSHAKE_PROTOCOL_VERSIONS.contains(&version.as_str()) {
            bail!("Unsupported protocol version from the server: {}", version);
        }
        if let Transport::Http {
            protocol_version, ..
        } = &mut self.transport
        {
            *protocol_version = Some(version);
        }
        self.notify("notifications/initialized").await?;
        self.initialized = Some(result.clone());
        Ok(result)
    }

    /// Close the connection the way the SDK's context managers do.
    pub async fn close(mut self) {
        match &mut self.transport {
            Transport::Stdio { child, stdin, .. } => {
                stdin.take();
                if tokio::time::timeout(Duration::from_secs(2), child.wait())
                    .await
                    .is_err()
                {
                    let _ = child.kill().await;
                }
            }
            Transport::Http {
                client,
                url,
                headers,
                session_id: Some(id),
                ..
            } => {
                let _ = client
                    .delete(url.as_str())
                    .headers(header_map(headers))
                    .header("mcp-session-id", id.as_str())
                    .send()
                    .await;
            }
            Transport::Http { .. } => {}
            Transport::Sse { reader, .. } => reader.abort(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_events_split_and_join_data_lines() {
        let mut buf = String::from("event: endpoint\r\ndata: /messages?s=1\r\n\r\ndata: {\"a\":\ndata: 1}\n\n: ping\n\nevent: mess");
        let ev = drain_events(&mut buf);
        assert_eq!(
            ev,
            vec![
                ("endpoint".to_string(), "/messages?s=1".to_string()),
                ("message".to_string(), "{\"a\":\n1}".to_string())
            ]
        );
        assert_eq!(buf, "event: mess");
    }
}
