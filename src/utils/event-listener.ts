type EventListener<T extends Event = Event> = (event: T) => void;

export function useEventListener<T extends Event = Event>(
	target: EventTarget | null,
	event: string,
	handler: EventListener<T>,
	options?: AddEventListenerOptions
): void {
	if (!target) return;

	target.addEventListener(event, handler as EventListener, options);

	// Note: To remove the listener, you'll need to keep track of it separately
	// or use an effect cleanup if you're in a component context
}
