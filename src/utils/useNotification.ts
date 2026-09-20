import type { TonoDelAviso } from '@vasakgroup/vue-libvasak';
import { ref } from 'vue';

/**
 * Un aviso que aparece y se va.
 *
 * El tono se llama `tone` y no `type`, y su tipo es el de la librería: la pila
 * que los dibuja es la compartida, y dos vocabularios para los mismos colores
 * obligan a traducir en el medio.
 */
export interface Notification {
	id: number;
	message: string;
	tone: TonoDelAviso;
}

const notifications = ref<Notification[]>([]);
let nextId = 0;

export function useNotification() {
	function notify(message: string, tone: TonoDelAviso = 'success', timeout = 2000) {
		const id = nextId++;
		notifications.value.push({ id, message, tone });
		setTimeout(() => {
			const idx = notifications.value.findIndex((n) => n.id === id);
			if (idx !== -1) notifications.value.splice(idx, 1);
		}, timeout);
	}

	return { notifications, notify };
}
