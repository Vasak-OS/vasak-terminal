<script setup lang="ts">
import { ToastArea } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onMounted, ref } from 'vue';
import TerminalComponent from '@/components/terminal/TerminalComponent.vue';
import { useOverlay } from '@/composables/useOverlay';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab } from '@/types/workspaces';
import { useNotification } from '@/utils/useNotification';

const { notifications } = useNotification();
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
    class="h-screen w-screen flex flex-col p-0.5 bg-ui-shell rounded-t-corner-window overflow-hidden transition-[opacity,transform] duration-200 ease-ui-out will-change-[opacity,transform]"
    :class="isVisible ? 'translate-y-0 opacity-100' : 'translate-y-2 opacity-0'"
    @keydown.escape="hide"
  >
    <!-- La superficie del escritorio (`ui-shell`): el fondo de la ventana al
         85 %, translúcido y sin `backdrop-blur`. El desenfoque de lo de atrás
         lo pone Wayfire, y sólo se ve si la superficie deja pasar algo: en
         `ui-float`, opaca, lo tapaba (vue-libvasak `docs/once-ui.md` §13). El
         fondo de xterm es transparente (`TerminalComponent`), así que no pinta
         encima. La entrada va con los tiempos del taller —200 ms, `ease-ui-out`,
         que arranca rápido y frena— en vez de una transición escrita en `style`. -->
    <TerminalComponent
      v-for="tab in terminalTabs"
      :key="tab.id"
      v-show="tab.id === currentSessionId"
      :session-id="tab.id"
      :active="tab.id === currentSessionId"
    />
    <ToastArea :toasts="notifications" position="bottom-center" />
  </div>
</template>


