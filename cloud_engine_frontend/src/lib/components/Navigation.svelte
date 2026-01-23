<script lang="ts">
	import { page } from '$app/stores';
	import { isAuthenticated, principal, clearAllState } from '$lib/stores';
	import { logout } from '$lib/auth';
	import { get } from 'svelte/store';

	const navItems = [
		{ path: '/', label: 'Dashboard', icon: 'home' },
		{ path: '/vms', label: 'VMs', icon: 'server' },
		{ path: '/nodes', label: 'Nodes', icon: 'cpu' },
		{ path: '/subnets', label: 'Subnets', icon: 'network' },
		{ path: '/profile', label: 'Profile', icon: 'user' }
	];

	let mobileMenuOpen = false;

	async function handleLogout() {
		const token = localStorage.getItem('auth_token');
		await logout(token || undefined);
		clearAllState();
		window.location.href = '/';
	}

	function truncatePrincipal(p: string): string {
		if (p.length <= 16) return p;
		return `${p.slice(0, 8)}...${p.slice(-8)}`;
	}
</script>

<nav class="bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700">
	<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
		<div class="flex justify-between h-16">
			<div class="flex">
				<div class="flex-shrink-0 flex items-center">
					<svg
						class="h-8 w-8 text-primary-600"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
						/>
					</svg>
					<span class="ml-2 text-xl font-bold text-gray-900 dark:text-white"
						>Cloud Engine</span
					>
				</div>

				{#if $isAuthenticated}
					<div class="hidden sm:ml-6 sm:flex sm:space-x-4">
						{#each navItems as item}
							<a
								href={item.path}
								class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors
                  {$page.url.pathname === item.path
									? 'text-primary-600 bg-primary-50 dark:bg-primary-900/20'
									: 'text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700'}"
							>
								{item.label}
							</a>
						{/each}
					</div>
				{/if}
			</div>

			<div class="flex items-center">
				{#if $isAuthenticated}
					<div class="hidden sm:flex items-center space-x-4">
						<span class="text-sm text-gray-500 dark:text-gray-400">
							{truncatePrincipal($principal || '')}
						</span>
						<button on:click={handleLogout} class="btn-secondary text-sm">
							Logout
						</button>
					</div>
				{/if}

				<!-- Mobile menu button -->
				<div class="sm:hidden">
					<button
						on:click={() => (mobileMenuOpen = !mobileMenuOpen)}
						class="inline-flex items-center justify-center p-2 rounded-md text-gray-400 hover:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700"
					>
						<svg
							class="h-6 w-6"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
						>
							{#if mobileMenuOpen}
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							{:else}
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M4 6h16M4 12h16M4 18h16"
								/>
							{/if}
						</svg>
					</button>
				</div>
			</div>
		</div>
	</div>

	<!-- Mobile menu -->
	{#if mobileMenuOpen && $isAuthenticated}
		<div class="sm:hidden border-t border-gray-200 dark:border-gray-700">
			<div class="pt-2 pb-3 space-y-1">
				{#each navItems as item}
					<a
						href={item.path}
						on:click={() => (mobileMenuOpen = false)}
						class="block px-4 py-2 text-base font-medium
              {$page.url.pathname === item.path
							? 'text-primary-600 bg-primary-50 dark:bg-primary-900/20'
							: 'text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700'}"
					>
						{item.label}
					</a>
				{/each}
			</div>
			<div class="pt-4 pb-3 border-t border-gray-200 dark:border-gray-700">
				<div class="px-4 space-y-2">
					<p class="text-sm text-gray-500 dark:text-gray-400">
						{truncatePrincipal($principal || '')}
					</p>
					<button
						on:click={handleLogout}
						class="w-full btn-secondary text-sm"
					>
						Logout
					</button>
				</div>
			</div>
		</div>
	{/if}
</nav>
