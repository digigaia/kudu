use std::time::Duration;
use std::{process, thread, time};

use regex::Regex;
use tracing::{debug, info, warn, error, trace};
use serde_json::Value;

use antelope_core::config::EOS_FEATURES;
use crate::docker::{Docker, DockerCommand, print_streams, from_stream};
use crate::configini::NodeConfig;


pub struct Dune {
    docker: Docker,

    config: NodeConfig, // FIXME: Option<NodeConfig>?
}

const DEFAULT_BASE_IMAGE: &str = "ubuntu:22.04";

impl Dune {
    /// the Dune constructor ensures that everything needed is up and running
    /// properly, and getting an instance fully created means we have a running
    /// container
    /// In contrast, the Docker constructor is barebones and doesn't perform additional actions
    pub fn new(container: String, image: String) -> Dune {
        // make sure we have a docker image ready in case we need one to build
        // a new container off of it
        let eos_image = duct::cmd!("docker", "images", "-q", &image).read().unwrap();
        if eos_image.is_empty() {
            info!("No appropriate image found, building one before starting container");
            Self::build_image(&image, DEFAULT_BASE_IMAGE);
        }

        let docker = Docker::new(container, image);
        docker.start(true);

        // FIXME: if config exists in container pull it from there
        Dune { docker, config: NodeConfig::default() }
    }

    pub fn list_running_containers(&self) -> Vec<Value> {
        Docker::list_running_containers()
    }

    pub fn list_all_containers(&self) -> Vec<Value> {
        Docker::list_all_containers()
    }


    // TODO: this should be private
    pub fn build_image(name: &str, base_image: &str) {
        // first make sure we are able to run pyinfra
        let status = duct::cmd!("which", "pyinfra")
            .stdout_capture()
            .unchecked().run()
            .unwrap()
            .status;

        if !status.success() {
            error!(concat!("You need to have `pyinfra` installed and available, in an activated ",
                           "virtual env or (recommended) through `pipx` to be able to build the EOS image"));
            process::exit(1);
        }

        debug!("Building EOS image with a {base_image} base");
        let output = duct::cmd!("pyinfra", "-y", format!("@docker/{base_image}"), "scripts/build_eos_image.py")
            .stdout_capture().stderr_capture().unchecked().run().unwrap();

        match output.status.success() {
            true => {
                let stderr = std::str::from_utf8(&output.stderr).unwrap();
                debug!("Image built successfully!");
                let re = Regex::new(r"image ID: ([0-9a-f]+)").unwrap();
                let m = re.captures(stderr).unwrap();
                let image_id = &m[1];
                info!("Image built successfully with image ID: {:?}", &m[1]);

                Docker::docker_command(&["tag", image_id, name]).run();
                info!("Image tagged as: `{}`", &name);
            },
            false => {
                warn!("Error while building image");
                print_streams!(warn, &output);
                process::exit(1);
            },
        }
    }

    pub fn is_node_running(&self) -> bool {
        self.docker.find_pid("nodeos").is_some()
    }

    pub fn start_node(&self, replay_blockchain: bool) {
        if self.is_node_running() {
            info!("Node is already running");
            return;
        }

        // TODO: check if we restart or not, whether to (over)write config.ini or not
        self.docker.write_file("/app/config.ini", &self.config.get_config_ini());

        let mut args = vec!["/app/launch_bg.sh", "nodeos", "--data-dir=/app/datadir"];
        args.push("--config-dir=/app");
        if replay_blockchain {
            args.push("--replay-blockchain");
        }

        info!("Starting nodeos...");
        let output = self.docker.command(&args).run();

        // print_streams(&output);

        if output.status.success() && self.is_node_running() {
            self.wait_blockchain_ready();
            info!("Node active!");
        }
        else {
            info!("Could not start node");
        }
    }

    pub fn stop_node(&self) {
        let max_wait_time_seconds = 30;
        let mut waited = 0;

        match self.docker.find_pid("nodeos") {
            None => { debug!("Trying to stop node but it is not running"); },
            Some(pid) => {
                self.docker.command(&["kill", &pid.to_string()]).run();
                debug!("Waiting for node to shutdown, PID: {pid} (max wait: {max_wait_time_seconds}s)");

                loop {
                    thread::sleep(time::Duration::from_secs(1));
                    if !self.is_node_running() { break; }

                    waited += 1;
                    if waited > max_wait_time_seconds {
                        warn!("Cannot stop node with PID: {}", pid);
                        process::exit(1);
                    }
                }

                info!("Stopped node successfully!");
            }
        }
    }

    pub fn bootstrap_system(&self, full: bool) {
        let currency = "SYS";
        let max_value     = "10000000000.0000";
        let initial_value =  "1000000000.0000";

        self.preactivate_features();

        if full {
            // create account for multisig contract
            self.create_account("eosio.msig", Some("eosio"));
            // create account for token contract
            self.create_account("eosio.token", Some("eosio"));
            // create accounts needed by core contract
            self.create_account("eosio.bpay", Some("eosio"));
            self.create_account("eosio.names", Some("eosio"));
            self.create_account("eosio.ram", Some("eosio"));
            self.create_account("eosio.ramfee", Some("eosio"));
            self.create_account("eosio.saving", Some("eosio"));
            self.create_account("eosio.stake", Some("eosio"));
            self.create_account("eosio.vpay", Some("eosio"));
            self.create_account("eosio.rex", Some("eosio"));
        }

        info!("Deploying boot contract");
        self.deploy_contract("/app/reference_contracts/build/contracts/eosio.boot", "eosio");

        self.activate_features();

        if full {
            info!("Deploying system contracts");
            self.deploy_contract("/app/reference_contracts/build/contracts/eosio.msig", "eosio.msig");
            self.deploy_contract("/app/reference_contracts/build/contracts/eosio.token", "eosio.token");
            self.deploy_contract("/app/reference_contracts/build/contracts/eosio.system", "eosio");

            info!("Setting up `{currency}` token");
            self.setup_token(currency, max_value, initial_value);
        }
    }

    fn setup_token(&self, currency: &str, max_value: &str, initial_value: &str) {
        // Create the currency with a maximum value of max_value tokens
        self.send_action(
            "create",
            "eosio.token",
            &format!(r#"[ "eosio", "{max_value} {currency}" ]"#),
            "eosio.token@active"
        );

        // Issue initial_value tokens (Remaining tokens not in circulation can be considered to be held in reserve.)
        self.send_action(
            "issue",
            "eosio.token",
            &format!(r#"[ "eosio", "{initial_value} {currency}", "memo" ]"#),
            "eosio@active"
        );

        // Initialize the system account with code zero (needed at initialization time) and currency / token with precision 4
        self.send_action(
            "init",
            "eosio",
            &format!(r#"["0", "4,{currency}"]"#),
            "eosio@active"
        );

    }

    /// TODO: use builder pattern like so:
    /// dune.new_account("name").with_creator("eosio").with_pubkey("...").create();
    fn create_account(&self, name: &str, creator: Option<&str>) {
        let (private, public) = self.create_key();
        info!("Creating account `{name}` with public key: {public}");
        let creator = creator.unwrap_or("eosio");
        self.cleos_cmd(&["create", "account", creator, name, &public]);
        self.import_key(&private);
    }

    /// Return a newly created (private, public) keypair
    /// TODO: use antelope types (or a tagged type), to avoid confusion between
    ///       private and public
    fn create_key(&self) -> (String, String) {
        let output = self.cleos_cmd(&["create", "key", "--to-console"]);
        let mut stdout = std::str::from_utf8(&output.stdout).unwrap().lines();
        let private = stdout.next().unwrap().split(": ").nth(1).unwrap().to_string();
        let public = stdout.next().unwrap().split(": ").nth(1).unwrap().to_string();
        (private, public)
    }

    fn import_key(&self, privkey: &str) {
        self.unlock_wallet();
        self.cleos_cmd(&["wallet", "import", "--private-key", privkey]);
    }

    fn wait_blockchain_ready(&self) {
        let url = format!("{}/v1/chain/get_info", self.config.http_addr());

        loop {
            let output = self.docker.command(&["curl", "--request", "POST", &url]).check_status(false).run();
            if output.status.success() { break; }
            debug!("blockchain not ready yet, waiting 1 second before retrying");
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn preactivate_features(&self) {
        let url = format!("{}/v1/producer/schedule_protocol_feature_activations",
                          self.config.http_addr());
        let feature = "0ec7e080177b2c02b278d5088611686b49d739925a92d9bfcacd7fc6b74053bd";
        let data = format!(r#"{{"protocol_features_to_activate": ["{feature}"]}}"#);

        let args = &["curl", "--no-progress-meter", "--request", "POST", &url, "-d", &data];

        debug!("Preactivating features");
        self.docker.command(args).run();
    }

    fn activate_features(&self) {
        for (feature, addr) in EOS_FEATURES.iter() {
            debug!("Activating blockchain feature: {feature}");
            let features = format!(r#"["{addr}"]"#);
            self.send_action("activate", "eosio", &features, "eosio@active");
        }
    }

    fn send_action(&self, action: &str, account: &str, data: &str, permission: &str) {
        self.cleos_cmd(&["push", "action", account, action, data, "-p", permission]);
    }

    pub fn deploy_contract(&self, location: &str, account: &str) {
        debug!("Deploying `{account}` contract (from: {location})");
        self.cleos_cmd(&["set", "account", "permission", account, "active", "--add-code"]);
        self.cleos_cmd(&["set", "contract", account, location]);
    }

    pub fn command(&self, args: &[&str]) -> DockerCommand {
        self.docker.command(args).capture_output(false)
    }

    pub fn color_command(&self, args: &[&str]) -> DockerCommand {
        self.docker.color_command(args).capture_output(false)
    }

    pub fn cmake_build(&self, location: &str) {
        let container_dir = Docker::abs_host_path(location);
        let build_dir = format!("{container_dir}/build");
        self.docker.command(&["mkdir", "-p", &build_dir]).run();
        // TODO: make sure we have colors
        self.color_command(&[
            "cmake", "-S", &container_dir, "-B", &build_dir,
        ]).run();
        self.color_command(&["cmake", "--build", &build_dir]).run();
    }

    fn cleos_cmd(&self, cmd: &[&str]) -> process::Output {
        trace!("Running cleos command: {:?}", cmd);
        self.unlock_wallet();
        let url = format!("http://{}", self.config.http_addr());
        let mut cleos_cmd = vec!["cleos", "--verbose", "-u", &url];
        cleos_cmd.extend_from_slice(cmd);
        self.docker.command(&cleos_cmd).run()
    }

    pub fn get_wallet_password(&self) -> String {
        let output = self.docker.command(&["cat", "/app/.wallet.pw"]).run();
        String::from_utf8(output.stdout).unwrap()
    }

    pub fn unlock_wallet(&self) {
        let command = self.docker.command(&[
            "cleos", "wallet", "unlock", "--password", &self.get_wallet_password()
        ]).check_status(false);

        let output = command.run();

        if !output.status.success() {
            let stderr = from_stream(&output.stderr);
            if stderr.contains("Already unlocked") {
                // all good, we don't want to fail here
                return;
            }

            command.handle_error(&output);
        }
    }

    pub fn system_newaccount(&self, account: &str, creator: &str) {
        let (private, public) = self.create_key();
        self.import_key(&private);

        self.cleos_cmd(&[
            "system", "newaccount",
            "--stake-net", "1.0000 SYS",
            "--stake-cpu", "1.0000 SYS",
            "--buy-ram-kbytes", "512",
            creator, account, &public,
        ]);
    }
}
