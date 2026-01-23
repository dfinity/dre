<script lang="ts">
	import { errorMessage, successMessage } from '$lib/stores';
	import { fade, fly } from 'svelte/transition';
</script>

{#if $errorMessage}
	<div
		transition:fly={{ y: -20, duration: 300 }}
		class="fixed top-4 right-4 z-50 max-w-md"
	>
		<div
			class="bg-red-50 dark:bg-red-900/50 border border-red-200 dark:border-red-800 rounded-lg p-4 shadow-lg"
		>
			<div class="flex items-start">
				<div class="flex-shrink-0">
					<svg
						class="h-5 w-5 text-red-400"
						viewBox="0 0 20 20"
						fill="currentColor"
					>
						<path
							fill-rule="evenodd"
							d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
							clip-rule="evenodd"
						/>
					</svg>
				</div>
				<div class="ml-3">
					<p class="text-sm font-medium text-red-800 dark:text-red-200">
						{$errorMessage}
					</p>
				</div>
				<button
					on:click={() => errorMessage.set(null)}
					class="ml-auto -mx-1.5 -my-1.5 p-1.5 rounded-lg text-red-500 hover:bg-red-100 dark:hover:bg-red-800"
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</div>
		</div>
	</div>
{/if}

{#if $successMessage}
	<div
		transition:fly={{ y: -20, duration: 300 }}
		class="fixed top-4 right-4 z-50 max-w-md"
	>
		<div
			class="bg-green-50 dark:bg-green-900/50 border border-green-200 dark:border-green-800 rounded-lg p-4 shadow-lg"
		>
			<div class="flex items-start">
				<div class="flex-shrink-0">
					<svg
						class="h-5 w-5 text-green-400"
						viewBox="0 0 20 20"
						fill="currentColor"
					>
						<path
							fill-rule="evenodd"
							d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
							clip-rule="evenodd"
						/>
					</svg>
				</div>
				<div class="ml-3">
					<p class="text-sm font-medium text-green-800 dark:text-green-200">
						{$successMessage}
					</p>
				</div>
				<button
					on:click={() => successMessage.set(null)}
					class="ml-auto -mx-1.5 -my-1.5 p-1.5 rounded-lg text-green-500 hover:bg-green-100 dark:hover:bg-green-800"
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</div>
		</div>
	</div>
{/if}
