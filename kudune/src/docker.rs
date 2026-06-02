use std::fs;
use std::io::Write;

use color_eyre::{Result, eyre::{eyre, WrapErr}};
use ratatui::layout::Alignment;
use ratatui::{
    prelude::Modifier,
    widgets::Paragraph,
};
use ratatui_macros::{line, span};
use regex::Regex;
use serde_json::Value;
use tempfile::NamedTempFile;
use tracing::{info, debug, trace, warn};

pub use crate::command::{DockerCommand, DockerCommandJson};
use crate::ratatui::{make_block, make_table, render, terminal_size};

pub struct Docker {
    /// the container name in which we run the docker commands
    pub container: String,

    /// the list of network port mappings, from outward-facing port to inward-facing one
    pub ports: Vec<(u16, u16)>,

    /// the image used to build the container if we haven't one already
    pub image: String,

    /// host folder that we want to bind mount inside the container
    /// Ideally we would make this simply "/" but it seems that creates
    /// issues with recursively mounting the overlay folder inside the container
    /// so you should pick a path that doesn't containe the docker data dir
    pub host_mount: String,
}

const HOST_MOUNT_PATH: &str = "/host";

impl Docker {
    /// the Docker constructor is pretty barebones and doesn't ensure
    /// anything is running. You have to call the `start()` method yourself
    /// if you need to ensure the container is running
    pub fn new(container: String, ports: Vec<(u16, u16)>, image: String, host_mount: String) -> Docker {
        Docker { container, ports, image, host_mount }
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
    /// the given docker container
    pub fn docker_container_command(container: &str, args: &[&str]) -> DockerCommand {
        let mut docker_cmd = vec!["container", "exec"];

        docker_cmd.extend_from_slice(&["-w", "/app"]);
        docker_cmd.push(container);
        docker_cmd.extend_from_slice(args);

        Self::docker_command(&docker_cmd)
    }

    /// Return a `DockerCommand` builder that you can later run inside
    /// the docker container
    pub fn command(&self, args: &[&str]) -> DockerCommand {
        Self::docker_container_command(&self.container, args)
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

    pub fn info(container: &str) -> Result<()> {
        let get_apt_version = |package| -> String {
            let pkg_info = String::from_utf8(
                Self::docker_container_command(container, &["apt-cache", "show", package])
                    .capture_output(true)
                    .run()
                    .stdout
            ).unwrap();
            let version_re = Regex::new(r"Version: (.*)\n").unwrap();
            let caps = version_re.captures(&pkg_info).unwrap();
            caps.get(1).unwrap().as_str().to_string()
        };
        let get_git_version = |folder| -> String {
            String::from_utf8(
                Self::docker_container_command(container, &["git", "-C", folder, "describe", "--tags"])
                    .capture_output(true)
                    .run()
                    .stdout
            ).unwrap().trim().to_string()
        };
        let kudune_version = kudu::config::VERSION;

        let (mut width, _height) = terminal_size();
        width = width.min(100);

        // -----------------------------------------------------------------------------
        //     print kudune version
        // -----------------------------------------------------------------------------

        let kudune_version = render(width, 1, |f| {
            f.render_widget(Paragraph::new(format!("Kudune version: {kudune_version}"))
                            .alignment(Alignment::Center),
                            f.area());
        });

        println!("{}", kudune_version);

        // -----------------------------------------------------------------------------
        //     print vaulta images
        // -----------------------------------------------------------------------------

        let images: Vec<_> = Self::list_images().into_iter()
            .filter(|image| image["Repository"] == "vaulta")
            .collect();

        let images_output = render(width, (images.len() as u16) + 6, |f| {
            let block = make_block("Vaulta images");
            let table = make_table(&images, &["Repository", "Tag", "ID", "CreatedSince", "Size"]);
            f.render_widget(table.block(block), f.area());
        });

        println!("{}", images_output);

        // -----------------------------------------------------------------------------
        //     print vaulta containers
        // -----------------------------------------------------------------------------

        let containers = Self::list_running_containers();
        let containers_output = render(width, (containers.len() as u16) + 6, |f| {
            let block = make_block("Running containers");
            let table = make_table(&containers, &["ID", "Image", "RunningFor", "Ports", "Names"]);
            f.render_widget(table.block(block), f.area());
        });

        println!("{}", containers_output);

        // -----------------------------------------------------------------------------
        //     print version of the components inside the main container
        // -----------------------------------------------------------------------------

        if !Self::is_running(container) {
            // only get info from container if it is running, otherwise exit here
            return Ok(());
        }

        let spring_version = get_apt_version("antelope-spring");
        let cdt_version = get_apt_version("cdt");
        let system_contracts_version = get_git_version("/app/system_contracts/");
        let vaulta_contract_version = get_git_version("/app/vaulta_system_contract/");

        let main_container_output = render(width, 8, |f| {
            // let title = format!("Container: {}", self.docker.container);
            let title = line!["Container: ", span!(Modifier::BOLD; container)];
            let block = make_block(title);
            let paragraph = Paragraph::new(format!(concat!(
                "- Spring version: {}\n",
                "- CDT version: {}\n",
                "- System contracts version: {}\n",
                "- Vaulta contract version: {}\n"
            ),
            spring_version, cdt_version, system_contracts_version, vaulta_contract_version));
            f.render_widget(paragraph.block(block), f.area());
        });

        println!("{}", main_container_output);

        Ok(())
    }

    /// Start the docker container if needed. Show log output if `log=true`.
    pub fn start(&self, log: bool) {
        let name = &self.container;

        // check first if a container with the same name already exists
        if let Some(c) = Docker::find_container(name) {
            match c["State"].as_str().unwrap() {
                "created" => {
                    // FIXME!! do we want this or to fall through out of the match?
                    if log { debug!("Container `{}` created but not running. Starting it", name); }
                    Self::docker_command(&["container", "start", name]).run();
                }
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
        let ports: Vec<_> = self.ports.iter().map(|(pout, pin)| format!("127.0.0.1:{pout}:{pin}/tcp")).collect();
        let mount = format!("type=bind,source={},target={}", src, dest);
        let cname = format!("--name={}", &self.container);

        let mut args = vec!["run"];
        #[allow(clippy::needless_range_loop)]  // we want a & that outlives the for loop
        for i in 0..ports.len() {
            args.push("-p");
            args.push(&ports[i]);
        }
        args.extend_from_slice(&[
            "--mount", &mount,
            "--detach",
            &cname,
            &self.image,
            "/sbin/my_init",
        ]);
        Self::docker_command(&args[..]).run();
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
