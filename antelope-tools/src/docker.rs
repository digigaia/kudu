use std::process::{self, Output};
use duct::cmd;
use serde_json::Value;
use regex::Regex;
use tracing::{debug, error, info, warn};

pub struct Docker {
    /// the container name in which we run the docker commands
    container: String,

    /// the image used to build the container if we haven't one already
    image: String,
}

pub fn print_streams(output: &Output) {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    let stderr = std::str::from_utf8(&output.stderr).unwrap();

    if !stdout.is_empty() {
        debug!("================ STDOUT ================\n{}", stdout);
    }
    if !stderr.is_empty() {
        debug!("================ STDERR ================\n{}", stderr);
    }
    if stdout.is_empty() && stderr.is_empty() {
        debug!("=============== NO OUTPUT ==============");
    }
    debug!("========================================");
}

impl Docker {
    pub fn new(container: String, image: String) -> Docker {
        let docker = Docker { container, image };
        // let output = docker.execute_docker_cmd(&["container", "ls"]);
        for c in docker.list_all_containers() {
            let name = c["Names"].as_str().unwrap();
            if name == docker.container {
                match c["State"].as_str().unwrap() {
                    "running" => {},
                    "exited" => { docker.execute_docker_cmd(&["container", "start", name]); },
                    s => panic!("unknown state for container: {}", s),
                }
                return docker;
            }
        }

        // we didn't find an appropriate container, start one now
        // docker.build_image();

        docker
    }

    // TODO: this should be private
    pub fn build_image(&self) {
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

        let base_image = "ubuntu:22.04";

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

                self.execute_docker_cmd(&["tag", image_id, &self.image]);
                info!("Image tagged as: `{}`", &self.image);
            },
            false => {
                warn!("Error while building image");
                print_streams(&output);
                process::exit(1);
            },
        }
    }

    // TODO: make this a static function?
    // TODO: use as_ref on `args` argument here?
    // or like this: https://docs.rs/duct/latest/duct/fn.cmd.html
    pub fn execute_docker_cmd_json(&self, args: &[&str]) -> Vec<Value> {
        let capture_output = true;

        let mut args = args.to_vec();
        args.push("--format='{{json .}}'");

        let expr = if capture_output {
            cmd("docker", args).stdout_capture().stderr_capture()
        }
        else {
            cmd("docker", args)
        };

        let output = expr.run().unwrap();
        let stdout = std::str::from_utf8(&output.stdout).unwrap();

        stdout.lines()
            // first and last chars are single quotes, remove them before parsing json
            .map(|l| serde_json::from_str(&l[1..l.len()-1]).unwrap())
            .collect()
    }

    pub fn execute_docker_cmd(&self, args: &[&str]) -> Output {
        let capture_output = true;

        let expr = if capture_output {
            cmd("docker", args).stdout_capture().stderr_capture()
        }
        else {
            cmd("docker", args)
        };

        expr.run().unwrap()
    }

    pub fn list_running_containers(&self) -> Vec<Value> {
        self.execute_docker_cmd_json(&["container", "ls"])
    }

    pub fn list_all_containers(&self) -> Vec<Value> {
        self.execute_docker_cmd_json(&["container", "ls", "-a"])
    }
}
