<script module lang="ts">
	export type PlayerSubtitle = {
		id: string;
		lang: string;
		label: string;
		url: string;
	};
</script>

<script lang="ts">
	import '@videojs/html/video/player';
	import '@videojs/html/media/dash-video';
	import './videojs/video/skin';
	import RectangleHorizontalIcon from '@lucide/svelte/icons/rectangle-horizontal';
	import {
		selectPlayback,
		selectPlaybackRate,
		selectTextTrack,
		selectTime,
		selectVolume
	} from '@videojs/html';
	import type { DashEngineConfig } from '@videojs/dash-video';
	import type { VideoPlayerElement } from '@videojs/html/video';
	import { onMount } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { cn } from '#lib/utils.js';
	import { readPreferences, savePreferences } from './preferences';
	import { bridgeDashAudioTracks, type AudioTrackHost } from './dash-audio-tracks';
	import { createProgressEmitter, type ProgressReport } from './progress';

	let {
		src,
		poster,
		subtitles = [],
		storyboard,
		startTime = 0,
		autoplay = false,
		dashSettings,
		theater = false,
		globalHotkeys = true,
		ontheatertoggle,
		ontimeupdate,
		onprogress,
		onended,
		class: className
	}: {
		/** DASH manifest URL. */
		src: string;
		poster?: string;
		subtitles?: PlayerSubtitle[];
		/** WebVTT thumbnails track for seek previews. */
		storyboard?: string;
		/** Seconds to start from, e.g. from `?t=`. */
		startTime?: number;
		autoplay?: boolean;
		/** dash.js settings, e.g. codec and quality preferences from the user's settings. */
		dashSettings?: DashEngineConfig['dashJs'];
		theater?: boolean;
		/** Keyboard shortcuts work anywhere on the page, not only while the player has focus. */
		globalHotkeys?: boolean;
		/** Shows a theater mode button when set. */
		ontheatertoggle?: () => void;
		ontimeupdate?: (currentTime: number, duration: number) => void;
		/** Fires every ~10 s while playing, and on pause, end and unload. */
		onprogress?: (report: ProgressReport) => void;
		/** Fires when playback reaches the end. */
		onended?: () => void;
		class?: string;
	} = $props();

	const uid = $props.id();
	const hotkeyTarget = $derived(globalHotkeys ? 'document' : 'player');
	const theaterLabel = $derived(theater ? m.player_theater_exit() : m.player_theater_enter());
	// No CMCD: it would tag every segment request with session data.
	const PRIVATE_DEFAULTS = {
		streaming: { cmcd: { enabled: false, applyParametersFromMpd: false } }
	};
	// dash.js resolves some URLs without a base, so a relative manifest URL is made absolute.
	const dashSource = $derived({
		src: new URL(src, document.baseURI).href,
		engine: {
			dashJs: {
				...dashSettings,
				streaming: { ...dashSettings?.streaming, ...PRIVATE_DEFAULTS.streaming }
			}
		}
	});

	let player: VideoPlayerElement;
	let media: HTMLElement;

	/** Jumps to `seconds` and keeps playing, as when a timestamp in the description is clicked. */
	export function seek(seconds: number) {
		const state = player?.store.state;
		if (!state || !player.store.target) return;
		void selectTime(state)?.seek(seconds);
		void selectPlayback(state)?.play();
	}

	onMount(() => {
		const store = player.store;
		const prefs = readPreferences();
		// Bound once: on teardown, the props may already describe the next video.
		const report = onprogress;
		const progress = createProgressEmitter((r) => report?.(r));

		let restoredVolume = false;
		let restoredCaptions = false;
		let startApplied = false;
		let last = { ...prefs };

		const snapshot = () => {
			const time = selectTime(store.state);
			return { position: time?.currentTime ?? 0, duration: time?.duration ?? 0 };
		};

		const restoreVolume = () => {
			if (restoredVolume || !store.target) return;
			restoredVolume = true;
			const volume = selectVolume(store.state);
			if (!volume) return;
			if (prefs.volume !== undefined) volume.setVolume(prefs.volume);
			// After setVolume, which unmutes above zero.
			if (prefs.muted) volume.setMuted(true);
		};

		const restoreCaptions = () => {
			const tracks = selectTextTrack(store.state);
			const list = tracks?.textTrackList.filter(
				(t) => t.kind === 'subtitles' || t.kind === 'captions'
			);
			if (restoredCaptions || !tracks || !list?.length) return;
			restoredCaptions = true;
			if (!prefs.captions || prefs.captions === 'off') return;
			const match = list.find((t) => t.language === prefs.captions);
			if (match) tracks.selectSubtitlesTrack(match.id);
		};

		const persist = () => {
			const volume = selectVolume(store.state);
			const rate = selectPlaybackRate(store.state);
			const tracks = selectTextTrack(store.state);
			const next = { ...last };
			if (restoredVolume && volume) {
				next.volume = volume.volume;
				next.muted = volume.muted;
			}
			if (startApplied && rate) next.playbackRate = rate.playbackRate;
			if (restoredCaptions && tracks) {
				const showing = tracks.textTrackList.find(
					(t) => (t.kind === 'subtitles' || t.kind === 'captions') && t.mode === 'showing'
				);
				next.captions = showing?.language ?? 'off';
			}
			if (
				next.volume !== last.volume ||
				next.muted !== last.muted ||
				next.playbackRate !== last.playbackRate ||
				next.captions !== last.captions
			) {
				last = next;
				savePreferences(next);
			}
		};

		const unbridgeAudio = bridgeDashAudioTracks(media as unknown as AudioTrackHost);
		const unsubscribe = store.subscribe(() => {
			restoreVolume();
			restoreCaptions();
			persist();
		});

		// Loading media resets the playback rate, so rate and start time wait for the metadata.
		const onLoadedMetadata = () => {
			if (startApplied) return;
			startApplied = true;
			if (prefs.playbackRate) selectPlaybackRate(store.state)?.setPlaybackRate(prefs.playbackRate);
			if (startTime > 0) void selectTime(store.state)?.seek(startTime);
		};
		const onTimeUpdate = () => {
			const { position, duration } = snapshot();
			ontimeupdate?.(position, duration);
			progress.update(position, duration, !selectPlayback(store.state)?.paused);
		};
		const flush = (reason: 'pause' | 'ended' | 'unload') => () => {
			const { position, duration } = snapshot();
			progress.flush(position, duration, reason);
		};
		const onPause = flush('pause');
		const flushEnded = flush('ended');
		const onEnded = () => {
			flushEnded();
			onended?.();
		};
		const onUnload = flush('unload');
		const onVisibility = () => {
			if (document.visibilityState === 'hidden') onUnload();
		};

		media.addEventListener('loadedmetadata', onLoadedMetadata);
		media.addEventListener('timeupdate', onTimeUpdate);
		media.addEventListener('pause', onPause);
		media.addEventListener('ended', onEnded);
		window.addEventListener('pagehide', onUnload);
		document.addEventListener('visibilitychange', onVisibility);

		return () => {
			onUnload();
			unsubscribe();
			unbridgeAudio();
			media.removeEventListener('loadedmetadata', onLoadedMetadata);
			media.removeEventListener('timeupdate', onTimeUpdate);
			media.removeEventListener('pause', onPause);
			media.removeEventListener('ended', onEnded);
			window.removeEventListener('pagehide', onUnload);
			document.removeEventListener('visibilitychange', onVisibility);
		};
	});
</script>

<media-i18n>
	<video-player bind:this={player} {poster} class={cn('oxi-player block', className)}>
		<media-container
			class="media-skin media-container video-skin oxi-player-frame"
			data-theme="default"
			data-preset="video"
			data-theater={theater ? '' : undefined}
		>
			<dash-video bind:this={media} source={dashSource} playsinline preload="metadata" {autoplay}>
				{#each subtitles as track (track.id)}
					<track kind="subtitles" src={track.url} srclang={track.lang} label={track.label} />
				{/each}
				{#if storyboard}
					<track kind="metadata" label="thumbnails" src={storyboard} default />
				{/if}
			</dash-video>
			<media-poster class="media-poster">
				<img alt="" decoding="async" class="media-poster-image" />
			</media-poster>
			<media-buffering-indicator class="media-buffering-indicator">
				<media-icon name="spinner" class="media-buffering-indicator-spinner-icon"> </media-icon>
			</media-buffering-indicator>
			<media-error-dialog class="media-dialog-root">
				<media-dialog-backdrop class="media-dialog-backdrop"> </media-dialog-backdrop>
				<media-dialog-popup class="media-dialog-popup">
					<div class="media-dialog-content">
						<media-dialog-title class="media-dialog-title"> </media-dialog-title>
						<media-dialog-description class="media-dialog-description"> </media-dialog-description>
					</div>
					<div class="media-dialog-actions">
						<media-dialog-close class="media-button media-dialog-close"> </media-dialog-close>
					</div>
				</media-dialog-popup>
			</media-error-dialog>
			<media-title class="media-title"> </media-title>
			<media-controls>
				<media-buffering-indicator class="video-controls-center-buffering">
					<media-controls-group class="video-controls-center">
						<media-seek-button
							class="media-button media-seek-button video-controls-center-button video-controls-center-seek"
							seconds="-10"
						>
							<div class="media-seek-button-content">
								<media-icon name="seek" class="media-button-icon media-seek-button-backward-icon">
								</media-icon>
								<span class="media-seek-button-label media-seek-button-backward-label">10</span>
							</div>
						</media-seek-button>
						<media-play-button
							class="media-button media-play-button video-controls-center-button video-controls-center-play"
						>
							<media-icon
								name="restart"
								class="media-button-icon media-play-button-restart-icon video-controls-center-play-icon"
							>
							</media-icon>
							<media-icon
								name="play"
								class="media-button-icon media-play-button-play-icon video-controls-center-play-icon"
							>
							</media-icon>
							<media-icon
								name="pause"
								class="media-button-icon media-play-button-pause-icon video-controls-center-play-icon"
							>
							</media-icon>
						</media-play-button>
						<media-seek-button
							class="media-button media-seek-button video-controls-center-button video-controls-center-seek"
							seconds="10"
						>
							<div class="media-seek-button-content">
								<media-icon name="seek" class="media-button-icon"> </media-icon>
								<span class="media-seek-button-label media-seek-button-forward-label">10</span>
							</div>
						</media-seek-button>
					</media-controls-group>
				</media-buffering-indicator>
				<media-controls-backdrop class="video-controls-backdrop"> </media-controls-backdrop>
				<media-controls-content class="video-controls video-controls-content">
					<media-tooltip-group>
						<media-controls-group class="video-controls-primary video-controls-compact-primary">
							<media-play-button class="media-button media-play-button" id="{uid}-play">
								<media-icon name="restart" class="media-button-icon media-play-button-restart-icon">
								</media-icon>
								<media-icon name="play" class="media-button-icon media-play-button-play-icon">
								</media-icon>
								<media-icon name="pause" class="media-button-icon media-play-button-pause-icon">
								</media-icon>
							</media-play-button>
							<media-tooltip
								trigger="{uid}-play"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							<media-mute-button
								commandfor="{uid}-volume-popup"
								class="media-button media-mute-button video-controls-volume-button"
								id="{uid}-volume"
							>
								<media-icon name="volume-off" class="media-button-icon media-mute-button-off-icon">
								</media-icon>
								<media-icon name="volume-low" class="media-button-icon media-mute-button-low-icon">
								</media-icon>
								<media-icon
									name="volume-high"
									class="media-button-icon media-mute-button-high-icon"
								>
								</media-icon>
							</media-mute-button>
							<media-tooltip
								trigger="{uid}-volume"
								delay="0"
								disabled
								sticky
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							<media-volume-popover
								open-on-hover
								delay="200"
								close-delay="100"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-volume-popover"
								id="{uid}-volume-popup"
							>
								<media-volume-slider
									class="media-slider media-volume-slider"
									thumb-alignment="edge"
									orientation="vertical"
								>
									<media-slider-track class="media-slider-track">
										<media-slider-fill class="media-slider-fill"> </media-slider-fill>
									</media-slider-track>
									<media-slider-thumb class="media-slider-thumb media-volume-slider-thumb">
									</media-slider-thumb>
								</media-volume-slider>
							</media-volume-popover>
							<media-controls-group class="video-time-slider-group">
								<media-time
									class="media-time-value video-controls-current-time-value"
									type="current"
								>
								</media-time>
								<media-time-slider
									class="media-slider media-time-slider video-controls-time-slider"
								>
									<media-time-slider-chapters class="media-time-slider-chapters">
										<template>
											<div class="media-time-slider-chapter">
												<media-slider-track
													class="media-slider-track media-time-slider-chapter-track"
												>
													<media-slider-buffer
														class="media-slider-buffer media-time-slider-chapter-layer"
													>
													</media-slider-buffer>
													<media-slider-fill
														class="media-slider-fill media-time-slider-chapter-layer"
													>
													</media-slider-fill>
												</media-slider-track>
											</div>
										</template>
									</media-time-slider-chapters>
									<media-slider-thumb class="media-slider-thumb media-time-slider-thumb">
									</media-slider-thumb>
									<media-slider-preview class="media-slider-preview" overflow="visible">
										<media-slider-thumbnail
											class="media-slider-preview-content media-popup-surface media-slider-thumbnail"
										>
											<img
												alt=""
												aria-hidden="true"
												decoding="async"
												class="media-slider-thumbnail-image"
											/>
											<media-icon name="spinner" class="media-slider-thumbnail-spinner-icon">
											</media-icon>
										</media-slider-thumbnail>
										<div class="media-slider-preview-content media-time-slider-preview-content">
											<media-time-slider-chapter-title class="media-time-slider-chapter-title">
											</media-time-slider-chapter-title>
											<media-slider-value class="media-time-slider-value" type="pointer">
											</media-slider-value>
										</div>
									</media-slider-preview>
								</media-time-slider>
								<media-time
									class="media-time-toggle video-controls-remaining-time-value"
									type="remaining"
									toggle
								>
								</media-time>
							</media-controls-group>
							<media-captions-button
								class="media-button media-captions-button video-controls-captions-button"
								id="{uid}-captions"
							>
								<media-icon
									name="captions-off"
									class="media-button-icon media-captions-button-off-icon"
								>
								</media-icon>
								<media-icon
									name="captions-on"
									class="media-button-icon media-captions-button-on-icon"
								>
								</media-icon>
							</media-captions-button>
							<media-tooltip
								trigger="{uid}-captions"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							<button
								commandfor="{uid}-settings-menu"
								class="media-button media-settings-menu-trigger video-controls-settings-button"
								id="{uid}-settings"
							>
								<media-icon
									name="gear"
									class="media-button-icon-base media-settings-menu-trigger-icon"
								>
								</media-icon>
								<media-text class="media-settings-menu-trigger-label" token="menu.settings">
									Settings
								</media-text>
							</button>
							<media-tooltip
								trigger="{uid}-settings"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-text token="menu.settings">Settings</media-text>
							</media-tooltip>
							<media-menu
								side="top"
								align="center"
								class="media-popup media-popup-surface media-menu-popup media-menu-resizable-popup"
								id="{uid}-settings-menu"
							>
								<media-menu-content class="media-menu-content">
									<media-menu-item commandfor="{uid}-quality" class="media-menu-trigger-item">
										<media-icon name="switches" class="media-menu-trigger-item-icon"> </media-icon>
										<media-text token="menu.quality">Quality</media-text>
										<span class="media-menu-hint">
											<span data-part="value" class="media-menu-hint-label"> </span>
											<media-icon name="chevron" class="media-menu-forward-chevron"> </media-icon>
										</span>
									</media-menu-item>
									<media-menu-content class="media-menu-content" id="{uid}-quality">
										<media-menu-item class="media-menu-back-item">
											<media-icon name="chevron" class="media-menu-back-chevron"> </media-icon>
											<media-text token="menu.quality">Quality</media-text>
										</media-menu-item>
										<media-menu-separator class="media-menu-separator"> </media-menu-separator>
										<media-quality-radio-group class="media-menu-radio-group">
											<template>
												<media-menu-radio-item class="media-menu-radio-item">
													<span>
														<span data-part="label"> </span>
														<sup data-part="tier" class="media-menu-tier"> </sup>
													</span>
													<span data-part="badge" class="media-menu-badge"> </span>
													<media-menu-item-indicator force-mount class="media-menu-item-indicator">
														<media-icon name="check" class="media-menu-radio-item-icon">
														</media-icon>
													</media-menu-item-indicator>
												</media-menu-radio-item>
											</template>
										</media-quality-radio-group>
									</media-menu-content>
									<media-menu-item commandfor="{uid}-audio" class="media-menu-trigger-item">
										<media-icon name="speech" class="media-menu-trigger-item-icon"> </media-icon>
										<media-text token="menu.audio">Audio</media-text>
										<span class="media-menu-hint">
											<span data-part="value" class="media-menu-hint-label"> </span>
											<media-icon name="chevron" class="media-menu-forward-chevron"> </media-icon>
										</span>
									</media-menu-item>
									<media-menu-content class="media-menu-content" id="{uid}-audio">
										<media-menu-item class="media-menu-back-item">
											<media-icon name="chevron" class="media-menu-back-chevron"> </media-icon>
											<media-text token="menu.audio">Audio</media-text>
										</media-menu-item>
										<media-menu-separator class="media-menu-separator"> </media-menu-separator>
										<media-audio-track-radio-group class="media-menu-radio-group">
											<template>
												<media-menu-radio-item class="media-menu-radio-item">
													<span data-part="label"> </span>
													<media-menu-item-indicator force-mount class="media-menu-item-indicator">
														<media-icon name="check" class="media-menu-radio-item-icon">
														</media-icon>
													</media-menu-item-indicator>
												</media-menu-radio-item>
											</template>
										</media-audio-track-radio-group>
									</media-menu-content>
									<media-menu-item commandfor="{uid}-speed" class="media-menu-trigger-item">
										<media-icon name="speed" class="media-menu-trigger-item-icon"> </media-icon>
										<media-text token="menu.speed">Speed</media-text>
										<span class="media-menu-hint">
											<span data-part="value" class="media-menu-hint-label"> </span>
											<media-icon name="chevron" class="media-menu-forward-chevron"> </media-icon>
										</span>
									</media-menu-item>
									<media-menu-content class="media-menu-content" id="{uid}-speed">
										<media-menu-item class="media-menu-back-item">
											<media-icon name="chevron" class="media-menu-back-chevron"> </media-icon>
											<media-text token="menu.speed">Speed</media-text>
										</media-menu-item>
										<media-menu-separator class="media-menu-separator"> </media-menu-separator>
										<media-playback-rate-radio-group class="media-menu-radio-group">
											<template>
												<media-menu-radio-item class="media-menu-radio-item">
													<span data-part="label"> </span>
													<media-menu-item-indicator force-mount class="media-menu-item-indicator">
														<media-icon name="check" class="media-menu-radio-item-icon">
														</media-icon>
													</media-menu-item-indicator>
												</media-menu-radio-item>
											</template>
										</media-playback-rate-radio-group>
									</media-menu-content>
									<media-menu-item commandfor="{uid}-captions-menu" class="media-menu-trigger-item">
										<media-icon name="captions-off" class="media-menu-trigger-item-icon">
										</media-icon>
										<media-text token="menu.captions">Captions</media-text>
										<span class="media-menu-hint">
											<span data-part="value" class="media-menu-hint-label"> </span>
											<media-icon name="chevron" class="media-menu-forward-chevron"> </media-icon>
										</span>
									</media-menu-item>
									<media-menu-content class="media-menu-content" id="{uid}-captions-menu">
										<media-menu-item class="media-menu-back-item">
											<media-icon name="chevron" class="media-menu-back-chevron"> </media-icon>
											<media-text token="menu.captions">Captions</media-text>
										</media-menu-item>
										<media-menu-separator class="media-menu-separator"> </media-menu-separator>
										<media-captions-radio-group class="media-menu-radio-group">
											<template>
												<media-menu-radio-item class="media-menu-radio-item">
													<span data-part="label"> </span>
													<span data-part="badge" class="media-menu-badge"> </span>
													<media-menu-item-indicator force-mount class="media-menu-item-indicator">
														<media-icon name="check" class="media-menu-radio-item-icon">
														</media-icon>
													</media-menu-item-indicator>
												</media-menu-radio-item>
											</template>
										</media-captions-radio-group>
									</media-menu-content>
								</media-menu-content>
							</media-menu>
						</media-controls-group>
						<media-controls-group class="video-controls-secondary video-controls-top-controls">
							<media-cast-button class="media-button media-cast-button" id="{uid}-cast">
								<media-icon
									name="cast-enter"
									class="media-button-icon media-cast-button-enter-icon"
								>
								</media-icon>
								<media-icon name="cast-exit" class="media-button-icon media-cast-button-exit-icon">
								</media-icon>
							</media-cast-button>
							<media-tooltip
								trigger="{uid}-cast"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							<media-airplay-button class="media-button media-airplay-button" id="{uid}-airplay">
								<media-icon
									name="airplay-enter"
									class="media-button-icon media-airplay-button-enter-icon"
								>
								</media-icon>
								<media-icon
									name="airplay-exit"
									class="media-button-icon media-airplay-button-exit-icon"
								>
								</media-icon>
							</media-airplay-button>
							<media-tooltip
								trigger="{uid}-airplay"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							<media-pip-button class="media-button media-pip-button" id="{uid}-pip">
								<media-icon name="pip-enter" class="media-button-icon media-pip-button-enter-icon">
								</media-icon>
								<media-icon name="pip-exit" class="media-button-icon media-pip-button-exit-icon">
								</media-icon>
							</media-pip-button>
							<media-tooltip
								trigger="{uid}-pip"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
							{#if ontheatertoggle}
								<button
									type="button"
									class="media-button oxi-theater-button"
									id="{uid}-theater"
									aria-pressed={theater}
									aria-keyshortcuts="t"
									aria-label={theaterLabel}
									onclick={ontheatertoggle}
								>
									<RectangleHorizontalIcon class="media-button-icon" aria-hidden="true" />
								</button>
								<media-tooltip
									trigger="{uid}-theater"
									side="top"
									class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip oxi-theater-tooltip"
								>
									<span>{theaterLabel}</span>
									<span class="media-tooltip-shortcut">T</span>
								</media-tooltip>
							{/if}
							<media-fullscreen-button
								class="media-button media-fullscreen-button"
								id="{uid}-fullscreen"
							>
								<media-icon
									name="fullscreen-enter"
									class="media-button-icon media-fullscreen-button-enter-icon"
								>
								</media-icon>
								<media-icon
									name="fullscreen-exit"
									class="media-button-icon media-fullscreen-button-exit-icon"
								>
								</media-icon>
							</media-fullscreen-button>
							<media-tooltip
								trigger="{uid}-fullscreen"
								side="top"
								class="media-popup media-popup-safe-area media-popup-transition media-popup-surface media-tooltip"
							>
								<media-tooltip-label> </media-tooltip-label>
								<media-tooltip-shortcut class="media-tooltip-shortcut"> </media-tooltip-shortcut>
							</media-tooltip>
						</media-controls-group>
					</media-tooltip-group>
				</media-controls-content>
			</media-controls>
			<media-hotkey target={hotkeyTarget} keys="Space" action="togglePaused"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="k" action="togglePaused"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="m" action="toggleMuted"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="ArrowRight" action="seekStep"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="ArrowLeft" action="seekStep"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="l" action="seekStep"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="j" action="seekStep"> </media-hotkey>
			<media-hotkey keys="ArrowUp" action="volumeStep"> </media-hotkey>
			<media-hotkey keys="ArrowDown" action="volumeStep"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="0-9" action="seekToPercent"> </media-hotkey>
			<media-hotkey keys="Home" action="seekToPercent" value="0"> </media-hotkey>
			<media-hotkey keys="End" action="seekToPercent" value="100"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys=">" action="speedUp"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="<" action="speedDown"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="f" action="toggleFullscreen"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="c" action="toggleSubtitles"> </media-hotkey>
			<media-hotkey target={hotkeyTarget} keys="i" action="togglePictureInPicture"> </media-hotkey>
			<media-gesture type="tap" action="togglePaused" pointer="mouse" region="center">
			</media-gesture>
			<media-gesture type="tap" action="toggleControls" pointer="touch"> </media-gesture>
			<media-gesture type="doubletap" action="seekStep" region="left"> </media-gesture>
			<media-gesture type="doubletap" action="toggleFullscreen" region="center"> </media-gesture>
			<media-gesture type="doubletap" action="seekStep" region="right"> </media-gesture>
			<media-status-announcer class="media-status-announcer"> </media-status-announcer>
			<div class="video-status-indicators">
				<media-volume-indicator class="media-indicator media-volume-indicator">
					<media-volume-indicator-fill class="media-indicator-content media-volume-indicator-fill">
						<media-icon name="volume-high" class="media-volume-indicator-high-icon"> </media-icon>
						<media-icon name="volume-low" class="media-volume-indicator-low-icon"> </media-icon>
						<media-icon name="volume-off" class="media-volume-indicator-off-icon"> </media-icon>
						<media-volume-indicator-value class="media-volume-indicator-value">
						</media-volume-indicator-value>
					</media-volume-indicator-fill>
				</media-volume-indicator>
				<media-status-indicator
					actions="toggleSubtitles,toggleFullscreen,togglePictureInPicture"
					class="media-indicator media-status-indicator"
				>
					<div class="media-indicator-content media-status-indicator-content">
						<media-icon name="captions-on" class="media-status-indicator-captions-on-icon">
						</media-icon>
						<media-icon name="captions-off" class="media-status-indicator-captions-off-icon">
						</media-icon>
						<media-icon
							name="fullscreen-enter"
							class="media-status-indicator-fullscreen-enter-icon"
						>
						</media-icon>
						<media-icon name="fullscreen-exit" class="media-status-indicator-fullscreen-exit-icon">
						</media-icon>
						<media-icon name="pip-enter" class="media-status-indicator-pip-enter-icon">
						</media-icon>
						<media-icon name="pip-exit" class="media-status-indicator-pip-exit-icon"> </media-icon>
						<media-status-indicator-value class="media-status-indicator-value">
						</media-status-indicator-value>
					</div>
				</media-status-indicator>
				<media-seek-indicator class="media-seek-indicator">
					<media-icon name="chevron" class="media-seek-indicator-icon"> </media-icon>
					<media-seek-indicator-value class="media-seek-indicator-value">
					</media-seek-indicator-value>
				</media-seek-indicator>
				<media-status-indicator actions="togglePaused" class="media-playback-status-indicator">
					<media-icon name="play" class="media-playback-status-indicator-play-icon"> </media-icon>
					<media-icon name="pause" class="media-playback-status-indicator-pause-icon"> </media-icon>
				</media-status-indicator>
			</div>
		</media-container>
	</video-player>
</media-i18n>

<style>
	.oxi-player :global(.oxi-player-frame) {
		aspect-ratio: 16 / 9;
	}

	.oxi-player :global(.oxi-player-frame[data-theater]) {
		--media-video-border-radius: 0;
	}

	.oxi-player :global(.oxi-player-frame[data-theater])::after {
		border-color: transparent;
	}

	/* Video thumbnails often come as 4:3 with letterbox bars; cover crops exactly those bars. */
	.oxi-player :global(.media-poster-image) {
		object-fit: cover;
	}

	.oxi-player :global(.oxi-theater-button svg) {
		width: calc(var(--media-spacing) * 5);
		height: calc(var(--media-spacing) * 5);
		stroke-width: 1.75;
	}

	/* Theater mode only changes the page layout from the large breakpoint up. */
	@media (width < 64rem) {
		.oxi-player :global(.oxi-theater-button),
		.oxi-player :global(.oxi-theater-tooltip) {
			display: none;
		}
	}
</style>
