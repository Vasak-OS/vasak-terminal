<script lang="ts" setup>
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { onBeforeUnmount, onMounted, nextTick, ref } from "vue";

const terminalElement = ref<HTMLElement | null>(null);

const fitAddon = new FitAddon();
const term = new Terminal({
  fontFamily: "Jetbrains Mono",
  theme: {
    background: "rgb(47, 47, 47)",
  },
});

let unlistenPtyData: (() => void) | null = null;
let terminalDataDisposable: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let isPtyReadLoopActive = false;

function onResize() {
  fitTerminal();
}

// Make the terminal fit all the window size
async function fitTerminal() {
  if (!terminalElement.value) {
    return;
  }

  fitAddon.fit();
  void invoke<string>("async_resize_pty", {
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
      const data = await invoke<string | null>("async_read_from_pty");
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
    data,
  });
}
function initShell() {
  invoke("async_create_shell").catch((error) => {
    // on linux it seem to to "Operation not permitted (os error 1)" but it still works because echo $SHELL give /bin/bash
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

onBeforeUnmount(() => {
  isPtyReadLoopActive = false;
  unlistenPtyData?.();
  unlistenPtyData = null;
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