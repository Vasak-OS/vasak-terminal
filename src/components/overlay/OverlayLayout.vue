<script setup lang="ts">
import { computed, ref, onMounted, nextTick } from 'vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import NotificationToast from '@/components/ui/notification/NotificationToast.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import { useOverlay } from '@/composables/useOverlay';
import type { Tab } from '@/types/workspaces';

const workspacesStore = useWorkspacesStore();
const { hide, isVisible } = useOverlay();

const currentSessionId = computed(() => workspacesStore.currentTab?.id ?? '');
const terminalTabs = computed<Tab[]>(() =>
	(workspacesStore.currentWorkspace?.tabGroups ?? [])
		.map((tabGroup) => tabGroup?.[0])
		.filter((tab): tab is Tab => Boolean(tab))
);

const rootEl = ref<HTMLElement | null>(null);

onMounted(async () => {
	await nextTick();
	rootEl.value?.focus();
});
</script>

<template>
  <div
    ref="rootEl"
    tabindex="-1"
    class="h-screen w-screen flex flex-col p-0.5 bg-ui-bg/80 rounded-t-corner-window overflow-hidden"
    :style="{
      opacity: isVisible ? 1 : 0,
      transform: isVisible ? 'translateY(0)' : 'translateY(8px)',
      transition: 'opacity 200ms ease-out, transform 200ms ease-out',
      willChange: 'opacity, transform',
    }"
    @keydown.escape="hide"
  >
    <TerminalComponent
      v-for="tab in terminalTabs"
      :key="tab.id"
      v-show="tab.id === currentSessionId"
      :session-id="tab.id"
      :active="tab.id === currentSessionId"
    />
    <NotificationToast />
  </div>
</template>


