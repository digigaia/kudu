use std::io::Write;

use serde_json::Value;
use tempfile::NamedTempFile;
use tracing::{debug, info};

pub use crate::command::{DockerCommand, DockerCommandJson, from_stream, print_streams};

pub struct Docker {
    /// the container name in which we run the docker commands
    container: String,

    /// the image used to build the container if we haven't one already
    image: String,
}

impl Docker {
    pub fn new(container: String, image: String) -> Docker {
        let docker = Docker { container, image };
        docker.start();
        docker
    }

    /// Return a `DockerCommand` builder that you can later run.
    pub fn docker_command(args: &[&str]) -> DockerCommand {
        DockerCommand::new(args)
    }

    /// Return a `DockerCommandJson` builder that you can later run.
    pub fn docker_command_json(args: &[&str]) -> DockerCommandJson {
        DockerCommandJson::new(args)
    }

    /// Return a `DockerCommand` builder that you can later run inside
    /// the docker container
    pub fn command(&self, args: &[&str]) -> DockerCommand {
        let mut docker_cmd = vec!["container", "exec"];

        docker_cmd.extend_from_slice(&["-w", "/app"]);
        docker_cmd.push(&self.container);
        docker_cmd.extend_from_slice(args);

        Self::docker_command(&docker_cmd)
    }


    pub fn list_running_containers() -> Vec<Value> {
        Self::docker_command_json(&["container", "ls"]).run()
    }

    pub fn list_all_containers() -> Vec<Value> {
        Self::docker_command_json(&["container", "ls", "-a"]).run()
    }

    fn start(&self) {
        for c in Docker::list_all_containers() {
            let name = c["Names"].as_str().unwrap();
            if name == self.container {
                match c["State"].as_str().unwrap() {
                    "running" => {
                        debug!("Container `{}` already running, using it", name);
                    },
                    "exited" => {
                        debug!("Container `{}` existing but stopped. Restarting it", name);
                        Self::docker_command(&["container", "start", name]).run();
                    },
                    s => panic!("unknown state for container: {}", s),
                }
                return;
            }
        }

        // we didn't find an already existing container,
        // start one from scratch now

        info!("Starting container...");
        Self::docker_command(&[
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
        ]).run();
    }

    pub fn stop(&self) {
        info!("Stopping docker container `{}`...'", &self.container);
        Docker::docker_command(&["container", "stop", &self.container]).run();
    }

    pub fn destroy(&self) {
        self.stop();
        info!("Destroying docker container `{}`...'", &self.container);
        Docker::docker_command(&["container", "rm", &self.container]).run();
        info!("Docker container `{}` destroyed successfully!", &self.container);
    }

    /// this is a very crude implementation
    pub fn find_pid(&self, pattern: &str) -> Option<usize> {
        let output = self.command(&["ps", "ax"]).run();
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
        Docker::docker_command(&["cp", host_file, &dest]).run();
    }

    pub fn write_file(&self, filename: &str, content: &str) {
        let mut temp_file = NamedTempFile::new().unwrap();
        let _ = temp_file.write(content.as_bytes()).unwrap();

        self.cp_host_to_container(temp_file.path().to_str().unwrap(), filename);
    }
}
