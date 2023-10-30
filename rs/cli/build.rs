// build.rs

use std::process::Command;
fn main() {
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    let git_hash = option_env!("GIT_HASH").map(String::from).unwrap_or_else(|| {
        String::from_utf8(Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap()
    });
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
