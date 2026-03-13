<script setup lang="ts">
interface Props {
	class?: string;
}

defineProps<Props>();
</script>

<template>
	<div
		:class="['px-3 py-2 text-sm font-semibold text-tx-main', $attrs.class]"
	>
		<slot />
	</div>
</template>

