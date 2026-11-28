/** A compact filter control, as in search and the subscription feed. */
export const chipClass =
	'inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md bg-muted px-3 text-sm font-medium whitespace-nowrap text-foreground transition-colors duration-(--motion-in) ease-ui outline-none hover:bg-muted-foreground/15 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring active:scale-[0.97]';

/** Added to a chip whose filter is set. */
export const chipActiveClass =
	'bg-foreground text-background hover:bg-foreground/85 dark:hover:bg-foreground/90';
