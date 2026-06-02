<script lang="ts" setup>
import { computed } from 'vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import { useReactiveIcon } from '@/utils/useReactiveIcon';
import type { Tab } from '@/types/workspaces';

const workspacesStore = useWorkspacesStore();
const currentSessionId = computed(() => workspacesStore.currentTab?.id ?? '');
const terminalTabs = computed<Tab[]>(() =>
	(workspacesStore.currentWorkspace?.tabGroups ?? [])
		.map((tabGroup) => tabGroup?.[0])
		.filter((tab): tab is Tab => Boolean(tab))
);
const { terminalIcon } = useReactiveIcon({ terminalIcon: { name: 'terminal', type: 'icon' } });
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
