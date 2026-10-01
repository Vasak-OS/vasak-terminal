/**
 * La terminal desplegable (el modo superpuesto), con la forma del taller.
 *
 * Llevaba `bg-ui-bg/80` —el fondo de la ventana, transparente— y la entrada
 * escrita en `style` con su propia curva. La superficie de lo que flota es
 * `ui-float`, opaca: una superficie de capa transparente no deja ver el
 * escritorio. Y la entrada va con los tiempos de `tokens.css`.
 */

import { afterEach, describe, expect, test } from 'bun:test';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import OverlayLayout from '@/components/overlay/OverlayLayout.vue';

let mounted: VueWrapper | null = null;

afterEach(() => {
	mounted?.unmount();
	mounted = null;
});

function mountOverlay() {
	setActivePinia(createPinia());
	mounted = mount(OverlayLayout, {
		global: { stubs: { TerminalComponent: true, ToastArea: true } },
	});
	return mounted;
}

describe('la terminal desplegable', () => {
	test('va sobre la superficie que flota, opaca', () => {
		const root = mountOverlay();

		expect(root.classes()).toContain('bg-ui-float');
		expect(root.classes().some((c) => c.startsWith('bg-ui-bg'))).toBe(false);
	});

	test('entra con los tiempos del taller y no con un estilo en línea', () => {
		const root = mountOverlay();

		expect(root.attributes('style') ?? '').not.toMatch(/transition|transform|opacity/);
		expect(root.classes()).toEqual(expect.arrayContaining(['duration-200', 'ease-ui-out']));
	});

	test('arranca escondida y corrida hacia abajo, como antes', () => {
		// Los mismos 8 px de antes (`translateY(8px)`), en la escala de Tailwind.
		const root = mountOverlay();

		expect(root.classes()).toEqual(expect.arrayContaining(['opacity-0', 'translate-y-2']));
	});
});
