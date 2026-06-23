import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';

export function useOverlay() {
	const isVisible = ref(false);
	let unlistenToggle: (() => void) | null = null;
	let keydownHandler: ((e: KeyboardEvent) => void) | null = null;

	async function show() {
		await invoke('show_overlay');
		isVisible.value = true;
	}

	async function hide() {
		isVisible.value = false;
		await invoke('hide_overlay');
	}

	async function toggle() {
		if (isVisible.value) {
			await hide();
		} else {
			await show();
		}
	}

	onMounted(async () => {
		await show();

		unlistenToggle = await listen('vterminal:toggle-overlay', () => {
			toggle();
		});

		keydownHandler = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && isVisible.value) {
				hide();
			}
		};
		document.addEventListener('keydown', keydownHandler, { capture: true });
	});

	onUnmounted(() => {
		unlistenToggle?.();
		if (keydownHandler) {
			document.removeEventListener('keydown', keydownHandler, { capture: true });
		}
	});

	return { isVisible, show, hide, toggle };
}
