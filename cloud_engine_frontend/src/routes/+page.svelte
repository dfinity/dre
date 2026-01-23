<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { login } from '$lib/auth';
	import {
		isAuthenticated,
		authToken,
		principal,
		userProfile,
		vms,
		nodes,
		subnets,
		isLoading,
		setError
	} from '$lib/stores';
	import { api } from '$lib/api';
	import { LoadingSpinner } from '$lib/components';

	let loginInProgress = false;

	onMount(async () => {
		if ($isAuthenticated && $authToken) {
			await loadDashboardData();
		}
	});

	async function handleLogin() {
		loginInProgress = true;
		try {
			const result = await login();
			if (result) {
				authToken.set(result.token);
				principal.set(result.principal);
				await loadDashboardData();
			} else {
				setError('Login failed. Please try again.');
			}
		} catch (error) {
			setError('Login failed: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			loginInProgress = false;
		}
	}

	async function loadDashboardData() {
		if (!$authToken) return;

		isLoading.set(true);
		try {
			const [profileData, vmsData, nodesData, subnetsData] = await Promise.all([
				api.getProfile($authToken),
				api.listVms($authToken).catch(() => ({ vms: [], total: 0 })),
				api.listNodes($authToken).catch(() => ({ nodes: [], total: 0 })),
				api.listSubnets($authToken).catch(() => ({ subnets: [], total: 0 }))
			]);

			userProfile.set(profileData);
			vms.set(vmsData.vms);
			nodes.set(nodesData.nodes);
			subnets.set(subnetsData.subnets);
		} catch (error) {
			setError('Failed to load data: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			isLoading.set(false);
		}
	}
</script>

<svelte:head>
	<title>Dashboard - Cloud Engine Controller</title>
</svelte:head>

{#if !$isAuthenticated}
	<!-- Login Page -->
	<div class="flex flex-col items-center justify-center min-h-[80vh] px-4">
		<div class="max-w-md w-full">
			<div class="text-center mb-8">
				<svg
					class="mx-auto h-16 w-16 text-primary-600"
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
				<h1 class="mt-4 text-3xl font-bold text-gray-900 dark:text-white">
					Cloud Engine Controller
				</h1>
				<p class="mt-2 text-gray-600 dark:text-gray-400">
					Manage your GCP VMs and ICP node associations
				</p>
			</div>

			<div class="card p-8">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white text-center mb-6">
					Sign in with Internet Identity
				</h2>
				<button
					on:click={handleLogin}
					disabled={loginInProgress}
					class="w-full btn-primary py-3 text-base"
				>
					{#if loginInProgress}
						<svg class="animate-spin -ml-1 mr-3 h-5 w-5" fill="none" viewBox="0 0 24 24">
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
						Signing in...
					{:else}
						<svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
							<path
								d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm-1-13h2v6h-2zm0 8h2v2h-2z"
							/>
						</svg>
						Sign in with Internet Identity
					{/if}
				</button>

				<p class="mt-4 text-sm text-gray-500 dark:text-gray-400 text-center">
					Secure authentication powered by the Internet Computer
				</p>
			</div>

			<div class="mt-8 grid grid-cols-3 gap-4 text-center">
				<div>
					<div class="text-2xl font-bold text-primary-600">VMs</div>
					<div class="text-sm text-gray-500 dark:text-gray-400">Manage GCP</div>
				</div>
				<div>
					<div class="text-2xl font-bold text-primary-600">Nodes</div>
					<div class="text-sm text-gray-500 dark:text-gray-400">ICP Registry</div>
				</div>
				<div>
					<div class="text-2xl font-bold text-primary-600">Subnets</div>
					<div class="text-sm text-gray-500 dark:text-gray-400">NNS Proposals</div>
				</div>
			</div>
		</div>
	</div>
{:else}
	<!-- Dashboard -->
	<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		{#if $isLoading}
			<div class="flex items-center justify-center h-64">
				<LoadingSpinner size="lg" message="Loading dashboard..." />
			</div>
		{:else}
			<div class="mb-8">
				<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
				<p class="mt-1 text-gray-600 dark:text-gray-400">
					Welcome back! Here's an overview of your infrastructure.
				</p>
			</div>

			<!-- Stats Grid -->
			<div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
				<a href="/vms" class="card p-6 hover:shadow-md transition-shadow">
					<div class="flex items-center">
						<div
							class="p-3 rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400"
						>
							<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2"
								/>
							</svg>
						</div>
						<div class="ml-4">
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400">
								Virtual Machines
							</p>
							<p class="text-2xl font-bold text-gray-900 dark:text-white">
								{$vms.length}
							</p>
						</div>
					</div>
				</a>

				<a href="/nodes" class="card p-6 hover:shadow-md transition-shadow">
					<div class="flex items-center">
						<div
							class="p-3 rounded-lg bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400"
						>
							<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z"
								/>
							</svg>
						</div>
						<div class="ml-4">
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400">
								ICP Nodes
							</p>
							<p class="text-2xl font-bold text-gray-900 dark:text-white">
								{$nodes.length}
							</p>
						</div>
					</div>
				</a>

				<a href="/subnets" class="card p-6 hover:shadow-md transition-shadow">
					<div class="flex items-center">
						<div
							class="p-3 rounded-lg bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400"
						>
							<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"
								/>
							</svg>
						</div>
						<div class="ml-4">
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400">
								Subnets
							</p>
							<p class="text-2xl font-bold text-gray-900 dark:text-white">
								{$subnets.length}
							</p>
						</div>
					</div>
				</a>
			</div>

			<!-- Quick Actions -->
			<div class="card p-6">
				<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
					Quick Actions
				</h2>
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
					<a href="/vms" class="btn-primary text-center">
						Provision VM
					</a>
					<a href="/nodes" class="btn-secondary text-center">
						View Nodes
					</a>
					<a href="/subnets" class="btn-secondary text-center">
						Manage Subnets
					</a>
					<a href="/profile" class="btn-secondary text-center">
						Edit Profile
					</a>
				</div>
			</div>

			<!-- Profile Summary -->
			{#if $userProfile}
				<div class="mt-8 card p-6">
					<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
						Profile Summary
					</h2>
					<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
						<div>
							<h3 class="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase mb-2">
								GCP Account
							</h3>
							{#if $userProfile.gcp_account}
								<p class="text-gray-900 dark:text-white">
									{$userProfile.gcp_account.project_id}
								</p>
								<p class="text-sm text-gray-500 dark:text-gray-400">
									{$userProfile.gcp_account.region} / {$userProfile.gcp_account.zone}
								</p>
							{:else}
								<p class="text-gray-400 dark:text-gray-500">Not configured</p>
								<a href="/profile" class="text-sm text-primary-600 hover:text-primary-700">
									Configure now
								</a>
							{/if}
						</div>
						<div>
							<h3 class="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase mb-2">
								Node Operator
							</h3>
							{#if $userProfile.node_operator}
								<p class="text-gray-900 dark:text-white">
									{$userProfile.node_operator.display_name || 'Unnamed'}
								</p>
								<p class="text-sm font-mono text-gray-500 dark:text-gray-400 truncate">
									{$userProfile.node_operator.node_operator_id}
								</p>
							{:else}
								<p class="text-gray-400 dark:text-gray-500">Not configured</p>
								<a href="/profile" class="text-sm text-primary-600 hover:text-primary-700">
									Configure now
								</a>
							{/if}
						</div>
					</div>
				</div>
			{/if}
		{/if}
	</div>
{/if}
