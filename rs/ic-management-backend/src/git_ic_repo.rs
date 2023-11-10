use custom_error::custom_error;
use fs2::FileExt;
use log::info;
use std::fs::{metadata, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, fs::File};

const DEFAULT_DEPTH: usize = 10000;

custom_error! {IoError
    Io {
        source: std::io::Error,
        path: PathBuf
    } = @{format!("{path}: {source}", source=source, path=path.display())},
}

// Define the IcRepo struct
pub struct IcRepo {
    repo_path: PathBuf,
    cache: HashMap<String, Vec<String>>,
    lock_file_path: String,
}

impl IcRepo {
    // Initialize the IcRepo struct with optional depth for shallow clone
    pub fn new(depth: Option<usize>) -> anyhow::Result<Self> {
        let repo_path = std::env::var("HOME").unwrap_or(".".to_string()) + "/.cache/git/ic";
        let lock_file_path = format!("{}.lock", &repo_path);
        info!("IC git repo path: {}, lock file path: {}", &repo_path, &lock_file_path);

        let repo_path = PathBuf::from(repo_path);
        if !repo_path.exists() {
            std::fs::create_dir_all(&repo_path).map_err(|e| IoError::Io {
                source: e,
                path: repo_path.to_path_buf(),
            })?;
        }

        let lock_file = File::create(&lock_file_path).map_err(|e| IoError::Io {
            source: e,
            path: PathBuf::from(&lock_file_path),
        })?;
        lock_file.lock_exclusive()?;

        let repo = Self {
            repo_path: repo_path.clone(),
            cache: HashMap::new(),
            lock_file_path,
        };

        if repo_path.exists() {
            // If the directory exists, but git status does not return success, remove the
            // directory
            if !match Command::new("git")
                .args(["-C", repo_path.to_str().unwrap(), "status"])
                .output()
            {
                Ok(output) => output.status.success(),
                Err(_) => false,
            } {
                std::fs::remove_dir_all(&repo_path).map_err(|e| IoError::Io {
                    source: e,
                    path: repo_path.to_path_buf(),
                })?;
            }
        }

        if repo_path.exists() {
            info!(
                "Repo {} already exists, fetching new updates",
                &repo_path.to_str().unwrap()
            );
            repo.refetch(depth.unwrap_or(DEFAULT_DEPTH))?;
        } else {
            info!("Repo {} does not exist, cloning", &repo_path.to_str().unwrap());
            Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    &depth.unwrap_or(DEFAULT_DEPTH).to_string(),
                    "https://github.com/dfinity/ic",
                    repo_path.to_str().unwrap(),
                ])
                .status()?;
        }

        lock_file.unlock()?;

        Ok(repo)
    }

    fn refetch(&self, depth: usize) -> anyhow::Result<()> {
        let meta = metadata(&self.lock_file_path)?;
        let last_modified = meta.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        if now - last_modified >= 60 {
            Command::new("git")
                .args([
                    "-C",
                    self.repo_path.to_str().unwrap(),
                    "fetch",
                    "origin",
                    "master",
                    "--depth",
                    &depth.to_string(),
                ])
                .output()?;

            // Touch the file to update its last modified time
            OpenOptions::new()
                .write(true)
                .open(&self.lock_file_path)?
                .write_all(b"")?;
        }
        Ok(())
    }

    pub fn get_branches_with_commit(&mut self, commit_sha: &str) -> anyhow::Result<Vec<String>> {
        let branches = match self.cache.get(commit_sha) {
            Some(branches) => branches.clone(),
            None => {
                self.refetch(DEFAULT_DEPTH)?;
                let output = Command::new("git")
                    .args([
                        "-C",
                        self.repo_path.to_str().unwrap(),
                        "branch",
                        "--remote",
                        "--color=never",
                        "--format=%(refname)",
                        "--contains",
                        commit_sha,
                    ])
                    .output()?;

                let branches: Vec<String> = String::from_utf8(output.stdout)?
                    .lines()
                    .map(|s| s.trim().trim_start_matches("refs/remotes/origin/").to_string())
                    .collect();

                self.cache.insert(commit_sha.to_string(), branches.clone());
                branches
            }
        };
        Ok(branches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_repo() {
        let ic_repo = IcRepo::new(Some(50));
        assert!(ic_repo.is_ok());
    }

    #[test]
    fn test_get_branches_with_commit() {
        let mut ic_repo = IcRepo::new(Some(50)).unwrap();
        let branches = ic_repo.get_branches_with_commit("some_commit_sha");
        assert!(branches.is_ok());
    }

    #[test]
    fn test_get_branches_with_nonexistent_commit() {
        let mut ic_repo = IcRepo::new(Some(50)).unwrap();
        let branches = ic_repo.get_branches_with_commit("nonexistent_commit_sha");
        assert!(branches.is_ok());
        assert!(branches.unwrap().is_empty());
    }
}
