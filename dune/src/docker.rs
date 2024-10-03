use std::process::{self, Output};
use std::io::Write;

use duct::cmd;
use serde_json::Value;
use tempfile::NamedTempFile;
use tracing::{debug, error, info};

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
        for c in Docker::list_all_containers() {
            let name = c["Names"].as_str().unwrap();
            if name == docker.container {
                match c["State"].as_str().unwrap() {
                    "running" => {
                        debug!("Container already running, using it");
                    },
                    "exited" => {
                        debug!("Container existing but stopped. Restarting it");
                        Self::execute_docker_cmd(&["container", "start", name]);
                    },
                    s => panic!("unknown state for container: {}", s),
                }
                return docker;
            }
        }

        // we didn't find an appropriate container, start one now

        info!("Starting container");
        docker.start_container();

        docker
    }

    fn start_container(&self) {
        Self::execute_docker_cmd(&[
            "run",
            "-p", "127.0.0.1:8888:8888/tcp",
            "-p", "127.0.0.1:9876:9876/tcp",
            // "-p", "127.0.0.1:8080:8080/tcp",
            // "-p", "127.0.0.1:3000:3000/tcp",
            // "-p", "127.0.0.1:8000:8000/tcp",
            "-v", "/:/host", "-d",
            &format!("--name={}", &self.container),
            &self.image,
            "tail", "-f", "/dev/null",
        ]);
    }


    // TODO: use as_ref on `args` argument here?
    // or like this: https://docs.rs/duct/latest/duct/fn.cmd.html
    pub fn execute_docker_cmd_json(args: &[&str]) -> Vec<Value> {
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

    pub fn execute_docker_cmd(args: &[&str]) -> Output {
        let capture_output = true;

        let expr = if capture_output {
            cmd("docker", args).stdout_capture().stderr_capture()
        }
        else {
            cmd("docker", args)
        };

        let output = expr.unchecked().run().unwrap();
        if !output.status.success() {
            error!("Error executing docker command: {:?}", args);
            print_streams(&output);
            process::exit(1);
        }
        output
    }

    pub fn execute_cmd(&self, args: &[&str]) -> Output {
        let mut docker_cmd = vec!["container", "exec"];

        docker_cmd.extend_from_slice(&["-w", "/app"]);
        docker_cmd.push(&self.container);
        docker_cmd.extend_from_slice(args);

        Self::execute_docker_cmd(&docker_cmd)
    }

    pub fn list_running_containers() -> Vec<Value> {
        Self::execute_docker_cmd_json(&["container", "ls"])
    }

    pub fn list_all_containers() -> Vec<Value> {
        Self::execute_docker_cmd_json(&["container", "ls", "-a"])
    }

    pub fn start(&self) {
        info!("Starting docker container `{}`'", &self.container);
        Docker::execute_docker_cmd(&["container", "start", &self.container]);
    }

    pub fn stop(&self) {
        info!("Stopping docker container `{}`'", &self.container);
        Docker::execute_docker_cmd(&["container", "stop", &self.container]);
    }

    pub fn destroy(&self) {
        self.stop();
        info!("Destroying docker container `{}`'", &self.container);
        Docker::execute_docker_cmd(&["container", "rm", &self.container]);
    }

    /// this is a very crude implementation
    pub fn find_pid(&self, pattern: &str) -> Option<usize> {
        let output = self.execute_cmd(&["ps", "ax"]);
        let stdout = std::str::from_utf8(&output.stdout).unwrap();
        for line in stdout.lines().skip(1) {
            if line.contains(pattern) {
                return Some(line.split_whitespace().next().unwrap().parse().unwrap());
            }
        }
        None
    }

    pub fn cp_host_to_container(&self, host_file: &str, container_file: &str) {
        let dest = format!("{}:{}", &self.container, container_file);
        Docker::execute_docker_cmd(&["cp", host_file, &dest]);
    }

    pub fn write_file(&self, filename: &str, content: &str) {
        let mut temp_file = NamedTempFile::new().unwrap();
        let _ = temp_file.write(content.as_bytes()).unwrap();

        self.cp_host_to_container(temp_file.path().to_str().unwrap(), filename);
    }
}
