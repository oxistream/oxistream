import { redirect } from '@sveltejs/kit';
import { session } from '#lib/session.svelte.js';
import { sectionHref, visibleSections } from '#lib/settings/sections.js';
import type { PageLoad } from './$types';

// The overview list is for phones; wider screens open the first section next to the navigation.
export const load: PageLoad = () => {
	const [first] = visibleSections(session.user !== null);
	if (first && matchMedia('(min-width: 768px)').matches) redirect(307, sectionHref(first));
};
