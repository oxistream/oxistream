import type { QueueEntry, QueueOp, QueueRemote, QueueState } from '#lib/player/queue.svelte.js';
import { api } from './client';
import { pickImage, unwrap } from './mapping';
import type { components } from './schema';

type Schemas = components['schemas'];

const CHANNEL_ID = /^UC[\w-]{22}$/;
const MAX_TITLE_CHARS = 500;

export function fromQueue(queue: Schemas['UpNextQueue']): QueueState {
	return {
		revision: queue.revision,
		entries: queue.entries.map(({ key, video }) => ({
			key,
			id: video.id,
			title: video.title,
			channel: video.channel ? { id: video.channel.id, name: video.channel.name } : null,
			thumbnailUrl: pickImage(video.thumbnails, 320),
			durationSeconds: video.duration_seconds ?? undefined
		}))
	};
}

export function toNewEntry(entry: QueueEntry): Schemas['NewUpNextEntry'] {
	const channel = entry.channel && CHANNEL_ID.test(entry.channel.id) ? entry.channel : null;
	return {
		key: entry.key,
		video_id: entry.id,
		title: [...(entry.title.trim() || entry.id)].slice(0, MAX_TITLE_CHARS).join(''),
		channel_id: channel?.id ?? null,
		channel_name: channel?.name.slice(0, 200) ?? null,
		duration_seconds:
			entry.durationSeconds === undefined ? null : Math.max(0, Math.round(entry.durationSeconds))
	};
}

/** The signed-in user's queue on the server. */
export const queueRemote: QueueRemote = {
	async load() {
		return fromQueue(unwrap(await api.GET('/api/v0/queue')));
	},
	async apply(op: QueueOp) {
		switch (op.type) {
			case 'add':
				return fromQueue(
					unwrap(
						await api.POST('/api/v0/queue/entries', {
							body: {
								entries: op.entries.map(toNewEntry),
								position: op.position,
								skip_queued: op.skipQueued ?? false
							}
						})
					)
				);
			case 'remove':
				return fromQueue(
					unwrap(
						await api.DELETE('/api/v0/queue/entries/{key}', {
							params: { path: { key: op.key } }
						})
					)
				);
			case 'move':
				return fromQueue(
					unwrap(
						await api.PUT('/api/v0/queue/entries/{key}/position', {
							params: { path: { key: op.key } },
							body: { index: op.index }
						})
					)
				);
			case 'clear':
				return fromQueue(unwrap(await api.DELETE('/api/v0/queue')));
		}
	}
};
