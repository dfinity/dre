CREATE TABLE subnet_update_nodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  subnet VARCHAR NOT NULL,
  nodes_to_add TEXT,
  nodes_to_remove TEXT,
  proposal_id_for_add INTEGER,
  proposal_executed_add BOOLEAN NOT NULL DEFAULT 0,
  proposal_id_for_remove INTEGER,
  proposal_executed_remove BOOLEAN NOT NULL DEFAULT 0
);