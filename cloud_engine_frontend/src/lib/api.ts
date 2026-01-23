import type {
	VerifyDelegationRequest,
	VerifyDelegationResponse,
	SessionInfo,
	UserProfile,
	GcpAccount,
	NodeOperatorInfo,
	VmListResponse,
	VmProvisionRequest,
	VmProvisionResponse,
	NodeListResponse,
	NodeInfo,
	SubnetListResponse,
	SubnetProposal,
	SubnetProposalResponse,
	ApiError
} from './types';

// Use the configured backend URL or default to current origin
const API_BASE = import.meta.env.VITE_API_URL || '/api';

class ApiClient {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(
		endpoint: string,
		method: string = 'GET',
		body?: unknown
	): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;
		const options: RequestInit = {
			method,
			headers: {
				'Content-Type': 'application/json'
			}
		};

		if (body) {
			options.body = JSON.stringify(body);
		}

		const response = await fetch(url, options);

		if (!response.ok) {
			const error: ApiError = await response.json().catch(() => ({
				error: 'request_failed',
				message: `Request failed with status ${response.status}`
			}));
			throw new Error(error.message || 'Request failed');
		}

		return response.json();
	}

	private authenticatedBody<T>(token: string, data?: T): { token: string } & (T extends void ? object : { data: T }) {
		if (data !== undefined) {
			return { token, ...data } as { token: string } & (T extends void ? object : { data: T });
		}
		return { token } as { token: string } & (T extends void ? object : { data: T });
	}

	// Auth endpoints
	async verifyDelegation(request: VerifyDelegationRequest): Promise<VerifyDelegationResponse> {
		return this.request('/auth/verify', 'POST', request);
	}

	async getSession(token: string): Promise<SessionInfo> {
		return this.request('/auth/session', 'POST', { token });
	}

	async logout(token: string): Promise<void> {
		return this.request('/auth/logout', 'POST', { token });
	}

	// User endpoints
	async getProfile(token: string): Promise<UserProfile> {
		return this.request('/user/profile', 'POST', { token });
	}

	async setGcpAccount(token: string, gcpAccount: GcpAccount): Promise<UserProfile> {
		return this.request('/user/gcp-account', 'POST', { token, ...gcpAccount });
	}

	async setNodeOperator(token: string, nodeOperator: NodeOperatorInfo): Promise<UserProfile> {
		return this.request('/user/node-operator', 'POST', { token, ...nodeOperator });
	}

	// VM endpoints
	async listVms(token: string): Promise<VmListResponse> {
		return this.request('/vms/list', 'POST', { token });
	}

	async provisionVm(token: string, request: VmProvisionRequest): Promise<VmProvisionResponse> {
		return this.request('/vms/provision', 'POST', { token, ...request });
	}

	async deleteVm(token: string, vmId: string): Promise<void> {
		return this.request('/vms/delete', 'POST', { token, vm_id: vmId });
	}

	// Node endpoints
	async listNodes(token: string): Promise<NodeListResponse> {
		return this.request('/nodes/list', 'POST', { token });
	}

	async getNode(token: string, nodeId: string): Promise<NodeInfo> {
		return this.request('/nodes/get', 'POST', { token, node_id: nodeId });
	}

	// Subnet endpoints
	async listSubnets(token: string): Promise<SubnetListResponse> {
		return this.request('/subnets/list', 'POST', { token });
	}

	async createSubnetProposal(token: string, proposal: SubnetProposal): Promise<SubnetProposalResponse> {
		return this.request('/subnets/create', 'POST', { token, ...proposal });
	}

	async deleteSubnetProposal(token: string, subnetId: string): Promise<SubnetProposalResponse> {
		return this.request('/subnets/delete', 'POST', { token, subnet_id: subnetId });
	}
}

export const api = new ApiClient();
export default api;
