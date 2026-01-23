<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		isAuthenticated,
		authToken,
		principal,
		userProfile,
		setError,
		setSuccess
	} from '$lib/stores';
	import { api } from '$lib/api';
	import { LoadingSpinner } from '$lib/components';
	import type { GcpAccount, NodeOperatorInfo } from '$lib/types';

	let loading = true;
	let savingGcp = false;
	let savingOperator = false;

	// GCP form state
	let gcpProjectId = '';
	let gcpServiceAccountEmail = '';
	let gcpRegion = 'us-central1';
	let gcpZone = 'us-central1-a';

	// Node operator form state
	let nodeOperatorId = '';
	let nodeOperatorDisplayName = '';

	const gcpRegions = [
		{ region: 'us-central1', zones: ['us-central1-a', 'us-central1-b', 'us-central1-c'] },
		{ region: 'us-east1', zones: ['us-east1-b', 'us-east1-c', 'us-east1-d'] },
		{ region: 'us-west1', zones: ['us-west1-a', 'us-west1-b', 'us-west1-c'] },
		{ region: 'europe-west1', zones: ['europe-west1-b', 'europe-west1-c', 'europe-west1-d'] },
		{ region: 'europe-west2', zones: ['europe-west2-a', 'europe-west2-b', 'europe-west2-c'] },
		{ region: 'asia-east1', zones: ['asia-east1-a', 'asia-east1-b', 'asia-east1-c'] }
	];

	$: availableZones = gcpRegions.find((r) => r.region === gcpRegion)?.zones || [];

	onMount(async () => {
		if (!$isAuthenticated) {
			goto('/');
			return;
		}

		try {
			const profile = await api.getProfile($authToken!);
			userProfile.set(profile);

			if (profile.gcp_account) {
				gcpProjectId = profile.gcp_account.project_id;
				gcpServiceAccountEmail = profile.gcp_account.service_account_email;
				gcpRegion = profile.gcp_account.region;
				gcpZone = profile.gcp_account.zone;
			}

			if (profile.node_operator) {
				nodeOperatorId = profile.node_operator.node_operator_id;
				nodeOperatorDisplayName = profile.node_operator.display_name || '';
			}
		} catch (error) {
			setError('Failed to load profile');
		} finally {
			loading = false;
		}
	});

	async function saveGcpAccount() {
		if (!gcpProjectId || !gcpServiceAccountEmail) {
			setError('Project ID and Service Account Email are required');
			return;
		}

		savingGcp = true;
		try {
			const gcpAccount: GcpAccount = {
				project_id: gcpProjectId,
				service_account_email: gcpServiceAccountEmail,
				region: gcpRegion,
				zone: gcpZone
			};
			const profile = await api.setGcpAccount($authToken!, gcpAccount);
			userProfile.set(profile);
			setSuccess('GCP account saved successfully');
		} catch (error) {
			setError('Failed to save GCP account: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			savingGcp = false;
		}
	}

	async function saveNodeOperator() {
		if (!nodeOperatorId) {
			setError('Node Operator ID is required');
			return;
		}

		savingOperator = true;
		try {
			const nodeOperator: NodeOperatorInfo = {
				node_operator_id: nodeOperatorId,
				display_name: nodeOperatorDisplayName || undefined
			};
			const profile = await api.setNodeOperator($authToken!, nodeOperator);
			userProfile.set(profile);
			setSuccess('Node operator saved successfully');
		} catch (error) {
			setError('Failed to save node operator: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			savingOperator = false;
		}
	}
</script>

<svelte:head>
	<title>Profile - Cloud Engine Controller</title>
</svelte:head>

<div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	{#if loading}
		<div class="flex items-center justify-center h-64">
			<LoadingSpinner size="lg" message="Loading profile..." />
		</div>
	{:else}
		<div class="mb-8">
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Profile Settings</h1>
			<p class="mt-1 text-gray-600 dark:text-gray-400">
				Manage your GCP account and node operator settings
			</p>
		</div>

		<!-- Principal Info -->
		<div class="card p-6 mb-6">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
				Internet Identity
			</h2>
			<div>
				<label class="block text-sm font-medium text-gray-500 dark:text-gray-400">
					Principal ID
				</label>
				<p class="mt-1 font-mono text-gray-900 dark:text-white break-all">
					{$principal}
				</p>
			</div>
		</div>

		<!-- GCP Account -->
		<div class="card p-6 mb-6">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
				GCP Account
			</h2>
			<form on:submit|preventDefault={saveGcpAccount} class="space-y-4">
				<div>
					<label
						for="gcpProjectId"
						class="block text-sm font-medium text-gray-700 dark:text-gray-300"
					>
						Project ID *
					</label>
					<input
						type="text"
						id="gcpProjectId"
						bind:value={gcpProjectId}
						class="input mt-1"
						placeholder="my-gcp-project"
						required
					/>
				</div>

				<div>
					<label
						for="gcpServiceAccount"
						class="block text-sm font-medium text-gray-700 dark:text-gray-300"
					>
						Service Account Email *
					</label>
					<input
						type="email"
						id="gcpServiceAccount"
						bind:value={gcpServiceAccountEmail}
						class="input mt-1"
						placeholder="service-account@project.iam.gserviceaccount.com"
						required
					/>
				</div>

				<div class="grid grid-cols-2 gap-4">
					<div>
						<label
							for="gcpRegion"
							class="block text-sm font-medium text-gray-700 dark:text-gray-300"
						>
							Region
						</label>
						<select id="gcpRegion" bind:value={gcpRegion} class="input mt-1">
							{#each gcpRegions as r}
								<option value={r.region}>{r.region}</option>
							{/each}
						</select>
					</div>

					<div>
						<label
							for="gcpZone"
							class="block text-sm font-medium text-gray-700 dark:text-gray-300"
						>
							Zone
						</label>
						<select id="gcpZone" bind:value={gcpZone} class="input mt-1">
							{#each availableZones as zone}
								<option value={zone}>{zone}</option>
							{/each}
						</select>
					</div>
				</div>

				<div class="flex justify-end">
					<button type="submit" disabled={savingGcp} class="btn-primary">
						{#if savingGcp}
							<svg class="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
								></circle>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								></path>
							</svg>
							Saving...
						{:else}
							Save GCP Account
						{/if}
					</button>
				</div>
			</form>
		</div>

		<!-- Node Operator -->
		<div class="card p-6">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
				Node Operator
			</h2>
			<form on:submit|preventDefault={saveNodeOperator} class="space-y-4">
				<div>
					<label
						for="nodeOperatorId"
						class="block text-sm font-medium text-gray-700 dark:text-gray-300"
					>
						Node Operator Principal ID *
					</label>
					<input
						type="text"
						id="nodeOperatorId"
						bind:value={nodeOperatorId}
						class="input mt-1 font-mono"
						placeholder="xxxxx-xxxxx-xxxxx-xxxxx-xxxxx"
						required
					/>
					<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						Your node operator principal from the NNS registry
					</p>
				</div>

				<div>
					<label
						for="displayName"
						class="block text-sm font-medium text-gray-700 dark:text-gray-300"
					>
						Display Name (optional)
					</label>
					<input
						type="text"
						id="displayName"
						bind:value={nodeOperatorDisplayName}
						class="input mt-1"
						placeholder="My Node Operation"
					/>
				</div>

				<div class="flex justify-end">
					<button type="submit" disabled={savingOperator} class="btn-primary">
						{#if savingOperator}
							<svg class="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
								></circle>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								></path>
							</svg>
							Saving...
						{:else}
							Save Node Operator
						{/if}
					</button>
				</div>
			</form>
		</div>
	{/if}
</div>
