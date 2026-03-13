<script lang="ts" setup>
import { computed } from 'vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import { useWorkspacesStore } from '@/stores/workspaces';

const workspacesStore = useWorkspacesStore();
const currentSessionId = computed(() => workspacesStore.currentTab?.id ?? '');
</script>
<template>
  <div
    class="h-screen w-screen bg-ui-bg/80 rounded-corner-window flex flex-col border border-ui-border overflow-hidden">
    <TopBarComponent>
      <TabBarComponent teleport-target="" />
    </TopBarComponent>
    <div class="flex-1 flex p-1">
      <TerminalComponent v-if="currentSessionId" :key="currentSessionId" :session-id="currentSessionId" />
    </div>
  </div>
</template>
