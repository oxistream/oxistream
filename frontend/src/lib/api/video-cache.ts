function idOf(item: unknown): string | undefined {
	if (!item || typeof item !== 'object') return undefined;
	const { id } = item as { id?: unknown };
	return typeof id === 'string' ? id : undefined;
}

/** Drops every list entry with the video's ID from cached query data, however deep. */
export function withoutVideo<T>(data: T, videoId: string): T {
	if (Array.isArray(data)) {
		return (data as unknown[])
			.filter((item) => idOf(item) !== videoId)
			.map((item) => withoutVideo(item, videoId)) as T;
	}
	if (data && typeof data === 'object') {
		return Object.fromEntries(
			Object.entries(data).map(([key, value]) => [key, withoutVideo(value, videoId)])
		) as T;
	}
	return data;
}
