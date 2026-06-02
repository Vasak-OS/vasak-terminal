<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { useWorkspacesStore } from '@/stores/workspaces';

const unListenConfig = ref<UnlistenFn | null>(null);
const workspacesStore = useWorkspacesStore();

onMounted(async () => {
	try {
		await workspacesStore.init();

		const configStore = useConfigStore() as Store<
			'config',
			{ config: any; loadConfig: () => Promise<void> }
		>;
		await configStore.loadConfig();

		unListenConfig.value = await listen('config-changed', async () => {
			document.startViewTransition(() => {
				configStore.loadConfig();
			});
		});
	} catch (error: any) {
		console.error('Error al cargar configuración en App.vue', error);
	}
});

onUnmounted(() => {
	if (unListenConfig.value !== null) {
		unListenConfig.value();
	}
});
</script>

<template>
  <WindowAppLayout />
</template>
