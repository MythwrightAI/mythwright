// How the factory's setting for coming by blueprints is said in the
// client: each choice in a line, and what is so now in a sentence.

import type { TextsSetting } from './comparing.ts';

/** What each way of coming by texts is, in a line. */
export const COME: Record<TextsSetting['come'], string> = {
  usual: 'As usual here',
  held: 'Held until somebody takes them',
  follow: 'Followed as they change',
};

/** What is so now, in a sentence. */
export function standing(now: Pick<TextsSetting, 'come' | 'usual'>): string {
  if (now.come === 'follow') return 'A blueprint that changes is taken as it changes by everything running that reaches it, and nobody is asked.';
  if (now.come === 'held' || now.usual === 'held')
    return 'What is running goes on with the blueprints it has until somebody applies it saying to take them as they now are.';
  return "This factory is run from its checkout: the checkout's blueprints are followed as they change, and a project's own are run as they were vouched for.";
}
