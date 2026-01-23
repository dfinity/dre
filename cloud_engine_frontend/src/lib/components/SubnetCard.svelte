<script lang="ts">
	import type { SubnetInfo } from '$lib/types';
	import { createEventDispatcher } from 'svelte';

	export let subnet: SubnetInfo;
	export let loading: boolean = false;

	const dispatch = createEventDispatcher<{ delete: string }>();
</script>

<div class="card p-6 hover:shadow-md transition-shadow">
	<div class="flex justify-between items-start">
		<div class="flex-1 min-w-0">
			<div class="flex items-center gap-3">
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
					Subnet
				</h3>
				<span class="badge-info">
					{subnet.subnet_type}
				</span>
			</div>
			<p
				class="mt-1 text-sm font-mono text-gray-500 dark:text-gray-400 truncate"
				title={subnet.subnet_id}
			>
				{subnet.subnet_id.slice(0, 24)}...
			</p>
		</div>
	</div>

	<div class="mt-4 grid grid-cols-2 gap-4">
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Nodes
			</p>
			<p class="mt-1 text-2xl font-bold text-gray-900 dark:text-gray-100">
				{subnet.node_count}
			</p>
		</div>
		<div>
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
				Replica Version
			</p>
			<p
				class="mt-1 text-sm font-mono text-gray-900 dark:text-gray-100 truncate"
				title={subnet.replica_version}
			>
				{subnet.replica_version.slice(0, 16)}...
			</p>
		</div>
	</div>

	<div class="mt-4 flex justify-end">
		<button
			on:click={() => dispatch('delete', subnet.subnet_id)}
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
			Delete Proposal
		</button>
	</div>
</div>
