import { ref } from 'vue';

export interface Notification {
	id: number;
	message: string;
	type: 'success' | 'info' | 'error';
}

const notifications = ref<Notification[]>([]);
let nextId = 0;

export function useNotification() {
	function notify(message: string, type: Notification['type'] = 'success', timeout = 2000) {
		const id = nextId++;
		notifications.value.push({ id, message, type });
		setTimeout(() => {
			const idx = notifications.value.findIndex((n) => n.id === id);
			if (idx !== -1) notifications.value.splice(idx, 1);
		}, timeout);
	}

	return { notifications, notify };
}
