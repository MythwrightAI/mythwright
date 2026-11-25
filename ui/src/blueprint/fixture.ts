// What this folder's tests read blueprints with.
//
// The language is a copy of what the daemon describes, held to it by a
// test of the API (`the_copy_of_the_language_the_client_is_tested_against_
// is_the_language`). Which actions a device has is its own business, so
// these are a few made up in the shape of real ones.

import { readFileSync } from 'node:fs';
import type { ActionSpec, BlueprintLanguage, EventSpec } from '../api.ts';
import type { World } from './complete.ts';

const actions: ActionSpec[] = [
  { name: 'noop', summary: 'Do nothing.', args: [], open: false, capability: null, outputs: [], idempotent: true },
  {
    name: 'emit',
    summary: 'Publish an event for anything listening.',
    args: [{ name: 'topic', type: 'string', required: true, help: 'where to publish' }],
    open: true,
    capability: null,
    outputs: [],
    idempotent: true,
  },
  {
    name: 'shell',
    summary: 'Run a command and wait for it to finish.',
    args: [
      { name: 'run', type: 'string', required: false, help: 'a shell script' },
      { name: 'cwd', type: 'string', required: false, help: 'directory to run in' },
      { name: 'clear_env', type: 'bool', required: false, help: 'start from an almost empty environment' },
    ],
    open: false,
    capability: 'shell',
    outputs: [
      ['code', 'the exit code'],
      ['stdout', 'the last lines of standard output'],
    ],
    idempotent: false,
  },
  {
    name: 'channel.post',
    summary: 'Say something in a channel.',
    args: [
      { name: 'channel', type: 'string', required: true, help: 'which channel' },
      { name: 'body', type: 'string', required: true, help: 'what to say' },
      { name: 'as', type: 'string', required: false, help: 'the bot to say it as' },
    ],
    open: false,
    capability: 'channels:write',
    outputs: [['id', 'the message']],
    idempotent: false,
  },
  {
    name: 'agent.consult',
    summary: 'Ask an agent to choose.',
    args: [
      { name: 'prompt', type: 'string', required: true, help: 'the question' },
      { name: 'options', type: 'list', required: false, help: 'the answers it may give' },
    ],
    open: false,
    capability: 'agents:write',
    outputs: [['decision', 'the answer']],
    idempotent: false,
  },
];

/**
 * Three that say what is written inside an argument, in each way that is
 * said: things by a name that each take keys, keys taken directly, and a
 * list whose items take them. Apart from the rest, which the tests of
 * what is offered count.
 */
const telling: ActionSpec[] = [
  {
    name: 'collection.ensure',
    summary: 'Declare a table.',
    args: [
      { name: 'name', type: 'string', required: true, help: 'what it is called' },
      {
        name: 'fields',
        type: 'object',
        required: true,
        help: 'its fields, each by its name',
        within: {
          each: 'a field of the table',
          keys: [
            {
              name: 'type',
              type: 'string',
              help: 'what it holds',
              words: [
                { name: 'text', help: 'words' },
                { name: 'list', help: 'several things' },
                { name: 'flag', help: '' },
              ],
            },
            { name: 'quiet', type: 'bool', help: 'a change to it alone is not news' },
            { name: 'hidden', type: 'bool', help: 'left out of a view until it is shown there' },
            { name: 'takes', type: 'any', help: 'what giving it a value takes', within: { each: 'a value of the field, with the scope giving it that value takes' } },
            { name: 'unsaid', type: 'list', help: 'what an empty list stands for' },
          ],
        },
      },
      {
        name: 'views',
        type: 'object',
        required: false,
        help: 'the ways it is laid out, each by its name',
        within: {
          each: 'a way of laying the table out',
          keys: [
            { name: 'kind', type: 'string', help: 'how it is laid out', words: [{ name: 'table', help: 'rows and columns' }, { name: 'board', help: 'a column for each choice' }] },
            { name: 'by', type: 'string', help: 'the field it goes by' },
            {
              name: 'figures',
              type: 'object',
              help: 'a figure at the foot of a column',
              within: { each: 'a column, with the figure at its foot', words: [{ name: 'sum', help: 'its numbers added up' }, { name: 'filled', help: 'how many rows say something' }] },
            },
          ],
        },
      },
      { name: 'history', type: 'any', required: false, help: 'which events are kept' },
    ],
    open: false,
    capability: 'projects:write',
    outputs: [],
    idempotent: true,
  },
  {
    name: 'service.start',
    summary: 'Start a long-lived process.',
    args: [
      { name: 'name', type: 'string', required: true, help: 'what to call it' },
      {
        name: 'health',
        type: 'object',
        required: false,
        help: 'how to tell it is healthy',
        within: {
          keys: [
            { name: 'tcp', type: 'bool', help: 'it answers on its port' },
            { name: 'http', type: 'string', help: 'a path it answers' },
            { name: 'interval', type: 'duration', help: 'how often it is asked' },
          ],
        },
      },
    ],
    open: false,
    capability: 'services',
    outputs: [],
    idempotent: true,
  },
  {
    name: 'rows.start',
    summary: 'Give a table its first rows.',
    args: [
      { name: 'collection', type: 'string', required: true, help: 'the table' },
      {
        name: 'rows',
        type: 'list',
        required: true,
        help: 'the rows, in order',
        within: { keys: [{ name: 'title', type: 'string', help: 'what the row is called' }, { name: 'count', type: 'int', help: 'how many' }] },
      },
    ],
    open: false,
    capability: 'records:write',
    outputs: [],
    idempotent: true,
  },
];

/** A few events, in the shape of real ones: what a device emits is its own business too. */
const events: EventSpec[] = [
  {
    topic: 'board.item.moved',
    summary: 'A card was moved to another column.',
    fields: [
      { name: 'id', type: 'string', always: true, help: 'the card' },
      { name: 'to', type: 'string', always: true, help: 'the column it is in now' },
      { name: 'note', type: 'string', always: false, help: 'what was said as it was moved' },
    ],
    open: false,
    kept: true,
  },
  {
    topic: 'board.item.removed',
    summary: 'A card was taken off its board.',
    fields: [{ name: 'id', type: 'string', always: true, help: 'the card' }],
    open: false,
    kept: true,
  },
  {
    topic: 'log.line',
    summary: 'A line something printed.',
    fields: [{ name: 'text', type: 'string', always: true, help: 'the line' }],
    open: false,
    kept: false,
  },
];

export const language: BlueprintLanguage = {
  ...(JSON.parse(readFileSync(new URL('./language.json', import.meta.url), 'utf8')) as Omit<BlueprintLanguage, 'actions' | 'events'>),
  actions,
  events,
};

/** The same language on a device whose actions also say what is written inside their arguments. */
export const told: BlueprintLanguage = { ...language, actions: [...actions, ...telling] };

/**
 * And on one whose actions are the real ones that say so, as a node
 * hands them out: a copy, held to that by a test of the API
 * (`the_copy_of_what_actions_say_is_written_inside_their_arguments_is_
 * what_they_say`).
 */
export const real: BlueprintLanguage = {
  ...language,
  actions: [...actions, ...(JSON.parse(readFileSync(new URL('./inside.json', import.meta.url), 'utf8')) as ActionSpec[])],
};

export const world: World = {
  blueprints: [
    {
      name: 'std/slot',
      file: null,
      description: 'One version of a service.',
      params: [
        { name: 'version', type: 'string', default: null, required: true, description: 'which version to run', choices: [] },
        { name: 'port', type: 'int', default: 8080, required: false, description: null, choices: [] },
      ],
      outputs: ['address'],
    },
    { name: 'deploy', file: 'blueprints/deploy.bp', description: null, params: [], outputs: [] },
  ],
  agents: ['builder', 'reviewer'],
  devices: ['builder-1'],
};

/** A blueprint's text with a `|` where the caret is: the text without it, and where it was. */
export function caret(marked: string): { text: string; offset: number } {
  const offset = marked.indexOf('|');
  if (offset < 0) throw new Error('the text has no `|` to say where the caret is');
  return { text: marked.slice(0, offset) + marked.slice(offset + 1), offset };
}
