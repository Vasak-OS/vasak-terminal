<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import OverlayLayout from '@/components/overlay/OverlayLayout.vue';
import { useWorkspacesStore } from '@/stores/workspaces';

const isOverlay = ref(false);
const unListenConfig = ref<UnlistenFn | null>(null);
const workspacesStore = useWorkspacesStore();

onMounted(async () => {
	try {
		isOverlay.value = await invoke<boolean>('is_overlay_mode');

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
  <WindowAppLayout v-if="!isOverlay" />
  <OverlayLayout v-else />
</template>
