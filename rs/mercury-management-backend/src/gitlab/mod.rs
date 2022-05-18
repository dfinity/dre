mod commit_refs;
pub use commit_refs::CommitRefs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitRef {
    #[serde(alias = "type")]
    pub kind: String,
    pub name: String,
}
