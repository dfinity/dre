import { AuthClient } from '@dfinity/auth-client';
import { DelegationIdentity, DelegationChain } from '@dfinity/identity';
import type { VerifyDelegationRequest, DelegationChain as DelegationChainType } from './types';
import { api } from './api';

// Internet Identity URL - configurable via environment
const II_URL = import.meta.env.VITE_II_URL || 'https://identity.ic0.app';

let authClient: AuthClient | null = null;

export async function getAuthClient(): Promise<AuthClient> {
	if (!authClient) {
		authClient = await AuthClient.create();
	}
	return authClient;
}

export async function login(): Promise<{ token: string; principal: string } | null> {
	const client = await getAuthClient();

	return new Promise((resolve) => {
		client.login({
			identityProvider: II_URL,
			maxTimeToLive: BigInt(8) * BigInt(3_600_000_000_000), // 8 hours
			onSuccess: async () => {
				try {
					const identity = client.getIdentity();
					if (identity instanceof DelegationIdentity) {
						const delegationChain = identity.getDelegation();
						const request = convertDelegationChain(delegationChain);
						const response = await api.verifyDelegation(request);
						resolve({
							token: response.token,
							principal: response.principal
						});
					} else {
						console.error('Expected DelegationIdentity');
						resolve(null);
					}
				} catch (error) {
					console.error('Failed to verify delegation:', error);
					resolve(null);
				}
			},
			onError: (error) => {
				console.error('Login failed:', error);
				resolve(null);
			}
		});
	});
}

export async function logout(token?: string): Promise<void> {
	const client = await getAuthClient();
	if (token) {
		try {
			await api.logout(token);
		} catch (error) {
			console.error('Failed to logout from backend:', error);
		}
	}
	await client.logout();
}

export async function isAuthenticated(): Promise<boolean> {
	const client = await getAuthClient();
	return client.isAuthenticated();
}

function convertDelegationChain(chain: DelegationChain): VerifyDelegationRequest {
	const delegations = chain.delegations.map((signedDelegation) => {
		const delegation = signedDelegation.delegation;
		return {
			delegation: {
				pubkey: Array.from(delegation.pubkey),
				expiration: delegation.expiration.toString(),
				targets: delegation.targets?.map((t) => t.toText())
			},
			signature: Array.from(signedDelegation.signature)
		};
	});

	const delegationChain: DelegationChainType = {
		delegations,
		publicKey: Array.from(chain.publicKey)
	};

	return { delegation_chain: delegationChain };
}
