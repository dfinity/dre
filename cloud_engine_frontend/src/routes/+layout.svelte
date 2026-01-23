<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { Navigation, Toast } from '$lib/components';
	import { initializeFromStorage, isAuthenticated } from '$lib/stores';

	onMount(() => {
		initializeFromStorage();
	});
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900">
	<Navigation />
	<Toast />
	<main>
		<slot />
	</main>
</div>
