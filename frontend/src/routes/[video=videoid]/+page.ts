import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';
import { watchPath } from '#lib/youtube.js';
import type { PageLoad } from './$types';

// youtu.be-style short links: /{videoId}?t=42
export const load: PageLoad = ({ params, url }) => {
	redirect(307, resolve(watchPath(params.video, url.searchParams) as `/watch?${string}`));
};
