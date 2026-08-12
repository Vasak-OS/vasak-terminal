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
app.use(pinia);

app.mount('#app');
