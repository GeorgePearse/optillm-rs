//! Port of upstream `optillm/plugins/proxy_plugin.py` and the `proxy/`
//! package: multi-provider load balancing (round_robin / weighted /
//! failover), health checks, model mapping, system-message support probing,
//! and wrapping other approaches or plugins with the proxy client.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::approaches::{self, ApproachResult, Ctx};
use crate::client::{ChatCompletion, LlmClient, LlmError, OpenAiClient, SharedClient};
use crate::config::ServerConfig;
use crate::parse::msg;
use crate::py;

const OPTILLM_PARAMS: [&str; 12] = [
    "optillm_approach",
    "proxy_wrap",
    "wrapped_approach",
    "wrap",
    "mcts_simulations",
    "mcts_exploration",
    "mcts_depth",
    "best_of_n",
    "rstar_max_depth",
    "rstar_num_rollouts",
    "rstar_c",
    "\u{0}",
];

#[rustfmt::skip]
const DEFAULT_CONFIG: &str = concat!(
    "# OptiLLM Proxy Plugin Configuration\n",
    "# \n",
    "# This is an auto-generated configuration file.\n",
    "# Add your LLM provider endpoints and API keys below.\n",
    "# \n",
    "# Environment variables are supported: ${VAR_NAME} or ${VAR_NAME:-default_value}\n",
    "\n",
    "providers:\n",
    "  # Example OpenAI provider (uncomment and configure)\n",
    "  # - name: openai_primary\n",
    "  #   base_url: https://api.openai.com/v1\n",
    "  #   api_key: ${OPENAI_API_KEY}\n",
    "  #   weight: 1\n",
    "\n",
    "routing:\n",
    "  strategy: round_robin  # Options: weighted, round_robin, failover\n",
    "  health_check:\n",
    "    enabled: true\n",
    "    interval: 30  # seconds\n",
    "    timeout: 5    # seconds\n",
    "\n",
    "timeouts:\n",
    "  request: 30     # Maximum time for a request (seconds)\n",
    "  connect: 5      # Maximum time for connection (seconds)\n",
    "\n",
    "queue:\n",
    "  max_concurrent: 100  # Maximum concurrent requests\n",
    "  timeout: 60          # Maximum time in queue (seconds)\n",
    "\n",
    "monitoring:\n",
    "  # log_level: DEBUG  # Overrides the server --log level for proxy logs\n",
    "  track_latency: true\n",
    "  track_errors: true\n",
    "\n",
    "# See proxy/README.md for full documentation\n",
);

// ---------------------------------------------------------------------------
// ProxyConfig
// ---------------------------------------------------------------------------

static CONFIG_CACHE: Mutex<Option<Value>> = Mutex::new(None);

fn interpolate(v: Value) -> Value {
    match v {
        Value::String(s) => {
            let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            Value::String(
                re.replace_all(&s, |c: &regex::Captures| {
                    let expr = &c[1];
                    if let Some((name, default)) = expr.split_once(":-") {
                        std::env::var(py::strings::strip(name))
                            .unwrap_or_else(|_| default.to_string())
                    } else {
                        std::env::var(py::strings::strip(expr)).unwrap_or_else(|_| c[0].to_string())
                    }
                })
                .to_string(),
            )
        }
        Value::Array(a) => Value::Array(a.into_iter().map(interpolate).collect()),
        Value::Object(m) => {
            Value::Object(m.into_iter().map(|(k, v)| (k, interpolate(v))).collect())
        }
        other => other,
    }
}

fn setdefault(m: &mut Map<String, Value>, k: &str, v: Value) {
    if !m.contains_key(k) {
        m.insert(k.into(), v);
    }
}

fn apply_defaults(config: &mut Map<String, Value>) {
    for section in ["providers", "routing", "monitoring", "timeouts", "queue"] {
        setdefault(
            config,
            section,
            if section == "providers" {
                json!([])
            } else {
                json!({})
            },
        );
    }
    if let Some(Value::Object(r)) = config.get_mut("routing") {
        setdefault(r, "strategy", json!("round_robin"));
        setdefault(r, "health_check", json!({}));
        if let Some(Value::Object(h)) = r.get_mut("health_check") {
            setdefault(h, "enabled", json!(true));
            setdefault(h, "interval", json!(30));
            setdefault(h, "timeout", json!(5));
        }
    }
    if let Some(Value::Object(m)) = config.get_mut("monitoring") {
        setdefault(m, "track_latency", json!(true));
        setdefault(m, "track_errors", json!(true));
    }
    if let Some(Value::Object(t)) = config.get_mut("timeouts") {
        setdefault(t, "request", json!(30));
        setdefault(t, "connect", json!(5));
    }
    if let Some(Value::Object(q)) = config.get_mut("queue") {
        setdefault(q, "max_concurrent", json!(100));
        setdefault(q, "timeout", json!(60));
    }
    if let Some(Value::Array(ps)) = config.get_mut("providers") {
        for (i, p) in ps.iter_mut().enumerate() {
            if let Value::Object(p) = p {
                setdefault(p, "name", json!(format!("provider_{}", i)));
                setdefault(p, "weight", json!(1));
                setdefault(p, "fallback_only", json!(false));
                setdefault(p, "model_map", json!({}));
                setdefault(p, "max_concurrent", Value::Null);
            }
        }
    }
}

fn validate(config: &mut Map<String, Value>) -> Result<()> {
    if let Some(Value::Array(ps)) = config.get_mut("providers") {
        for p in ps.iter_mut() {
            let name = p
                .get("name")
                .map(py::json::to_str)
                .unwrap_or_else(|| "unknown".into());
            if p.get("base_url").is_none() {
                return Err(anyhow!("Provider {} missing base_url", name));
            }
            if p.get("api_key").is_none() {
                return Err(anyhow!("Provider {} missing api_key", name));
            }
            if p.get("weight")
                .and_then(|w| w.as_f64())
                .map(|w| w <= 0.0)
                .unwrap_or(false)
            {
                p["weight"] = json!(1);
            }
            let mc = p.get("max_concurrent").cloned().unwrap_or(Value::Null);
            if !mc.is_null() && !(mc.is_i64() && mc.as_i64().unwrap() > 0) {
                p["max_concurrent"] = Value::Null;
            }
        }
    }
    if let Some(Value::Object(r)) = config.get_mut("routing") {
        let s = r.get("strategy").and_then(|v| v.as_str()).unwrap_or("");
        if !["weighted", "round_robin", "failover"].contains(&s) {
            r.insert("strategy".into(), json!("round_robin"));
        }
    }
    Ok(())
}

fn minimal_config() -> Value {
    json!({
        "providers": [],
        "routing": {"strategy": "round_robin", "health_check": {"enabled": false}},
        "timeouts": {"request": 30, "connect": 5},
        "queue": {"max_concurrent": 100, "timeout": 60},
        "monitoring": {"track_latency": false, "track_errors": true}
    })
}

/// `ProxyConfig.load()`.
pub fn load_config() -> Value {
    if let Some(c) = CONFIG_CACHE.lock().unwrap().clone() {
        return c;
    }
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let candidates = [
        home.join(".optillm/proxy_config.yaml"),
        home.join(".optillm/proxy_config.yml"),
    ];
    let path = match candidates.iter().find(|p| p.exists()) {
        Some(p) => p.clone(),
        None => {
            let p = candidates[0].clone();
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&p, DEFAULT_CONFIG);
            p
        }
    };
    let loaded: Result<Value> = (|| {
        let text = std::fs::read_to_string(&path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&text)?;
        let mut config = match serde_json::to_value(yaml)? {
            Value::Null => Map::new(),
            Value::Object(m) => m,
            _ => return Err(anyhow!("Configuration must be a dictionary")),
        };
        config = match interpolate(Value::Object(config)) {
            Value::Object(m) => m,
            _ => unreachable!(),
        };
        apply_defaults(&mut config);
        validate(&mut config)?;
        Ok(Value::Object(config))
    })();
    match loaded {
        Ok(c) => {
            // `if cls._cached_config` - an empty dict is not cached.
            if c.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                *CONFIG_CACHE.lock().unwrap() = Some(c.clone());
            }
            c
        }
        Err(e) => {
            tracing::error!("Failed to load proxy config from {}: {}", path.display(), e);
            minimal_config()
        }
    }
}

// ---------------------------------------------------------------------------
// Providers, routing, health
// ---------------------------------------------------------------------------

struct Provider {
    name: String,
    base_url: String,
    weight: f64,
    fallback_only: bool,
    model_map: Map<String, Value>,
    client: OpenAiClient,
    healthy: Mutex<bool>,
    slots: Option<Arc<tokio::sync::Semaphore>>,
}

impl Provider {
    fn new(cfg: &Value) -> Self {
        let base_url = py::json::to_str(&cfg["base_url"]);
        let api_key = py::json::to_str(&cfg["api_key"]);
        let client = if base_url.to_lowercase().contains("azure") {
            OpenAiClient::with_flavor(
                api_key,
                base_url.clone(),
                crate::client::Flavor::Azure {
                    api_version: "2024-02-01".into(),
                },
                true,
                None,
            )
        } else {
            OpenAiClient::new(api_key, base_url.clone())
        }
        .with_max_retries(0);
        Provider {
            name: py::json::to_str(&cfg["name"]),
            base_url,
            weight: cfg.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0),
            fallback_only: cfg
                .get("fallback_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            model_map: cfg
                .get("model_map")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default(),
            client,
            healthy: Mutex::new(true),
            slots: cfg
                .get("max_concurrent")
                .and_then(|v| v.as_u64())
                .map(|n| Arc::new(tokio::sync::Semaphore::new(n as usize))),
        }
    }

    fn map_model(&self, model: &str) -> String {
        self.model_map
            .get(model)
            .map(py::json::to_str)
            .unwrap_or_else(|| model.to_string())
    }

    fn is_healthy(&self) -> bool {
        *self.healthy.lock().unwrap()
    }
}

/// `list_provider_models(config)` for `/v1/models` with `--approach proxy`.
pub async fn list_provider_models(config: &Value) -> Vec<Value> {
    let mut models: Vec<(String, Value)> = Vec::new();
    for pc in config
        .get("providers")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let p = Provider::new(&pc);
        if let Ok(list) = p.client.list_models().await {
            for m in list
                .get("data")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let id = m.get("id").map(py::json::to_str).unwrap_or_default();
                if !models.iter().any(|(k, _)| *k == id) {
                    models.push((id, m));
                }
            }
        }
        for alias in p.model_map.keys() {
            if !models.iter().any(|(k, _)| k == alias) {
                models.push((
                    alias.clone(),
                    json!({"id": alias, "object": "model", "created": 0, "owned_by": p.name}),
                ));
            }
        }
    }
    models.into_iter().map(|(_, v)| v).collect()
}

struct ProxyClient {
    providers: Vec<Arc<Provider>>,
    strategy: String,
    rr_index: Mutex<usize>,
    fallback: SharedClient,
    request_timeout: f64,
    queue: Arc<tokio::sync::Semaphore>,
    queue_timeout: f64,
    track_errors: bool,
    sys_support: Mutex<HashMap<String, bool>>,
    rng: Mutex<py::random::PyRandom>,
}

impl ProxyClient {
    fn new(config: &Value, fallback: SharedClient) -> Arc<Self> {
        let providers: Vec<Arc<Provider>> = config["providers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|p| Arc::new(Provider::new(p)))
            .collect();
        let hc = &config["routing"]["health_check"];
        let client = Arc::new(ProxyClient {
            providers,
            strategy: config["routing"]["strategy"]
                .as_str()
                .unwrap_or("round_robin")
                .to_string(),
            rr_index: Mutex::new(0),
            fallback,
            request_timeout: config["timeouts"]["request"].as_f64().unwrap_or(30.0),
            queue: Arc::new(tokio::sync::Semaphore::new(
                config["queue"]["max_concurrent"].as_u64().unwrap_or(100) as usize,
            )),
            queue_timeout: config["queue"]["timeout"].as_f64().unwrap_or(60.0),
            track_errors: config["monitoring"]["track_errors"]
                .as_bool()
                .unwrap_or(true),
            sys_support: Mutex::new(HashMap::new()),
            rng: Mutex::new(py::random::PyRandom::from_entropy()),
        });
        if hc.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true) {
            let interval = hc.get("interval").and_then(|v| v.as_f64()).unwrap_or(30.0);
            let providers = client.providers.clone();
            tokio::spawn(async move {
                loop {
                    for p in &providers {
                        let ok = p.client.list_models().await.is_ok();
                        *p.healthy.lock().unwrap() = ok;
                    }
                    tokio::time::sleep(Duration::from_secs_f64(interval)).await;
                }
            });
        }
        client
    }

    fn active(&self) -> Vec<Arc<Provider>> {
        self.providers
            .iter()
            .filter(|p| !p.fallback_only)
            .cloned()
            .collect()
    }

    fn select(&self, available: &[Arc<Provider>]) -> Option<Arc<Provider>> {
        if available.is_empty() {
            return None;
        }
        match self.strategy.as_str() {
            "failover" => available.first().cloned(),
            "weighted" => {
                let total: f64 = available.iter().map(|p| p.weight).sum();
                let mut rng = self.rng.lock().unwrap();
                if total == 0.0 {
                    return rng.choice(available).cloned();
                }
                let r = rng.random() * total;
                let mut cum = 0.0;
                for p in available {
                    cum += p.weight;
                    if r <= cum {
                        return Some(p.clone());
                    }
                }
                available.last().cloned()
            }
            _ => {
                if available.len() == 1 {
                    return available.first().cloned();
                }
                let all = self.active();
                let mut index = self.rr_index.lock().unwrap();
                for _ in 0..all.len() {
                    let current = all[*index % all.len()].clone();
                    *index = (*index + 1) % all.len();
                    if available.iter().any(|p| Arc::ptr_eq(p, &current)) {
                        return Some(current);
                    }
                }
                available.first().cloned()
            }
        }
    }

    async fn supports_system(&self, provider: &Provider, model: &str) -> bool {
        let key = format!("{}:{}", provider.name, model);
        if let Some(v) = self.sys_support.lock().unwrap().get(&key) {
            return *v;
        }
        let result = provider
            .client
            .create(json!({
                "model": model,
                "messages": [msg("system", "test"), msg("user", "hi")],
                "max_tokens": 1,
                "temperature": 0
            }))
            .await;
        let supported = match result {
            Ok(_) => true,
            Err(e) => {
                let m = e.to_string().to_lowercase();
                ![
                    "developer instruction",
                    "system message",
                    "not enabled",
                    "not supported",
                ]
                .iter()
                .any(|p| m.contains(p))
            }
        };
        self.sys_support.lock().unwrap().insert(key, supported);
        supported
    }

    async fn format_messages(
        &self,
        provider: &Provider,
        model: &str,
        messages: Vec<Value>,
    ) -> Vec<Value> {
        if !messages
            .iter()
            .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
        {
            return messages;
        }
        if self.supports_system(provider, model).await {
            return messages;
        }
        let mut out = Vec::new();
        let mut system: Option<Value> = None;
        for m in messages {
            match m.get("role").and_then(|r| r.as_str()) {
                Some("system") => system = Some(m.get("content").cloned().unwrap_or(json!(""))),
                Some("user") => {
                    let truthy =
                        matches!(&system, Some(v) if !(v.is_null() || v.as_str() == Some("")));
                    if truthy {
                        let content = m.get("content").cloned().unwrap_or(json!(""));
                        out.push(json!({
                            "role": "user",
                            "content": format!("Instructions: {}\n\nUser: {}", py::json::to_str(system.as_ref().unwrap()), py::json::to_str(&content))
                        }));
                        system = None;
                    } else {
                        out.push(m);
                    }
                }
                _ => out.push(m),
            }
        }
        out
    }

    fn filter(body: &Value) -> Map<String, Value> {
        body.as_object()
            .map(|o| {
                o.iter()
                    .filter(|(k, _)| !OPTILLM_PARAMS.contains(&k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn create_inner(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError> {
        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();
        let mut healthy: Vec<Arc<Provider>> = self
            .active()
            .into_iter()
            .filter(|p| p.is_healthy())
            .collect();
        if healthy.is_empty() {
            healthy = self
                .providers
                .iter()
                .filter(|p| p.fallback_only)
                .cloned()
                .collect();
        }
        if healthy.is_empty() {
            healthy = self.active();
        }
        let mut attempted: Vec<Arc<Provider>> = Vec::new();
        let mut errors: Vec<(String, String)> = Vec::new();
        loop {
            let available: Vec<Arc<Provider>> = healthy
                .iter()
                .filter(|p| !attempted.iter().any(|a| Arc::ptr_eq(a, p)))
                .cloned()
                .collect();
            if available.is_empty() {
                break;
            }
            let Some(provider) = self.select(&available) else {
                break;
            };
            attempted.push(provider.clone());
            let _slot = match &provider.slots {
                Some(s) => {
                    match tokio::time::timeout(Duration::from_secs(10), s.clone().acquire_owned())
                        .await
                    {
                        Ok(Ok(p)) => Some(p),
                        _ => {
                            errors
                                .push((provider.name.clone(), "At max concurrent requests".into()));
                            continue;
                        }
                    }
                }
                None => None,
            };
            let mut req = Self::filter(&body);
            let mapped = provider.map_model(&model);
            req.insert("model".into(), json!(mapped));
            if let Some(Value::Array(msgs)) = req.get("messages").cloned() {
                let formatted = self.format_messages(&provider, &mapped, msgs).await;
                req.insert("messages".into(), Value::Array(formatted));
            }
            let call = provider.client.create(Value::Object(req));
            match tokio::time::timeout(Duration::from_secs_f64(self.request_timeout), call).await {
                Ok(Ok(r)) => return Ok(r),
                Ok(Err(e)) => {
                    let msg_l = e.message.to_lowercase();
                    if msg_l.contains("timeout") || msg_l.contains("timed out") {
                        errors.push((
                            provider.name.clone(),
                            format!(
                                "Request to {} timed out after {}s",
                                provider.name,
                                py::json::float_repr(self.request_timeout)
                            ),
                        ));
                        if self.track_errors {
                            *provider.healthy.lock().unwrap() = false;
                        }
                        continue;
                    }
                    errors.push((provider.name.clone(), e.message.clone()));
                    if let Some(s) = e.status {
                        if (400..500).contains(&s) && s != 408 && s != 429 {
                            continue;
                        }
                    }
                    if self.track_errors {
                        *provider.healthy.lock().unwrap() = false;
                    }
                }
                Err(_) => {
                    errors.push((
                        provider.name.clone(),
                        format!(
                            "Request to {} timed out after {}s",
                            provider.name,
                            py::json::float_repr(self.request_timeout)
                        ),
                    ));
                    if self.track_errors {
                        *provider.healthy.lock().unwrap() = false;
                    }
                }
            }
        }
        match self
            .fallback
            .create(Value::Object(Self::filter(&body)))
            .await
        {
            Ok(r) => return Ok(r),
            Err(e) => errors.push(("fallback_client".into(), e.message.clone())),
        }
        let rendered: Vec<String> = errors
            .iter()
            .map(|(a, b)| format!("({}, {})", py::json::str_repr(a), py::json::str_repr(b)))
            .collect();
        Err(LlmError {
            status: None,
            message: format!("All providers failed. Errors: [{}]", rendered.join(", ")),
            body: None,
        })
    }
}

struct ProxyHandle(Arc<ProxyClient>);

#[async_trait]
impl LlmClient for ProxyHandle {
    async fn create(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError> {
        let pc = &self.0;
        let _permit = match tokio::time::timeout(
            Duration::from_secs_f64(pc.queue_timeout),
            pc.queue.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(p)) => p,
            _ => {
                return Err(LlmError {
                    status: None,
                    message: format!(
                        "Request queue timeout after {}s - server overloaded",
                        py::json::float_repr(pc.queue_timeout)
                    ),
                    body: None,
                })
            }
        };
        pc.create_inner(body).await
    }

    async fn list_models(&self) -> std::result::Result<Value, LlmError> {
        self.0.fallback.list_models().await
    }

    fn base_url(&self) -> String {
        self.0
            .providers
            .first()
            .map(|p| p.base_url.clone())
            .unwrap_or_default()
    }

    fn api_key(&self) -> String {
        String::new()
    }
}

static CLIENT_CACHE: OnceLock<Mutex<HashMap<String, Arc<ProxyClient>>>> = OnceLock::new();

fn request_messages(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> Vec<Value> {
    match &ctx.messages {
        Some(m) if !m.is_empty() => m.clone(),
        _ => vec![msg("system", system_prompt), msg("user", initial_query)],
    }
}

/// `client.chat.completions.create(model=..., messages=..., **api_config)`.
async fn passthrough(
    client: &dyn LlmClient,
    model: &str,
    messages: Vec<Value>,
    rc: &Map<String, Value>,
) -> Result<Value> {
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("messages".into(), Value::Array(messages));
    for (k, v) in rc {
        if k != "stream" {
            body.insert(k.clone(), v.clone());
        }
    }
    Ok(client
        .create(Value::Object(body))
        .await
        .map_err(|e| anyhow!(e))?
        .into_value())
}

/// `ApproachHandler.handle`: `None` when `name` is not a known approach / plugin.
async fn handle(
    name: &str,
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
    model: &str,
) -> Option<ApproachResult> {
    const APPROACHES: [&str; 13] = [
        "mcts",
        "bon",
        "moa",
        "rto",
        "self_consistency",
        "pvg",
        "z3",
        "rstar",
        "cot_reflection",
        "plansearch",
        "leap",
        "re2",
        "cepo",
    ];
    let is_plugin = approaches::PLUGIN_SLUGS.contains(&name) && name != "proxy";
    if !APPROACHES.contains(&name) && !is_plugin {
        return None;
    }
    let mut sub = ctx.clone();
    sub.model = model.to_string();
    sub.request_id = None;
    // Handlers run with their own function defaults, not the server's settings.
    sub.config = Arc::new(ServerConfig {
        random_seed: ctx.config.random_seed,
        cepo_overrides: ctx.config.cepo_overrides.clone(),
        ..ServerConfig::default()
    });
    let result = match name {
        "rstar" => {
            // RStar(s, c, m, **request_config) rejects the request's own keys.
            let allowed = [
                "max_depth",
                "num_rollouts",
                "c",
                "request_config",
                "request_id",
            ];
            match ctx.request_config.keys().find(|k| {
                !allowed.contains(&k.as_str())
                    && !["model", "messages", "system_prompt", "initial_query"]
                        .contains(&k.as_str())
            }) {
                Some(k) => Err(anyhow!(
                    "RStar.__init__() got an unexpected keyword argument '{}'",
                    k
                )),
                None => {
                    sub.request_config = Map::new();
                    approaches::execute_single_approach("rstar", system_prompt, initial_query, &sub)
                        .await
                }
            }
        }
        "cepo" => Err(anyhow!(
            "cepo() missing 1 required positional argument: 'cepo_config'"
        )),
        "z3" => {
            sub.request_config = Map::new();
            approaches::execute_single_approach("z3", system_prompt, initial_query, &sub).await
        }
        "mcts" => {
            for k in ["mcts_simulations", "mcts_exploration", "mcts_depth"] {
                sub.request_config.shift_remove(k);
            }
            approaches::execute_single_approach("mcts", system_prompt, initial_query, &sub).await
        }
        other => {
            approaches::execute_single_approach(other, system_prompt, initial_query, &sub).await
        }
    };
    Some(result)
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let rc = ctx.request_config.clone();
    let messages = request_messages(system_prompt, initial_query, ctx);
    let main: Result<(Value, i64)> = async {
        let config = load_config();
        let has_providers = config["providers"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if !has_providers {
            return Ok((
                passthrough(ctx.client.as_ref(), &ctx.model, messages.clone(), &rc).await?,
                0,
            ));
        }
        let key = py::json::dumps(&config);
        let pc = {
            let cache = CLIENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            let mut cache = cache.lock().unwrap();
            cache
                .entry(key)
                .or_insert_with(|| ProxyClient::new(&config, ctx.client.clone()))
                .clone()
        };
        let proxy: SharedClient = Arc::new(ProxyHandle(pc));
        let mut pctx = ctx.clone();
        pctx.client = proxy.clone();

        let truthy = |v: Option<&Value>| {
            v.filter(|x| !(x.is_null() || x.as_str() == Some("") || x.as_bool() == Some(false)))
                .cloned()
        };
        let wrapped = truthy(rc.get("proxy_wrap"))
            .or_else(|| truthy(rc.get("wrapped_approach")))
            .or_else(|| truthy(rc.get("wrap")));
        if let Some(w) = &wrapped {
            let name = py::json::to_str(w);
            if let Some(result) =
                handle(&name, system_prompt, initial_query, &pctx, &ctx.model).await
            {
                return result;
            }
        }
        if ctx.model.contains('-') && wrapped.is_none() {
            let (potential, actual) = ctx.model.split_once('-').unwrap();
            if let Some(result) =
                handle(potential, system_prompt, initial_query, &pctx, actual).await
            {
                return result;
            }
        }
        Ok((
            passthrough(proxy.as_ref(), &ctx.model, messages.clone(), &rc).await?,
            0,
        ))
    }
    .await;
    match main {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!("Proxy plugin error: {}", e);
            Ok((
                passthrough(ctx.client.as_ref(), &ctx.model, messages, &rc).await?,
                0,
            ))
        }
    }
}
