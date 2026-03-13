<script setup lang="ts">
import { inject, onMounted, ref } from 'vue';

interface Props {
	asChild?: boolean;
	disabled?: boolean;
}

withDefaults(defineProps<Props>(), {
	asChild: false,
	disabled: false,
});

const dropdown = inject<any>('dropdown');
const triggerRef = ref<HTMLElement | null>(null);

const resolveTriggerElement = () => {
	if (!triggerRef.value) return null;
	const child = triggerRef.value.firstElementChild as HTMLElement | null;
	return child ?? triggerRef.value;
};

const handleClick = (event: MouseEvent) => {
	const element = (event.currentTarget as HTMLElement) ?? resolveTriggerElement();
	dropdown?.setTriggerElement?.(element);
	dropdown?.toggleDropdown();
};

onMounted(() => {
	dropdown?.setTriggerElement?.(resolveTriggerElement());
});
</script>

<template>
	<div
		ref="triggerRef"
		class="dropdown-menu-trigger inline-block"
		@click="handleClick"
	>
		<slot />
	</div>
</template>

