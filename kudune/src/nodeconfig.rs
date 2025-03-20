use std::collections::HashMap;
use std::process;

use tracing::error;

const DEFAULT_HTTP_ADDR: &str = "0.0.0.0:8888";

const DEFAULT_PARAMS: &[(&str, &str)] = &[
    ("wasm-runtime", "eos-vm-jit"),
    ("abi-serializer-max-time-ms", "15"),
    ("chain-state-db-size-mb", "65536"),
    ("contracts-console", "true"),
    ("http-server-address", DEFAULT_HTTP_ADDR),
    ("p2p-listen-endpoint", "0.0.0.0:9876"),
    ("state-history-endpoint", "0.0.0.0:8080"),
    ("verbose-http-errors", "true"),
    ("agent-name", "EOS Test Node"),
    ("net-threads", "2"),
    ("max-transaction-time", "100"),
    ("producer-name", "eosio"),
    ("enable-stale-production", "true"),
    ("resource-monitor-not-shutdown-on-threshold-exceeded", "true"),
    ("http-validate-host", "false"),
    ("read-only-read-window-time-us", "120000"),
];

const DEFAULT_PLUGINS: &[&str] = &[
    "eosio::chain_api_plugin",
    "eosio::http_plugin",
    "eosio::producer_plugin",
    "eosio::producer_api_plugin"
];

#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub params: HashMap<String, String>,
    pub plugins: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            params: DEFAULT_PARAMS.iter()
                .map(|(k,v)| (k.to_string(), v.to_string()))
                .collect(),
            plugins: DEFAULT_PLUGINS.iter()
                .map(|p| p.to_string())
                .collect(),
        }
    }
}

impl NodeConfig {
    pub fn new() -> Self {
        NodeConfig { params: HashMap::new(), plugins: Vec::new() }
    }

    pub fn to_ini(&self) -> String {
        let mut lines = vec![];
        for (k, v) in self.params.iter() {
            lines.push(format!("{k} = {v}"));
        }
        lines.push("".to_string());
        for p in self.plugins.iter() {
            lines.push(format!("plugin = {p}"));
        }
        lines.join("\n")
    }

    pub fn from_ini(ini: &str) -> NodeConfig {
        let mut result = NodeConfig::new();
        for line in ini.lines() {
            if line.starts_with('#') { continue; }  // comment
            if line.trim().is_empty()   { continue; }  // blank line
            result.add_param(line).unwrap_or_else(|msg| {
                error!("{}", msg);
                process::exit(1);
            });
        }
        result
    }

    /// Add a parameter given a line representing `<key> = <value>`
    pub fn add_param(&mut self, param: &str) -> Result<(), String> {
        let kv: Vec<_> = param.split('=').collect();
        if kv.len() != 2 {
            return Err(format!("following line cannot be parsed as '<key> = <value>'\n{}", param));
        }
        let (k, v) = (kv[0].trim(), kv[1].trim());
        if k == "plugin" {
            self.plugins.push(v.to_string());
        }
        else {
            self.params.insert(k.to_string(), v.to_string());
        }
        Ok(())
    }

    pub fn http_addr(&self) -> &str {
        self.params.get("http-server-address").map_or(DEFAULT_HTTP_ADDR, |x| x)
    }
}
