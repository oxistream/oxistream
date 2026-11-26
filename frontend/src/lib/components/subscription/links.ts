import { resolve } from '$app/paths';
import type { ResolvedPathname } from '$app/types';

// Built separately; resolve() only accepts routes that exist in this tree.
export function importExportHref(): ResolvedPathname {
	return `${resolve('/')}subscriptions/import-export` as ResolvedPathname;
}
