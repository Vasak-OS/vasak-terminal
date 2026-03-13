type Fn = () => void;

interface TimeoutFnReturn {
	start: () => void;
	stop: () => void;
}

export function useTimeoutFn(
	fn: Fn,
	delayMs = 200,
	options?: { immediate?: boolean }
): TimeoutFnReturn {
	let timer: ReturnType<typeof setTimeout> | null = null;

	const start = () => {
		if (timer) {
			clearTimeout(timer);
		}

		timer = setTimeout(() => {
			timer = null;
			fn();
		}, delayMs);
	};

	const stop = () => {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
	};

	if (options?.immediate) {
		fn();
	}

	return {
		start,
		stop,
	};
}
