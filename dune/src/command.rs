use std::process::{self, Output};

use duct::cmd;
use serde_json::Value;
use tracing::error;

use crate::{print_streams, util::join_quote};

#[derive(Debug)]
pub struct DockerCommand {
    args: Vec<String>,
    check_status: bool,
    capture_output: bool,
}


impl DockerCommand {
    // TODO: find a way to accept Vec<_> to (potentially) avoid a copy
    pub fn new(args: &[&str]) -> DockerCommand {
        DockerCommand {
            args: args.iter().map(|x| x.to_string()).collect(),
            check_status: true,
            capture_output: true,
        }
    }

    pub fn check_status(self, check_status: bool) -> Self {
        DockerCommand { check_status, ..self }
    }

    pub fn capture_output(self, capture_output: bool) -> Self {
        DockerCommand { capture_output, ..self }
    }

    pub fn run(&self) -> Output {
        let expr = if self.capture_output {
            cmd("docker", &self.args).stdout_capture().stderr_capture()
        }
        else {
            cmd("docker", &self.args)
        };

        let output = expr.unchecked().run().unwrap();

        if self.check_status {
            self.handle_error(&output);
        }
        output
    }

    pub fn handle_error(&self, output: &Output) {
        if !output.status.success() {
            error!("Error executing docker command:");
            error!("{:?}", self.args);
            error!("pretty -->  {}", &self.pretty_command());
            print_streams!(error, output);
            process::exit(1);
        }
    }

    pub fn args_ref(&self) -> Vec<&str> {
        self.args.iter().map(|x| x.as_str()).collect()
    }

    /// Return a string repr of the command that can be easily copy-pasted.
    /// If it is executed within a container, try to return only that part
    /// (ie: without the "docker container exec" prefix, etc.)
    pub fn pretty_command(&self) -> String {
        let mut args = self.args_ref();
        args.insert(0, "docker");
        match &args[..] {
            &["docker", "container", "exec", ref tail @ ..] => {
                match tail {
                    ["-w", _workdir, _container, rest @ ..] => join_quote(rest),
                    [_container, rest @ ..] => join_quote(rest),
                    _ => join_quote(&args)
                }
            }
            _ => join_quote(&args)
        }
    }
}

pub struct DockerCommandJson {
    command: DockerCommand,
}

impl DockerCommandJson {
    pub fn new(args: &[&str]) -> DockerCommandJson {
        let mut args: Vec<_> = args.iter().map(|x| x.to_string()).collect();
        args.push("--format='{{json .}}'".to_string());

        DockerCommandJson {
            command: DockerCommand {
                args,
                check_status: true,
                capture_output: true,
            }
        }
    }

    pub fn run(&self) -> Vec<Value> {
        let output = self.command.run();
        let stdout = std::str::from_utf8(&output.stdout).unwrap();

        stdout.lines()
            // first and last chars are single quotes, remove them before parsing json
            .map(|l| serde_json::from_str(&l[1..l.len()-1]).unwrap())
            .collect()
    }
}
