// build.rs

use std::process::Command;
fn main() {
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    let git_rev = option_env!("GIT_REV").map(String::from).unwrap_or_else(|| {
        String::from_utf8(Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap()
    });
    if !git_rev.is_empty() {
        println!(
            "cargo:rustc-env=CARGO_PKG_VERSION={}",
            option_env!("CARGO_PKG_VERSION")
                .map(|v| format!("{}-{}", v, git_rev))
                .unwrap_or_default()
        );
    }
}
