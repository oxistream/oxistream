/** localStorage access that never throws: private modes, blocked storage and quota errors read as empty. */
export function readJson<T>(key: string, isValid: (value: unknown) => value is T): T | null {
	try {
		const raw = localStorage.getItem(key);
		if (raw === null) return null;
		const value: unknown = JSON.parse(raw);
		return isValid(value) ? value : null;
	} catch {
		return null;
	}
}

export function writeJson(key: string, value: unknown): void {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// Storage unavailable or full; the value simply is not remembered.
	}
}
