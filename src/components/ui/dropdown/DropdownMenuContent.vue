<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

interface Props {
	side?: 'top' | 'bottom' | 'left' | 'right';
	align?: 'start' | 'center' | 'end';
	sideOffset?: number;
	class?: string;
}

const props = withDefaults(defineProps<Props>(), {
	side: 'bottom',
	align: 'start',
	sideOffset: 4,
});

const dropdown = inject<any>('dropdown');
const triggerElement = computed(() => dropdown?.triggerElement?.value ?? null);

const isOpen = computed(() => dropdown?.isOpen.value ?? false);
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
			break;
		case 'top':
			top = triggerRect.top - contentRect.height - props.sideOffset;
			break;
		case 'left':
			left = triggerRect.left - contentRect.width - props.sideOffset;
			top = triggerRect.top;
			break;
		case 'right':
			left = triggerRect.right + props.sideOffset;
			top = triggerRect.top;
			break;
	}

	switch (props.align) {
		case 'start':
			if (props.side === 'bottom' || props.side === 'top') {
				left = triggerRect.left;
			}
			break;
		case 'center':
			if (props.side === 'bottom' || props.side === 'top') {
				left = triggerRect.left + triggerRect.width / 2 - contentRect.width / 2;
			}
			break;
		case 'end':
			if (props.side === 'bottom' || props.side === 'top') {
				left = triggerRect.right - contentRect.width;
			}
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

const updatePositionIfOpen = () => {
	if (isOpen.value) {
		calculatePosition();
	}
};

const handleClickOutside = (event: MouseEvent) => {
	const target = event.target as HTMLElement;
	const clickedInsideContent = !!target.closest('[dropdown-content]');
	const clickedOnTrigger = !!triggerElement.value?.contains(target);

	if (!clickedInsideContent && !clickedOnTrigger) {
		dropdown?.closeDropdown();
	}
};

onMounted(() => {
	document.addEventListener('click', handleClickOutside);
	window.addEventListener('resize', updatePositionIfOpen);
	window.addEventListener('scroll', updatePositionIfOpen, true);
});

onBeforeUnmount(() => {
	document.removeEventListener('click', handleClickOutside);
	window.removeEventListener('resize', updatePositionIfOpen);
	window.removeEventListener('scroll', updatePositionIfOpen, true);
});
</script>

<template>
	<Teleport to="body">
		<Transition
			enter-active-class="transition-all duration-150 ease-in-out"
			leave-active-class="transition-all duration-150 ease-in-out"
			enter-from-class="opacity-0 scale-95"
			leave-to-class="opacity-0 scale-95"
			@enter="(el) => (el as HTMLElement).offsetHeight"
			@leave="(el) => (el as HTMLElement).offsetHeight"
		>
			<div
				v-show="isOpen"
				ref="contentRef"
				:class="[$attrs.class]"
				:style="{ position: 'fixed', top: `${position.top}px`, left: `${position.left}px`, zIndex: 50 }"
				dropdown-content
				class="min-w-30 rounded-corner border border-primary bg-ui-bg/80 shadow-lg"
				@click="(e) => e.stopPropagation()"
			>
				<div class="py-1">
					<slot />
				</div>
			</div>
		</Transition>
	</Teleport>
</template>


