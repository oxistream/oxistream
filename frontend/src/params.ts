import { defineParams } from '@sveltejs/kit/params';
// Loaded by Node during the build, which resolves neither aliases nor .js to .ts.
import { isHandle, isVideoId } from './lib/youtube.ts';

export const params = defineParams({
	videoid: (param) => (isVideoId(param) ? param : undefined),
	handle: (param) => (isHandle(`@${param}`) ? param : undefined)
});
