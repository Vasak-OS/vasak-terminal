<script lang="ts" setup>
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab } from '@/types/workspaces';

const workspacesStore = useWorkspacesStore();
const currentSessionId = computed(() => workspacesStore.currentTab?.id ?? '');
const terminalTabs = computed<Tab[]>(() =>
	(workspacesStore.currentWorkspace?.tabGroups ?? [])
		.map((tabGroup) => tabGroup?.[0])
		.filter((tab): tab is Tab => Boolean(tab))
);
const terminalIcon = ref('');

onMounted(async () => {
	terminalIcon.value = await getIconSource('terminal');
});
</script>
<template>
  <div
    class="h-screen w-screen bg-ui-bg/80 rounded-corner-window flex flex-col border border-ui-border overflow-hidden">
    <TopBarComponent>
      <img :src="terminalIcon" alt="Terminal Icon" class="w-7 h-7 mr-2" />
      <TabBarComponent teleport-target="" />
    </TopBarComponent>
    <div class="flex-1 flex p-1">
      <TerminalComponent
        v-for="tab in terminalTabs"
        :key="tab.id"
        v-show="tab.id === currentSessionId"
        :session-id="tab.id"
        :active="tab.id === currentSessionId"
      />
    </div>
  </div>
</template>
