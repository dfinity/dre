CREATE TABLE subnet_update_nodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  subnet VARCHAR NOT NULL,
  in_progress BOOLEAN NOT NULL DEFAULT true,
  nodes_to_add TEXT,
  nodes_to_remove TEXT,
  proposal_add_id BIGINT,
  proposal_remove_id BIGINT
);