<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import LogoMark from './logo-mark.svelte';
	import Wordmark from './wordmark.svelte';

	const { Story } = defineMeta({
		title: 'Brand/Logo',
		component: Wordmark
	});
</script>

<Story name="Wordmark">
	<Wordmark />
</Story>

<Story name="Logomark sizes" asChild>
	<div class="flex items-end gap-4 text-primary">
		<LogoMark class="size-4" />
		<LogoMark class="size-6" />
		<LogoMark class="size-10" />
		<LogoMark class="size-16" />
	</div>
</Story>
