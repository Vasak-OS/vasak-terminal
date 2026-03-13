<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import {
	getSchemeById,
	useConfigStore,
	VSKConfig,
} from '@vasakgroup/plugin-config-manager';
import { Store } from 'pinia';
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';

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
const terminalElement = ref<HTMLElement | null>(null);

const fitAddon = new FitAddon();
const term = new Terminal({
	allowTransparency: true,
	fontFamily: 'MesloLGL Nerd Font Mono',
	theme: {
		background: 'rgba(0, 0, 0, 0)',
	},
});

let terminalDataDisposable: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let isPtyReadLoopActive = false;
let isShellReady = false;

function onResize() {
	fitTerminal();
}

// Make the terminal fit all the window size
async function fitTerminal() {
	if (!terminalElement.value || !props.active) {
		return;
	}

	fitAddon.fit();

	if (!isShellReady) {
		return;
	}

	void invoke<string>('async_resize_pty', {
		sessionId: props.sessionId,
		rows: term.rows,
		cols: term.cols,
	});
}

function sleep(ms: number) {
	return new Promise<void>((resolve) => {
		setTimeout(resolve, ms);
	});
}

async function startPtyReadLoop() {
	isPtyReadLoopActive = true;

	while (isPtyReadLoopActive) {
		try {
			const data = await invoke<string | null>('async_read_from_pty', {
				sessionId: props.sessionId,
			});
			if (data) {
				await writeToTerminal(data);
			} else {
				await sleep(16);
			}
		} catch (error) {
			console.error('Error reading from pty:', error);
			await sleep(100);
		}
	}
}

const setTerminalConfig = async () => {
	const conf = configStore.config as VSKConfig | null;
	if (!conf) return;

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

// Write data from pty into the terminal
function writeToTerminal(data: string) {
	return new Promise<void>((r) => {
		term.write(data, () => r());
	});
}

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
	}).catch((error) => {
		console.error('Error creating shell:', error);
	});
}

onMounted(async () => {
	if (!terminalElement.value) {
		return;
	}

	term.loadAddon(fitAddon);
	term.open(terminalElement.value);

	void nextTick().then(async () => {
		fitAddon.fit();
		await initShell();
		isShellReady = true;
		fitTerminal();
	});

	void startPtyReadLoop();

	// Listen for terminal input and write it to the pty
	const onDataDisposable = term.onData((data) => {
		writeToPty(data);
	});
	terminalDataDisposable = () => {
		onDataDisposable.dispose();
	};

	// Handle window resize
	window.addEventListener('resize', onResize);
	resizeObserver = new ResizeObserver(() => {
		fitTerminal();
	});
	resizeObserver.observe(terminalElement.value);
});

watch(
	() => {
		const conf = configStore.config as VSKConfig | null;
		return conf?.style ?? null;
	},
	() => {
		void setTerminalConfig();
	},
	{ deep: true, immediate: true }
);

watch(
	() => props.active,
	(isActive) => {
		if (isActive) {
			void nextTick().then(() => {
				fitTerminal();
			});
		}
	}
);

onBeforeUnmount(() => {
	isPtyReadLoopActive = false;
	terminalDataDisposable?.();
	terminalDataDisposable = null;
	resizeObserver?.disconnect();
	resizeObserver = null;
	window.removeEventListener('resize', onResize);
	term.dispose();
});
</script>

<template>
  <div id="terminal" ref="terminalElement" class="h-full w-full"></div>
</template>