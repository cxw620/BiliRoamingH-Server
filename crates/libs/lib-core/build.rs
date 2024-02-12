use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let version = get_git_version();
    File::create(Path::new(&env::var("OUT_DIR")?).join("VERSION"))?
        .write_all(version.trim().as_bytes())?;

    Ok(())
}

fn get_git_version() -> String {
    let main_version = env!("CARGO_PKG_VERSION");
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap())
        .unwrap();
    let commit = Command::new("git")
        .args(["describe", "--always"])
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap())
        .unwrap();
    let release_mode = if cfg!(debug_assertions) || cfg!(test) {
        "DEBUG"
    } else {
        "RELEASE"
    };
    format!("{}-{}-{}-{}", main_version, branch, commit, release_mode).replace("\n", "")
}
