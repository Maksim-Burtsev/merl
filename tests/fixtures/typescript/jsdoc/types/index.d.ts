export namespace Lint {
	interface LimitExceeded {
		warnLimit: number;
	}

	interface Styler {
		outStyle: string;
	}

	interface Marker {
		popMark(): void;
	}
}
