use std::collections::{BTreeMap, BTreeSet};

use ic_management_types::Status;
use ic_types::PrincipalId;

use crate::notification::Notification;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub struct NodesStatus {
    nodes: BTreeMap<PrincipalId, Status>,
}

impl From<BTreeMap<PrincipalId, Status>> for NodesStatus {
    fn from(tree: BTreeMap<PrincipalId, Status>) -> Self {
        Self { nodes: tree }
    }
}

impl NodesStatus {
    pub fn _new() -> Self {
        Self { nodes: BTreeMap::new() }
    }

    pub fn get_set_of_node_ids(&self) -> BTreeSet<PrincipalId> {
        self.nodes.keys().copied().collect()
    }

    fn get(&self, id: PrincipalId) -> Option<&Status> {
        self.nodes.get(&id)
    }

    pub fn updated_from_map(&self, map: BTreeMap<PrincipalId, Status>) -> (NodesStatus, Vec<Notification>) {
        self.updated(Self::from(map))
    }

    pub fn updated(&self, new_statuses: NodesStatus) -> (NodesStatus, Vec<Notification>) {
        let mut notifications = vec![];

        // If node in new_statuses and in current, test status change
        // else, if node in new_statuses only, add notification of addition
        // else, add notification of removal

        let current_status_node_ids = self.get_set_of_node_ids();
        let new_status_node_ids = new_statuses.get_set_of_node_ids();

        let added_nodes: BTreeSet<PrincipalId> = new_status_node_ids
            .difference(&current_status_node_ids)
            .cloned()
            .collect();

        for node_id in added_nodes {
            notifications.push(Notification {
                node_id,
                status_change: (
                    Status::Unknown,
                    new_statuses
                        .get(node_id)
                        .unwrap_or_else(|| panic!("New statuses map should contain id {}", node_id))
                        .clone(),
                ),
                node_provider: None,
            })
        }

        let removed_nodes: BTreeSet<PrincipalId> = current_status_node_ids
            .difference(&new_status_node_ids)
            .cloned()
            .collect();

        for node_id in removed_nodes {
            notifications.push(Notification {
                node_id,
                status_change: (
                    self.get(node_id)
                        .unwrap_or_else(|| panic!("Current statuses map should contain id {}", node_id))
                        .clone(),
                    Status::Unknown,
                ),
                node_provider: None,
            })
        }

        let kept_nodes: BTreeSet<PrincipalId> = current_status_node_ids
            .intersection(&new_status_node_ids)
            .cloned()
            .collect();

        for node_id in kept_nodes {
            if self.get(node_id) != new_statuses.get(node_id) {
                notifications.push(Notification {
                    node_id,
                    status_change: (
                        self.get(node_id)
                            .unwrap_or_else(|| panic!("Current statuses map should contain id {}", node_id))
                            .clone(),
                        new_statuses
                            .get(node_id)
                            .unwrap_or_else(|| panic!("New statuses map should contain id {}", node_id))
                            .clone(),
                    ),
                    node_provider: None,
                })
            }
        }

        (new_statuses, notifications)
    }
}

#[cfg(test)]
mod tests {
    use ic_management_types::Status;
    use ic_types::PrincipalId;

    use super::*;

    #[test]
    fn test_nodes_status_updates() {
        // Node changed
        // Node added
        // Node removed
        // Node unchanged
        let ids = vec![
            PrincipalId::new_node_test_id(0),
            PrincipalId::new_node_test_id(1),
            PrincipalId::new_node_test_id(2),
            PrincipalId::new_node_test_id(3),
        ];

        let statuses = NodesStatus {
            nodes: BTreeMap::from([
                (ids[0], Status::Healthy),
                (ids[1], Status::Healthy),
                (ids[2], Status::Healthy),
            ]),
        };
        let new_statuses = NodesStatus {
            nodes: BTreeMap::from([
                (ids[0], Status::Healthy),
                (ids[1], Status::Degraded),
                (ids[3], Status::Healthy),
            ]),
        };
        let (statuses, notifications) = statuses.updated(new_statuses.clone());

        assert_eq!(statuses, new_statuses);
        assert_eq!(notifications.len(), 3);
        assert!(notifications.contains(&Notification {
            node_id: ids[1],
            node_provider: None,
            status_change: (Status::Healthy, Status::Degraded),
        }));
        assert!(notifications.contains(&Notification {
            node_id: ids[2],
            node_provider: None,
            status_change: (Status::Healthy, Status::Unknown),
        }));
        assert!(notifications.contains(&Notification {
            node_id: ids[3],
            node_provider: None,
            status_change: (Status::Unknown, Status::Healthy),
        }));
    }
}
