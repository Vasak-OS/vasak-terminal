/**
 * Lo que la terminal deja de tener propio.
 *
 * Tenía dos cosas. Una **no la usaba nadie**: las tres piezas del tooltip, 193
 * líneas copiadas del gestor de archivos que ningún archivo importaba. La otra
 * sí se usaba y no se oía: la pila de avisos que aparece al copiar al
 * portapapeles no tenía ningún rol ARIA, así que un lector de pantalla no
 * anunciaba nada. Copiar y que falle copiar sonaban igual: a silencio.
 *
 * Lo que se comprueba acá es eso —que los avisos se anuncien, y que un error
 * interrumpa mientras lo demás espera su turno— y que las etiquetas de los
 * botones de la ventana sigan en pie después de cambiarles el espacio de
 * nombres para que la librería las encuentre sola.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { olvidarLosIconosDelTema, ToastArea, WindowControls } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { useNotification } from '@/utils/useNotification';
import { olvidarTodo, responder } from './dobles';

/**
 * Lo montado acá, para desmontarlo pase lo que pase.
 *
 * `enableAutoUnmount` no sirve: sólo se puede llamar una vez por proceso y ya
 * la llama `marco-y-pestanas.test.ts`, así que llamarla de nuevo revienta la
 * corrida entera con un error que no nombra a ninguna prueba.
 */
const montadas: VueWrapper[] = [];

function abrir() {
	const vista = mount(WindowAppLayout);
	montadas.push(vista);
	return vista;
}

beforeEach(() => {
	olvidarTodo();
	olvidarLosIconosDelTema();
	setActivePinia(createPinia());
	responder('list_workspaces', []);
	// La cola es de la aplicación, así que se vacía a mano entre pruebas: vive
	// en el módulo y no se va con el desmontaje.
	useNotification().notifications.value.splice(0);
});

afterEach(() => {
	for (const vista of montadas.splice(0)) vista.unmount();
	// La pila se teletransporta al `body`, así que no se va con el desmontaje.
	for (const suelto of document.body.querySelectorAll('[role="status"], [role="alert"]')) {
		suelto.closest('.fixed')?.remove();
	}
});

describe('los avisos del portapapeles', () => {
	test('ahora se anuncian, que antes no', async () => {
		// La copia de acá no tenía ni `role` ni nada: el aviso aparecía en
		// pantalla y para quien no la ve no pasaba nada.
		const { notify } = useNotification();
		abrir();
		notify('Copiado');
		await nextTick();

		expect(document.body.querySelector('[role="status"]')).not.toBeNull();
	});

	test('y un error interrumpe mientras lo demás espera turno', async () => {
		// Es la diferencia entre enterarse de que no se pudo copiar y
		// enterarse cuando ya se pegó otra cosa.
		const { notify } = useNotification();
		abrir();
		notify('Copiado', 'success');
		notify('No se pudo copiar', 'error');
		await nextTick();

		expect(document.body.querySelectorAll('[role="status"]')).toHaveLength(1);
		expect(document.body.querySelectorAll('[role="alert"]')).toHaveLength(1);
	});

	test('se apilan abajo y al medio, como estaban', async () => {
		// La esquina compite con lo que la terminal dibuja ahí.
		const { notify } = useNotification();
		abrir();
		notify('Copiado');
		await nextTick();

		const pila = document.body.querySelector('.fixed') as HTMLElement;
		expect(pila.className).toContain('left-1/2');
	});

	test('y la pila no se come los clics de la terminal que hay debajo', async () => {
		const { notify } = useNotification();
		abrir();
		notify('Copiado');
		await nextTick();

		const pila = document.body.querySelector('.fixed') as HTMLElement;
		expect(pila.className).toContain('pointer-events-none');
	});

	test('es la pila compartida y no una escrita acá', () => {
		const vista = abrir();

		expect(vista.findComponent(ToastArea).exists()).toBe(true);
	});
});

describe('los botones de la ventana', () => {
	test('siguen teniendo nombre sin que el marco se lo pase', () => {
		// Las claves de acá se llamaban `windowControls.*` y la librería busca
		// `ventana.*`, así que este marco se las pasaba a mano. Se renombraron
		// en el catálogo para que las encuentre sola; lo que no puede pasar es
		// que al renombrarlas se pierdan, y eso no daría ningún error: un botón
		// con sólo un icono adentro se anuncia «botón» y nada más.
		const vista = abrir();
		const botones = vista.findComponent(WindowControls).findAll('button');

		expect(botones.map((b) => b.attributes('aria-label'))).toEqual([
			'ventana.minimizar',
			'ventana.maximizar',
			'ventana.cerrar',
		]);
	});
});

describe('y el catálogo tiene las claves que la librería busca', () => {
	// La prueba de arriba monta con el doble de traducciones, que devuelve la
	// clave tal cual: comprueba que la librería **pida** `ventana.minimizar`,
	// no que exista. Si el renombre del catálogo estuviera mal escrito, allá
	// seguiría en verde y en la ventana de verdad los botones dirían
	// «ventana.minimizar».
	const idiomas = ['es', 'en'] as const;

	for (const idioma of idiomas) {
		test(`en ${idioma}`, async () => {
			const catalogo = (await Bun.file(
				new URL(`../src-tauri/locales/${idioma}.json`, import.meta.url).pathname
			).json()) as Record<string, Record<string, string>>;

			expect(Object.keys(catalogo.ventana ?? {}).sort()).toEqual([
				'cerrar',
				'maximizar',
				'minimizar',
			]);
			for (const texto of Object.values(catalogo.ventana)) {
				expect(texto.length).toBeGreaterThan(0);
			}
		});
	}

	test('y no quedó el espacio de nombres viejo', () => {
		// Dos juegos de claves para lo mismo es lo que hace que una se quede
		// sin traducir sin que nadie lo note.
		for (const idioma of idiomas) {
			const catalogo = require(`../src-tauri/locales/${idioma}.json`);
			expect(catalogo.windowControls).toBeUndefined();
		}
	});
});
