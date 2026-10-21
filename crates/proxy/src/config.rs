//! Server configuration: the same flags, environment variables and defaults
//! as upstream `server.py::parse_args()`.

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use crate::approaches::{KNOWN_APPROACHES, PLUGIN_SLUGS};

const LOG_LEVELS: [&str; 6] = ["notset", "debug", "info", "warning", "error", "critical"];

/// Equivalent of upstream's global `server_config` dict.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub approach: String,
    pub mcts_simulations: i64,
    pub mcts_exploration: f64,
    pub mcts_depth: i64,
    pub best_of_n: i64,
    pub model: String,
    pub rstar_max_depth: i64,
    pub rstar_num_rollouts: i64,
    pub rstar_c: f64,
    pub n: i64,
    pub base_url: String,
    pub optillm_api_key: String,
    pub return_full_response: bool,
    pub host: String,
    pub port: u16,
    pub log: String,
    pub launch_gui: bool,
    pub plugins_dir: String,
    pub log_conversations: bool,
    pub conversation_log_dir: String,
    pub ssl_verify: bool,
    pub ssl_cert_path: String,
    pub batch_mode: bool,
    pub batch_size: usize,
    pub batch_wait_ms: u64,
    /// CePO overrides passed as `--cepo_<field>`.
    pub cepo_overrides: Map<String, Value>,
    pub cepo_config_file: Option<String>,
    /// Seed for the per-request Python-compatible RNG (parity testing).
    pub random_seed: Option<u64>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        // `server_config` before argparse runs (used by library callers and tests).
        Self {
            approach: "none".into(),
            mcts_simulations: 2,
            mcts_exploration: 0.2,
            mcts_depth: 1,
            best_of_n: 3,
            model: "gpt-4o-mini".into(),
            rstar_max_depth: 3,
            rstar_num_rollouts: 5,
            rstar_c: 1.4,
            n: 1,
            base_url: String::new(),
            optillm_api_key: String::new(),
            return_full_response: false,
            host: "127.0.0.1".into(),
            port: 8000,
            log: "info".into(),
            launch_gui: false,
            plugins_dir: String::new(),
            log_conversations: false,
            conversation_log_dir: default_conversation_dir(),
            ssl_verify: true,
            ssl_cert_path: String::new(),
            batch_mode: false,
            batch_size: 4,
            batch_wait_ms: 50,
            cepo_overrides: Map::new(),
            cepo_config_file: None,
            random_seed: None,
        }
    }
}

fn default_conversation_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{}/.optillm/conversations", home)
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn env_bool(name: &str, default: bool) -> bool {
    match env(name) {
        Some(v) => matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"),
        None => default,
    }
}

/// CePO dataclass fields and the Python type argparse converts them with.
pub const CEPO_FIELDS: [(&str, &str); 22] = [
    ("bestofn_n", "int"),
    ("bestofn_temperature", "float"),
    ("bestofn_max_tokens", "int"),
    ("bestofn_rating_type", "str"),
    ("planning_n", "int"),
    ("planning_m", "int"),
    ("planning_temperature_step1", "float"),
    ("planning_temperature_step2", "float"),
    ("planning_temperature_direct_resp", "float"),
    ("planning_temperature_step3", "float"),
    ("planning_temperature_step4", "float"),
    ("planning_max_tokens_step1", "int"),
    ("planning_max_tokens_step2", "int"),
    ("planning_max_tokens_direct_resp", "int"),
    ("planning_max_tokens_step3", "int"),
    ("planning_max_tokens_step4", "int"),
    ("use_plan_diversity", "bool"),
    ("use_reasoning", "bool"),
    ("use_reasoning_fallback", "bool"),
    ("num_of_retries", "int"),
    ("rating_model", "str"),
    ("print_output", "bool"),
];

fn approach_choices() -> Vec<String> {
    KNOWN_APPROACHES
        .iter()
        .chain(PLUGIN_SLUGS.iter())
        .map(|s| s.to_string())
        .collect()
}

/// The argparse definition, as a clap command.
pub fn command() -> Command {
    let s = |v: Option<String>, d: &str| v.unwrap_or_else(|| d.to_string());
    let mut cmd = Command::new("optillm")
        .about("Run LLM inference with various approaches.")
        .version(concat!(env!("CARGO_PKG_VERSION"), " (parity target optillm 0.4.0)"))
        .arg(
            Arg::new("optillm_api_key")
                .long("optillm-api-key")
                .default_value(s(env("OPTILLM_API_KEY"), ""))
                .help("Optional API key for client authentication to optillm"),
        )
        .arg(
            Arg::new("approach")
                .long("approach")
                // argparse does not validate defaults against `choices`, so the
                // default ("auto" / $OPTILLM_APPROACH) is applied after parsing.
                .value_parser(clap::builder::PossibleValuesParser::new(approach_choices()))
                .hide_possible_values(true)
                .help("Inference approach to use"),
        )
        .arg(
            Arg::new("mcts_simulations")
                .long("mcts-simulations")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_SIMULATIONS"), "2"))
                .help("Number of MCTS simulations"),
        )
        .arg(
            Arg::new("mcts_exploration")
                .long("mcts-exploration")
                .value_parser(clap::value_parser!(f64))
                .default_value(s(env("OPTILLM_EXPLORATION"), "0.2"))
                .help("Exploration weight for MCTS"),
        )
        .arg(
            Arg::new("mcts_depth")
                .long("mcts-depth")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_DEPTH"), "1"))
                .help("Simulation depth for MCTS"),
        )
        .arg(
            Arg::new("model")
                .long("model")
                .default_value(s(env("OPTILLM_MODEL"), "gpt-4o-mini"))
                .help("OpenAI model to use"),
        )
        .arg(
            Arg::new("rstar_max_depth")
                .long("rstar-max-depth")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_RSTAR_MAX_DEPTH"), "3"))
                .help("Maximum depth for rStar algorithm"),
        )
        .arg(
            Arg::new("rstar_num_rollouts")
                .long("rstar-num-rollouts")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_RSTAR_NUM_ROLLOUTS"), "5"))
                .help("Number of rollouts for rStar algorithm"),
        )
        .arg(
            Arg::new("rstar_c")
                .long("rstar-c")
                .value_parser(clap::value_parser!(f64))
                .default_value(s(env("OPTILLM_RSTAR_C"), "1.4"))
                .help("Exploration constant for rStar algorithm"),
        )
        .arg(
            Arg::new("n")
                .long("n")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_N"), "1"))
                .help("Number of final responses to be returned"),
        )
        .arg(
            Arg::new("return_full_response")
                .long("return-full-response")
                .action(ArgAction::SetTrue)
                .help("Return the full response including the CoT with <thinking> tags"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .default_value(s(env("OPTILLM_HOST"), "127.0.0.1"))
                .help("Host address to bind the server to (use 0.0.0.0 to allow external connections)"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .value_parser(clap::value_parser!(u16))
                .default_value(s(env("OPTILLM_PORT"), "8000"))
                .help("Specify the port to run the proxy"),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .default_value(s(env("OPTILLM_LOG").map(|v| v.to_lowercase()), "info"))
                .value_parser(|v: &str| -> Result<String, String> {
                    let v = v.to_lowercase();
                    if LOG_LEVELS.contains(&v.as_str()) {
                        Ok(v)
                    } else {
                        Err(format!("invalid choice: '{}' (choose from {})", v, LOG_LEVELS.join(", ")))
                    }
                })
                .help("Specify the logging level"),
        )
        .arg(
            Arg::new("launch_gui")
                .long("launch-gui")
                .action(ArgAction::SetTrue)
                .help("Launch a Gradio chat interface (not supported by optillm-rs)"),
        )
        .arg(
            Arg::new("plugins_dir")
                .long("plugins-dir")
                .default_value(s(env("OPTILLM_PLUGINS_DIR"), ""))
                .help("Path to the plugins directory"),
        )
        .arg(
            Arg::new("log_conversations")
                .long("log-conversations")
                .action(ArgAction::SetTrue)
                .help("Enable conversation logging with full metadata"),
        )
        .arg(
            Arg::new("conversation_log_dir")
                .long("conversation-log-dir")
                .default_value(s(env("OPTILLM_CONVERSATION_LOG_DIR"), &default_conversation_dir()))
                .help("Directory to save conversation logs"),
        )
        .arg(
            Arg::new("best_of_n")
                .long("best-of-n")
                .visible_alias("best_of_n")
                .value_parser(clap::value_parser!(i64))
                .default_value(s(env("OPTILLM_BEST_OF_N"), "3"))
                .help("Number of samples for best_of_n approach"),
        )
        .arg(
            Arg::new("base_url")
                .long("base-url")
                .visible_alias("base_url")
                .default_value(s(env("OPTILLM_BASE_URL"), ""))
                .help("Base url for OpenAI compatible endpoint"),
        )
        .arg(
            Arg::new("ssl_verify")
                .long("ssl-verify")
                .action(ArgAction::SetTrue)
                .help("Enable SSL certificate verification (default: True)"),
        )
        .arg(
            Arg::new("no_ssl_verify")
                .long("no-ssl-verify")
                .action(ArgAction::SetTrue)
                .help("Disable SSL certificate verification"),
        )
        .arg(
            Arg::new("ssl_cert_path")
                .long("ssl-cert-path")
                .default_value(s(env("OPTILLM_SSL_CERT_PATH"), ""))
                .help("Path to custom CA certificate bundle for SSL verification"),
        )
        .arg(
            Arg::new("batch_mode")
                .long("batch-mode")
                .action(ArgAction::SetTrue)
                .help("Enable automatic request batching (fail-fast, no fallback)"),
        )
        .arg(
            Arg::new("batch_size")
                .long("batch-size")
                .value_parser(clap::value_parser!(usize))
                .default_value(s(env("OPTILLM_BATCH_SIZE"), "4"))
                .help("Maximum batch size for request batching"),
        )
        .arg(
            Arg::new("batch_wait_ms")
                .long("batch-wait-ms")
                .value_parser(clap::value_parser!(u64))
                .default_value(s(env("OPTILLM_BATCH_WAIT_MS"), "50"))
                .help("Maximum wait time in milliseconds for batch formation"),
        )
        .arg(
            Arg::new("cepo_config_file")
                .long("cepo_config_file")
                .help("Path to CePO configuration file"),
        )
        .arg(
            Arg::new("random_seed")
                .long("random-seed")
                .value_parser(clap::value_parser!(u64))
                .help("optillm-rs extension: seed the per-request RNG (env OPTILLM_RANDOM_SEED)"),
        );
    for (field, _) in CEPO_FIELDS.iter() {
        let id: &'static str = Box::leak(format!("cepo_{}", field).into_boxed_str());
        cmd = cmd.arg(
            Arg::new(id)
                .long(id)
                .help(format!("CePO configuration for {}", field)),
        );
    }
    cmd
}

impl ServerConfig {
    /// Parse `std::env::args()` the way upstream `parse_args()` does.
    pub fn from_args() -> Self {
        Self::from_matches(&command().get_matches())
    }

    pub fn try_from_iter<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Ok(Self::from_matches(&command().try_get_matches_from(args)?))
    }

    fn from_matches(m: &ArgMatches) -> Self {
        let get_s = |k: &str| m.get_one::<String>(k).cloned().unwrap_or_default();
        // store_true flags: the env var sets the default, the flag forces True.
        let flag = |k: &str, envname: &str| m.get_flag(k) || env_bool(envname, false);
        // --ssl-verify / --no-ssl-verify share dest; the env var sets the default.
        let mut ssl_verify = env_bool("OPTILLM_SSL_VERIFY", true);
        let ssl_on = m.get_flag("ssl_verify");
        let ssl_off = m.get_flag("no_ssl_verify");
        if ssl_on || ssl_off {
            // argparse applies flags left to right; the later one wins.
            let on_idx = m
                .indices_of("ssl_verify")
                .and_then(|mut i| i.next_back())
                .unwrap_or(0);
            let off_idx = m
                .indices_of("no_ssl_verify")
                .and_then(|mut i| i.next_back())
                .unwrap_or(0);
            ssl_verify = if ssl_on && ssl_off {
                on_idx > off_idx
            } else if ssl_off {
                false
            } else {
                // `--ssl-verify` is store_true when the env default is true and
                // store_false when the env default is false.
                env_bool("OPTILLM_SSL_VERIFY", true)
            };
        }
        let mut cepo_overrides = Map::new();
        for (field, ty) in CEPO_FIELDS.iter() {
            if let Some(v) = m.get_one::<String>(&format!("cepo_{}", field)) {
                let value = match *ty {
                    "int" => v.parse::<i64>().map(Value::from).unwrap_or(Value::Null),
                    "float" => crate::py::strings::parse_float(v)
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                    // argparse's type=bool: any non-empty string is True.
                    "bool" => Value::Bool(!v.is_empty()),
                    _ => Value::String(v.clone()),
                };
                cepo_overrides.insert(field.to_string(), value);
            }
        }
        let random_seed = m
            .get_one::<u64>("random_seed")
            .copied()
            .or_else(|| env("OPTILLM_RANDOM_SEED").and_then(|v| v.parse().ok()));
        Self {
            approach: m
                .get_one::<String>("approach")
                .cloned()
                .or_else(|| env("OPTILLM_APPROACH"))
                .unwrap_or_else(|| "auto".into()),
            mcts_simulations: *m.get_one::<i64>("mcts_simulations").unwrap(),
            mcts_exploration: *m.get_one::<f64>("mcts_exploration").unwrap(),
            mcts_depth: *m.get_one::<i64>("mcts_depth").unwrap(),
            best_of_n: *m.get_one::<i64>("best_of_n").unwrap(),
            model: get_s("model"),
            rstar_max_depth: *m.get_one::<i64>("rstar_max_depth").unwrap(),
            rstar_num_rollouts: *m.get_one::<i64>("rstar_num_rollouts").unwrap(),
            rstar_c: *m.get_one::<f64>("rstar_c").unwrap(),
            n: *m.get_one::<i64>("n").unwrap(),
            base_url: get_s("base_url"),
            optillm_api_key: get_s("optillm_api_key"),
            return_full_response: flag("return_full_response", "OPTILLM_RETURN_FULL_RESPONSE"),
            host: get_s("host"),
            port: *m.get_one::<u16>("port").unwrap(),
            log: get_s("log"),
            launch_gui: flag("launch_gui", "OPTILLM_LAUNCH_GUI"),
            plugins_dir: get_s("plugins_dir"),
            log_conversations: flag("log_conversations", "OPTILLM_LOG_CONVERSATIONS"),
            conversation_log_dir: get_s("conversation_log_dir"),
            ssl_verify,
            ssl_cert_path: get_s("ssl_cert_path"),
            batch_mode: m.get_flag("batch_mode")
                || env("OPTILLM_BATCH_MODE")
                    .map(|v| v.to_lowercase() == "true")
                    .unwrap_or(false),
            batch_size: *m.get_one::<usize>("batch_size").unwrap(),
            batch_wait_ms: *m.get_one::<u64>("batch_wait_ms").unwrap(),
            cepo_overrides,
            cepo_config_file: m.get_one::<String>("cepo_config_file").cloned(),
            random_seed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_upstream_argparse() {
        let c = ServerConfig::try_from_iter(["optillm"]).unwrap();
        assert_eq!(c.approach, "auto");
        assert_eq!(c.port, 8000);
        assert_eq!(c.best_of_n, 3);
        assert_eq!(c.mcts_exploration, 0.2);
        assert!(c.ssl_verify);
    }

    #[test]
    fn flags_parse() {
        let c = ServerConfig::try_from_iter([
            "optillm",
            "--approach",
            "moa",
            "--best_of_n",
            "5",
            "--base_url",
            "http://x/v1",
            "--no-ssl-verify",
            "--cepo_planning_n",
            "2",
            "--cepo_use_reasoning",
            "False",
        ])
        .unwrap();
        assert_eq!(c.approach, "moa");
        assert_eq!(c.best_of_n, 5);
        assert_eq!(c.base_url, "http://x/v1");
        assert!(!c.ssl_verify);
        assert_eq!(c.cepo_overrides["planning_n"], 2);
        assert_eq!(c.cepo_overrides["use_reasoning"], true);
        assert!(ServerConfig::try_from_iter(["optillm", "--approach", "auto"]).is_err());
    }
}
