// Construct a useful and specific version string for the CLI
use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use std::{env, fs, process::Command};

include!("cli.rs");

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

    generate_completions()
}

fn generate_completions() {
    let outdir = match env::var_os("OUT_DIR") {
        None => {
            println!("cargo:warning=OUT_DIR var not set, not generating completions");
            return;
        }
        Some(o) => o,
    };
    let completions_dir = PathBuf::from(outdir).join("completions");
    if let Err(e) = fs::create_dir_all(&completions_dir) {
        println!("cargo:warning=Couldn't create '{}' dir: {:?}", completions_dir.display(), e);
        return;
    }

    let mut command = Opts::command();

    for &shell in Shell::value_variants() {
        if let Err(e) = generate_to(shell, &mut command, "dre", &completions_dir) {
            println!("cargo:warning=Couldn't write completions due to: {:?}", e);
            continue;
        };
    }

    if let Some(val) = env::var_os("COMPLETIONS_OUT_DIR") {
        if let Err(e) = fs::create_dir_all(&val) {
            println!("cargo:warning=Couldn't create '{}' due to: {:?}", val.into_string().unwrap(), e);
            return;
        }
        for entry in fs::read_dir(completions_dir).unwrap() {
            let entry = entry.unwrap();
            let val = PathBuf::from(&val).join(entry.file_name());
            if let Err(e) = fs::copy(entry.path(), &val) {
                println!("cargo:warning=Couldn't copy completions to '{}' due to: {:?}", val.display(), e);
            }
        }
    }
}
