import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import '@/assets/main.css';
import 'xterm/css/xterm.css';

/**
 * Los valores que la especificación de CSP informa en lugar de una URL.
 *
 * Van tal cual: no son rutas y recortarlos los volvería ilegibles.
 */
const MARCADORES_CSP = new Set([
	'inline',
	'eval',
	'wasm-eval',
	'data',
	'blob',
	'filesystem',
	'self',
	'unsafe-eval',
	'unsafe-inline',
]);

/**
 * Saca de una URL lo que no debería quedar en un registro.
 *
 * Se conserva el esquema y la autoridad usando `href`, y no `origin + pathname`:
 * para esquemas propios como `asset:` o `ipc:` el `origin` es la cadena «null»,
 * así que esa forma escribía `null/ruta` y perdía justamente lo que permite
 * entender qué se bloqueó.
 *
 * El caso que faltaba cubrir es el del `catch`: una ruta relativa o
 * protocol-relative hace que `new URL` falle, y devolverla tal cual dejaba la
 * query y el fragmento en el registro — o sea, exactamente lo que esta función
 * viene a evitar. Ahora sólo pasan sin tocar los marcadores de la
 * especificación; cualquier otra cosa se corta antes de `?` o `#`.
 */
const sanearUrl = (valor: string | null | undefined): string => {
	if (!valor) {
		return '';
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
		if (MARCADORES_CSP.has(valor)) {
			return valor;
		}
		return valor.split(/[?#]/)[0];
	}
};

// Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
// medias sin decir nada. Se sanean **las dos** URLs, porque `sourceFile` también
// puede llevar query con datos sensibles.
document.addEventListener('securitypolicyviolation', (evento) => {
	// El respaldo se decide antes de sanear: `sanearUrl` nunca devuelve vacío
	// para una entrada con contenido, así que un `|| 'documento'` después de
	// llamarla era código muerto.
	const recurso = evento.blockedURI ? sanearUrl(evento.blockedURI) : '(en línea)';
	const origen = evento.sourceFile ? sanearUrl(evento.sourceFile) : 'documento';
	console.error(
		`[CSP] bloqueado ${recurso} por la directiva ` +
			`«${evento.violatedDirective}» en ${origen}:${evento.lineNumber}`
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
