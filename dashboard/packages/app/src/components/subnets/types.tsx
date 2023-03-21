export interface Provider {
  principal: string;
  name?: string;
  website?: string;
}

export interface DatacenterOwner {
  name: string;
}

export interface Datacenter {
  name: string;
  owner: DatacenterOwner;
  city: string;
  country: string;
  continent: string;
}


export interface Operator {
  principal: string;
  provider: Provider;
  allowance: number;
  datacenter?: Datacenter;
}

export interface Node {
  principal: string;
  ip_addr: string;
  operator: Operator;
  hostname: string;
  label?: string;
  subnet?: string;
  dfinity_owned: boolean;
  proposal?: TopologyProposal;
}

export type NodeHealth = "Healthy" | "Degraded" | "Dead" | "Unknown";

export interface TopologyProposal {
  id: number;
}

type nodeLabelTypes = "DFINITY" | "Old CUP" | "NNS ready";

export interface NodeLabel {
  name: nodeLabelTypes;
  description: string;
}

export interface Guest {
  datacenter: string;
  ipv6: string;
  name: string;
  dfinity_owned: boolean;
}

export interface SubnetMetadata {
  name: string;
  labels?: string[],
  applications?: VerifiedApplication[],
}

export interface Subnet {
  principal: string;
  nodes: Node[];
  subnet_type: string;
  metadata: SubnetMetadata;
  replica_version: string;
  replica_release: ReplicaRelease;
  proposal?: { id: number };
}

export type VerifiedApplication = "Fleek" | "Distrikt";

export type RolloutStatus = "Active" | "Scheduled" | "Complete";

export interface Rollout {
  status: RolloutStatus;
  latest_release: ReplicaRelease;
  stages: RolloutStage[];
}

export interface RolloutStage {
  start_time?: string;
  start_date_time: number;
  updates: SubnetUpdate[];
  active: boolean;
}

export interface SubnetUpdate {
  subnet_id: string;
  subnet_name: string;
  state: SubnetUpdateState;
  proposal?: SubnetUpdateProposal;
  patches_available: ReplicaRelease[];
  replica_release: ReplicaRelease;
}

export type SubnetUpdateState = "scheduled" | "submitted" | "preparing" | "updating" | "baking" | "complete" | "unknown";

export interface SubnetUpdateProposal {
  info: SubnetUpdateProposalInfo
  payload: SubnetUpdateProposalPayload;
}

export interface SubnetUpdateProposalInfo {
  id: number,
  proposal_timestamp_seconds: number;
  executed: boolean,
}

export interface SubnetUpdateProposalPayload {
  subnet_id: string;
  replica_version_id: string;
}

export interface ReplicaVersionUpdateProposal {
  id: number;
  pending: boolean;
}

export interface ReplicaRelease {
  commit_hash: string;
  name: string;
  branch: string;
  time: string;
  previous_patch_release?: ReplicaRelease;
}

export interface ChangePreview {
  added: string[]
  removed: string[]
  subnet_id: string
  score_before: NakamotoScore
  score_after: NakamotoScore
  feature_diff: { [f: string]: { [n: string]: [number, number] } }
  proposal_id?: number
  comment?: string
  run_log?: string[]
}

export interface NakamotoScore {
  coefficients: Coefficients
  value_counts: ValueCounts
  controlled_nodes: ControlledNodes
  avg_linear: number
  avg_log2: number
  min: number
}

export interface Coefficients {
  node_provider: number
  data_center: number
  data_center_owner: number
  city: number
  country: number
  continent: number
}

export interface ValueCounts {
  node_provider: [string, number][]
  data_center: [string, number][]
  data_center_owner: [string, number][]
  city: [string, number][]
  country: [string, number][]
  continent: [string, number][]
}

export interface ControlledNodes {
  node_provider: number
  data_center: number
  data_center_owner: number
  city: number
  country: number
  continent: number
}
