table! {
    subnet_update_nodes (id) {
        id -> Integer,
        subnet -> Text,
        nodes_to_add -> Nullable<Text>,
        nodes_to_remove -> Nullable<Text>,
        proposal_id_for_add -> Nullable<Integer>,
        proposal_executed_add -> Bool,
        proposal_id_for_remove -> Nullable<Integer>,
        proposal_executed_remove -> Bool,
    }
}
