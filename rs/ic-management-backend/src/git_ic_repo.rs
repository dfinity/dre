use custom_error::custom_error;
use fs2::FileExt;
use log::{info, warn};
use std::path::PathBuf;
use std::process::Command;
use std::{collections::HashMap, fs::File};

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
}

impl IcRepo {
    // Initialize the IcRepo struct, to work with a local clone of the IC repo
    pub fn new() -> anyhow::Result<Self> {
        let repo_path = match std::env::var("REPO_CACHE_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => match dirs::cache_dir() {
                Some(cache_dir) => cache_dir,
                None => PathBuf::from("/tmp"),
            },
        }
        .join("git")
        .join("ic");
        let lock_file_path = format!("{}.lock", &repo_path.display());
        info!(
            "IC git repo path: {}, lock file path: {}",
            &repo_path.display(),
            &lock_file_path
        );

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
            repo.refetch()?;
        } else {
            info!("Repo {} does not exist, cloning", &repo_path.to_str().unwrap());
            Command::new("git")
                .args(["clone", "https://github.com/dfinity/ic", repo_path.to_str().unwrap()])
                .status()?;
        }

        lock_file.unlock()?;

        Ok(repo)
    }

    fn refetch(&self) -> anyhow::Result<()> {
        Command::new("git")
            .args(["-C", self.repo_path.to_str().unwrap(), "pull", "--force", "origin"])
            .output()?;
        Ok(())
    }

    pub fn get_branches_with_commit(&mut self, commit_sha: &str) -> anyhow::Result<Vec<String>> {
        let branches = match self.cache.get(commit_sha) {
            Some(branches) => branches.clone(),
            None => {
                self.refetch()?;
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

                if branches.is_empty() {
                    warn!(
                        "No branches found for commit {} -- do you have a full repo clone?",
                        commit_sha
                    )
                } else {
                    self.cache.insert(commit_sha.to_string(), branches.clone());
                }
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
    fn test_get_branches_with_nonexistent_commit() {
        let mut ic_repo = IcRepo::new().unwrap();
        let branches = ic_repo.get_branches_with_commit("80a6745673a28ee53d257b3fe19dcd6b7efa93d1");
        assert!(branches.is_ok());
        assert!(!branches.unwrap().is_empty());
    }
}
