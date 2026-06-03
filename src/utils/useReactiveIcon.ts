import { listen } from '@tauri-apps/api/event';
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { onUnmounted, type Ref, ref } from 'vue';

type IconConfig = string | { name: string; type?: 'icon' | 'symbol' };

export function useReactiveIcon<T extends Record<string, IconConfig>>(
	icons: T
): { [K in keyof T]: Ref<string> } {
	const result = {} as { [K in keyof T]: Ref<string> };
	const entries = Object.entries(icons);

	for (const [key] of entries) {
		(result as Record<string, Ref<string>>)[key] = ref('');
	}

	async function fetchAll() {
		for (const [key, config] of entries) {
			const resolved =
				typeof config === 'string'
					? { name: config, type: 'symbol' as const }
					: { name: config.name, type: config.type ?? ('symbol' as const) };

			const source =
				resolved.type === 'icon'
					? await getIconSource(resolved.name)
					: await getSymbolSource(resolved.name);

			(result as Record<string, Ref<string>>)[key].value = source;
		}
	}

	fetchAll();

	let cleanup: (() => void) | undefined;
	listen('vicons:theme-changed', () => {
		fetchAll();
	}).then((unlisten) => {
		cleanup = unlisten;
	});

	onUnmounted(() => {
		cleanup?.();
	});

	return result;
}
