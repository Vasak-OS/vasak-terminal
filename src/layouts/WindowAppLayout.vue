<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { WindowFrame } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import NotificationToast from '@/components/ui/notification/NotificationToast.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab } from '@/types/workspaces';
import { useReactiveIcon } from '@/utils/useReactiveIcon';

const { t } = useI18n();
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
  <WindowFrame
    :minimize-label="t('windowControls.minimize')"
    :maximize-label="t('windowControls.maximize')"
    :close-label="t('windowControls.close')">
    <template #identidad>
      <!-- Decorativo: el nombre de la ventana lo dice el gestor de ventanas, y
           un `alt` que lo repita se lo hace leer dos veces a un lector de
           pantalla. -->
      <img :src="terminalIcon" alt="" aria-hidden="true" class="h-7 w-7" />
    </template>

    <template #barra>
      <TabBarComponent />
    </template>

    <div class="flex min-h-0 min-w-0 flex-1 p-1">
      <TerminalComponent
        v-for="tab in terminalTabs"
        :key="tab.id"
        v-show="tab.id === currentSessionId"
        :session-id="tab.id"
        :active="tab.id === currentSessionId"
      />
    </div>
    <NotificationToast />
  </WindowFrame>
</template>
