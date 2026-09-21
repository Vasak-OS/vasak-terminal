<script lang="ts" setup>
import { ThemeIcon, ToastArea, WindowFrame } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab } from '@/types/workspaces';
import { useNotification } from '@/utils/useNotification';

const { notifications } = useNotification();
const workspacesStore = useWorkspacesStore();
const currentSessionId = computed(() => workspacesStore.currentTab?.id ?? '');
const terminalTabs = computed<Tab[]>(() =>
	(workspacesStore.currentWorkspace?.tabGroups ?? [])
		.map((tabGroup) => tabGroup?.[0])
		.filter((tab): tab is Tab => Boolean(tab))
);
</script>
<template>
  <WindowFrame>
    <template #identidad>
      <!-- Decorativo: el nombre de la ventana lo dice el gestor de ventanas, y
           un `alt` que lo repita se lo hace leer dos veces a un lector de
           pantalla. `ThemeIcon` deja el `alt` vacío por omisión, que es lo que
           hace que un lector lo ignore.

           A color y no el símbolo: es la identidad de la ventana, como en el
           resto del escritorio. `icon` es lo que `ThemeIcon` trae por omisión,
           pero acá va escrito porque es una decisión y no un descuido. -->
      <ThemeIcon name="terminal" type="icon" :size="28" />
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
    <ToastArea :toasts="notifications" position="bottom-center" />
  </WindowFrame>
</template>
