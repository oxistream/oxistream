<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { mockVideos } from '#lib/mocks/videos.js';
	import LikeToggle from './like-toggle.svelte';
	import LikedPlaylistCard from './liked-playlist-card.svelte';

	const { Story } = defineMeta({
		title: 'Like/LikeToggle',
		component: LikeToggle,
		args: { liked: false }
	});
</script>

<script lang="ts">
	let liked = $state(false);
	let likedCount = $state(false);
	let likedOverlay = $state(true);
</script>

<Story name="Toggle" asChild>
	<div class="flex flex-wrap items-center gap-2">
		<LikeToggle {liked} onclick={() => (liked = !liked)} />
		<LikeToggle liked={likedCount} count={18_420} onclick={() => (likedCount = !likedCount)} />
		<LikeToggle liked />
		<LikeToggle loading />
		<LikeToggle signInHref="/login" count={18_420} />
	</div>
</Story>

<Story name="Over video" asChild>
	<div class="flex w-fit flex-col items-center gap-3 rounded-xl bg-neutral-900 p-3">
		<LikeToggle
			variant="overlay"
			liked={likedOverlay}
			onclick={() => (likedOverlay = !likedOverlay)}
		/>
		<LikeToggle variant="overlay" />
		<LikeToggle variant="overlay" signInHref="/login" />
	</div>
</Story>

<Story name="Library card" asChild>
	<div class="@container max-w-3xl">
		<ul class="video-list" data-layout="grid">
			<li class="min-w-0">
				<LikedPlaylistCard count={12} thumbnailUrl={mockVideos[3].thumbnailUrl} />
			</li>
			<li class="min-w-0"><LikedPlaylistCard count={0} /></li>
		</ul>
	</div>
</Story>
