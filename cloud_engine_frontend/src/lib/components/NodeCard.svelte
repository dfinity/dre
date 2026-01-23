<script lang="ts">
	import type { NodeInfo } from '$lib/types';

	export let node: NodeInfo;

	function getStatusColor(status: string): string {
		switch (status.toLowerCase()) {
			case 'healthy':
			case 'up':
				return 'badge-success';
			case 'degraded':
			case 'warning':
				return 'badge-warning';
			case 'down':
			case 'unhealthy':
				return 'badge-danger';
			default:
				return 'badge-info';
		}
	}
</script>

<div class="card p-6 hover:shadow-md transition-shadow">
	<div class="flex justify-between items-start">
		<div class="flex-1 min-w-0">
			<div class="flex items-center gap-3">
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white truncate">
					Node
				</h3>
				<span class={getStatusColor(node.status)}>
					{node.status}
				</span>
			</div>
			<p
				class="mt-1 text-sm font-mono text-gray-500 dark:text-gray-400 truncate"
				title={node.node_id}
			>
				{node.node_id.slice(0, 24)}...
			</p>
		</div>
	</div>

	<div class="mt-4 grid grid-cols-2 gap-4">
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				IP Address
			</p>
			<p class="mt-1 text-sm font-mono text-gray-900 dark:text-gray-100">
				{node.ip_address}
			</p>
		</div>
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Data Center
			</p>
			<p class="mt-1 text-sm text-gray-900 dark:text-gray-100">{node.dc_id}</p>
		</div>
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Node Operator
			</p>
			<p class="mt-1 text-sm text-gray-900 dark:text-gray-100 truncate" title={node.node_operator_id}>
				{node.node_operator_id.slice(0, 16)}...
			</p>
		</div>
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Node Provider
			</p>
			<p class="mt-1 text-sm text-gray-900 dark:text-gray-100 truncate" title={node.node_provider_id}>
				{node.node_provider_id.slice(0, 16)}...
			</p>
		</div>
	</div>

	{#if node.subnet_id}
		<div
			class="mt-4 p-3 bg-accent-50 dark:bg-accent-900/20 rounded-lg border border-accent-200 dark:border-accent-800"
		>
			<p class="text-xs font-medium text-accent-600 dark:text-accent-400 uppercase">
				Subnet
			</p>
			<p
				class="mt-1 text-sm font-mono text-accent-900 dark:text-accent-100 truncate"
				title={node.subnet_id}
			>
				{node.subnet_id.slice(0, 24)}...
			</p>
		</div>
	{:else}
		<div
			class="mt-4 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-600"
		>
			<p class="text-sm text-gray-500 dark:text-gray-400">
				Unassigned (available for subnet)
			</p>
		</div>
	{/if}
</div>
