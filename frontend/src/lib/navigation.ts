export function isActive(pathname: string, href: string): boolean {
	if (href === '/') return pathname === '/';
	return pathname === href || pathname.startsWith(`${href}/`);
}

export function pageTitle(title?: string): string {
	return title ? `${title} · Oxistream` : 'Oxistream';
}
