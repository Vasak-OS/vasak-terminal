<script setup lang="ts">
import { computed, inject, nextTick, ref, watch } from 'vue';

interface Props {
	side?: 'top' | 'bottom' | 'left' | 'right';
	align?: 'start' | 'center' | 'end';
	sideOffset?: number;
	delayDuration?: number;
}

const props = withDefaults(defineProps<Props>(), {
	side: 'bottom',
	align: 'center',
	sideOffset: 4,
	delayDuration: 200,
});

const tooltip = inject<any>('tooltip');
const triggerElement = computed(() => tooltip?.triggerElement?.value ?? null);

const isOpen = computed(() => tooltip?.isOpen.value ?? false);
const contentRef = ref<HTMLElement | null>(null);
const position = ref({ top: 0, left: 0 });

const calculatePosition = () => {
	if (!triggerElement.value || !contentRef.value) return;

	const triggerRect = triggerElement.value.getBoundingClientRect();
	const contentRect = contentRef.value.getBoundingClientRect();

	let top = 0;
	let left = 0;

	switch (props.side) {
		case 'bottom':
			top = triggerRect.bottom + props.sideOffset;
			left = triggerRect.left + triggerRect.width / 2 - contentRect.width / 2;
			break;
		case 'top':
			top = triggerRect.top - contentRect.height - props.sideOffset;
			left = triggerRect.left + triggerRect.width / 2 - contentRect.width / 2;
			break;
		case 'left':
			left = triggerRect.left - contentRect.width - props.sideOffset;
			top = triggerRect.top + triggerRect.height / 2 - contentRect.height / 2;
			break;
		case 'right':
			left = triggerRect.right + props.sideOffset;
			top = triggerRect.top + triggerRect.height / 2 - contentRect.height / 2;
			break;
	}

	position.value = { top, left };
};

watch(isOpen, async (open) => {
	if (open) {
		await nextTick();
		requestAnimationFrame(() => {
			calculatePosition();
		});
	}
});
</script>

<template>
	<Teleport to="body">
		<Transition
			enter-active-class="transition-all duration-200 ease-in-out"
			leave-active-class="transition-all duration-200 ease-in-out"
			enter-from-class="opacity-0 scale-95"
			leave-to-class="opacity-0 scale-95"
			@enter="(el) => (el as HTMLElement).offsetHeight"
			@leave="(el) => (el as HTMLElement).offsetHeight"
		>
			<div
				v-show="isOpen"
				ref="contentRef"
				:style="{ position: 'fixed', top: `${position.top}px`, left: `${position.left}px`, zIndex: 50 }"
				class="pointer-events-none whitespace-nowrap rounded-corner px-2 py-1 text-sm shadow-md bg-ui-bg/80 border border-secondary"
			>
				<slot />
			</div>
		</Transition>
	</Teleport>
</template>


