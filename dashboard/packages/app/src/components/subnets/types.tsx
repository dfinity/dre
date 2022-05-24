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
  subnet?: string;
  dfinity_owned: boolean;
  proposal?: TopologyProposal;
}

export type NodeHealth = "Healthy" | "Unhealthy" | "Unknown";

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
}

export type VerifiedApplication = "Fleek" | "Distrikt";

export interface Rollout {
  release: ReplicaRelease;
  stages: RolloutStage[];
}

export interface RolloutStage {
  name: string;
  subnets: SubnetRolloutStatus[];
}

export interface SubnetRolloutStatus {
  principal: string;
  latest_release: boolean;
  upgrading: boolean;
  upgrading_release: boolean;
  replica_release: ReplicaRelease;
  patches_available: ReplicaRelease[];
  name: string;
  proposal?: ReplicaVersionUpdateProposal;
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
