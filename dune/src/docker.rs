use std::fs;
use std::io::Write;

use serde_json::Value;
use tempfile::NamedTempFile;
use tracing::{info, debug, trace};

pub use crate::command::{DockerCommand, DockerCommandJson};

pub struct Docker {
    /// the container name in which we run the docker commands
    container: String,

    /// the image used to build the container if we haven't one already
    image: String,
}

const HOST_MOUNT_PATH: &str = "/host";

impl Docker {
    // the Docker constructor is pretty barebones and doesn't ensure
    // anything is running. You have to call the `start()` method yourself
    // if you need to ensure the container is running
    pub fn new(container: String, image: String) -> Docker {
        Docker { container, image }
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

    /// Return a `DockerCommand` builder that you can later run inside
    /// the docker container
    pub fn color_command(&self, args: &[&str]) -> DockerCommand {
        let mut docker_cmd = vec!["container", "exec"];

        docker_cmd.extend_from_slice(&["-t", "-e", "TERM=xterm-256color"]);
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

    pub fn list_images() -> Vec<Value> {
        Self::docker_command_json(&["images"]).run()
    }

    /// Start the docker container if needed. Show log output if `log=true`.
    pub fn start(&self, log: bool) {
        let name = &self.container;

        // check first if a container with the same name already exists
        if let Some(c) = Docker::find_container(name) {
            match c["State"].as_str().unwrap() {
                "running" => {
                    if log { debug!("Container `{}` already running, using it", name); }
                },
                "exited" => {
                    if log { debug!("Container `{}` existing but stopped. Restarting it", name); }
                    Self::docker_command(&["container", "start", name]).run();
                },
                s => panic!("unknown state for container: {}", s),
            }
            return;
        }

        // we didn't find an already existing container,
        // start one from scratch now

        if log { info!("Starting container..."); }
        Self::docker_command(&[
            "run",
            "-p", "127.0.0.1:8888:8888/tcp",
            "-p", "127.0.0.1:9876:9876/tcp",
            // "-p", "127.0.0.1:8080:8080/tcp",
            // "-p", "127.0.0.1:3000:3000/tcp",
            // "-p", "127.0.0.1:8000:8000/tcp",
            "-v", &format!("/:{}", HOST_MOUNT_PATH), "-d",
            &format!("--name={}", &self.container),
            &self.image,
            "/sbin/my_init",
        ]).run();
    }

    fn find_container(name: &str) -> Option<Value> {
        Docker::list_all_containers().into_iter()
            .find(|c| c["Names"].as_str().unwrap() == name)
    }

    pub fn abs_host_path(path: &str) -> String {
        let path = fs::canonicalize(path).expect("Given path does not exist...");
        let path = path.to_str().expect("Given path is not valid utf-8!...");
        format!("{}{}", HOST_MOUNT_PATH, path)
    }

    pub fn stop(container_name: &str) {
        info!("Stopping docker container `{}`...", container_name);
        Docker::docker_command(&["container", "stop", container_name]).run();
    }

    pub fn destroy(container_name: &str) {
        Docker::stop(container_name);
        info!("Destroying docker container `{}`...", container_name);
        Docker::docker_command(&["container", "rm", container_name]).run();
        info!("Docker container `{}` destroyed successfully!", container_name);
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

    // -----------------------------------------------------------------------------
    //     File management methods
    // -----------------------------------------------------------------------------

    pub fn file_exists(&self, filename: &str) -> bool {
        self.command(&["test", "-f", filename])
            .check_status(false).run()
            .status.success()
    }

    pub fn cp_host_to_container(&self, host_file: &str, container_file: &str) {
        trace!("Copy file {} from host to container:{}", host_file, container_file);
        let dest = format!("{}:{}", &self.container, container_file);
        Docker::docker_command(&["cp", host_file, &dest]).run();
    }

    pub fn cp_container_to_host(&self, container_file: &str, host_file: &str) {
        trace!("Copy file {} from container to host:{}", container_file, host_file);
        let src = format!("{}:{}", &self.container, container_file);
        Docker::docker_command(&["cp", &src, host_file]).run();
    }

    pub fn write_file(&self, filename: &str, content: &str) {
        let mut temp_file = NamedTempFile::new().unwrap();
        let _ = temp_file.write(content.as_bytes()).unwrap();

        self.cp_host_to_container(temp_file.path().to_str().unwrap(), filename);
    }

    pub fn read_file(&self, filename: &str) -> String {
        // FIXME: this impl sucks!
        let temp_file = "/tmp/tempfile";
        self.cp_container_to_host(filename, temp_file);
        let result = std::fs::read_to_string(temp_file).unwrap();
        std::fs::remove_file(temp_file).unwrap();
        result
    }
}
