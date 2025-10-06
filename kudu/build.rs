use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn get_git_version() -> Result<String, String> {
    let stdout = Command::new("git").args(["describe", "--tags"])
        .output().map_err(|err| format!("failed to execute `git` process: {err}"))?
        .stdout;
    String::from_utf8(stdout).map_err(|_err| "process returned non-utf8 output".to_string())
}

fn get_version() -> String {
    let cargo_version = env::var("CARGO_PKG_VERSION").unwrap();
    get_git_version().unwrap_or(cargo_version)
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version");
    fs::write(
        &dest_path,
        get_version(),
    ).unwrap();
}
