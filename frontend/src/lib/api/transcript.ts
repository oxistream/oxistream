import { createQuery } from '@tanstack/svelte-query';
import { parseVtt, toTranscript, type TranscriptLine } from '#lib/components/watch/transcript.js';

export async function fetchTranscript(url: string): Promise<TranscriptLine[]> {
	const response = await fetch(url);
	if (!response.ok) throw new Error(`Subtitles failed with ${response.status}`);
	return toTranscript(parseVtt(await response.text()));
}

/** A subtitle track as transcript lines. The URL is signed, so the key is the video and track. */
export function useTranscript(
	track: () => { videoId: string; id: string; url: string } | null | undefined
) {
	return createQuery(() => {
		const current = track();
		return {
			queryKey: ['transcript', current?.videoId, current?.id],
			queryFn: () => fetchTranscript(current!.url),
			enabled: !!current,
			staleTime: Infinity,
			retry: 1
		};
	});
}
