<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isAuthenticated, authToken, vms, setError, setSuccess } from '$lib/stores';
	import { api } from '$lib/api';
	import { LoadingSpinner, VmCard, Modal } from '$lib/components';
	import type { VmProvisionRequest } from '$lib/types';

	let loading = true;
	let provisioning = false;
	let deletingVmId: string | null = null;
	let showProvisionModal = false;

	// Provision form state
	let vmName = '';
	let machineType = 'n1-standard-4';
	let zone = 'us-central1-a';
	let diskSizeGb = 100;
	let imageFamily = 'ubuntu-2204-lts';

	const machineTypes = [
		{ value: 'n1-standard-2', label: 'n1-standard-2 (2 vCPU, 7.5 GB)' },
		{ value: 'n1-standard-4', label: 'n1-standard-4 (4 vCPU, 15 GB)' },
		{ value: 'n1-standard-8', label: 'n1-standard-8 (8 vCPU, 30 GB)' },
		{ value: 'n1-standard-16', label: 'n1-standard-16 (16 vCPU, 60 GB)' },
		{ value: 'n2-standard-4', label: 'n2-standard-4 (4 vCPU, 16 GB)' },
		{ value: 'n2-standard-8', label: 'n2-standard-8 (8 vCPU, 32 GB)' }
	];

	const zones = [
		'us-central1-a',
		'us-central1-b',
		'us-central1-c',
		'us-east1-b',
		'us-east1-c',
		'us-west1-a',
		'us-west1-b',
		'europe-west1-b',
		'europe-west1-c'
	];

	const imageFamilies = [
		{ value: 'ubuntu-2204-lts', label: 'Ubuntu 22.04 LTS' },
		{ value: 'ubuntu-2004-lts', label: 'Ubuntu 20.04 LTS' },
		{ value: 'debian-11', label: 'Debian 11' },
		{ value: 'debian-12', label: 'Debian 12' }
	];

	onMount(async () => {
		if (!$isAuthenticated) {
			goto('/');
			return;
		}
		await loadVms();
	});

	async function loadVms() {
		loading = true;
		try {
			const response = await api.listVms($authToken!);
			vms.set(response.vms);
		} catch (error) {
			setError('Failed to load VMs: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			loading = false;
		}
	}

	async function provisionVm() {
		if (!vmName.trim()) {
			setError('VM name is required');
			return;
		}

		provisioning = true;
		try {
			const request: VmProvisionRequest = {
				name: vmName,
				machine_type: machineType,
				zone,
				disk_size_gb: diskSizeGb,
				image_family: imageFamily
			};
			const response = await api.provisionVm($authToken!, request);
			vms.update((current) => [...current, response.vm]);
			setSuccess('VM provisioning started');
			showProvisionModal = false;
			resetForm();
		} catch (error) {
			setError('Failed to provision VM: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			provisioning = false;
		}
	}

	async function deleteVm(vmId: string) {
		if (!confirm('Are you sure you want to delete this VM? This action cannot be undone.')) {
			return;
		}

		deletingVmId = vmId;
		try {
			await api.deleteVm($authToken!, vmId);
			vms.update((current) => current.filter((vm) => vm.id !== vmId));
			setSuccess('VM deleted successfully');
		} catch (error) {
			setError('Failed to delete VM: ' + (error instanceof Error ? error.message : 'Unknown error'));
		} finally {
			deletingVmId = null;
		}
	}

	function resetForm() {
		vmName = '';
		machineType = 'n1-standard-4';
		zone = 'us-central1-a';
		diskSizeGb = 100;
		imageFamily = 'ubuntu-2204-lts';
	}
</script>

<svelte:head>
	<title>VMs - Cloud Engine Controller</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="flex justify-between items-center mb-8">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Virtual Machines</h1>
			<p class="mt-1 text-gray-600 dark:text-gray-400">
				Manage your GCP VMs and their ICP node associations
			</p>
		</div>
		<button on:click={() => (showProvisionModal = true)} class="btn-primary">
			<svg class="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M12 4v16m8-8H4"
				/>
			</svg>
			Provision VM
		</button>
	</div>

	{#if loading}
		<div class="flex items-center justify-center h-64">
			<LoadingSpinner size="lg" message="Loading VMs..." />
		</div>
	{:else if $vms.length === 0}
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
					d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2"
				/>
			</svg>
			<h3 class="mt-4 text-lg font-medium text-gray-900 dark:text-white">No VMs found</h3>
			<p class="mt-2 text-gray-500 dark:text-gray-400">
				Get started by provisioning your first virtual machine
			</p>
			<button
				on:click={() => (showProvisionModal = true)}
				class="mt-4 btn-primary"
			>
				Provision VM
			</button>
		</div>
	{:else}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each $vms as vm (vm.id)}
				<VmCard
					{vm}
					loading={deletingVmId === vm.id}
					on:delete={(e) => deleteVm(e.detail)}
				/>
			{/each}
		</div>
	{/if}
</div>

<!-- Provision Modal -->
<Modal bind:open={showProvisionModal} title="Provision New VM" on:close={() => (showProvisionModal = false)}>
	<form on:submit|preventDefault={provisionVm} class="space-y-4">
		<div>
			<label for="vmName" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				VM Name *
			</label>
			<input
				type="text"
				id="vmName"
				bind:value={vmName}
				class="input mt-1"
				placeholder="my-icp-node"
				required
			/>
		</div>

		<div>
			<label for="machineType" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Machine Type
			</label>
			<select id="machineType" bind:value={machineType} class="input mt-1">
				{#each machineTypes as mt}
					<option value={mt.value}>{mt.label}</option>
				{/each}
			</select>
		</div>

		<div>
			<label for="zone" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Zone
			</label>
			<select id="zone" bind:value={zone} class="input mt-1">
				{#each zones as z}
					<option value={z}>{z}</option>
				{/each}
			</select>
		</div>

		<div>
			<label for="diskSize" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Disk Size (GB)
			</label>
			<input
				type="number"
				id="diskSize"
				bind:value={diskSizeGb}
				min="50"
				max="2000"
				class="input mt-1"
			/>
		</div>

		<div>
			<label for="imageFamily" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				Image Family
			</label>
			<select id="imageFamily" bind:value={imageFamily} class="input mt-1">
				{#each imageFamilies as img}
					<option value={img.value}>{img.label}</option>
				{/each}
			</select>
		</div>

		<div class="flex justify-end gap-3 pt-4">
			<button
				type="button"
				on:click={() => (showProvisionModal = false)}
				class="btn-secondary"
			>
				Cancel
			</button>
			<button type="submit" disabled={provisioning} class="btn-primary">
				{#if provisioning}
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
					Provisioning...
				{:else}
					Provision
				{/if}
			</button>
		</div>
	</form>
</Modal>
