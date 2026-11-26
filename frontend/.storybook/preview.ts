import type { Decorator, Preview } from '@storybook/sveltekit';
import { baseLocale, locales, setLocale, type Locale } from '../src/lib/paraglide/runtime.js';
import '../src/routes/layout.css';

const withThemeAndLocale: Decorator = (story, context) => {
	const theme = context.globals.theme as 'light' | 'dark';
	const locale = context.globals.locale as Locale;
	document.documentElement.classList.toggle('dark', theme === 'dark');
	document.documentElement.style.colorScheme = theme;
	document.documentElement.lang = locale;
	void setLocale(locale, { reload: false });
	return story();
};

const preview: Preview = {
	globalTypes: {
		theme: {
			description: 'Color theme',
			toolbar: {
				title: 'Theme',
				icon: 'mirror',
				items: [
					{ value: 'light', title: 'Light', icon: 'sun' },
					{ value: 'dark', title: 'Dark', icon: 'moon' }
				],
				dynamicTitle: true
			}
		},
		locale: {
			description: 'Locale',
			toolbar: {
				title: 'Locale',
				icon: 'globe',
				items: locales.map((value) => ({ value, title: value })),
				dynamicTitle: true
			}
		}
	},
	initialGlobals: {
		theme: 'light',
		locale: baseLocale
	},
	decorators: [withThemeAndLocale],
	parameters: {
		controls: {
			matchers: {
				color: /(background|color)$/i,
				date: /Date$/i
			}
		}
	}
};

export default preview;
