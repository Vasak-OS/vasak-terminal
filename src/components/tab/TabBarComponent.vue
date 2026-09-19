<script setup lang="ts">
/**
 * Las pestañas de la terminal, sobre la barra compartida.
 *
 * Lo que dibuja y cómo se comporta una pestaña —elegir, cerrar, reordenar, el
 * menú, el teclado, amoldarse a una barra vertical— es de
 * `@vasakgroup/vue-libvasak`: tres aplicaciones del escritorio tenían su propia
 * versión y ninguna hacía exactamente lo mismo.
 *
 * Lo que queda acá es lo que **sí** es de la terminal: cómo se llama una
 * pestaña. El nombre sale del comando que está corriendo, y si no hay ninguno,
 * de la última carpeta del directorio de trabajo. Un grupo partido en dos
 * paneles muestra los dos separados por una barra.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { type ElementoDePestana, TabBar } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import DropdownMenu from '@/components/ui/dropdown/DropdownMenu.vue';
import DropdownMenuContent from '@/components/ui/dropdown/DropdownMenuContent.vue';
import DropdownMenuItem from '@/components/ui/dropdown/DropdownMenuItem.vue';
import DropdownMenuTrigger from '@/components/ui/dropdown/DropdownMenuTrigger.vue';
import { useWorkspacesStore } from '@/stores/workspaces';
import type { Tab, TabGroup } from '@/types/workspaces';

const workspacesStore = useWorkspacesStore();
const { t } = useI18n();

const { openNewTabGroup, closeTabGroup, setTabs, openTabGroup } = workspacesStore;

/** El menú de una pestaña, con el grupo sobre el que se abrió. */
const menuAbierto = ref(false);
const grupoDelMenu = ref<TabGroup | null>(null);
/**
 * Dónde se apretó, para que el menú salga ahí.
 *
 * El desplegable se ubica contra un elemento —le pide su rectángulo—, y la
 * librería emite coordenadas. El puente es un ancla de cero por cero fija en
 * ese punto: sin ella el menú no tenía contra qué medirse y salía en la esquina
 * de la ventana.
 */
const anclaDelMenu = ref({ x: 0, y: 0 });

const grupos = computed<TabGroup[]>(() => workspacesStore.currentWorkspace?.tabGroups ?? []);

/** Lo que muestra una pestaña: el comando, o la carpeta, o el nombre. */
function etiquetaDe(tab: Tab): string {
	const comando = tab.runtimeCommand?.trim();
	if (comando) return comando;

	const cwd = tab.runtimeCwd?.trim();
	if (cwd) {
		const partes = cwd.replace(/\/$/, '').split('/').filter(Boolean);
		return partes[partes.length - 1] || '/';
	}

	return tab.name || tab.path;
}

/** Un grupo partido en dos paneles muestra los dos. */
function etiquetaDelGrupo(grupo: TabGroup): string {
	const nombres = grupo.map(etiquetaDe).filter(Boolean);
	return nombres.join(' | ');
}

const pestanas = computed<ElementoDePestana[]>(() =>
	grupos.value.map((grupo) => ({
		id: grupo[0]?.id ?? '',
		label: etiquetaDelGrupo(grupo),
		// El directorio entero en el texto de ayuda: en la pestaña sólo entra la
		// última carpeta, y dos proyectos con la misma última carpeta se ven
		// idénticos.
		tooltip: grupo[0]?.runtimeCwd || grupo[0]?.path || undefined,
	}))
);

const activa = computed(
	() => grupos.value[workspacesStore.currentWorkspace?.currentTabGroupIndex ?? 0]?.[0]?.id ?? ''
);

function grupoDe(id: string): TabGroup | undefined {
	return grupos.value.find((grupo) => grupo[0]?.id === id);
}

function elegir(id: string) {
	const grupo = grupoDe(id);
	if (grupo) openTabGroup(grupo);
}

function cerrar(id: string) {
	const grupo = grupoDe(id);
	if (grupo) closeTabGroup(grupo);
}

function abrirElMenu(carga: { id: string; x: number; y: number }) {
	grupoDelMenu.value = grupoDe(carga.id) ?? null;
	anclaDelMenu.value = { x: carga.x, y: carga.y };
	menuAbierto.value = true;
}

async function cerrarLasDemas() {
	if (grupoDelMenu.value) await workspacesStore.closeOtherTabGroups(grupoDelMenu.value);
	menuAbierto.value = false;
}

async function cerrarTodas() {
	await workspacesStore.closeAllTabGroups();
	menuAbierto.value = false;
}

/** La lista nueva llega entera: se guarda tal cual. */
function reordenar(nuevas: ElementoDePestana[]) {
	const porId = new Map(grupos.value.map((grupo) => [grupo[0]?.id ?? '', grupo]));
	const ordenados = nuevas
		.map((pestana) => porId.get(pestana.id))
		.filter((grupo): grupo is TabGroup => Boolean(grupo));
	setTabs(ordenados);
}
</script>

<template>
  <TabBar
    :tabs="pestanas"
    :model-value="activa"
    :new-label="t('tabs.newTab')"
    :close-label="t('tabs.close')"
    @select="elegir"
    @close="cerrar"
    @new="openNewTabGroup()"
    @reorder="reordenar"
    @menu="abrirElMenu" />

  <DropdownMenu v-model:open="menuAbierto">
    <!-- El ancla: cero por cero, en el punto donde se abrió el menú. Es lo que
         el desplegable mide para ubicarse. -->
    <DropdownMenuTrigger>
      <span
        class="pointer-events-none fixed size-0"
        :style="{ left: `${anclaDelMenu.x}px`, top: `${anclaDelMenu.y}px` }"
        aria-hidden="true" />
    </DropdownMenuTrigger>
    <DropdownMenuContent>
      <DropdownMenuItem @select="cerrarLasDemas">{{ t('tabs.closeOtherTabs') }}</DropdownMenuItem>
      <DropdownMenuItem @select="cerrarTodas">{{ t('tabs.closeAllTabs') }}</DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
</template>
