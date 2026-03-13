<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import TabComponent from '@/components/tab/TabComponent.vue';
import TabDraggableComponent from '@/components/tab/TabDraggableComponent.vue';
import Tooltip from '@/components/ui/tooltip/Tooltip.vue';
import TooltipContent from '@/components/ui/tooltip/TooltipContent.vue';
import TooltipTrigger from '@/components/ui/tooltip/TooltipTrigger.vue';
import { useWorkspacesStore } from '@/stores/storage/workspaces';
import type { TabGroup, Tab as TabType } from '@/types/workspaces';

const props = withDefaults(
	defineProps<{
		teleportTarget?: string;
		compact?: boolean;
	}>(),
	{
		teleportTarget: '.window-toolbar-primary-teleport-target',
		compact: false,
	}
);

const workspacesStore = useWorkspacesStore();

const teleportDisabled = computed(() => !props.teleportTarget);
const teleportTo = computed(() => props.teleportTarget || 'body');
const { openNewTabGroup, closeTabGroup, setTabs } = workspacesStore;

const previewEnabled = ref(true);
const scrollContainerRef = ref<HTMLElement | null>(null);
const plusIcon = ref('');
let scrollDisableTimeoutId: ReturnType<typeof setTimeout> | null = null;

function handleScrollActivity() {
	previewEnabled.value = false;

	if (scrollDisableTimeoutId !== null) {
		clearTimeout(scrollDisableTimeoutId);
	}

	scrollDisableTimeoutId = globalThis.setTimeout(() => {
		previewEnabled.value = true;
	}, 200);
}

function handleWheel(event: WheelEvent) {
	const container = scrollContainerRef.value;
	if (!container) return;
	container.scrollLeft += event.deltaY || event.deltaX || 0;
	handleScrollActivity();
}

function onScroll() {
	handleScrollActivity();
}

onMounted(async () => {
	plusIcon.value = await getSymbolSource('gtk-add');
});

onBeforeUnmount(() => {
	if (scrollDisableTimeoutId !== null) {
		clearTimeout(scrollDisableTimeoutId);
	}
});
</script>

<template>
  <Teleport :to="teleportTo" :disabled="teleportDisabled">
    <div class="flex max-w-[calc(100vw-288px)] h-full items-center gap-1 animate-sigma-ui-fade-in">
      <div ref="scrollContainerRef" class="flex overflow-auto items-center" @wheel.prevent="handleWheel" @scroll="onScroll">
        <div class="tab-bar__base flex w-fit items-center justify-center h-fit">
          <TabDraggableComponent :items="workspacesStore.currentWorkspace?.tabGroups || []"
            :draggable-bg-color-var="'window-toolbar-color'" parent-selector=".tab-bar"
            @set="setTabs($event as TabGroup[])" @drag-start="previewEnabled = false" @drag-end="previewEnabled = true">
            <template #item="{ item }">
              <TabComponent :tab-group="((item as TabType[]) || [])" :preview-enabled="previewEnabled"
                @close-tab="closeTabGroup($event)" />
            </template>
          </TabDraggableComponent>
        </div>
      </div>

      <Tooltip>
        <TooltipTrigger as-child>
          <button class="rounded-corner p-1 flex justify-center items-center bg-primary text-tx-on-primary h-5 w-5" variant="ghost" size="xs" @click="openNewTabGroup()">
            <img v-if="plusIcon" :src="plusIcon" alt="Add Tab" class="w-3.5 h-3.5" />
          </button>
        </TooltipTrigger>
        <TooltipContent>
          'tabs.newTab'
        </TooltipContent>
      </Tooltip>
    </div>
  </Teleport>
</template>
