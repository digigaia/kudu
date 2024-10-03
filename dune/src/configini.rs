

const CONFIG_ARGS: &[(&str, &str)] = &[
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

const PLUGINS: &[&str] = &[
    "eosio::chain_api_plugin",
    "eosio::http_plugin",
    "eosio::producer_plugin",
    "eosio::producer_api_plugin"
];


pub fn get_config_ini() -> String {
    let mut lines = vec![];
    for (k, v) in CONFIG_ARGS {
        lines.push(format!("{k} = {v}"));
    }
    lines.push("".to_string());
    for p in PLUGINS {
        lines.push(format!("plugin = {p}"));
    }
    lines.join("\n")
}
