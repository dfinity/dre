<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { fade, scale } from 'svelte/transition';

	export let open: boolean = false;
	export let title: string = '';

	const dispatch = createEventDispatcher<{ close: void }>();

	function handleClose() {
		dispatch('close');
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			handleClose();
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 overflow-y-auto"
		aria-labelledby="modal-title"
		role="dialog"
		aria-modal="true"
	>
		<div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0">
			<!-- Background overlay -->
			<div
				transition:fade={{ duration: 200 }}
				class="fixed inset-0 bg-gray-500 dark:bg-gray-900 bg-opacity-75 dark:bg-opacity-75 transition-opacity"
				on:click={handleClose}
				on:keydown={handleKeydown}
				role="button"
				tabindex="0"
				aria-label="Close modal"
			></div>

			<!-- Center modal -->
			<span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true"
				>&#8203;</span
			>

			<!-- Modal panel -->
			<div
				transition:scale={{ start: 0.95, duration: 200 }}
				class="relative inline-block align-bottom bg-white dark:bg-gray-800 rounded-lg px-4 pt-5 pb-4 text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full sm:p-6"
			>
				<div class="absolute top-0 right-0 pt-4 pr-4">
					<button
						on:click={handleClose}
						class="rounded-md text-gray-400 hover:text-gray-500 dark:hover:text-gray-300 focus:outline-none focus:ring-2 focus:ring-primary-500"
					>
						<span class="sr-only">Close</span>
						<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
				</div>

				{#if title}
					<h3
						class="text-lg font-semibold text-gray-900 dark:text-white mb-4"
						id="modal-title"
					>
						{title}
					</h3>
				{/if}

				<slot />
			</div>
		</div>
	</div>
{/if}
