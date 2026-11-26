/** Underlined tab link of the channel and search tab bars; `aria-current="page"` marks the open tab. */
export const tabLinkClass =
	'relative inline-flex h-10 items-center rounded-t-md px-3 text-sm font-medium text-muted-foreground transition-colors duration-(--motion-in) ease-ui after:absolute after:inset-x-3 after:bottom-0 after:h-0.5 after:rounded-full after:bg-transparent after:transition-colors hover:text-foreground focus-visible:outline-offset-[-2px] aria-[current=page]:text-foreground aria-[current=page]:after:bg-foreground';
