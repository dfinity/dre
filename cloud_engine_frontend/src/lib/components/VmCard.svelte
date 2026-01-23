<script lang="ts">
	import type { Vm } from '$lib/types';
	import { createEventDispatcher } from 'svelte';

	export let vm: Vm;
	export let loading: boolean = false;

	const dispatch = createEventDispatcher<{ delete: string }>();

	function getStatusColor(status: string): string {
		switch (status.toLowerCase()) {
			case 'running':
				return 'badge-success';
			case 'stopped':
			case 'terminated':
				return 'badge-danger';
			case 'pending':
			case 'staging':
				return 'badge-warning';
			default:
				return 'badge-info';
		}
	}
</script>

<div class="card p-6 hover:shadow-md transition-shadow">
	<div class="flex justify-between items-start">
		<div class="flex-1">
			<div class="flex items-center gap-3">
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
					{vm.name}
				</h3>
				<span class={getStatusColor(vm.status)}>
					{vm.status}
				</span>
			</div>
			<p class="mt-1 text-sm text-gray-500 dark:text-gray-400">{vm.id}</p>
		</div>
	</div>

	<div class="mt-4 grid grid-cols-2 gap-4">
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Machine Type
			</p>
			<p class="mt-1 text-sm text-gray-900 dark:text-gray-100">
				{vm.machine_type}
			</p>
		</div>
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Zone
			</p>
			<p class="mt-1 text-sm text-gray-900 dark:text-gray-100">{vm.zone}</p>
		</div>
		{#if vm.external_ip}
			<div>
				<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
					External IP
				</p>
				<p class="mt-1 text-sm font-mono text-gray-900 dark:text-gray-100">
					{vm.external_ip}
				</p>
			</div>
		{/if}
		{#if vm.internal_ip}
			<div>
				<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
					Internal IP
				</p>
				<p class="mt-1 text-sm font-mono text-gray-900 dark:text-gray-100">
					{vm.internal_ip}
				</p>
			</div>
		{/if}
	</div>

	{#if vm.icp_node}
		<div
			class="mt-4 p-3 bg-primary-50 dark:bg-primary-900/20 rounded-lg border border-primary-200 dark:border-primary-800"
		>
			<p class="text-xs font-medium text-primary-600 dark:text-primary-400 uppercase">
				ICP Node Mapping
			</p>
			<p class="mt-1 text-sm font-mono text-primary-900 dark:text-primary-100">
				{vm.icp_node.node_id.slice(0, 20)}...
			</p>
			{#if vm.icp_node.subnet_id}
				<p class="mt-1 text-xs text-primary-600 dark:text-primary-400">
					Subnet: {vm.icp_node.subnet_id.slice(0, 16)}...
				</p>
			{/if}
		</div>
	{/if}

	<div class="mt-4 flex justify-between items-center">
		<p class="text-xs text-gray-400 dark:text-gray-500">
			Created: {new Date(vm.created_at).toLocaleDateString()}
		</p>
		<button
			on:click={() => dispatch('delete', vm.id)}
			disabled={loading}
			class="btn-danger text-xs"
		>
			{#if loading}
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
			{/if}
			Delete
		</button>
	</div>
</div>
