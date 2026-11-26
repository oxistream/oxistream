import type { User } from './video';

// Placeholder until auth (#8, #9) provides the signed-in user.
export const session = $state<{ user: User | null }>({ user: null });
