import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';
import type { PageLoad } from './$types';

// Invite links from before parties were app-wide.
export const load: PageLoad = ({ params }) => {
	redirect(308, resolve('/party/[id]', { id: params.room }));
};
