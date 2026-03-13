<script setup lang="ts">
import { inject, onMounted, ref } from 'vue';

interface Props {
	asChild?: boolean;
}

withDefaults(defineProps<Props>(), {
	asChild: false,
});

const tooltip = inject<any>('tooltip');
const triggerRef = ref<HTMLElement | null>(null);

const resolveTriggerElement = () => {
	if (!triggerRef.value) return null;
	const child = triggerRef.value.firstElementChild as HTMLElement | null;
	return child ?? triggerRef.value;
};

const handleMouseEnter = () => {
	const element = resolveTriggerElement();
	tooltip?.setTriggerElement?.(element);
	tooltip?.open();
};

const handleMouseLeave = () => {
	tooltip?.close();
};

const handleFocus = () => {
	const element = resolveTriggerElement();
	tooltip?.setTriggerElement?.(element);
	tooltip?.open();
};

const handleBlur = () => {
	tooltip?.close();
};

onMounted(() => {
	tooltip?.setTriggerElement?.(resolveTriggerElement());
});
</script>

<template>
	<div
		ref="triggerRef"
		class="inline-block "
		@mouseenter="handleMouseEnter"
		@mouseleave="handleMouseLeave"
		@focus="handleFocus"
		@blur="handleBlur"
	>
		<slot />
	</div>
</template>


