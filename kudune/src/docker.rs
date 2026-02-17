use std::fs;
use std::io::Write;

use color_eyre::{Result, eyre::{eyre, WrapErr}};
use serde_json::Value;
use tempfile::NamedTempFile;
use tracing::{info, debug, trace, warn};

pub use crate::command::{DockerCommand, DockerCommandJson};

pub struct Docker {
    /// the container name in which we run the docker commands
    container: String,

    /// the image used to build the container if we haven't one already
    image: String,

    /// host folder that we want to bind mount inside the container
    /// Ideally we would make this simply "/" but it seems that creates
    /// issues with recursively mounting the overlay folder inside the container
    /// so you should pick a path that doesn't containe the docker data dir
    host_mount: String,
}

const HOST_MOUNT_PATH: &str = "/host";

impl Docker {
    // the Docker constructor is pretty barebones and doesn't ensure
    // anything is running. You have to call the `start()` method yourself
    // if you need to ensure the container is running
    pub fn new(container: String, image: String, host_mount: String) -> Docker {
        Docker { container, image, host_mount }
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
        Self::docker_command_json(&["images", "--all"]).run()
    }

    pub fn is_running(container: &str) -> bool {
        Docker::list_running_containers().into_iter()
            .any(|c| c["Names"].as_str().unwrap() == container)
    }

    pub fn container_exists(container: &str) -> bool {
        Docker::list_all_containers().into_iter()
            .any(|c| c["Names"].as_str().unwrap() == container)
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
        let src = &self.host_mount;
        let dest = HOST_MOUNT_PATH;
        Self::docker_command(&[
            "run",
            "-p", "127.0.0.1:8888:8888/tcp",
            "-p", "127.0.0.1:9876:9876/tcp",
            // "-p", "127.0.0.1:8080:8080/tcp",
            // "-p", "127.0.0.1:3000:3000/tcp",
            // "-p", "127.0.0.1:8000:8000/tcp",
            "--mount", &format!("type=bind,source={},target={}", src, dest),
            "--detach",
            &format!("--name={}", &self.container),
            &self.image,
            "/sbin/my_init",
        ]).run();
    }

    fn find_container(name: &str) -> Option<Value> {
        Docker::list_all_containers().into_iter()
            .find(|c| c["Names"].as_str().unwrap() == name)
    }

    /// Given a path to a file or dir on the host, return the equivalent path as
    /// seen from within the container.
    pub fn host_to_container_path(&self, path: &str) -> Result<String> {
        let path = fs::canonicalize(path).wrap_err_with(|| {
            format!("Could not get canonical path for: {}", path)
        })?;
        let path = path.to_str().expect("Given path is not valid utf-8!...");
        let path = path.strip_prefix(&self.host_mount).ok_or_else(|| {
            eyre!("Trying to map host path: \"{}\" in container but it is not part of the mount point: \"{}\"", path, self.host_mount)
        })?;

        Ok(format!("{}{}", HOST_MOUNT_PATH, path))
    }

    pub fn stop(container_name: &str) {
        info!("Stopping docker container `{}`...", container_name);
        if !Docker::is_running(container_name) {
            warn!("Container {} is not running", container_name);
            return;
        }
        Docker::docker_command(&["container", "stop", container_name]).run();
    }

    pub fn destroy(container_name: &str) {
        if !Docker::container_exists(container_name) {
            warn!("Container {} does not exist...", container_name);
            return;
        }
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
