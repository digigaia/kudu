use std::{process, thread, time};

use regex::Regex;
use tracing::{debug, info, warn, error};
use serde_json::Value;

use crate::docker::{Docker, print_streams};
use crate::configini::get_config_ini;

pub struct Dune {
    docker: Docker,
}

const DEFAULT_BASE_IMAGE: &str = "ubuntu:22.04";

impl Dune {
    pub fn new(container: String, image: String) -> Dune {
        // make sure we have a docker image ready in case we need one to build
        // a new container off of it
        let eos_image = duct::cmd!("docker", "images", "-q", &image).read().unwrap();
        if eos_image.is_empty() {
            info!("No appropriate image found, building one before starting container");
            Self::build_image(&image, DEFAULT_BASE_IMAGE);
        }

        let docker = Docker::new(container, image);

        Dune { docker }
    }

    pub fn list_running_containers(&self) -> Vec<Value> {
        Docker::list_running_containers()
    }

    pub fn list_all_containers(&self) -> Vec<Value> {
        Docker::list_all_containers()
    }


    pub fn get_wallet_password(&self) -> String {
        let output = self.docker.execute_cmd(&["cat", "/app/.wallet.pw"]);
        String::from_utf8(output.stdout).unwrap()
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
        let output = duct::cmd!("pyinfra", "-y", format!("@docker/{base_image}"), "python/build_eos_image.py")
            .stdout_capture().stderr_capture().unchecked().run().unwrap();

        match output.status.success() {
            true => {
                let stderr = std::str::from_utf8(&output.stderr).unwrap();
                debug!("Image built successfully!");
                let re = Regex::new(r"image ID: ([0-9a-f]+)").unwrap();
                let m = re.captures(stderr).unwrap();
                let image_id = &m[1];
                info!("Image built successfully with image ID: {:?}", &m[1]);

                Docker::execute_docker_cmd(&["tag", image_id, name]);
                info!("Image tagged as: `{}`", &name);
            },
            false => {
                warn!("Error while building image");
                print_streams(&output);
                process::exit(1);
            },
        }
    }

    pub fn is_node_running(&self) -> bool {
        self.docker.find_pid("nodeos").is_some()
    }

    pub fn stop_node(&self) {
        let max_wait_time_seconds = 30;
        let mut waited = 0;

        match self.docker.find_pid("nodeos") {
            None => { debug!("Trying to stop node but it is not running"); },
            Some(pid) => {
                self.docker.execute_cmd(&["kill", &pid.to_string()]);
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

    pub fn start_node(&self, replay_blockchain: bool) {
        if self.is_node_running() {
            info!("Node is already running");
            return;
        }

        // TODO: check if we restart or not, whether to (over)write config.ini or not
        self.docker.write_file("/app/config.ini", &get_config_ini());

        let mut args = vec!["/app/launch_bg.sh", "nodeos", "--data-dir=/app/datadir"];
        args.push("--config-dir=/app");
        if replay_blockchain {
            args.push("--replay-blockchain");
        }

        let output = self.docker.execute_cmd(&args);

        // print_streams(&output);

        if output.status.success() && self.is_node_running() {
            info!("Node active!");
        }
        else {
            info!("Could not start node");
        }
    }
}
