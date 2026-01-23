// API Types matching the backend OpenAPI schema

export interface Delegation {
	pubkey: number[];
	expiration: string;
	targets?: string[];
}

export interface SignedDelegation {
	delegation: Delegation;
	signature: number[];
}

export interface DelegationChain {
	delegations: SignedDelegation[];
	publicKey: number[];
}

export interface VerifyDelegationRequest {
	delegation_chain: DelegationChain;
}

export interface VerifyDelegationResponse {
	principal: string;
	token: string;
	expires_at: string;
}

export interface SessionInfo {
	principal: string;
	expires_at: string;
}

export interface GcpAccount {
	project_id: string;
	service_account_email: string;
	region: string;
	zone: string;
}

export interface NodeOperatorInfo {
	node_operator_id: string;
	display_name?: string;
}

export interface UserProfile {
	principal: string;
	gcp_account?: GcpAccount;
	node_operator?: NodeOperatorInfo;
	created_at: string;
	updated_at: string;
}

export type VmStatus = 'running' | 'stopped' | 'terminated' | 'pending' | 'staging' | 'suspending' | 'suspended' | 'unknown';

export interface IcpNodeMapping {
	node_id: string;
	subnet_id?: string;
	node_operator_id: string;
}

export interface Vm {
	id: string;
	name: string;
	status: VmStatus;
	machine_type: string;
	zone: string;
	external_ip?: string;
	internal_ip?: string;
	icp_node?: IcpNodeMapping;
	created_at: string;
}

export interface VmListResponse {
	vms: Vm[];
	total: number;
}

export interface VmProvisionRequest {
	name: string;
	machine_type: string;
	zone: string;
	disk_size_gb: number;
	image_family: string;
}

export interface VmProvisionResponse {
	vm: Vm;
	message: string;
}

export interface NodeInfo {
	node_id: string;
	subnet_id?: string;
	node_operator_id: string;
	ip_address: string;
	dc_id: string;
	node_provider_id: string;
	status: string;
}

export interface NodeListResponse {
	nodes: NodeInfo[];
	total: number;
}

export type ProposalStatus = 'pending' | 'executed' | 'failed' | 'rejected';

export interface SubnetInfo {
	subnet_id: string;
	subnet_type: string;
	replica_version: string;
	node_count: number;
}

export interface SubnetProposal {
	action: 'create' | 'delete';
	subnet_type?: string;
	node_ids: string[];
	replica_version?: string;
}

export interface SubnetProposalResponse {
	proposal_id: string;
	status: ProposalStatus;
	message: string;
}

export interface SubnetListResponse {
	subnets: SubnetInfo[];
	total: number;
}

export interface AuthenticatedRequest<T = void> {
	token: string;
	data?: T;
}

export interface ApiError {
	error: string;
	message: string;
}
