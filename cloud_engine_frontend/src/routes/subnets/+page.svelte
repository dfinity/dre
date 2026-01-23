<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isAuthenticated, authToken, subnets, nodes, setError, setSuccess } from '$lib/stores';
	import { api } from '$lib/api';
	import { LoadingSpinner, SubnetCard, Modal } from '$lib/components';
	import type { SubnetProposal } from '$lib/types';

	let loading = true;
	let creating = false;
	let deletingSubnetId: string | null = null;
	let showCreateModal = false;

	// Create form state
	let subnetType = 'application';
	let selectedNodeIds: string[] = [];
	let replicaVersion = '';

	const subnetTypes = ['application', 'system', 'verified_application'];

	$: availableNodes = $nodes.filter((node) => !node.subnet_id);

	onMount(async () => {
		if (!$isAuthenticated) {
			goto('/');
			return;
		}
		await Promise.all([loadSubnets(), loadNodes()]);
	});

	async function loadSubnets() {
		try {
			const response = await api.listSubnets($authToken!);
			subnets.set(response.subnets);
		} catch (error) {
			setError('Failed to load subnets: ' + (error instanceof Error ? error.message : 'Unknown error'));
		}
	}

	async function loadNodes() {
		try {
			const response = await api.listNodes($authToken!);
			nodes.set(response.nodes);
		} catch (error) {
			// Nodes already might be loaded from elsewhere
		} finally {
			loading = false;
		}
	}

	async function createSubnetProposal() {
		if (selectedNodeIds.length === 0) {
			setError('Please select at least one node');
			return;
		}

		creating = true;
		try {
			const proposal: SubnetProposal = {
				action: 'create',
				subnet_type: subnetType,
				node_ids: selectedNodeIds,
				replica_version: replicaVersion || undefined
			};
			const response = await api.createSubnetProposal($authToken!, proposal);
			setSuccess(`Subnet proposal created: ${response.proposal_id}`);
			showCreateModal = false;
			resetForm();
			await loadSubnets();
		} catch (error) {
			setError('Failed to create proposal: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			creating = false;
		}
	}

	async function deleteSubnetProposal(subnetId: string) {
		if (!confirm('Are you sure you want to create a delete proposal for this subnet?')) {
			return;
		}

		deletingSubnetId = subnetId;
		try {
			const response = await api.deleteSubnetProposal($authToken!, subnetId);
			setSuccess(`Delete proposal created: ${response.proposal_id}`);
		} catch (error) {
			setError('Failed to create delete proposal: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			deletingSubnetId = null;
		}
	}

	function toggleNode(nodeId: string) {
		if (selectedNodeIds.includes(nodeId)) {
			selectedNodeIds = selectedNodeIds.filter((id) => id !== nodeId);
		} else {
			selectedNodeIds = [...selectedNodeIds, nodeId];
		}
	}

	function resetForm() {
		subnetType = 'application';
		selectedNodeIds = [];
		replicaVersion = '';
	}
</script>

<svelte:head>
	<title>Subnets - Cloud Engine Controller</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="flex justify-between items-center mb-8">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Subnets</h1>
			<p class="mt-1 text-gray-600 dark:text-gray-400">
				Manage subnets via NNS proposals
			</p>
		</div>
		<button on:click={() => (showCreateModal = true)} class="btn-primary">
			<svg class="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M12 4v16m8-8H4"
				/>
			</svg>
			Create Subnet Proposal
		</button>
	</div>

	{#if loading}
		<div class="flex items-center justify-center h-64">
			<LoadingSpinner size="lg" message="Loading subnets..." />
		</div>
	{:else if $subnets.length === 0}
		<div class="card p-12 text-center">
			<svg
				class="mx-auto h-12 w-12 text-gray-400"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"
				/>
			</svg>
			<h3 class="mt-4 text-lg font-medium text-gray-900 dark:text-white">No subnets found</h3>
			<p class="mt-2 text-gray-500 dark:text-gray-400">
				You don't have any nodes assigned to subnets yet
			</p>
			<button
				on:click={() => (showCreateModal = true)}
				class="mt-4 btn-primary"
			>
				Create Subnet Proposal
			</button>
		</div>
	{:else}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each $subnets as subnet (subnet.subnet_id)}
				<SubnetCard
					{subnet}
					loading={deletingSubnetId === subnet.subnet_id}
					on:delete={(e) => deleteSubnetProposal(e.detail)}
				/>
			{/each}
		</div>
	{/if}
</div>

<!-- Create Subnet Modal -->
<Modal bind:open={showCreateModal} title="Create Subnet Proposal" on:close={() => (showCreateModal = false)}>
	<form on:submit|preventDefault={createSubnetProposal} class="space-y-4">
		<div>
			<label for="subnetType" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Subnet Type
			</label>
			<select id="subnetType" bind:value={subnetType} class="input mt-1">
				{#each subnetTypes as st}
					<option value={st}>{st}</option>
				{/each}
			</select>
		</div>

		<div>
			<label for="replicaVersion" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Replica Version (optional)
			</label>
			<input
				type="text"
				id="replicaVersion"
				bind:value={replicaVersion}
				class="input mt-1 font-mono"
				placeholder="Leave blank for latest"
			/>
		</div>

		<div>
			<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
				Select Nodes ({selectedNodeIds.length} selected)
			</label>
			{#if availableNodes.length === 0}
				<p class="text-sm text-gray-500 dark:text-gray-400">
					No available nodes. All your nodes are already assigned to subnets.
				</p>
			{:else}
				<div class="max-h-48 overflow-y-auto border border-gray-200 dark:border-gray-600 rounded-lg">
					{#each availableNodes as node (node.node_id)}
						<label
							class="flex items-center px-4 py-3 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer border-b border-gray-200 dark:border-gray-600 last:border-b-0"
						>
							<input
								type="checkbox"
								checked={selectedNodeIds.includes(node.node_id)}
								on:change={() => toggleNode(node.node_id)}
								class="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
							/>
							<div class="ml-3 min-w-0 flex-1">
								<p class="text-sm font-mono text-gray-900 dark:text-gray-100 truncate">
									{node.node_id.slice(0, 20)}...
								</p>
								<p class="text-xs text-gray-500 dark:text-gray-400">
									{node.ip_address} | {node.dc_id}
								</p>
							</div>
						</label>
					{/each}
				</div>
			{/if}
		</div>

		<div class="flex justify-end gap-3 pt-4">
			<button
				type="button"
				on:click={() => (showCreateModal = false)}
				class="btn-secondary"
			>
				Cancel
			</button>
			<button
				type="submit"
				disabled={creating || selectedNodeIds.length === 0}
				class="btn-primary"
			>
				{#if creating}
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
					Creating...
				{:else}
					Create Proposal
				{/if}
			</button>
		</div>
	</form>
</Modal>
