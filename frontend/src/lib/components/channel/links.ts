import type { ResolvedPathname } from '$app/types';
import { channelTabHref, type ChannelSort, type ChannelTab } from '#lib/channel-tabs.js';

/** `channelTabHref` below a base path that already went through `resolve()`. */
export function channelHref(
	base: ResolvedPathname,
	tab: ChannelTab,
	options?: { sort?: ChannelSort; query?: string }
): ResolvedPathname {
	return channelTabHref(base, tab, options) as ResolvedPathname;
}
