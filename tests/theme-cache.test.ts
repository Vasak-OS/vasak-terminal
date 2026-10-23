import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { applyTheme, saveTheme, THEME_KEY } from '../src/tools/theme-cache';

/**
 * Nota: `vasak-terminal` no corre pruebas de JS en el CI (ver AGENTS.md §9);
 * estas se corren en local con `bun test`.
 */

/** Un `Storage` mínimo en memoria, para no tocar el `localStorage` global. */
function memoryStorage(): Storage {
	const data = new Map<string, string>();
	return {
		get length() {
			return data.size;
		},
		clear: () => data.clear(),
		getItem: (key: string) => data.get(key) ?? null,
		key: (index: number) => Array.from(data.keys())[index] ?? null,
		removeItem: (key: string) => void data.delete(key),
		setItem: (key: string, value: string) => void data.set(key, String(value)),
	};
}

/** Un `Storage` que lanza siempre, como un `localStorage` bloqueado. */
function brokenStorage(): Storage {
	const fail = () => {
		throw new Error('localStorage no disponible');
	};
	return { ...memoryStorage(), getItem: fail, setItem: fail };
}

describe('theme-cache', () => {
	test('la clave compartida con index.html es vsk-theme', () => {
		expect(THEME_KEY).toBe('vsk-theme');
	});

	test('saveTheme guarda la clase oscura y sólo las variables --*', () => {
		const root = document.createElement('div');
		root.classList.add('dark');
		root.style.setProperty('--ui-bg', '#111');
		root.style.setProperty('--primary', '#abc');
		root.style.color = 'red';
		const storage = memoryStorage();

		saveTheme(root, storage);

		expect(JSON.parse(storage.getItem(THEME_KEY) as string)).toEqual({
			dark: true,
			vars: { '--ui-bg': '#111', '--primary': '#abc' },
		});
	});

	test('saveTheme guarda dark en falso si la raíz no es oscura', () => {
		const root = document.createElement('div');
		const storage = memoryStorage();

		saveTheme(root, storage);

		expect(JSON.parse(storage.getItem(THEME_KEY) as string).dark).toBe(false);
	});

	test('applyTheme reaplica la clase oscura y las variables', () => {
		const storage = memoryStorage();
		storage.setItem(THEME_KEY, JSON.stringify({ dark: true, vars: { '--ui-bg': '#222' } }));
		const root = document.createElement('div');

		applyTheme(root, storage);

		expect(root.classList.contains('dark')).toBe(true);
		expect(root.style.getPropertyValue('--ui-bg')).toBe('#222');
	});

	test('ida y vuelta: lo guardado de una raíz se reaplica en otra igual', () => {
		const storage = memoryStorage();
		const first = document.createElement('div');
		first.classList.add('dark');
		first.style.setProperty('--primary', '#abc');
		saveTheme(first, storage);

		const second = document.createElement('div');
		applyTheme(second, storage);

		expect(second.classList.contains('dark')).toBe(true);
		expect(second.style.getPropertyValue('--primary')).toBe('#abc');
	});

	test('sin clave guardada, applyTheme no hace nada', () => {
		const root = document.createElement('div');

		applyTheme(root, memoryStorage());

		expect(root.classList.contains('dark')).toBe(false);
		expect(root.getAttribute('style')).toBeNull();
	});

	test('con JSON inválido, applyTheme no lanza ni toca la raíz', () => {
		const storage = memoryStorage();
		storage.setItem(THEME_KEY, '{no es json');
		const root = document.createElement('div');

		expect(() => applyTheme(root, storage)).not.toThrow();
		expect(root.classList.contains('dark')).toBe(false);
	});

	test('un Storage que lanza no rompe ni saveTheme ni applyTheme', () => {
		const root = document.createElement('div');

		expect(() => saveTheme(root, brokenStorage())).not.toThrow();
		expect(() => applyTheme(root, brokenStorage())).not.toThrow();
	});
});

describe('cableado del tema recordado', () => {
	const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');

	test('index.html reaplica el tema desde el <head> antes del módulo', () => {
		const html = read('../index.html');
		const script = html.indexOf("localStorage.getItem(\"vsk-theme\"");
		expect(script).toBeGreaterThan(-1);
		expect(html).toContain('classList.add("dark")');
		expect(html).toContain('setProperty');
		expect(script).toBeLessThan(html.indexOf('type="module"'));
	});

	test('App.vue guarda el tema tras cada loadConfig', () => {
		const app = read('../src/App.vue');
		expect(app).toContain("from '@/tools/theme-cache'");
		// Tras la primera lectura…
		expect(app).toMatch(/await configStore\.loadConfig\(\);[\s\S]*?saveTheme\(\)/);
		// …y tras la recarga por `config-changed`.
		expect(app).toMatch(/configStore\.loadConfig\(\)\.then\(\(\) => saveTheme\(\)\)/);
	});
});
