<script lang="ts">
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import { hideVideo, unhideVideo } from '#lib/api/hidden.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let { videoId }: { videoId: string } = $props();

	const client = useQueryClient();
	const failed = () => toast.error(m.video_action_failed());

	function select() {
		const id = videoId;
		const hidden = hideVideo(client, id);
		const notice = toast(m.video_hidden(), {
			duration: 8000,
			action: {
				label: m.action_undo(),
				// After the hide settled, so the two requests cannot overtake each other.
				onClick: () =>
					void hidden
						.catch(() => undefined)
						.then(() => unhideVideo(client, id))
						.catch(failed)
			}
		});
		hidden.catch(() => {
			toast.dismiss(notice);
			failed();
		});
	}
</script>

<DropdownMenu.Item onSelect={select} data-testid="not-interested">
	<EyeOffIcon />
	{m.video_not_interested()}
</DropdownMenu.Item>
