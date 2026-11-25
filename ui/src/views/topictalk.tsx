// A topic's talk: the reply that answered it, while it stands answered,
// drawn first so that it is the first thing read; then its replies as a
// tree under a box that begins a new chain (`replies.tsx`). It is the
// second of the page's two tabs where the topic is an article, and on a
// thread it is the page under its head and summary, with the pages under it
// (`between`) after what answered it.

import type { ComponentChildren } from 'preact';
import { answerOf } from '../replies.ts';
import type { Reply } from '../replies.ts';
import { mayAnswer } from '../topicpage.ts';
import type { TopicWhole } from '../topicpage.ts';
import { Prose } from '../ui.tsx';
import { when } from '../util.ts';
import { Replies, ReplyBox } from './replies.tsx';

interface Props {
  topic: TopicWhole;
  replies: Reply[];
  /** Whether whoever reads it may reply, and mark what answered it. */
  mayChange: boolean;
  /** The reply the page was asked to show (an inbox line, a link to it): marked, its chain unfolded, brought into view once. */
  asked: number | null;
  onChanged: () => void;
  /** Whether it is a tab's panel, beside the article; on a thread it is not. */
  tabbed: boolean;
  /** What a thread draws between what answered it and its replies: the pages under it. */
  between?: ComponentChildren;
}

/** How many replies there are, in words. */
export const repliesSaid = (count: number): string => (count === 1 ? '1 reply' : `${count} replies`);

export function Talk(props: Props) {
  const { topic, replies, mayChange } = props;
  // What answered it is drawn while it stands answered: one opened again or closed since keeps the reply named, and says nothing of it.
  const answered = topic.status === 'answered' ? topic.answer : null;
  const answer = answerOf(replies, answered);
  const parts = (
    <>
      {answer && (
        <section class="topic-answer" aria-label="What answered it">
          <h2 class="topic-part-label">Answered by {answer.who || answer.by}</h2>
          <div class="topic-answer-words">
            <Prose text={answer.body} attached={topic.id} />
          </div>
          <time class="faint" dateTime={new Date(answer.at).toISOString()}>
            {when(answer.at)}
          </time>
        </section>
      )}
      {props.between}
      <section class="topic-replies-part" aria-label="Replies">
        <h2 class="topic-part-label">{repliesSaid(replies.length)}</h2>
        {mayChange && <ReplyBox topic={topic.id} parent={null} label="Reply to the topic" onDone={props.onChanged} />}
        <Replies topic={topic.id} replies={replies} answer={answered} asked={props.asked} mayReply={mayChange} mayMark={mayChange && mayAnswer(topic.status)} onChanged={props.onChanged} />
      </section>
    </>
  );
  if (!props.tabbed) return parts;
  return (
    <div class="topic-talk topic-parts" role="tabpanel" id={`topic-talk-${topic.id}`} aria-labelledby={`topic-tab-talk-${topic.id}`}>
      {parts}
    </div>
  );
}
