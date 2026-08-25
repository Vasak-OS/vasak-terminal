import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import '@/assets/main.css';
import 'xterm/css/xterm.css';

/**
 * Saca de una URL lo que no debería quedar en un registro.
 *
 * Se conserva el esquema y la autoridad completos usando `href`, y no
 * `origin + pathname`: para esquemas propios como `asset:` o `ipc:` el `origin`
 * es la cadena «null», así que esa forma escribía `null/ruta` y perdía
 * justamente lo que permite entender qué se bloqueó.
 */
const sanearUrl = (valor: string | null | undefined): string => {
	if (!valor) {
		return '(en línea)';
	}
	try {
		const url = new URL(valor);
		if (url.protocol === 'data:') {
			return 'data:(recortado)';
		}
		// Credenciales, query y fragmento: ahí es donde viajan los tokens.
		url.username = '';
		url.password = '';
		url.search = '';
		url.hash = '';
		return url.href;
	} catch {
		// No era una URL absoluta —'inline', 'eval', una ruta relativa—: tal cual.
		return valor;
	}
};

document.addEventListener('securitypolicyviolation', (evento) => {
	// Se sanean **las dos** URLs. `sourceFile` también puede llevar query con
	// datos sensibles, y antes se escribía sin tocar.
	console.error(
		`[CSP] bloqueado ${sanearUrl(evento.blockedURI)} por la directiva ` +
			`«${evento.violatedDirective}» en ${sanearUrl(evento.sourceFile) || 'documento'}:${evento.lineNumber}`
	);
});

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
