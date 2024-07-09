use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use ic_management_types::{Artifact, ArtifactReleases, Network, Release};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error};
use regex::Regex;

use crate::git_ic_repo::IcRepo;

pub struct LazyGit {
    guestos_releases: RefCell<Option<Rc<ArtifactReleases>>>,
    hostos_releases: RefCell<Option<Rc<ArtifactReleases>>>,
    ic_repo: RefCell<IcRepo>,
    network: Network,
    blessed_replica_versions: Vec<String>,
    elected_hostos_versions: Vec<String>,
}

impl LazyGit {
    pub fn new(network: Network, blessed_replica_versions: Vec<String>, elected_hostos_versions: Vec<String>) -> anyhow::Result<Self> {
        Ok(Self {
            guestos_releases: RefCell::new(None),
            hostos_releases: RefCell::new(None),
            ic_repo: RefCell::new(IcRepo::new()?),
            network,
            blessed_replica_versions,
            elected_hostos_versions,
        })
    }

    pub async fn guestos_releases(&self) -> anyhow::Result<Rc<ArtifactReleases>> {
        if let Some(releases) = self.guestos_releases.borrow().as_ref() {
            return Ok(releases.to_owned());
        }

        self.update_releases().await?;
        self.guestos_releases
            .borrow()
            .as_ref()
            .map(|n| n.to_owned())
            .ok_or(anyhow::anyhow!("Failed to update releases"))
    }

    pub async fn hostos_releases(&self) -> anyhow::Result<Rc<ArtifactReleases>> {
        if let Some(releases) = self.hostos_releases.borrow().as_ref() {
            return Ok(releases.to_owned());
        }

        self.update_releases().await?;
        self.hostos_releases
            .borrow()
            .as_ref()
            .map(|n| n.to_owned())
            .ok_or(anyhow::anyhow!("Failed to update releases"))
    }

    async fn update_releases(&self) -> anyhow::Result<()> {
        if !self.network.eq(&Network::mainnet_unchecked()?) {
            *self.guestos_releases.borrow_mut() = Some(Rc::new(ArtifactReleases::new(Artifact::GuestOs)));
            *self.hostos_releases.borrow_mut() = Some(Rc::new(ArtifactReleases::new(Artifact::HostOs)));
            return Ok(());
        }

        lazy_static! {
            // TODO: We don't need to distinguish release branch and name, they can be the same
            static ref RELEASE_BRANCH_GROUP: &'static str = "release_branch";
            static ref RELEASE_NAME_GROUP: &'static str = "release_name";
            static ref DATETIME_NAME_GROUP: &'static str = "datetime";
            // example: rc--2021-09-13_18-32
            static ref RE: Regex = Regex::new(&format!(r#"(?P<{}>(?P<{}>rc--(?P<{}>\d{{4}}-\d{{2}}-\d{{2}}_\d{{2}}-\d{{2}}))(?P<discardable_suffix>.*))$"#,
                *RELEASE_BRANCH_GROUP,
                *RELEASE_NAME_GROUP,
                *DATETIME_NAME_GROUP,
            )).unwrap();
        }

        let blessed_versions: HashSet<&String> = self.blessed_replica_versions.iter().chain(self.elected_hostos_versions.iter()).collect();

        // A HashMap from the git revision to the latest commit branch in which the
        // commit is present
        let mut commit_to_release: HashMap<String, Release> = HashMap::new();
        blessed_versions.into_iter().for_each(|commit_hash| {
            let mut ic_repo = self.ic_repo.borrow_mut();
            match ic_repo.get_branches_with_commit(commit_hash) {
                // For each commit get a list of branches that have the commit
                Ok(branches) => {
                    debug!("Git rev {} ==> {} branches: {}", commit_hash, branches.len(), branches.join(", "));
                    for branch in branches.into_iter().sorted() {
                        match RE.captures(&branch) {
                            Some(capture) => {
                                let release_branch = capture.name(&RELEASE_BRANCH_GROUP).expect("release regex misconfiguration").as_str();
                                let release_name = capture.name(&RELEASE_NAME_GROUP).expect("release regex misconfiguration").as_str();
                                let release_datetime = chrono::NaiveDateTime::parse_from_str(
                                    capture.name(&DATETIME_NAME_GROUP).expect("release regex misconfiguration").as_str(),
                                    "%Y-%m-%d_%H-%M",
                                )
                                .expect("invalid datetime format");

                                commit_to_release.insert(
                                    commit_hash.clone(),
                                    Release {
                                        name: release_name.to_string(),
                                        branch: release_branch.to_string(),
                                        commit_hash: commit_hash.clone(),
                                        previous_patch_release: None,
                                        time: release_datetime,
                                    },
                                );
                                break;
                            }
                            None => {
                                if branch != "master" && branch != "HEAD" {
                                    debug!("Git rev {}: branch {} does not match the RC regex", &commit_hash, &branch);
                                }
                            }
                        };
                    }
                }
                Err(e) => error!("failed to find branches for git rev: {}; {}", &commit_hash, e),
            }
        });

        for (blessed_versions, mut to_update, artifact_type) in [
            (&self.blessed_replica_versions, self.guestos_releases.borrow_mut(), Artifact::GuestOs),
            (&self.elected_hostos_versions, self.hostos_releases.borrow_mut(), Artifact::HostOs),
        ] {
            let releases = blessed_versions
                .iter()
                .map(|version| commit_to_release.get(version).unwrap().clone())
                .sorted_by_key(|rr| rr.time)
                .collect::<Vec<Release>>();
            debug!("Updated {} releases to {:?}", artifact_type, releases);
            *to_update = Some(Rc::new(ArtifactReleases {
                artifact: artifact_type,
                releases,
            }));
        }

        Ok(())
    }
}
