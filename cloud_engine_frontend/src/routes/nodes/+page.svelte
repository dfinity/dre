<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isAuthenticated, authToken, nodes, setError } from '$lib/stores';
	import { api } from '$lib/api';
	import { LoadingSpinner, NodeCard } from '$lib/components';

	let loading = true;
	let searchQuery = '';

	$: filteredNodes = $nodes.filter((node) => {
		if (!searchQuery) return true;
		const query = searchQuery.toLowerCase();
		return (
			node.node_id.toLowerCase().includes(query) ||
			node.ip_address.toLowerCase().includes(query) ||
			node.dc_id.toLowerCase().includes(query) ||
			node.node_operator_id.toLowerCase().includes(query) ||
			(node.subnet_id && node.subnet_id.toLowerCase().includes(query))
		);
	});

	onMount(async () => {
		if (!$isAuthenticated) {
			goto('/');
			return;
		}
		await loadNodes();
	});

	async function loadNodes() {
		loading = true;
		try {
			const response = await api.listNodes($authToken!);
			nodes.set(response.nodes);
		} catch (error) {
			setError('Failed to load nodes: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Nodes - Cloud Engine Controller</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-8">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">ICP Nodes</h1>
			<p class="mt-1 text-gray-600 dark:text-gray-400">
				View your ICP nodes from the NNS registry
			</p>
		</div>
		<div class="w-full sm:w-auto">
			<input
				type="search"
				bind:value={searchQuery}
				placeholder="Search nodes..."
				class="input w-full sm:w-64"
			/>
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center h-64">
			<LoadingSpinner size="lg" message="Loading nodes..." />
		</div>
	{:else if $nodes.length === 0}
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
					d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z"
				/>
			</svg>
			<h3 class="mt-4 text-lg font-medium text-gray-900 dark:text-white">No nodes found</h3>
			<p class="mt-2 text-gray-500 dark:text-gray-400">
				Configure your node operator ID in your profile to view your nodes
			</p>
			<a href="/profile" class="mt-4 btn-primary inline-block">
				Configure Profile
			</a>
		</div>
	{:else if filteredNodes.length === 0}
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
					d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
				/>
			</svg>
			<h3 class="mt-4 text-lg font-medium text-gray-900 dark:text-white">
				No nodes match your search
			</h3>
			<p class="mt-2 text-gray-500 dark:text-gray-400">
				Try adjusting your search terms
			</p>
			<button on:click={() => (searchQuery = '')} class="mt-4 btn-secondary">
				Clear Search
			</button>
		</div>
	{:else}
		<div class="mb-4 text-sm text-gray-500 dark:text-gray-400">
			Showing {filteredNodes.length} of {$nodes.length} nodes
		</div>
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each filteredNodes as node (node.node_id)}
				<NodeCard {node} />
			{/each}
		</div>
	{/if}
</div>
