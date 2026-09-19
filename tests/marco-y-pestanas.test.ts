/**
 * La ventana de la terminal, sobre el marco compartido.
 *
 * El marco, la barra, los botones de ventana y las pestañas eran suyos; ahora
 * son de `@vasakgroup/vue-libvasak`, que es lo que hace que esta ventana se lea
 * como parte del mismo escritorio que las otras.
 *
 * Lo que se comprueba acá es lo que **sigue siendo** de la terminal: cómo se
 * llama una pestaña, y que elegirla, cerrarla y reordenarlas lleguen al store.
 * Cómo se dibuja una pestaña y cómo se amolda a una barra vertical es de la
 * librería y se prueba allá.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { TabBar, WindowFrame } from '@vasakgroup/vue-libvasak';
import { enableAutoUnmount, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import TabBarComponent from '@/components/tab/TabBarComponent.vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab } from '@/types/workspaces';
import { olvidarTodo, responder } from './dobles';

function unaPestana(cambios: Partial<Tab> = {}): Tab {
	return {
		id: 'uno',
		name: 'bash',
		path: '/home/pato',
		type: 'directory',
		paneWidth: 100,
		filterQuery: '',
		...cambios,
	};
}

/**
 * Desmontar lo de cada prueba antes de la siguiente.
 *
 * El menú se teletransporta al `body` y las pruebas lo buscan ahí, no en el
 * envoltorio: sin desmontar, el menú de una prueba sigue puesto en la que
 * viene y `[role="menuitem"]` devuelve las opciones de las tres.
 */
enableAutoUnmount(afterEach);

/** La barra de pestañas con los grupos que se le pongan al store. */
async function montarLaBarra(grupos: Tab[][]) {
	const pinia = createPinia();
	setActivePinia(pinia);
	const store = useWorkspacesStore();
	const espacio = store.currentWorkspace;
	if (espacio) {
		espacio.tabGroups = grupos;
		espacio.currentTabGroupIndex = 0;
	}
	const vista = mount(TabBarComponent, {
		attachTo: document.body,
		global: { plugins: [pinia] },
	});
	await nextTick();
	return { vista, store };
}

/** Las opciones del menú, buscadas en el documento como las busca un lector. */
function lasOpciones(): HTMLElement[] {
	return Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'));
}

beforeEach(() => {
	olvidarTodo();
	responder('get_shells', []);
});


describe('la ventana', () => {
	test('usa el marco compartido y ya no dibuja el suyo', async () => {
		// El marco estaba copiado en dieciocho repositorios con catorce formas
		// distintas. Éste era una de ellas.
		const pinia = createPinia();
		setActivePinia(pinia);
		const vista = mount(WindowAppLayout, { global: { plugins: [pinia] } });
		await nextTick();

		expect(vista.findComponent(WindowFrame).exists()).toBe(true);
	});

	test('y las pestañas van en la barra, no sueltas en la ventana', async () => {
		const pinia = createPinia();
		setActivePinia(pinia);
		const vista = mount(WindowAppLayout, { global: { plugins: [pinia] } });
		await nextTick();

		const barra = vista.findComponent(WindowFrame).find('[data-tauri-drag-region]');
		expect(barra.findComponent(TabBar).exists()).toBe(true);
	});
});

describe('cómo se llama una pestaña', () => {
	test('con el comando que está corriendo', async () => {
		// Es lo más útil mientras algo corre: saber cuál de las cuatro está
		// compilando.
		const { vista } = await montarLaBarra([[unaPestana({ runtimeCommand: 'cargo build' })]]);

		expect(vista.findComponent(TabBar).props('tabs')[0].label).toBe('cargo build');
	});

	test('y si no hay ninguno, con la última carpeta', async () => {
		const { vista } = await montarLaBarra([
			[unaPestana({ runtimeCwd: '/home/pato/VasakOS/vasak-terminal' })],
		]);

		expect(vista.findComponent(TabBar).props('tabs')[0].label).toBe('vasak-terminal');
	});

	test('la raíz se llama «/» y no queda vacía', async () => {
		const { vista } = await montarLaBarra([[unaPestana({ runtimeCwd: '/' })]]);

		expect(vista.findComponent(TabBar).props('tabs')[0].label).toBe('/');
	});

	test('un grupo partido en dos muestra los dos', async () => {
		const { vista } = await montarLaBarra([
			[
				unaPestana({ id: 'a', runtimeCommand: 'vim' }),
				unaPestana({ id: 'b', runtimeCommand: 'htop' }),
			],
		]);

		expect(vista.findComponent(TabBar).props('tabs')[0].label).toBe('vim | htop');
	});

	test('el directorio entero va en el texto de ayuda', async () => {
		// En la pestaña sólo entra la última carpeta, y dos proyectos con la
		// misma última carpeta se ven idénticos.
		const { vista } = await montarLaBarra([
			[unaPestana({ runtimeCwd: '/home/pato/VasakOS/vasak-terminal' })],
		]);

		expect(vista.findComponent(TabBar).props('tabs')[0].tooltip).toBe(
			'/home/pato/VasakOS/vasak-terminal'
		);
	});
});

describe('lo que llega al store', () => {
	test('reordenar guarda los grupos en el orden nuevo', async () => {
		// La librería emite la lista de pestañas; acá se traduce de vuelta a
		// grupos, que es lo que el store entiende.
		const grupos = [[unaPestana({ id: 'a' })], [unaPestana({ id: 'b' })]];
		const { vista, store } = await montarLaBarra(grupos);

		const pestanas = vista.findComponent(TabBar).props('tabs');
		vista.findComponent(TabBar).vm.$emit('reorder', [pestanas[1], pestanas[0]]);
		await nextTick();

		expect(store.currentWorkspace?.tabGroups.map((grupo) => grupo[0]?.id)).toEqual(['b', 'a']);
	});
});

describe('el menú de una pestaña', () => {
	test('se abre donde se apretó y no en la esquina', async () => {
		// El desplegable se ubica midiendo un elemento, y la librería emite
		// coordenadas: sin un ancla en ese punto el menú salía en (0, 0).
		const { vista } = await montarLaBarra([[unaPestana({ id: 'a' })]]);

		vista.findComponent(TabBar).vm.$emit('menu', { id: 'a', x: 240, y: 96 });
		await nextTick();

		const ancla = vista.find('.fixed.size-0');
		expect(ancla.exists()).toBe(true);
		expect(ancla.attributes('style')).toContain('left: 240px');
		expect(ancla.attributes('style')).toContain('top: 96px');
	});

	test('y sabe sobre cuál se abrió', async () => {
		// «Cerrar las demás» necesita saber cuál es «ésta».
		const grupos = [[unaPestana({ id: 'a' })], [unaPestana({ id: 'b' })]];
		const { vista, store } = await montarLaBarra(grupos);
		let cerradas: string[] = [];
		store.closeOtherTabGroups = async (grupo) => {
			cerradas = [grupo[0]?.id ?? ''];
		};

		vista.findComponent(TabBar).vm.$emit('menu', { id: 'b', x: 10, y: 10 });
		await nextTick();
		lasOpciones()[0]?.click();
		await nextTick();

		expect(cerradas).toEqual(['b']);
	});

	test('es un menú de verdad, y se recorre con el teclado', async () => {
		// El desplegable era un `div` teletransportado con un `div` por opción:
		// esta misma prueba tenía que buscar la opción por componente porque en
		// el documento no había ningún selector con el que encontrarla. Ahora
		// es de `@vasakgroup/vue-libvasak` y se busca por `role`, que es lo que
		// mira un lector de pantalla.
		const grupos = [[unaPestana({ id: 'a' })], [unaPestana({ id: 'b' })]];
		const { vista, store } = await montarLaBarra(grupos);
		let todas = false;
		store.closeAllTabGroups = async () => {
			todas = true;
		};

		vista.findComponent(TabBar).vm.$emit('menu', { id: 'a', x: 10, y: 10 });
		await nextTick();
		await nextTick();

		const menu = document.querySelector('[role="menu"]');
		expect(menu).not.toBeNull();
		expect(lasOpciones()).toHaveLength(2);

		// Abierto con el puntero el foco va al menú; de ahí, las flechas.
		expect(document.activeElement).toBe(menu as HTMLElement);
		(document.activeElement as HTMLElement).dispatchEvent(
			new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true })
		);
		expect(document.activeElement).toBe(lasOpciones()[1] as HTMLElement);

		(document.activeElement as HTMLElement).dispatchEvent(
			new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
		);
		await nextTick();

		expect(todas).toBe(true);
	});
});
