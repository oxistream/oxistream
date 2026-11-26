<script lang="ts">
	import { resolve } from '$app/paths';
	import { formatDate, formatDuration, formatNumber, formatRelativeTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';
	import { parseDescription } from './description';

	let {
		description,
		viewCount,
		viewers,
		publishedAt,
		publishedText,
		durationSeconds,
		expanded = $bindable(false),
		now,
		onseek,
		class: className
	}: {
		description: string;
		viewCount?: number | null;
		/** Current viewers of a live stream, shown instead of the view count. */
		viewers?: number | null;
		publishedAt?: string | null;
		publishedText?: string | null;
		durationSeconds?: number | null;
		expanded?: boolean;
		/** Reference time for the relative date; fixed in stories. */
		now?: Date;
		onseek?: (seconds: number) => void;
		class?: string;
	} = $props();

	const locale = getLocale();
	const uid = $props.id();
	const segments = $derived(parseDescription(description.trim(), durationSeconds));
	// Short descriptions fit in the collapsed box, so they get no toggle.
	const collapsible = $derived(
		description.trim().split('\n').length > 3 || description.length > 220
	);
	const open = $derived(expanded || !collapsible);
</script>

<section
	aria-labelledby="{uid}-heading"
	class={cn('rounded-xl bg-muted/70 px-3.5 py-3 text-sm dark:bg-muted/60', className)}
>
	<h2 id="{uid}-heading" class="sr-only">{m.watch_description()}</h2>
	<p class="font-medium tabular">
		{#if viewers != null}
			{m.video_watching({ count: viewers, formatted: formatNumber(viewers, locale) })}
		{:else if viewCount != null}
			{m.video_views({ count: viewCount, formatted: formatNumber(viewCount, locale) })}
		{/if}
		{#if publishedAt}
			{#if viewers != null || viewCount != null}
				<span aria-hidden="true" class="mx-0.5 text-muted-foreground">·</span>
			{/if}
			<time datetime={publishedAt} title={formatDate(publishedAt, locale)}>
				{open ? formatDate(publishedAt, locale) : formatRelativeTime(publishedAt, locale, now)}
			</time>
		{:else if publishedText}
			{#if viewers != null || viewCount != null}
				<span aria-hidden="true" class="mx-0.5 text-muted-foreground">·</span>
			{/if}
			{publishedText}
		{/if}
	</p>

	<div
		id="{uid}-body"
		data-testid="video-description"
		data-expanded={open}
		class={cn(
			'mt-1.5 break-words whitespace-pre-line text-foreground/90',
			!open && 'line-clamp-3 @[60rem]:line-clamp-none'
		)}
	>
		{#if segments.length === 0}
			<span class="text-muted-foreground">{m.watch_no_description()}</span>
		{/if}
		{#each segments as segment, i (i)}
			{#if segment.type === 'text'}{segment.text}{:else if segment.type === 'link'}
				{#if segment.internal}
					<!-- toLocalPath() already returns an app path; the cast only satisfies the route types. -->
					<a
						href={resolve(segment.href as '/')}
						class="text-primary underline-offset-2 hover:underline"
					>
						{segment.text}
					</a>
				{:else}
					<!-- eslint-disable svelte/no-navigation-without-resolve -- external link -->
					<a
						href={segment.href}
						target="_blank"
						rel="noopener noreferrer nofollow"
						class="text-primary underline-offset-2 hover:underline"
					>
						{segment.text}
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{/if}
			{:else}
				<button
					type="button"
					class="rounded-sm font-medium text-primary tabular underline-offset-2 hover:underline"
					aria-label={m.watch_seek_to({ time: formatDuration(segment.seconds) })}
					onclick={() => onseek?.(segment.seconds)}
				>
					{segment.text}
				</button>
			{/if}
		{/each}
	</div>

	{#if collapsible}
		<button
			type="button"
			aria-expanded={expanded}
			aria-controls="{uid}-body"
			onclick={() => (expanded = !expanded)}
			class="-ms-1 mt-1.5 rounded-md px-1 py-0.5 font-medium transition-colors duration-(--motion-in) ease-ui hover:bg-foreground/5 @[60rem]:hidden"
		>
			{expanded ? m.watch_show_less() : m.watch_show_more()}
		</button>
	{/if}
</section>
