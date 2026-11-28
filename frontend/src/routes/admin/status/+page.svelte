<script lang="ts">
	import { createAdminStatusQuery, type AdminStatus } from '#lib/api/admin.js';
	import ErrorState from '#lib/components/state/error-state.svelte';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { formatDate, formatNumber, formatRelativeTime, formatUptime } from '#lib/format.js';
	import { pageTitle } from '#lib/navigation.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { cn } from '#lib/utils.js';

	type Tone = 'ok' | 'warn' | 'bad' | undefined;
	type Row = { label: string; value: string; tone?: Tone; title?: string; mono?: boolean };
	type Section = { id: string; title: string; rows: Row[] };

	const status = createAdminStatusQuery();
	const locale = $derived(getLocale());

	const DASH = '—';

	function count(value: number | null | undefined): string {
		return value === null || value === undefined ? DASH : formatNumber(value, locale);
	}

	function onOff(value: boolean): string {
		return value ? m.admin_on() : m.admin_off();
	}

	function ago(date: string | null | undefined): Pick<Row, 'value' | 'title'> {
		if (!date) return { value: DASH };
		return { value: formatRelativeTime(date, locale), title: formatDate(date, locale) };
	}

	function sections(s: AdminStatus): Section[] {
		const { feed, counts, auth } = s;
		const run = feed.last_run;
		return [
			{
				id: 'server',
				title: m.admin_status_server(),
				rows: [
					{ label: m.admin_status_version(), value: s.version, mono: true },
					...(s.git_sha
						? [{ label: m.admin_status_commit(), value: s.git_sha.slice(0, 12), mono: true }]
						: []),
					{
						label: m.admin_status_uptime(),
						value: formatUptime(s.uptime_seconds, locale),
						title: formatDate(s.started_at, locale)
					},
					{
						label: m.admin_status_botguard(),
						value: s.botguard_version ?? m.admin_status_not_found(),
						tone: s.botguard_version ? undefined : 'warn',
						mono: !!s.botguard_version
					},
					{ label: m.admin_status_player_cache(), value: count(s.player_cache_entries) }
				]
			},
			{
				id: 'database',
				title: m.admin_status_database(),
				rows: [
					{
						label: m.admin_status_connection(),
						value: s.database.reachable ? m.admin_status_reachable() : m.admin_status_unreachable(),
						tone: s.database.reachable ? 'ok' : 'bad'
					},
					{
						label: m.admin_status_postgres(),
						value: s.database.version ?? DASH,
						mono: !!s.database.version
					}
				]
			},
			{
				id: 'feed',
				title: m.admin_status_feed(),
				rows: [
					{
						label: m.admin_status_state(),
						value:
							feed.running === null || feed.running === undefined
								? DASH
								: feed.running
									? m.admin_status_running()
									: m.admin_status_idle(),
						tone: feed.running ? 'ok' : undefined
					},
					{
						label: m.admin_status_interval(),
						value:
							feed.interval_minutes === null || feed.interval_minutes === undefined
								? DASH
								: m.admin_status_minutes({ count: feed.interval_minutes })
					},
					{ label: m.admin_status_due(), value: count(feed.due) },
					{ label: m.admin_status_pending(), value: count(feed.pending) },
					{
						label: m.admin_status_failing(),
						value: count(feed.failing),
						tone: feed.failing ? 'warn' : undefined
					},
					{ label: m.admin_status_last_fetch(), ...ago(feed.last_checked_at) },
					{
						label: m.admin_status_last_run(),
						value: run
							? `${m.admin_status_run_summary({
									fetched: run.channels_fetched + run.playlists_fetched,
									failed: run.channels_failed + run.playlists_failed
								})} · ${formatRelativeTime(run.finished_at, locale)}`
							: DASH,
						title: run ? formatDate(run.finished_at, locale) : undefined
					},
					...(run?.rate_limited
						? [{ label: m.admin_status_rate_limited(), value: m.admin_on(), tone: 'warn' as const }]
						: []),
					...(feed.last_error_at
						? [
								{
									label: m.admin_status_last_error(),
									...ago(feed.last_error_at),
									tone: 'bad' as const
								}
							]
						: [])
				]
			},
			{
				id: 'data',
				title: m.admin_status_data(),
				rows: [
					{ label: m.admin_status_users(), value: count(counts?.users) },
					{ label: m.admin_status_admins(), value: count(counts?.admins) },
					{ label: m.admin_status_disabled(), value: count(counts?.disabled_users) },
					{ label: m.admin_status_channels(), value: count(counts?.channels) },
					{ label: m.admin_status_subscriptions(), value: count(counts?.subscriptions) },
					{ label: m.admin_status_playlists(), value: count(counts?.playlists) },
					{ label: m.admin_status_videos(), value: count(counts?.videos) }
				]
			},
			{
				id: 'config',
				title: m.admin_status_config(),
				rows: [
					{ label: m.admin_status_local(), value: onOff(auth.local_enabled) },
					{ label: m.admin_status_registration(), value: onOff(auth.registration_enabled) },
					{ label: m.admin_status_require_login(), value: onOff(auth.require_login) },
					{
						label: m.admin_status_sso(),
						value: !auth.oidc_enabled
							? m.admin_off()
							: auth.oidc_discovered
								? m.admin_status_discovered()
								: m.admin_status_discovering(),
						tone: auth.oidc_enabled && !auth.oidc_discovered ? 'warn' : undefined
					},
					{ label: m.admin_status_sponsorblock(), value: onOff(s.sponsorblock) },
					{ label: m.admin_status_trending_region(), value: s.trending_region, mono: true },
					{ label: m.admin_status_rate_limit(), value: onOff(s.rate_limit) }
				]
			}
		];
	}

	const toneClass: Record<NonNullable<Tone>, string> = {
		ok: 'bg-emerald-500',
		warn: 'bg-amber-500',
		bad: 'bg-destructive'
	};
</script>

<svelte:head><title>{pageTitle(m.admin_status())}</title></svelte:head>

{#if status.isPending}
	<div
		class="grid grid-cols-1 gap-4 md:grid-cols-2"
		role="status"
		aria-label={m.admin_status_loading()}
	>
		{#each { length: 4 }, i (i)}<Skeleton class="h-48 rounded-xl" />{/each}
	</div>
{:else if status.isError && !status.data}
	<ErrorState onretry={() => void status.refetch()} />
{:else if status.data}
	<div class="grid grid-cols-1 items-start gap-4 md:grid-cols-2" data-testid="admin-status">
		{#each sections(status.data) as section (section.id)}
			<section class="min-w-0 rounded-xl border px-4 py-3" data-testid="admin-status-{section.id}">
				<h2 class="pb-2 text-sm font-semibold tracking-tight">{section.title}</h2>
				<dl class="divide-y text-sm">
					{#each section.rows as row (row.label)}
						<div class="flex items-center justify-between gap-4 py-2">
							<dt class="shrink-0 text-muted-foreground">{row.label}</dt>
							<dd
								class={cn(
									'flex min-w-0 items-center gap-2 text-end font-medium tabular',
									row.mono && 'font-mono text-xs'
								)}
								title={row.title}
							>
								{#if row.tone}
									<span class={cn('size-2 shrink-0 rounded-full', toneClass[row.tone])}></span>
								{/if}
								<span class="break-words">{row.value}</span>
							</dd>
						</div>
					{/each}
				</dl>
			</section>
		{/each}
	</div>
{/if}
