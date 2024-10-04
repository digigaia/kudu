use std::collections::HashMap;

const DEFAULT_PARAMS: &[(&str, &str)] = &[
    ("wasm-runtime", "eos-vm-jit"),
    ("abi-serializer-max-time-ms", "15"),
    ("chain-state-db-size-mb", "65536"),
    ("contracts-console", "true"),
    ("http-server-address", "0.0.0.0:8888"),
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
    pub fn get_config_ini(&self) -> String {
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

    pub fn http_addr(&self) -> &str {
        self.params.get("http-server-address")
            .expect("config doesn't contain the `http-server-address` parameter")
    }
}
