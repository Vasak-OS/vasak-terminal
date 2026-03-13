<script setup lang="ts">
import { inject } from 'vue';

interface Props {
	disabled?: boolean;
	class?: string;
}

const props = withDefaults(defineProps<Props>(), {
	disabled: false,
});

const emit = defineEmits<{
	select: [];
}>();

const dropdown = inject<any>('dropdown');

const handleClick = () => {
	if (props.disabled) return;
	emit('select');
	dropdown?.closeDropdown();
};
</script>

<template>
	<div
		:class="[
			'px-3 py-2 text-sm transition-colors',
			{
				'opacity-50 cursor-not-allowed': disabled,
				'cursor-pointer hover:bg-primary hover:text-tx-on-primary': !disabled,
			},
			$attrs.class,
		]"
		@click="handleClick"
	>
		<slot />
	</div>
</template>
