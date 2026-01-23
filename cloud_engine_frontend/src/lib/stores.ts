import { writable, derived } from 'svelte/store';
import type { UserProfile, Vm, NodeInfo, SubnetInfo } from './types';

// Auth state
export const authToken = writable<string | null>(null);
export const principal = writable<string | null>(null);
export const isAuthenticated = derived(authToken, ($token) => $token !== null);

// User profile
export const userProfile = writable<UserProfile | null>(null);

// Data stores
export const vms = writable<Vm[]>([]);
export const nodes = writable<NodeInfo[]>([]);
export const subnets = writable<SubnetInfo[]>([]);

// Loading states
export const isLoading = writable<boolean>(false);
export const loadingMessage = writable<string>('');

// Error state
export const errorMessage = writable<string | null>(null);

// Clear error after timeout
export function setError(message: string, timeout: number = 5000): void {
	errorMessage.set(message);
	if (timeout > 0) {
		setTimeout(() => {
			errorMessage.set(null);
		}, timeout);
	}
}

// Success message
export const successMessage = writable<string | null>(null);

export function setSuccess(message: string, timeout: number = 3000): void {
	successMessage.set(message);
	if (timeout > 0) {
		setTimeout(() => {
			successMessage.set(null);
		}, timeout);
	}
}

// Clear all state on logout
export function clearAllState(): void {
	authToken.set(null);
	principal.set(null);
	userProfile.set(null);
	vms.set([]);
	nodes.set([]);
	subnets.set([]);
	errorMessage.set(null);
	successMessage.set(null);
}

// Initialize from localStorage if available
export function initializeFromStorage(): void {
	if (typeof window !== 'undefined') {
		const storedToken = localStorage.getItem('auth_token');
		const storedPrincipal = localStorage.getItem('principal');
		if (storedToken && storedPrincipal) {
			authToken.set(storedToken);
			principal.set(storedPrincipal);
		}
	}
}

// Persist auth to localStorage
authToken.subscribe((token) => {
	if (typeof window !== 'undefined') {
		if (token) {
			localStorage.setItem('auth_token', token);
		} else {
			localStorage.removeItem('auth_token');
		}
	}
});

principal.subscribe((p) => {
	if (typeof window !== 'undefined') {
		if (p) {
			localStorage.setItem('principal', p);
		} else {
			localStorage.removeItem('principal');
		}
	}
});
