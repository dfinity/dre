// Construct a useful and specific version string for the CLI
use std::process::Command;

fn main() {
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    let git_rev = option_env!("GIT_REV").map(String::from).unwrap_or_else(|| {
        String::from_utf8(
            // https://stackoverflow.com/questions/21017300/git-command-to-get-head-sha1-with-dirty-suffix-if-workspace-is-not-clean
            Command::new("git").args(["describe", "--always", "--dirty"]).output().unwrap().stdout,
        )
        .unwrap()
    });
    if !git_rev.is_empty() {
        println!(
            "cargo:rustc-env=CARGO_PKG_VERSION={}",
            option_env!("CARGO_PKG_VERSION").map(|v| format!("{}-{}", v, git_rev)).unwrap_or_default()
        );
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path_to_non_public_subnets =
        std::fs::canonicalize(option_env!("NON_PUBLIC_SUBNETS").unwrap_or("../../facts-db/non_public_subnets.csv")).unwrap();

    std::fs::copy(&path_to_non_public_subnets, format!("{}/non_public_subnets.csv", out_dir))
        .unwrap_or_else(|e| panic!("Error with file {}: {:?}", path_to_non_public_subnets.display(), e));

    println!("cargo:rustc-env=DO_NOT_FORMAT=1");
}
