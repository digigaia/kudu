use std::fs::{DirBuilder, write as write_file};
use std::path::Path;
use std::time::Duration;
use std::{process, thread};

use color_eyre::eyre::{eyre, Result};
use regex::Regex;
use tracing::{debug, info, warn, trace};
use serde_json::Value;

use kudu::config::EOS_FEATURES;
use crate::docker::{Docker, DockerCommand};
use crate::nodeconfig::NodeConfig;
use crate::util::eyre_from_output;


const DEFAULT_BASE_IMAGE: &str = "ubuntu:22.04";
const DEFAULT_HTTP_ADDR: &str = "0.0.0.0:8888";
const CONFIG_PATH: &str = "/app/config.ini";
const TEMP_FOLDER: &str = "/tmp/scratch";


fn unpack_scripts(scripts: &Path) -> Result<()> {
    DirBuilder::new().recursive(true).create(scripts)?;  // make sure dir exists

    write_file(scripts.join("launch_bg.sh"), include_str!("../scripts/launch_bg.sh"))?;
    write_file(scripts.join("build_eos_image.py"), include_str!("../scripts/build_eos_image.py"))?;
    write_file(scripts.join("my_init"), include_str!("../scripts/my_init"))?;
    Ok(())
}

/// A `Dune` instance manages a Docker container in which we can run a `nodeos` instance
/// that will run an EOS blockchain.
///
/// This is nice for contract development. TODO: write more doc here!
pub struct Dune {
    docker: Docker,

    http_addr: String,
}

impl Dune {
    /// the Dune constructor ensures that everything needed is up and running
    /// properly, and getting an instance fully created means we have a running
    /// container
    /// In contrast, the Docker constructor is barebones and doesn't perform additional actions
    pub fn new(container: String, image: String, host_mount: String) -> Result<Dune> {
        // make sure we have a docker image ready in case we need one to build
        // a new container off of it
        let eos_image = duct::cmd!("docker", "images", "-q", &image).read().unwrap();
        if eos_image.is_empty() {
            info!("No appropriate image found, building one before starting container");
            Self::build_image(&image, DEFAULT_BASE_IMAGE)?;
        }

        let docker = Docker::new(container, image, host_mount);
        docker.start(true);

        let mut result = Dune { docker, http_addr: DEFAULT_HTTP_ADDR.to_string() };
        result.sync_config();

        Ok(result)
    }

    pub fn list_running_containers(&self) -> Vec<Value> {
        Docker::list_running_containers()
    }

    pub fn list_all_containers(&self) -> Vec<Value> {
        Docker::list_all_containers()
    }

    pub fn host_to_container_path(&self, path: &str) -> Result<String> {
        self.docker.host_to_container_path(path)
    }

    pub fn build_image(name: &str, base_image: &str) -> Result<()> {
        // first make sure we are able to run pyinfra
        let status = duct::cmd!("which", "pyinfra")
            .stdout_capture()
            .unchecked().run()
            .unwrap()
            .status;

        if !status.success() {
            let msg = concat!(
                "You need to have `pyinfra` installed and available, in an activated ",
                "virtual env or (recommended) through `uv` or `pipx` to be able to build the EOS image"
             );
            return Err(eyre!(msg));
        }

        // unpack script files to a temporary location
        unpack_scripts(&Path::new(TEMP_FOLDER).join("scripts"))?;

        // build image using pyinfra
        debug!("Building EOS image with a {base_image} base");
        const CAPTURE_OUTPUT: bool = false;

        let command = duct::cmd!("pyinfra", "-y",
                                 format!("@docker/{base_image}"),
                                 "scripts/build_eos_image.py");

        let command = if CAPTURE_OUTPUT {
            command.stdout_capture().stderr_capture()
        }
        else {
            command
        }.dir(TEMP_FOLDER);

        let output = command.unchecked().run().unwrap();

        match output.status.success() {
            true => {
                debug!("Image built successfully!");
                let image_id = if CAPTURE_OUTPUT {
                    // we captured the output of the process, parse it to get the image ID
                    let stderr = std::str::from_utf8(&output.stderr)?;
                    let re = Regex::new(r"image ID: ([0-9a-f]+)").unwrap();
                    let m = re.captures(stderr).expect("could not parse image ID from stderr");
                    let image_id = &m[1];
                    image_id.to_string()
                }
                else {
                    // we didn't capture any output, get the image ID from the
                    // latest docker image and hope for the best
                    // FIXME: we should do this properly
                    Docker::list_images()[0]["ID"].as_str().unwrap().to_string()
                };

                info!("Image built successfully with image ID: {:?}", &image_id);
                Docker::docker_command(&["tag", &image_id, name]).run();
                info!("Image tagged as: `{}`", &name);

                Ok(())
            },
            false => {
                Err(eyre_from_output("Error while building image", &output))
            },
        }

        // TODO: remove $TEMP_FOLDER/scripts ?
    }

    // =============================================================================
    //
    //     Command builder methods
    //
    // =============================================================================

    pub fn command(&self, args: &[&str]) -> DockerCommand {
        self.docker.command(args).capture_output(false)
    }

    pub fn color_command(&self, args: &[&str]) -> DockerCommand {
        self.docker.color_command(args).capture_output(false)
    }

    fn cleos_cmd(&self, cmd: &[&str]) -> process::Output {
        trace!("Running cleos command: {:?}", cmd);
        self.unlock_wallet();
        let url = format!("http://{}", self.http_addr);
        let mut cleos_cmd = vec!["cleos", "--verbose", "-u", &url];
        cleos_cmd.extend_from_slice(cmd);
        self.docker.command(&cleos_cmd).run()
    }


    // =============================================================================
    //
    //     Config related methods (handling of `config.ini`, etc.)
    //
    // =============================================================================

    pub fn has_config(&self) -> bool {
        self.docker.file_exists(CONFIG_PATH)
    }

    pub fn rm_config(&self) {
        self.docker.command(&["rm", CONFIG_PATH]).run();
    }

    fn sync_config(&mut self) {
        self.http_addr = self.pull_config().http_addr().to_string();
    }

    /// Push the given `NodeConfig` to the `config.ini` file inside the container
    ///
    /// this also updates the cached `dune.http_addr` value (and others) if necessary
    pub fn push_config(&mut self, config: &NodeConfig) {
        self.docker.write_file(CONFIG_PATH, &config.to_ini());
        self.sync_config()
    }

    /// Pull the config from the `config.ini` file inside the container and return it
    /// as a `NodeConfig`. If it cannot be found, return a default config.
    pub fn pull_config(&self) -> NodeConfig {
        match self.docker.file_exists(CONFIG_PATH) {
            true => NodeConfig::from_ini(&self.docker.read_file(CONFIG_PATH)),
            false => NodeConfig::default(),
        }
    }


    // =============================================================================
    //
    //     Nodeos management methods
    //
    // =============================================================================

    pub fn is_node_running(&self) -> bool {
        self.docker.find_pid("nodeos").is_some()
    }

    pub fn start_node(&mut self, replay_blockchain: bool, clean: bool) {
        if self.is_node_running() {
            info!("Node is already running");
            return;
        }

        if clean {
            self.docker.command(&["rm", "-fr", "/app/datadir"]).run();
            self.docker.command(&["mkdir", "-p", "/app/datadir"]).run();
        }

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
                    thread::sleep(Duration::from_secs(1));
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

    fn wait_blockchain_ready(&self) {
        let url = format!("{}/v1/chain/get_info", self.http_addr);
        let max_wait_time_seconds = 10;
        let mut waited = 0;

        loop {
            let output = self.docker.command(&["curl", "--request", "POST", &url]).check_status(false).run();
            if output.status.success() { break; }
            debug!("blockchain not ready yet, waiting 1 second before retrying");
            thread::sleep(Duration::from_secs(1));
            waited += 1;
            if waited > max_wait_time_seconds {
                warn!("Cannot connect to blockchain to make sure it is ready, tried for {} seconds...",
                      max_wait_time_seconds);
                break;
            }
        }
    }


    // =============================================================================
    //
    //     Wallet management
    //
    // =============================================================================

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
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Already unlocked") {
                // all good, we don't want to fail here
                return;
            }

            command.handle_error(&output);
        }
    }



    // =============================================================================
    //
    //     Blockchain-related methods
    //
    // =============================================================================

    // TODO: check tests/eosio.system_tester.hpp in system-contracts
    // TODO: check https://github.com/AntelopeIO/spring/blob/main/tutorials/bios-boot-tutorial/bios-boot-tutorial.py
    pub fn bootstrap_system(&self) {
        let currency = "SYS";
        let max_value     = "10000000000.0000";
        let initial_value =  "1000000000.0000";

        self.preactivate_features(); // required for boot contract

        // wait a little bit for feature to be activated (one block should be enough?)
        thread::sleep(Duration::from_millis(500));

        info!("Deploying boot contract");
        self.deploy_contract("/app/system_contracts/build/contracts/eosio.boot", "eosio");

        info!("Activating features");
        self.activate_features();

        info!("Creating accounts needed for system contracts");
        self.create_account("eosio.msig", Some("eosio"));
        self.create_account("eosio.token", Some("eosio"));
        self.create_account("eosio.bpay", Some("eosio"));
        self.create_account("eosio.names", Some("eosio"));
        self.create_account("eosio.ram", Some("eosio"));
        self.create_account("eosio.ramfee", Some("eosio"));
        self.create_account("eosio.saving", Some("eosio"));
        self.create_account("eosio.stake", Some("eosio"));
        self.create_account("eosio.vpay", Some("eosio"));
        self.create_account("eosio.rex", Some("eosio"));
        self.create_account("eosio.fees", Some("eosio"));  // added in system-contracts v3.4.0
        self.create_account("eosio.powup", Some("eosio")); // added in system-contracts v3.4.0

        info!("Deploying system contracts");
        self.deploy_contract("/app/system_contracts/build/contracts/eosio.msig", "eosio.msig");
        self.deploy_contract("/app/system_contracts/build/contracts/eosio.token", "eosio.token");
        self.deploy_contract("/app/system_contracts/build/contracts/eosio.system", "eosio");
        self.deploy_contract("/app/eosio.fees", "eosio.fees");

        info!("Setting up `{currency}` token");
        self.setup_token(currency, max_value, initial_value);
    }

    fn setup_token(&self, currency: &str, max_value: &str, initial_value: &str) {
        // Create the currency with a maximum value of max_value tokens
        self.send_action(
            "create",
            "eosio.token",
            &format!(r#"[ "eosio", "{max_value} {currency}" ]"#),
            "eosio.token@active"
        );

        // Issue initial_value tokens (Remaining tokens not in circulation can be
        // considered to be held in reserve.)
        self.send_action(
            "issue",
            "eosio.token",
            &format!(r#"[ "eosio", "{initial_value} {currency}", "memo" ]"#),
            "eosio@active"
        );

        // Initialize the system account with code zero (needed at initialization time)
        // and currency / token with precision 4
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

    fn preactivate_features(&self) {
        let url = format!("{}/v1/producer/schedule_protocol_feature_activations",
                          self.http_addr);
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

    pub fn deploy_contract(&self, container_dir: &str, account: &str) {
        debug!("Deploying `{account}` contract (from: {container_dir})");
        self.cleos_cmd(&["set", "account", "permission", account, "active", "--add-code"]);
        self.cleos_cmd(&["set", "contract", account, container_dir]);
    }

    pub fn cmake_build(&self, container_dir: &str) {
        debug!("Building cmake project in: {container_dir}");
        let build_dir = format!("{container_dir}/build");
        self.docker.command(&["mkdir", "-p", &build_dir]).run();
        self.color_command(&[
            "cmake", "-S", container_dir, "-B", &build_dir,
        ]).run();
        self.color_command(&["cmake", "--build", &build_dir]).run();
    }

    pub fn system_newaccount(&self, account: &str, creator: &str) {
        let (private, public) = self.create_key();
        self.import_key(&private);

        self.cleos_cmd(&[
            "system", "newaccount",
            "--transfer",
            "--stake-net", "1.0000 SYS",
            "--stake-cpu", "1.0000 SYS",
            "--buy-ram-kbytes", "512",
            creator, account, &public,
        ]);
    }
}
