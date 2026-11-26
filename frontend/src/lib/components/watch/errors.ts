import { ApiError } from '#lib/api/mapping.js';
import { m } from '#lib/paraglide/messages.js';

export type WatchProblem = {
	kind: 'live' | 'unavailable' | 'error';
	title: string;
	description: string;
	/** Whether trying again could help. */
	retry: boolean;
	/** Whether YouTube itself may still play it, e.g. for signed-in or local viewers. */
	youtube: boolean;
};

const REASONS: Record<string, () => Omit<WatchProblem, 'retry' | 'kind'>> = {
	'age-restricted': () => ({
		title: m.watch_error_age_restricted_title(),
		description: m.watch_error_age_restricted_body(),
		youtube: true
	}),
	private: () => ({
		title: m.watch_error_private_title(),
		description: m.watch_error_private_body(),
		youtube: false
	}),
	deleted: () => ({
		title: m.watch_error_deleted_title(),
		description: m.watch_error_deleted_body(),
		youtube: false
	}),
	geoblocked: () => ({
		title: m.watch_error_geoblocked_title(),
		description: m.watch_error_geoblocked_body(),
		youtube: true
	}),
	'members-only': () => ({
		title: m.watch_error_members_only_title(),
		description: m.watch_error_members_only_body(),
		youtube: true
	}),
	'premium-only': () => ({
		title: m.watch_error_premium_title(),
		description: m.watch_error_premium_body(),
		youtube: true
	}),
	paid: () => ({
		title: m.watch_error_paid_title(),
		description: m.watch_error_paid_body(),
		youtube: true
	}),
	'offline stream': () => ({
		title: m.watch_error_offline_stream_title(),
		description: m.watch_error_offline_stream_body(),
		youtube: true
	}),
	no_playable_streams: () => ({
		title: m.watch_error_no_streams_title(),
		description: m.watch_error_no_streams_body(),
		youtube: true
	})
};

export function liveProblem(): WatchProblem {
	return {
		kind: 'live',
		title: m.watch_live_title(),
		description: m.watch_live_body(),
		retry: false,
		youtube: true
	};
}

/** Explains a failed video or manifest request in the user's words. */
export function describeProblem(error: unknown): WatchProblem {
	if (!(error instanceof ApiError)) {
		return {
			kind: 'error',
			title: m.watch_error_upstream_title(),
			description: m.error_load_body(),
			retry: true,
			youtube: false
		};
	}
	switch (error.code) {
		case 'not_found':
			return {
				kind: 'unavailable',
				title: m.watch_error_not_found_title(),
				description: m.watch_error_not_found_body(),
				retry: false,
				youtube: false
			};
		case 'bad_request':
			return {
				kind: 'unavailable',
				title: m.watch_error_bad_request_title(),
				description: m.watch_error_bad_request_body(),
				retry: false,
				youtube: false
			};
		case 'unavailable': {
			if (error.reason === 'livestream') return liveProblem();
			const known = error.reason ? REASONS[error.reason] : undefined;
			return {
				kind: 'unavailable',
				retry: false,
				...(known?.() ?? {
					title: m.watch_error_unavailable_title(),
					description: m.watch_error_unavailable_body(),
					youtube: true
				})
			};
		}
		case 'rate_limited':
		case 'botguard_failed':
			return {
				kind: 'error',
				title: m.watch_error_rate_limited_title(),
				description: m.watch_error_rate_limited_body(),
				retry: true,
				youtube: false
			};
		default:
			return {
				kind: 'error',
				title: m.watch_error_upstream_title(),
				description: error.code ? m.watch_error_upstream_body() : m.error_load_body(),
				retry: true,
				youtube: false
			};
	}
}
