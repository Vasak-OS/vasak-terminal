import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';

const ANIM_MS = 200;

function sleep(ms: number) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export function useOverlay() {
	const isVisible = ref(false);
	let unlistenToggle: (() => void) | null = null;
	let keydownHandler: ((e: KeyboardEvent) => void) | null = null;

	async function show() {
		await invoke('show_overlay');
		await new Promise((r) => requestAnimationFrame(r));
		isVisible.value = true;
	}

	async function hide() {
		isVisible.value = false;
		await sleep(ANIM_MS);
		await invoke('hide_overlay');
	}

	async function toggle() {
		if (isVisible.value) {
			await hide();
		} else {
			await show();
		}
	}

	function onBlur() {
		if (isVisible.value) {
			hide();
		}
	}

	onMounted(async () => {
		await show();

		unlistenToggle = await listen('vterminal:toggle-overlay', () => {
			toggle();
		});

		window.addEventListener('blur', onBlur);

		keydownHandler = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && isVisible.value) {
				hide();
			}
		};
		document.addEventListener('keydown', keydownHandler, { capture: true });
	});

	onUnmounted(() => {
		unlistenToggle?.();
		window.removeEventListener('blur', onBlur);
		if (keydownHandler) {
			document.removeEventListener('keydown', keydownHandler, { capture: true });
		}
	});

	return { isVisible, show, hide, toggle };
}
