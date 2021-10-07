table! {
    subnet_update_nodes (id) {
        id -> Integer,
        subnet -> Text,
        nodes_to_add -> Nullable<Text>,
        nodes_to_remove -> Nullable<Text>,
        proposal_add_id -> Nullable<Integer>,
        proposal_add_executed -> Bool,
        proposal_add_failed -> Nullable<Text>,
        proposal_remove_id -> Nullable<Integer>,
        proposal_remove_executed -> Bool,
        proposal_remove_failed -> Nullable<Text>,
    }
}
