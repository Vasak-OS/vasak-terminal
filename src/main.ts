import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import '@/assets/main.css';
import 'xterm/css/xterm.css';

const i18n = I18n.getInstance();
const app = createApp(App);
const pinia = createPinia();

// Nobody was asking the backend for the translations, so every label in the tab
// bar rendered as its own key.
i18n.load();

// El clic derecho abre el menú de VasakOS —el mismo de todo el escritorio— y no
// el del motor del navegador, que ofrecía «Recargar» e «Inspeccionar». Ctrl+F
// sigue libre a propósito: adentro del terminal esa combinación es de la consola
// y tiene que llegar al programa que corre adentro.
setupContextMenu({ iconResolver: getIconSource });

app.use(pinia);

app.mount('#app');
