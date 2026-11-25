// What a blueprint's Summary draws, held to what the daemon hands, for
// the checks that show one (`summary.mjs`, `forked.mjs`).
//
// The page lays the daemon's summary out (`summarylaid.ts`): a head of
// what it is with the facts of every part, the parts in the order a
// reader needs them, each handler a card drawn by the parts its line is
// handed in (`title`, `only`, `from`, `tags`), what it makes and starts
// a row of a table for each way it is started (`made`, `named`, `by`), each way to change it
// what is done and what is said of it (`label`, `title`), and where most handlers
// hear from said once over them. So what is held is not one list
// against another in order but every record both ways: each line, each
// part of a line taken apart and each fact the daemon hands is drawn
// somewhere, and nothing is drawn that it did not hand. What a fold puts
// away is counted only once it is open, so whoever asks opens them first.

/** What is set apart as written, without the marks that set it apart, as a reader reads it. */
export const bare = (words) => words?.replaceAll('`', '') ?? null;

/** The order the page reads the parts in (`readOrder` in `summarylaid.ts`). */
const READ = { is: 0, differs: 1, moved: 2, does: 3, hears: 4, ends: 5, makes: 6, may: 7, change: 8, back: 9 };
const rank = (kind) => READ[kind] ?? 7.5;

/** Every line of a summary as the daemon hands it out, those under others among them, in the order they are drawn. */
export const flat = (handed) => {
  const under = (lines) => lines.flatMap((line) => [line, ...under(line.under)]);
  return handed.parts.map((part) => ({ kind: part.kind, title: part.title, lines: under(part.lines) }));
};

/** What the Summary on the page draws: its parts in order, each line with its parts, each fact of the head, and what is said once for a part's handlers. */
export const onPage = (on) =>
  on.evaluate(() => {
    const shown = [...document.querySelectorAll('.summary')].find((one) => one.getClientRects().length > 0);
    const text = (el) => el?.textContent ?? null;
    const all = (el, sel) => [...el.querySelectorAll(sel)].map((one) => one.textContent);
    return {
      parts: [...shown.querySelectorAll('.summary-part')].map((part) => ({ kind: part.dataset.part, title: part.querySelector(':scope > h2')?.childNodes[0]?.textContent ?? '' })),
      lines: [...shown.querySelectorAll('.summary-line')].flatMap((line) => {
        const part = line.closest('.summary-part').dataset.part;
        const only = all(line, ':scope > .summary-only > li:not([data-from])');
        // A row of what it makes and starts stands for each line it gathers: one for each place it is started from.
        // It is a row of a table: what it is in a cell, and what it is called with as what and how in the next.
        if (line.classList.contains('summary-made-row')) {
          const name = line.querySelector(':scope > .summary-made-name');
          const said = { part, mark: line.dataset.mark ?? null, key: null, label: null, what: text(line.querySelector(':scope > .summary-made-what')) ?? '', words: text(name.querySelector(':scope > .summary-said > .summary-words')), only: all(name, ':scope > .summary-only > li'), from: null, tags: all(name, ':scope > .summary-said > .summary-tag'), named: text(name.querySelector(':scope > .summary-said > .summary-named > .summary-name')) };
          return [...line.querySelectorAll(':scope > .summary-by > .summary-step-of')].map((one) => ({ ...said, at: one.dataset.at ?? null, by: one.textContent, pressed: one.tagName === 'BUTTON' }));
        }
        const act = line.querySelector(':scope > .summary-said > .summary-act');
        return [{
          part,
          at: line.dataset.at ?? null,
          mark: line.dataset.mark ?? null,
          key: text(line.querySelector(':scope > .summary-said > .summary-key')),
          label: text(act),
          words: act ? text(line.querySelector(':scope > .summary-detail')) : text(line.querySelector(':scope > .summary-said > .summary-words')),
          only,
          from: text(line.querySelector(':scope > .summary-only > li[data-from]')),
          tags: all(line, ':scope > .summary-tags > .summary-tag'),
          named: null,
          by: null,
          pressed: line.querySelector(':scope > .summary-said > button.summary-words:not(.summary-fold)') !== null,
        }];
      }),
      facts: [...shown.querySelectorAll('.summary-fact[data-label]')].flatMap((fact) => [...fact.querySelectorAll('.summary-item')].map((item) => ({ label: fact.dataset.label, item: item.textContent }))),
      shared: [...shown.querySelectorAll('.summary-from')].map((one) => ({ part: one.closest('.summary-part').dataset.part, from: one.textContent })),
    };
  });

/** Where most of a part's handlers hear from, where two or more say it: what the page says once (`sharedFrom`). */
const mostFrom = (part) => {
  const counts = new Map();
  for (const line of part.lines) if (line.at?.endsWith('/@') && line.from) counts.set(line.from, (counts.get(line.from) ?? 0) + 1);
  let best = null;
  for (const [from, count] of counts) if (count >= 2 && (best === null || count > counts.get(best))) best = from;
  return best;
};

/** The parts whose facts are drawn in the head (`FACTS` in `summarylaid.ts`). */
const FACTS = ['is', 'may'];

/** What the page is to draw of what the daemon hands: a record of each line, and of each thing each fact of the head says. */
const toDraw = (theirs, shared) => {
  const [lines, facts] = [[], []];
  for (const part of theirs) {
    for (const line of part.lines) {
      if (line.label && FACTS.includes(part.kind)) {
        for (const item of line.items?.length ? line.items : [line.title]) facts.push({ label: line.label, item: bare(item) });
        continue;
      }
      const said = { part: part.kind, at: line.at, mark: line.mark, key: line.key, label: null, words: bare(line.text), only: [], from: null, tags: [], named: null, by: null };
      if (part.kind === 'makes') {
        // A row of what it makes or starts: what it is, as what, how, and from where (the handler, else the step).
        lines.push({ ...said, key: null, what: line.made?.what ?? '', words: bare(line.made ? (line.made.name ?? '') : (line.title ?? line.text)), only: (line.only ?? []).map(bare), tags: (line.tags ?? []).map(bare), named: bare(line.named ?? null), by: line.by ?? (line.at?.startsWith('uses/') ? '' : line.key) });
      } else if (line.label && line.title) {
        // A way to change it: what is done, and what is said of it.
        lines.push({ ...said, label: bare(line.label), words: bare(line.title) });
      } else if (line.title) {
        // A line handed taken apart is drawn by its parts; where most of a part's handlers hear from is said once, over them.
        const from = line.from && line.from !== shared[part.kind] ? bare(line.from) : null;
        lines.push({ ...said, words: bare(line.title), only: (line.only ?? []).map(bare), from, tags: (line.tags ?? []).map(bare) });
      } else lines.push(said);
    }
  }
  return { lines, facts };
};

/** Each record as one string, so that two lists are held to having the same records as often. */
const records = (list) => list.map((one) => JSON.stringify(Object.keys(one).sort().map((key) => [key, one[key]]))).sort();

/**
 * Holds the Summary showing on `on` to `handed`, the daemon's own summary
 * of the same text, both ways, with `expect(what, ok)`; every fold is to
 * be open. Gives the daemon's parts in its order, each line as a reader
 * reads it and with where it leads (`opens`, a word) and whether it is
 * pressed on the page, found by what it is about and its words.
 */
export async function heldToTheDaemon(on, handed, when, expect) {
  expect(`the daemon to have a summary ${when}`, handed && handed.parts.length > 0);
  const theirs = flat(handed);
  const ours = await onPage(on);
  const shared = Object.fromEntries(theirs.map((part) => [part.kind, mostFrom(part)]));
  for (const part of theirs) {
    expect(`a heading to have lines under it ${when}: ${part.title}`, part.lines.length > 0);
    const said = ours.shared.filter((one) => one.part === part.kind).map((one) => one.from);
    expect(`where most of "${part.title}" is heard from to be said once over it ${when}, got ${said.join(' | ')}`, said.join('|') === (shared[part.kind] ? bare(shared[part.kind]) : ''));
  }
  // The parts on the page are the daemon's, in the order a reader needs them; a part all of whose lines are facts of the head has no place of its own.
  const want = theirs.filter((part) => part.kind === 'is' || part.lines.some((line) => !line.label || !FACTS.includes(part.kind)));
  const order = [...want].sort((a, b) => rank(a.kind) - rank(b.kind));
  // What its tables are made of, together, is drawn from what it makes and is held with that (`summary.mjs`).
  const drawnParts = ours.parts.filter((part) => part.kind !== 'cost' && part.kind !== 'model');
  expect(`the parts ${when} to be the daemon's in the order they are read, got ${drawnParts.map((part) => part.kind).join(', ')}`, drawnParts.map((part) => `${part.kind}:${part.title}`).join('|') === order.map((part) => `${part.kind}:${part.title}`).join('|'));
  const expected = toDraw(theirs, shared);
  const got = { lines: ours.lines.filter((line) => line.part !== 'cost').map(({ pressed, ...line }) => line), facts: ours.facts };
  for (const what of ['lines', 'facts']) {
    const have = records(got[what]);
    const missing = records(expected[what]).filter((one) => {
      const at = have.indexOf(one);
      if (at < 0) return true;
      have.splice(at, 1);
      return false;
    });
    expect(`every ${what === 'lines' ? 'line and part of a line' : 'fact'} the daemon hands ${when} to be drawn, and nothing else; not drawn: ${missing.slice(0, 3).join(' ; ')}; drawn and not handed: ${have.slice(0, 3).join(' ; ')}`, missing.length === 0 && have.length === 0);
  }
  for (const line of [...ours.lines.flatMap((one) => [one.words, one.label, ...one.only, one.from, ...one.tags, one.named, one.by]), ...ours.facts.map((one) => one.item)]) expect(`no line ${when} to hold a hole: ${line}`, !line?.includes('${'));
  const pressed = (line) => ours.lines.some((one) => one.pressed && one.at === line.at && (one.words === bare(line.title ?? line.text) || one.label === bare(line.label ?? null)));
  return theirs.map((part) => ({ ...part, lines: part.lines.map((line) => ({ ...line, text: bare(line.text), opens: line.opens?.what ?? null, pressed: pressed(line) })) }));
}
