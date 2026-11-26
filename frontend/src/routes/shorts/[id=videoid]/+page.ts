import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';
import { watchPath } from '#lib/youtube.js';
import type { PageLoad } from './$types';

// There is no shorts UI yet, so shorts play on the regular watch page.
export const load: PageLoad = ({ params, url }) => {
	redirect(307, resolve(watchPath(params.id, url.searchParams) as `/watch?${string}`));
};
