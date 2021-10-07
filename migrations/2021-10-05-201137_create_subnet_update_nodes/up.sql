CREATE TABLE subnet_update_nodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  subnet VARCHAR NOT NULL,
  nodes_to_add TEXT,
  nodes_to_remove TEXT,
  proposal_add_id INTEGER,
  proposal_add_executed BOOLEAN NOT NULL DEFAULT 0,
  proposal_add_failed TEXT,
  proposal_remove_id INTEGER,
  proposal_remove_executed BOOLEAN NOT NULL DEFAULT 0,
  proposal_remove_failed TEXT
);