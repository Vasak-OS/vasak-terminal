<script setup lang="ts">
import { computed, provide, ref } from 'vue';

interface Props {
	open?: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
	'update:open': [value: boolean];
}>();

const internalOpen = ref(false);
const triggerElement = ref<HTMLElement | null>(null);

const isOpen = computed({
	get: () => props.open ?? internalOpen.value,
	set: (value: boolean) => {
		if (props.open === undefined) {
			internalOpen.value = value;
		}
		emit('update:open', value);
	},
});

const closeDropdown = () => {
	isOpen.value = false;
};

const toggleDropdown = () => {
	isOpen.value = !isOpen.value;
};

const setTriggerElement = (element: HTMLElement | null) => {
	triggerElement.value = element;
};

provide('dropdown', {
	isOpen,
	closeDropdown,
	toggleDropdown,
	triggerElement,
	setTriggerElement,
});
</script>

<template>
	<div class="relative inline-block">
		<slot />
	</div>
</template>
