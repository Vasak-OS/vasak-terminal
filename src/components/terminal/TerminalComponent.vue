<script lang="ts" setup>
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { onBeforeUnmount, onMounted, nextTick, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    sessionId: string;
    active?: boolean;
  }>(),
  {
    active: true,
  }
);

const terminalElement = ref<HTMLElement | null>(null);

const fitAddon = new FitAddon();
const term = new Terminal({
  fontFamily: "Jetbrains Mono",
  theme: {
    background: "rgb(47, 47, 47)",
  },
});

let terminalDataDisposable: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let isPtyReadLoopActive = false;

function onResize() {
  fitTerminal();
}

// Make the terminal fit all the window size
async function fitTerminal() {
  if (!terminalElement.value || !props.active) {
    return;
  }

  fitAddon.fit();
  void invoke<string>("async_resize_pty", {
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
      const data = await invoke<string | null>("async_read_from_pty", {
        sessionId: props.sessionId,
      });
      if (data) {
        await writeToTerminal(data);
      } else {
        await sleep(16);
      }
    } catch (error) {
      console.error("Error reading from pty:", error);
      await sleep(100);
    }
  }
}

// Write data from pty into the terminal
function writeToTerminal(data: string) {
  return new Promise<void>((r) => {
    term.write(data, () => r());
  });
}

// Write data from the terminal to the pty
function writeToPty(data: string) {
  void invoke("async_write_to_pty", {
    sessionId: props.sessionId,
    data,
  });
}
function initShell() {
  invoke("async_create_shell", {
    sessionId: props.sessionId,
  }).catch((error) => {
    console.error("Error creating shell:", error);
  });
}

onMounted(() => {
  if (!terminalElement.value) {
    return;
  }

  term.loadAddon(fitAddon);
  term.open(terminalElement.value);

  initShell();
  void nextTick().then(() => {
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
  window.addEventListener("resize", onResize);
  resizeObserver = new ResizeObserver(() => {
    fitTerminal();
  });
  resizeObserver.observe(terminalElement.value);
});

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
  window.removeEventListener("resize", onResize);
  term.dispose();
});
</script>

<template>
  <div id="terminal" ref="terminalElement" class="h-full w-full"></div>
</template>