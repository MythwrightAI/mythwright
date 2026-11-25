// The Topics panel: a forum's topics, a place of the rail of
// their own, just after the channels. It came out of the Messaging panel,
// where the topics were a node beside the channels;
// the panel itself is drawn by the frame (`app.tsx`) as the others are.
//
// A topic is a page of a wiki: it may hold pages under it, to
// any depth, and one with no page above it is a wiki. The panel lists the
// wikis of the workspace it is of (`holdsTopic`: a page carries its wiki's
// place, so a page and its wiki are always listed together), each in the
// order it stands with its pages under it; the wikis that are closed under
// a branch of their own. What is here is what the panel lists and finds;
// what draws it, with the `+` and the menu of each line, is `wikitree.tsx`.

import { effect } from '@preact/signals';
import { held } from '../chosen.ts';
import { covers } from '../holds.ts';
import { narrowed, topicItems } from '../panelsought.ts';
import type { Found, TopicListed } from '../panelsought.ts';
import { workspaces } from '../spaces.ts';
import { me, projects } from '../store.ts';
import { whereSaid } from '../topicpage.ts';
import type { TopicLine } from '../topicpage.ts';
import { forgetTopics, topics } from '../topicsread.ts';
import { listedWikis } from '../wikis.ts';
import type { Drawn, Tree } from '../wikis.ts';
import { holdsTopic, spaceOfTopic } from '../workspaces.ts';

// Let go of when nobody is signed in: read again, as whoever signs in next, when the panel is shown.
effect(() => {
  if (!me.value) forgetTopics();
});

/** The glyph a topic's line and its tab have: a post, where a channel has its `#`. */
export const TOPIC_GLYPH = '¶';

/** The wikis the panel lists, of the workspace it is of, with every page under each; and the tree they are read as, every workspace's. */
export function topicsListedNow(): { tree: Tree; listed: Drawn<TopicLine>[]; closed: Drawn<TopicLine>[] } {
  return listedWikis(
    topics.value,
    (topic) => spaceOfTopic(topic, workspaces.value, projects.value)?.id ?? null,
    (topic) => holdsTopic(held.value, topic, workspaces.value, projects.value),
  );
}

/** A page as the panel's box finds it: its words, where it is, and the pages under it. */
function sought(one: Drawn<TopicLine>): TopicListed {
  const { id, title, status, summary } = one.line;
  return { id, title, status, summary, where: whereSaid(one.line, projects.value), pages: one.pages.map(sought) };
}

/** What the panel's box finds among the pages: nothing where nothing is typed, which is the whole panel. */
export function topicsFound(text: string): Found | null {
  if (text.trim() === '') return null;
  const { listed, closed } = topicsListedNow();
  return narrowed(topicItems({ listed: listed.map(sought), closed: closed.map(sought) }), text);
}

/**
 * Whether whoever reads the page may make a page or a wiki, and change
 * where one is: write records; and, while the factory has no topics at
 * all, manage it, since the first one made sets topics up (`std/topics`,
 * which takes `admin`).
 */
export const mayMakeTopic = (): boolean => {
  const scopes = me.value?.principal.scopes ?? [];
  return covers(scopes, 'records:write') && (topics.value.length > 0 || covers(scopes, 'admin'));
};
