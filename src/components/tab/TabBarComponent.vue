<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import TabComponent from '@/components/tab/TabComponent.vue';
import TabDraggableComponent from '@/components/tab/TabDraggableComponent.vue';
import Tooltip from '@/components/ui/tooltip/Tooltip.vue';
import TooltipContent from '@/components/ui/tooltip/TooltipContent.vue';
import TooltipTrigger from '@/components/ui/tooltip/TooltipTrigger.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { TabGroup, Tab as TabType } from '@/types/workspaces';
import { useReactiveIcon } from '@/utils/useReactiveIcon';

const props = withDefaults(
	defineProps<{
		teleportTarget?: string;
	}>(),
	{
		teleportTarget: '.window-toolbar-primary-teleport-target',
	}
);

const workspacesStore = useWorkspacesStore();

const teleportDisabled = computed(() => !props.teleportTarget);
const teleportTo = computed(() => props.teleportTarget || 'body');
const { openNewTabGroup, closeTabGroup, setTabs } = workspacesStore;

const { plusIcon } = useReactiveIcon({ plusIcon: 'gtk-add' });
const previewEnabled = ref(true);
const scrollContainerRef = ref<HTMLElement | null>(null);
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
}

function onScroll() {
	handleScrollActivity();
}

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
