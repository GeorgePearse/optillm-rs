//! Port of upstream `optillm/plugins/mcp_plugin.py` (mcp 2.x client).
//!
//! Servers come from `~/.optillm/mcp_config.json` (`mcpServers`, transports
//! stdio / sse / streamable_http; websocket is reported as removed). Their
//! tools are offered to the model as `server.tool` functions, each call runs
//! on a fresh connection, and the results go back for one follow-up
//! completion. The MCP client is native (`session.rs`).

mod session;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};

use self::session::{McpError, Session};
use super::pyhelper;
use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

const STREAMABLE_HTTP_TRANSPORTS: [&str; 3] = ["streamable_http", "streamable-http", "http"];
const WEBSOCKET_REMOVED_ERROR: &str = "WebSocket transport is no longer supported (removed in mcp 2.x); use \"streamable_http\" or \"sse\" instead";

#[derive(Clone, Debug)]
struct ServerConfig {
    transport: String,
    command: Option<String>,
    args: Vec<String>,
    url: Option<String>,
    headers: IndexMap<String, String>,
    env: IndexMap<String, String>,
    description: Option<String>,
    timeout: f64,
    sse_read_timeout: f64,
}

/// `ServerConfig.from_dict` (`None` where Python would raise).
fn server_from_dict(c: &Value) -> Option<ServerConfig> {
    let c = c.as_object()?;
    let str_map = |k: &str| -> IndexMap<String, String> {
        c.get(k)
            .and_then(|v| v.as_object())
            .map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.as_str()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| py::json::to_str(v)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let opt_str = |k: &str| c.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    Some(ServerConfig {
        transport: match c.get("transport") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => "stdio".into(),
            Some(v) => py::json::to_str(v),
        },
        command: opt_str("command"),
        args: c
            .get("args")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| py::json::to_str(v))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        url: opt_str("url"),
        headers: str_map("headers"),
        env: str_map("env"),
        description: opt_str("description"),
        timeout: c.get("timeout").and_then(|v| v.as_f64()).unwrap_or(5.0),
        sse_read_timeout: c
            .get("sse_read_timeout")
            .and_then(|v| v.as_f64())
            .unwrap_or(300.0),
    })
}

fn config_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
        .join(".optillm")
        .join("mcp_config.json")
}

/// `MCPConfigManager.load_config`: `None` on any failure.
fn load_config(path: &Path) -> Option<IndexMap<String, ServerConfig>> {
    let text = std::fs::read_to_string(path).ok()?;
    let config: Value = serde_json::from_str(&text).ok()?;
    let config = config.as_object()?;
    if let Some(level) = config.get("log_level") {
        level.as_str()?;
    }
    let mut servers = IndexMap::new();
    match config.get("mcpServers") {
        None => {}
        Some(Value::Object(m)) => {
            for (name, c) in m {
                servers.insert(name.clone(), server_from_dict(c)?);
            }
        }
        Some(_) => return None,
    }
    Some(servers)
}

/// `MCPConfigManager.create_default_config`.
fn create_default_config(path: &Path) {
    if path.exists() {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(
        path,
        py::json::dumps_indent(&json!({"mcpServers": {}, "log_level": "INFO"}), 2),
    );
}

/// `os.access(path, os.X_OK)` for a regular file.
#[cfg(unix)]
fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.is_file()
        && p.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_exec(p: &Path) -> bool {
    p.is_file()
}

/// `find_executable`.
fn find_executable(cmd: &str) -> Option<String> {
    if is_exec(Path::new(cmd)) {
        return Some(cmd.to_string());
    }
    if !cmd.contains('/') {
        for dir in std::env::var("PATH").unwrap_or_default().split(':') {
            let p = Path::new(if dir.is_empty() { "." } else { dir }).join(cmd);
            if is_exec(&p) {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    for dir in [
        "/usr/local/bin".to_string(),
        "/usr/bin".into(),
        "/bin".into(),
        "/opt/homebrew/bin".into(),
        format!("{}/.npm-global/bin", home),
        format!("{}/.nvm/current/bin", home),
    ] {
        let p = Path::new(&dir).join(cmd);
        if is_exec(&p) {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

/// `expand_headers`: `${VAR}` values come from the environment (dropped when unset or empty).
fn expand_headers(headers: &IndexMap<String, String>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (k, v) in headers {
        if v.starts_with("${") && v.ends_with('}') && v.len() >= 3 {
            if let Ok(val) = std::env::var(&v[2..v.len() - 1]) {
                if !val.is_empty() {
                    out.insert(k.clone(), val);
                }
            }
        } else {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

fn merged_env(config: &ServerConfig) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    for (k, v) in &config.env {
        env.insert(k.clone(), v.clone());
    }
    env
}

#[derive(Default)]
struct Discovered {
    connected: bool,
    tools: Vec<Value>,
    resources: Vec<Value>,
    prompts: Vec<Value>,
}

fn capability(caps: &Value, key: &str) -> bool {
    caps.get(key).map(|v| !v.is_null()).unwrap_or(false)
}

fn list_field(result: &Value, key: &str) -> Vec<Value> {
    result
        .get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// `MCPServer.connect_stdio`: initialise and list tools / resources / prompts.
async fn discover(session: &mut Session) -> Result<Discovered> {
    let init = session.initialize().await?;
    let caps = init.get("capabilities").cloned().unwrap_or(json!({}));
    let mut d = Discovered::default();
    for (cap, method, key) in [
        ("tools", "tools/list", "tools"),
        ("resources", "resources/list", "resources"),
        ("prompts", "prompts/list", "prompts"),
    ] {
        if !capability(&caps, cap) {
            continue;
        }
        match session.request(method, None).await {
            Ok(r) => {
                let items = list_field(&r, key);
                match key {
                    "tools" => d.tools = items,
                    "resources" => d.resources = items,
                    _ => d.prompts = items,
                }
            }
            Err(e) if e.downcast_ref::<McpError>().is_some() => {
                tracing::warn!("Failed to list {}: {}", key, e);
            }
            Err(e) => return Err(e),
        }
    }
    d.connected = true;
    Ok(d)
}

async fn open_session(config: &ServerConfig) -> Result<Session> {
    let url = config.url.clone().unwrap_or_default();
    if config.transport == "sse" {
        Session::sse(
            &url,
            expand_headers(&config.headers),
            config.timeout,
            config.sse_read_timeout,
        )
        .await
    } else {
        Session::streamable_http(
            &url,
            expand_headers(&config.headers),
            config.timeout,
            config.sse_read_timeout,
        )
    }
}

/// `MCPServer.connect_and_discover`.
async fn connect_and_discover(name: &str, config: &ServerConfig) -> Discovered {
    let attempt: Result<Discovered> = async {
        let mut session = if config.transport == "stdio" {
            let command = config
                .command
                .as_deref()
                .ok_or_else(|| anyhow!("stdio transport requires command for server {}", name))?;
            let full = find_executable(command)
                .ok_or_else(|| anyhow!("Failed to find executable for command: {}", command))?;
            Session::stdio(&full, &config.args, &merged_env(config))?
        } else if config.transport == "sse"
            || STREAMABLE_HTTP_TRANSPORTS.contains(&config.transport.as_str())
        {
            if config.url.is_none() {
                anyhow::bail!(
                    "{} transport requires URL for server {}",
                    config.transport,
                    name
                );
            }
            open_session(config).await?
        } else if config.transport == "websocket" {
            anyhow::bail!("Server {}: {}", name, WEBSOCKET_REMOVED_ERROR);
        } else {
            anyhow::bail!("Unsupported transport type: {}", config.transport);
        };
        let result = discover(&mut session).await;
        session.close().await;
        result
    }
    .await;
    match attempt {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Error connecting to MCP server {}: {}", name, e);
            Discovered::default()
        }
    }
}

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

/// Python truthiness of an optional string field: `x or default`.
fn or_default<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    match str_field(v, key) {
        Some(s) if !s.is_empty() => s,
        _ => default,
    }
}

fn tools_for_model(servers: &IndexMap<String, (ServerConfig, Discovered)>) -> Vec<Value> {
    let mut tools = Vec::new();
    for (server_name, (_, d)) in servers {
        if !d.connected {
            continue;
        }
        for tool in &d.tools {
            let tool_name = str_field(tool, "name").unwrap_or_default();
            let fallback = format!("Tool {} from server {}", tool_name, server_name);
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": format!("{}.{}", server_name, tool_name),
                    "description": or_default(tool, "description", &fallback),
                    "parameters": tool.get("inputSchema").cloned().unwrap_or(Value::Null)
                }
            }));
        }
    }
    tools
}

/// `MCPServerManager.get_capabilities_description`.
fn capabilities_description(servers: &IndexMap<String, (ServerConfig, Discovered)>) -> String {
    if servers.is_empty() {
        return "No MCP servers available.".into();
    }
    let mut parts = Vec::new();
    for (server_name, (config, d)) in servers {
        if !d.connected {
            parts.push(format!(
                "## {}\nServer connection failed or not established.\n",
                server_name
            ));
            continue;
        }
        let mut s = format!("## {}\n", server_name);
        if let Some(desc) = config.description.as_deref().filter(|d| !d.is_empty()) {
            s += &format!("{}\n\n", desc);
        }
        if !d.tools.is_empty() {
            s += "### Tools\n";
            for t in &d.tools {
                s += &format!(
                    "- {}.{}: {}\n",
                    server_name,
                    str_field(t, "name").unwrap_or_default(),
                    or_default(t, "description", "No description")
                );
            }
            s += "\n";
        }
        if !d.resources.is_empty() {
            s += "### Resources\n";
            for r in &d.resources {
                s += &format!(
                    "- {}: {} - {}\n",
                    str_field(r, "uri").unwrap_or_default(),
                    or_default(r, "name", "No name"),
                    or_default(r, "description", "No description")
                );
            }
            s += "\n";
        }
        if !d.prompts.is_empty() {
            s += "### Prompts\n";
            for p in &d.prompts {
                s += &format!(
                    "- {}: {}\n",
                    str_field(p, "name").unwrap_or_default(),
                    or_default(p, "description", "No description")
                );
            }
            s += "\n";
        }
        parts.push(s);
    }
    parts.join("\n")
}

/// `execute_tool_with_session`.
async fn execute_with_session(session: &mut Session, tool_name: &str, arguments: &Value) -> Value {
    let attempt: Result<Value> = async {
        session.initialize().await?;
        let mut params = json!({"name": tool_name});
        if !arguments.is_null() {
            params["arguments"] = arguments.clone();
        }
        let result = session.request("tools/call", Some(params)).await?;
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !is_error {
            // ClientSession.validate_tool_result: re-list to learn the output schema.
            let listing = session.request("tools/list", None).await?;
            let schema = list_field(&listing, "tools")
                .into_iter()
                .find(|t| str_field(t, "name") == Some(tool_name))
                .and_then(|t| t.get("outputSchema").cloned())
                .filter(|s| !s.is_null());
            if schema.is_some() && result.get("structuredContent").is_none() {
                anyhow::bail!(
                    "Tool {} has an output schema but did not return structured content",
                    tool_name
                );
            }
        }
        let mut content = Vec::new();
        for c in list_field(&result, "content") {
            match str_field(&c, "type") {
                Some("text") => content.push(
                    json!({"type": "text", "text": c.get("text").cloned().unwrap_or(Value::Null)}),
                ),
                Some("image") => content.push(json!({
                    "type": "image",
                    "data": c.get("data").cloned().unwrap_or(Value::Null),
                    "mimeType": c.get("mimeType").cloned().unwrap_or(Value::Null)
                })),
                _ => {}
            }
        }
        Ok(json!({"result": content, "is_error": is_error}))
    }
    .await;
    attempt.unwrap_or_else(|e| json!({"error": format!("Error executing tool: {}", e)}))
}

/// `execute_tool`: a fresh connection per call.
async fn execute_tool(server_name: &str, tool_name: &str, arguments: &Value) -> Value {
    let Some(servers) = load_config(&config_path()) else {
        return json!({"error": "Failed to load MCP configuration"});
    };
    let Some(config) = servers.get(server_name) else {
        return json!({"error": format!("Server {} not found in configuration", server_name)});
    };
    let t = config.transport.as_str();
    let (label, session) = if t == "stdio" {
        let Some(command) = config.command.as_deref() else {
            return json!({"error": "stdio transport requires command"});
        };
        let Some(full) = find_executable(command) else {
            return json!({"error": format!("Failed to find executable for command: {}", command)});
        };
        (
            "stdio",
            Session::stdio(&full, &config.args, &merged_env(config)),
        )
    } else if t == "sse" {
        if config.url.is_none() {
            return json!({"error": "SSE transport requires URL"});
        }
        ("SSE", open_session(config).await)
    } else if STREAMABLE_HTTP_TRANSPORTS.contains(&t) {
        if config.url.is_none() {
            return json!({"error": "Streamable HTTP transport requires URL"});
        }
        ("Streamable HTTP", open_session(config).await)
    } else if t == "websocket" {
        if config.url.is_none() {
            return json!({"error": "WebSocket transport requires URL"});
        }
        return json!({"error": WEBSOCKET_REMOVED_ERROR});
    } else {
        return json!({"error": format!("Unsupported transport type: {}", t)});
    };
    match session {
        Ok(mut s) => {
            let r = execute_with_session(&mut s, tool_name, arguments).await;
            s.close().await;
            r
        }
        Err(e) => json!({"error": format!("Error executing tool via {}: {}", label, e)}),
    }
}

/// `json.loads(arguments)`, with CPython's error text via the helper when parsing fails.
async fn json_loads(arguments: &Value) -> Result<Value> {
    if let Value::String(s) = arguments {
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            return Ok(v);
        }
    }
    match pyhelper::call(json!({"op": "json_loads", "text": arguments}), 60).await {
        Ok(r) => match r.get("error").and_then(|e| e.as_str()) {
            Some(e) => Err(anyhow!(e.to_string())),
            None => Ok(r.get("ok").cloned().unwrap_or(Value::Null)),
        },
        Err(_) => match arguments {
            Value::String(s) => Err(anyhow!(serde_json::from_str::<Value>(s).unwrap_err())),
            _ => Err(anyhow!("the JSON object must be str, bytes or bytearray")),
        },
    }
}

async fn plain(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let r = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", system_prompt), msg("user", initial_query)],
            "temperature": 0.7
        }))
        .await?;
    let content = r
        .message(0)
        .ok_or_else(|| anyhow!("list index out of range"))?
        .get("content")
        .cloned()
        .unwrap_or(Value::Null);
    Ok((content, r.completion_tokens()?))
}

async fn run_inner(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let path = config_path();
    let configs = match load_config(&path) {
        Some(c) => c,
        None => {
            create_default_config(&path);
            match load_config(&path) {
                Some(c) => c,
                None => return plain(system_prompt, initial_query, ctx).await,
            }
        }
    };
    let mut servers: IndexMap<String, (ServerConfig, Discovered)> = IndexMap::new();
    for (name, config) in configs {
        let d = connect_and_discover(&name, &config).await;
        servers.insert(name, (config, d));
    }
    if !servers.values().any(|(_, d)| d.connected) {
        return plain(system_prompt, initial_query, ctx).await;
    }
    let tools = tools_for_model(&servers);
    if tools.is_empty() {
        return plain(system_prompt, initial_query, ctx).await;
    }
    let enhanced = format!(
        "{}\n\nYou have access to the following MCP capabilities:\n\n{}",
        system_prompt,
        capabilities_description(&servers)
    );
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &enhanced), msg("user", initial_query)],
            "tools": tools,
            "temperature": 0.7
        }))
        .await?;
    let message = response
        .message(0)
        .ok_or_else(|| anyhow!("list index out of range"))?;
    let content = message
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    let calls = message
        .get("tool_calls")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    if calls.is_empty() {
        return Ok((json!(content), response.completion_tokens()?));
    }
    let raw_calls = response
        .raw_message(0)
        .and_then(|m| m.get("tool_calls"))
        .cloned()
        .unwrap_or(Value::Array(calls.clone()));
    let mut messages = vec![
        msg("system", &enhanced),
        msg("user", initial_query),
        json!({"role": "assistant", "content": content, "tool_calls": raw_calls}),
    ];
    for call in &calls {
        let id = call.get("id").cloned().unwrap_or(Value::Null);
        let function = call
            .get("function")
            .filter(|f| f.is_object())
            .ok_or_else(|| {
                anyhow!(
                    "'{}' object has no attribute 'function'",
                    "ChatCompletionMessageCustomToolCall"
                )
            })?;
        let full_name = str_field(function, "name").unwrap_or_default().to_string();
        let tool_content = if let Some((server_name, tool_name)) = full_name.split_once('.') {
            match json_loads(function.get("arguments").unwrap_or(&Value::Null)).await {
                Ok(arguments) => {
                    py::json::dumps(&execute_tool(server_name, tool_name, &arguments).await)
                }
                Err(e) => py::json::dumps(&json!({"error": format!("Error: {}", e)})),
            }
        } else {
            py::json::dumps(&json!({"error": format!(
                "Invalid tool name format: {}. Expected format: server_name.tool_name",
                full_name
            )}))
        };
        messages.push(json!({"role": "tool", "tool_call_id": id, "content": tool_content}));
    }
    let final_response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": messages,
            "tools": tools,
            "temperature": 0.7
        }))
        .await?;
    let text = final_response
        .message(0)
        .ok_or_else(|| anyhow!("list index out of range"))?
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    Ok((json!(text), final_response.completion_tokens()?))
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    // Upstream creates its log directory when the plugin module loads.
    if let Some(dir) = config_path().parent() {
        let _ = std::fs::create_dir_all(dir.join("logs"));
    }
    match run_inner(system_prompt, initial_query, ctx).await {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!("Error in MCP plugin run: {}", e);
            plain(system_prompt, initial_query, ctx).await
        }
    }
}
