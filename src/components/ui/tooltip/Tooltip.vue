<script setup lang="ts">
import { computed, provide, ref } from 'vue';

interface Props {
	disabled?: boolean;
	delayDuration?: number;
}

const props = withDefaults(defineProps<Props>(), {
	disabled: false,
	delayDuration: 200,
});

const isOpen = ref(false);
const triggerElement = ref<HTMLElement | null>(null);
let delayTimer: ReturnType<typeof setTimeout> | null = null;

const open = () => {
	if (props.disabled) return;
	delayTimer = setTimeout(() => {
		isOpen.value = true;
	}, props.delayDuration);
};

const close = () => {
	if (delayTimer) clearTimeout(delayTimer);
	isOpen.value = false;
};

const setTriggerElement = (element: HTMLElement | null) => {
	triggerElement.value = element;
};

provide('tooltip', {
	isOpen: computed(() => isOpen.value),
	disabled: computed(() => props.disabled),
	open,
	close,
	triggerElement,
	setTriggerElement,
});
</script>

<template>
	<div class="inline-block">
		<slot />
	</div>
</template>


