<script lang="ts">
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let {
		open = $bindable(false),
		disabled = false,
		onconfirm
	}: {
		open?: boolean;
		disabled?: boolean;
		onconfirm: () => void;
	} = $props();
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Trigger class={buttonVariants({ variant: 'outline', size: 'sm' })} {disabled}>
		<Trash2Icon />
		{m.history_clear()}
	</AlertDialog.Trigger>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{m.history_clear_title()}</AlertDialog.Title>
			<AlertDialog.Description>{m.history_clear_body()}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action
				variant="destructive"
				onclick={() => {
					open = false;
					onconfirm();
				}}
			>
				{m.history_clear()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
