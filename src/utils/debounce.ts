type AnyFn = (...args: any[]) => void;

type DebouncedFn<T extends AnyFn> = ((...args: Parameters<T>) => void) & {
	cancel: () => void;
	flush: () => void;
};

export function useDebounceFn<T extends AnyFn>(fn: T, delayMs = 200): DebouncedFn<T> {
	let timer: ReturnType<typeof setTimeout> | null = null;
	let lastArgs: Parameters<T> | null = null;

	const debounced = (...args: Parameters<T>) => {
		lastArgs = args;

		if (timer) {
			clearTimeout(timer);
		}

		timer = setTimeout(() => {
			timer = null;

			if (lastArgs) {
				fn(...lastArgs);
			}
		}, delayMs);
	};

	debounced.cancel = () => {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
	};

	debounced.flush = () => {
		if (!timer || !lastArgs) {
			return;
		}

		clearTimeout(timer);
		timer = null;
		fn(...lastArgs);
	};

	return debounced;
}
