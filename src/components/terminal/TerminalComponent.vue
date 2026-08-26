<script lang="ts" setup>
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getSchemeById, useConfigStore, VSKConfig } from '@vasakgroup/plugin-config-manager';
import type { MenuEntry } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Store } from 'pinia';
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { useWorkspacesStore } from '@/stores/workspaces';
import { useNotification } from '@/utils/useNotification';

const props = withDefaults(
	defineProps<{
		sessionId: string;
		active?: boolean;
	}>(),
	{
		active: true,
	}
);

const configStore = useConfigStore() as Store<
	'config',
	{ config: VSKConfig; loadConfig: () => Promise<void> }
>;
const workspacesStore = useWorkspacesStore();
const { notify } = useNotification();
const { t } = useI18n();
const { show: showContextMenu } = useContextMenu();
const terminalElement = ref<HTMLElement | null>(null);

const DEFAULT_FONT_SIZE = 14;

function getSavedFontSize(): number {
	try {
		// Con base explícita: sin ella, un valor guardado como «0x10» se leería
		// como 16 en lugar de descartarse.
		return (
			Number.parseInt(localStorage.getItem('vterminal-font-size') || '', 10) || DEFAULT_FONT_SIZE
		);
	} catch {
		return DEFAULT_FONT_SIZE;
	}
}

function saveFontSize(size: number) {
	try {
		localStorage.setItem('vterminal-font-size', String(size));
	} catch {
		/* noop */
	}
}

const fitAddon = new FitAddon();
const term = new Terminal({
	allowTransparency: true,
	fontFamily: 'monospace',
	fontSize: getSavedFontSize(),
	theme: {
		background: 'rgba(0, 0, 0, 0)',
	},
});

let terminalDataDisposable: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let ptyUnlisteners: Array<() => void> = [];
let keydownHandler: ((e: KeyboardEvent) => void) | null = null;
let pasteHandler: ((e: ClipboardEvent) => void) | null = null;
let contextMenuHandler: ((e: MouseEvent) => void) | null = null;
let isShellReady = false;
let shellExited = false;
let shellStatusIntervalId: ReturnType<typeof setInterval> | null = null;
let onVisibilityChange: (() => void) | null = null;

type ShellStatus = {
	cwd?: string;
	running_command?: string;
};

function onResize() {
	fitTerminal();
}

// Make the terminal fit all the window size
async function fitTerminal() {
	if (!terminalElement.value || !props.active) {
		return;
	}

	fitAddon.fit();
	applyRectFit();

	if (!isShellReady) {
		return;
	}

	try {
		await invoke<string>('async_resize_pty', {
			sessionId: props.sessionId,
			rows: term.rows,
			cols: term.cols,
		});
	} catch (e) {
		console.error('resize PTY failed', e);
	}
}

function applyRectFit() {
	const el = terminalElement.value;
	if (!el) return;

	const rect = el.getBoundingClientRect();
	if (rect.width === 0 || rect.height === 0) return;

	const dims = (term as any)._core?._renderService?.dimensions;
	if (!dims || dims.css.cell.width === 0 || dims.css.cell.height === 0) return;

	const cols = Math.max(2, Math.floor(rect.width / dims.css.cell.width));
	const rows = Math.max(1, Math.floor(rect.height / dims.css.cell.height));

	if (term.rows !== rows || term.cols !== cols) {
		(term as any)._core._renderService.clear();
		term.resize(cols, rows);
	}
}

function sleep(ms: number) {
	return new Promise<void>((resolve) => {
		setTimeout(resolve, ms);
	});
}

/** Wait until the element has non-zero dimensions (handles overlay window
 *  being initially hidden/mis-sized on Wayland before the compositor
 *  responds with the proper layer-surface size).
 *  Returns false if the element is still 0×0 after the timeout. */
async function waitForRealSize(el: HTMLElement, timeout = 3000, interval = 30): Promise<boolean> {
	if (el.offsetWidth > 0 && el.offsetHeight > 0) return true;

	const start = Date.now();
	while (Date.now() - start < timeout) {
		await sleep(interval);
		const w = el.offsetWidth;
		const h = el.offsetHeight;
		if (w > 0 && h > 0) return true;
	}

	return false;
}

/**
 * El canal por donde llega la salida del PTY, en bytes crudos.
 *
 * Antes venía en base64 por un evento y se decodificaba acá con `atob` más un
 * bucle por byte. Medido, ese bucle corre a 267 MB/s contra 3117 MB/s de un
 * decodificado nativo, y base64 agregaba un tercio de bytes al IPC. Un canal de
 * Tauri manda los trozos grandes como binario real, así que lo que llega ya es
 * un `ArrayBuffer` y se le pasa a xterm.js sin tocarlo.
 *
 * Se le siguen dando bytes y no texto: xterm.js necesita los bytes para rearmar
 * las secuencias UTF-8 que quedan partidas entre dos trozos.
 */
function crearCanalDeSalida(): Channel<ArrayBuffer> {
	const canal = new Channel<ArrayBuffer>();
	canal.onmessage = (bytes) => {
		term.write(new Uint8Array(bytes));
	};
	return canal;
}

// La salida ya no pasa por un evento: el canal se le entrega a
// `async_create_shell`, así que existe antes de que arranque la shell y no hay
// ventana por la que se pueda perder lo primero que escriba. Sólo la salida de
// la shell sigue siendo un evento, que es una señal única.
async function setupPtyListeners() {
	const unExit = await listen(`pty://exit/${props.sessionId}`, async () => {
		if (!shellExited) {
			shellExited = true;
			await workspacesStore.handleShellExit(props.sessionId);
		}
	});
	ptyUnlisteners.push(unExit);
}

const setTerminalConfig = async () => {
	const conf = configStore.config as VSKConfig | null;
	if (!conf) return;

	// Apply font from config
	if (conf.fonts?.terminal) {
		term.options.fontFamily = conf.fonts.terminal;
	}

	const scheme = await getSchemeById(conf.style['color-scheme']);
	if (!scheme) return;

	const darkOrLight = conf.style.darkmode ? 'dark' : 'light';
	const ansi = scheme.scheme.colors[darkOrLight].terminal.ansi;
	const termColors = scheme.scheme.colors[darkOrLight].terminal;

	term.options.theme = {
		background: 'rgba(0, 0, 0, 0)',
		foreground: termColors.foreground,
		cursor: termColors.cursor,
		black: ansi.black,
		red: ansi.red,
		green: ansi.green,
		yellow: ansi.yellow,
		blue: ansi.blue,
		magenta: ansi.magenta,
		cyan: ansi.cyan,
		white: ansi.white,
		brightBlack: ansi.brightBlack,
		brightRed: ansi.brightRed,
		brightGreen: ansi.brightGreen,
		brightYellow: ansi.brightYellow,
		brightBlue: ansi.brightBlue,
		brightMagenta: ansi.brightMagenta,
		brightCyan: ansi.brightCyan,
		brightWhite: ansi.brightWhite,
	};
};

// Write data from the terminal to the pty
function writeToPty(data: string) {
	void invoke('async_write_to_pty', {
		sessionId: props.sessionId,
		data,
	});
}
function initShell() {
	return invoke('async_create_shell', {
		sessionId: props.sessionId,
		rows: term.rows,
		cols: term.cols,
		onOutput: crearCanalDeSalida(),
	});
}

// ─── Menú contextual ─────────────────────────────────────────────────────────
// El clic derecho abre el menú de VasakOS, el mismo de todo el escritorio, con
// lo que el terminal sabe hacer. Copiar y pegar trabajan sobre la selección real
// de xterm.js y contra el portapapeles del sistema: el webview no puede leerlo
// —WebKitGTK no implementa «clipboard-read»—, así que las dos operaciones pasan
// por GTK, que sí tiene la conexión con el compositor.

async function copySelection() {
	const selection = term.getSelection();
	if (!selection) {
		return;
	}

	try {
		await invoke('clipboard_write_text', { text: selection });
		notify(t('notifications.copied'));
	} catch (error) {
		console.error('Failed to copy the selection:', error);
		notify(t('notifications.clipboardError'), 'error');
	}

	term.focus();
}

async function pasteFromClipboard() {
	try {
		const text = await invoke<string>('clipboard_read_text');
		if (text) {
			writeToPty(text);
		}
	} catch (error) {
		console.error('Failed to read the clipboard:', error);
		notify(t('notifications.clipboardError'), 'error');
	}

	term.focus();
}

function selectAllOutput() {
	term.selectAll();
	term.focus();
}

function clearTerminal() {
	term.clear();
	term.focus();
}

function openNewTab() {
	void workspacesStore.openNewTabGroup();
}

async function closeCurrentTab() {
	const tabGroup = workspacesStore.currentWorkspace?.tabGroups.find((group) =>
		group.some((tab) => tab.id === props.sessionId)
	);

	if (tabGroup) {
		await workspacesStore.closeTabGroup(tabGroup);
	}
}

async function openTerminalContextMenu(event: MouseEvent) {
	// Sin texto seleccionado no hay nada que copiar, y con una sola pestaña
	// abierta cerrarla es cerrar la ventana: ninguno de los dos ítems aparece
	// cuando no significaría nada.
	const items: MenuEntry[] = [];

	if (term.hasSelection()) {
		items.push({
			id: 'copy',
			label: t('contextMenu.copy'),
			icon: 'edit-copy',
			accelerator: 'Ctrl+Shift+C',
		});
	}

	items.push(
		{
			id: 'paste',
			label: t('contextMenu.paste'),
			icon: 'edit-paste',
			accelerator: 'Ctrl+Shift+V',
		},
		{ id: 'selectAll', label: t('contextMenu.selectAll'), icon: 'edit-select-all' },
		{ type: 'separator' },
		{ id: 'clear', label: t('contextMenu.clear'), icon: 'edit-clear-all' },
		{ type: 'separator' },
		{ id: 'newTab', label: t('tabs.newTab'), icon: 'tab-new' }
	);

	if ((workspacesStore.currentWorkspace?.tabGroups.length ?? 0) > 1) {
		items.push({ id: 'closeTab', label: t('contextMenu.closeTab'), icon: 'window-close' });
	}

	const chosen = await showContextMenu(items, event);

	switch (chosen?.id) {
		case 'copy':
			await copySelection();
			break;
		case 'paste':
			await pasteFromClipboard();
			break;
		case 'selectAll':
			selectAllOutput();
			break;
		case 'clear':
			clearTerminal();
			break;
		case 'newTab':
			openNewTab();
			break;
		case 'closeTab':
			await closeCurrentTab();
			break;
	}
}

async function applyStartupCommandIfAny() {
	try {
		const startupCommand = await invoke<string | null>('async_take_startup_command', {
			sessionId: props.sessionId,
		});
		if (!startupCommand) {
			return;
		}

		await invoke('async_write_to_pty', {
			sessionId: props.sessionId,
			data: startupCommand,
		});

		await invoke('async_confirm_startup_command_delivered', {
			sessionId: props.sessionId,
			deliveredCommand: startupCommand,
		});
	} catch (error) {
		console.error('Error applying startup command:', error);
	}
}

async function syncShellStatus() {
	if (!isShellReady || shellExited) {
		return;
	}

	try {
		const status = await invoke<ShellStatus>('async_get_shell_status', {
			sessionId: props.sessionId,
		});

		workspacesStore.setTabRuntimeInfo(props.sessionId, {
			runtimeCwd: status.cwd || undefined,
			runtimeCommand: status.running_command || undefined,
		});
	} catch (error) {
		shellExited = true;
		await workspacesStore.handleShellExit(props.sessionId);
	}
}

onMounted(async () => {
	if (!terminalElement.value) {
		return;
	}

	term.loadAddon(fitAddon);
	await setTerminalConfig();
	term.open(terminalElement.value);
	term.focus();

	void nextTick().then(async () => {
		// La pestaña puede haberse cerrado entre `term.open()` y este tick, y
		// entonces el elemento ya no está. Con el `!` que había, lo que seguía
		// leía propiedades de null.
		const el = terminalElement.value;
		if (!el) return;

		// If the element is display:none (v-show hidden tab), skip the wait.
		// It will be resized when the tab becomes active via the watcher.
		const hidden =
			el.offsetWidth === 0 && el.offsetHeight === 0 && getComputedStyle(el).display === 'none';

		if (!hidden) {
			await waitForRealSize(el);
		}

		// Ensure the font is measured before fitting so xterm.js
		// computes the correct cell height from the start.
		const ff = term.options.fontFamily as string;
		const fs = term.options.fontSize as number;
		if (typeof document !== 'undefined' && document.fonts) {
			const specs = [`${fs}px ${ff}`];
			if (ff !== 'monospace') specs.push(`${fs}px monospace`);
			await Promise.race([
				Promise.all(specs.map((s) => document.fonts.load(s).catch(() => 0))),
				sleep(2000),
			]);
		}
		(term as any)._core?._charSizeService?.measure?.();
		await sleep(50);

		fitAddon.fit();
		applyRectFit();

		try {
			// Subscribe before creating the shell so no PTY output is missed.
			await setupPtyListeners();
			await initShell();
			isShellReady = true;
			await applyStartupCommandIfAny();
			await syncShellStatus();
			term.focus();
		} catch (error) {
			isShellReady = false;
			console.error('Error creating shell:', error);
		}

		// Attach ResizeObserver after font loading so premature fits
		// don't use a stale cell height.
		resizeObserver = new ResizeObserver(() => {
			fitTerminal();
		});
		resizeObserver.observe(el);
	});

	// Listen for terminal input and write it to the pty
	const onDataDisposable = term.onData((data) => {
		writeToPty(data);
	});
	terminalDataDisposable = () => {
		onDataDisposable.dispose();
	};

	// Handle window resize
	window.addEventListener('resize', onResize);

	// El estado de la shell —el directorio y el comando en primer plano— se
	// consulta una vez por segundo **por pestaña**, y cada consulta lee tres
	// archivos de /proc. Con cinco pestañas abiertas eran cinco idas y vueltas
	// por el IPC por segundo, para siempre.
	//
	// No se puede empujar desde el backend: no hay notificación del kernel
	// cuando cambia el grupo en primer plano de un PTY, así que alguien tiene
	// que preguntar. Lo que sí se puede es no preguntar cuando nadie mira: con
	// la ventana minimizada o en otro escritorio, el sondeo no tiene lectores.
	const arrancarSondeo = () => {
		if (shellStatusIntervalId !== null) {
			return;
		}
		shellStatusIntervalId = setInterval(() => {
			void syncShellStatus();
		}, 1000);
	};

	const detenerSondeo = () => {
		if (shellStatusIntervalId !== null) {
			clearInterval(shellStatusIntervalId);
			shellStatusIntervalId = null;
		}
	};

	onVisibilityChange = () => {
		if (document.hidden) {
			detenerSondeo();
			return;
		}
		// Al volver se consulta ya, sin esperar el próximo tick: el directorio
		// pudo haber cambiado mientras la ventana estaba tapada.
		void syncShellStatus();
		arrancarSondeo();
	};
	document.addEventListener('visibilitychange', onVisibilityChange);

	if (!document.hidden) {
		arrancarSondeo();
	}

	keydownHandler = (e: KeyboardEvent) => {
		// Zoom in: Ctrl++ (Ctrl+Shift+=) or Ctrl+NumpadAdd
		if (e.ctrlKey && ((e.code === 'Equal' && e.shiftKey) || e.code === 'NumpadAdd')) {
			e.preventDefault();
			e.stopPropagation();
			const cur = (term.options.fontSize as number) || DEFAULT_FONT_SIZE;
			term.options.fontSize = Math.min(40, cur + 1);
			saveFontSize(term.options.fontSize as number);
			fitTerminal();
			return;
		}

		// Zoom out: Ctrl+- or Ctrl+NumpadSubtract
		if (e.ctrlKey && (e.code === 'Minus' || e.code === 'NumpadSubtract')) {
			e.preventDefault();
			e.stopPropagation();
			const cur = (term.options.fontSize as number) || DEFAULT_FONT_SIZE;
			term.options.fontSize = Math.max(8, cur - 1);
			saveFontSize(term.options.fontSize as number);
			fitTerminal();
			return;
		}

		// Reset zoom: Ctrl+0
		if (e.ctrlKey && (e.code === 'Digit0' || e.code === 'Numpad0')) {
			e.preventDefault();
			e.stopPropagation();
			term.options.fontSize = DEFAULT_FONT_SIZE;
			saveFontSize(DEFAULT_FONT_SIZE);
			fitTerminal();
			return;
		}

		// Copy: Ctrl+Shift+C
		if (e.ctrlKey && e.shiftKey && (e.code === 'KeyC' || e.key === 'C')) {
			e.preventDefault();
			e.stopPropagation();
			void copySelection();
		}

		// Paste: Ctrl+Shift+V
		if (e.ctrlKey && e.shiftKey && (e.code === 'KeyV' || e.key === 'V')) {
			e.preventDefault();
			e.stopPropagation();
			void pasteFromClipboard();
		}
	};

	terminalElement.value.addEventListener('keydown', keydownHandler, { capture: true });

	contextMenuHandler = (e: MouseEvent) => {
		void openTerminalContextMenu(e);
	};
	terminalElement.value.addEventListener('contextmenu', contextMenuHandler);

	pasteHandler = (e: ClipboardEvent) => {
		const text = e.clipboardData?.getData('text/plain');
		if (text) {
			e.preventDefault();
			e.stopPropagation();
			writeToPty(text);
		}
	};
	terminalElement.value.addEventListener('paste', pasteHandler);
});

async function applyTerminalConfig() {
	await setTerminalConfig();
	fitTerminal();
}

watch(
	() => {
		const conf = configStore.config as VSKConfig | null;
		return {
			style: conf?.style ?? null,
			fonts: conf?.fonts ?? null,
		};
	},
	() => {
		void applyTerminalConfig();
	},
	{ deep: true, immediate: true }
);

watch(
	() => props.active,
	(isActive) => {
		if (isActive) {
			void nextTick().then(async () => {
				await fitTerminal();
				term.focus();
			});
		}
	}
);

onBeforeUnmount(() => {
	ptyUnlisteners.forEach((un) => {
		un();
	});
	ptyUnlisteners = [];
	terminalDataDisposable?.();
	terminalDataDisposable = null;
	resizeObserver?.disconnect();
	resizeObserver = null;
	if (shellStatusIntervalId) {
		clearInterval(shellStatusIntervalId);
		shellStatusIntervalId = null;
	}
	if (onVisibilityChange) {
		document.removeEventListener('visibilitychange', onVisibilityChange);
		onVisibilityChange = null;
	}
	if (keydownHandler) {
		terminalElement.value?.removeEventListener('keydown', keydownHandler, { capture: true });
		keydownHandler = null;
	}
	if (pasteHandler) {
		terminalElement.value?.removeEventListener('paste', pasteHandler);
		pasteHandler = null;
	}
	if (contextMenuHandler) {
		terminalElement.value?.removeEventListener('contextmenu', contextMenuHandler);
		contextMenuHandler = null;
	}
	window.removeEventListener('resize', onResize);
	term.dispose();
});
</script>

<template>
  <div id="terminal" ref="terminalElement" class="w-full min-h-0 self-stretch"></div>
</template>