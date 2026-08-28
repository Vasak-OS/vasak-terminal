<script lang="ts" setup>
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { useReactiveIcon } from '@/utils/useReactiveIcon';

const { t } = useI18n();
const appWindow = getCurrentWindow();
const { closeIcon, minimizeIcon, maximizeIcon } = useReactiveIcon({
	closeIcon: 'window-close',
	minimizeIcon: 'window-minimize',
	maximizeIcon: 'window-maximize',
});
</script>
<template>
  <!-- Botones y no `span`: son controles, y como `span` no los alcanza el
       tabulador ni los anuncia un lector de pantalla. El nombre va traducido y
       también como `title`, que es lo que se ve al pasar el mouse; antes estaba
       en inglés y escrito a mano en todas las aplicaciones del escritorio. -->
  <div class="flex gap-1" data-tauri-drag-region>
    <button
      type="button"
      class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-success border border-ui-border"
      :title="t('windowControls.minimize')"
      :aria-label="t('windowControls.minimize')"
      @click="appWindow.minimize()"
    >
      <img :src="minimizeIcon" class="h-6 w-6 inline-block" alt="">
    </button>
    <button
      type="button"
      class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-warning border border-ui-border"
      :title="t('windowControls.maximize')"
      :aria-label="t('windowControls.maximize')"
      @click="appWindow.toggleMaximize()"
    >
      <img :src="maximizeIcon" class="h-6 w-6 inline-block" alt="">
    </button>
    <button
      type="button"
      class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-error border border-ui-border"
      :title="t('windowControls.close')"
      :aria-label="t('windowControls.close')"
      @click="appWindow.close()"
    >
      <img :src="closeIcon" class="h-6 w-6 inline-block" alt="">
    </button>
  </div>
</template>