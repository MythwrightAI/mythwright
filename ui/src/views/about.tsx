// The page about the program, opened from its mark on the rail: what it is for, its guides and what is technical of
// it, a tab each in the page's head, as a Git page's are.
//
// All of it came with the program: the daemon hands what it is for and
// the guides marked public as pages (`/api/about`), made as any
// document's page is and drawn by what draws a document whole
// (`WholePage`), boxed off with nothing in it run; and nothing is fetched
// from anywhere to show any of it. What is technical is what the daemon
// says of itself to whoever reads it: where its data is kept only to
// whoever may manage the factory, the packages only to whoever may read
// them. The program's name is what the daemon says it is called.
//
// Which tab shows, which guide is read and what was read are kept by the
// page's tab (`useKept`), so the tab shown again is as it was left.

import { useEffect, useRef, useState } from 'preact/hooks';
import { ABOUT_TABS, type AboutTab, type Guide, type Told, asked, atOf, facts, guideOfLink, guideShown, guidesOf, packageSaid, packagesOf, tabCalled } from '../about.ts';
import { aboutAsked, readAbout, readGuide, sayAboutShowing } from '../aboutshown.ts';
import { showRelease } from '../releases.ts';
import { meta } from '../store.ts';
import { WholePage } from './file.tsx';
import { type Told as Meta, copyWhatToSay, readSystem, reportText } from './help.tsx';
import { Titled } from './pagehead.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { reveal, sideways } from './sideways.ts';
import { signs } from './signs.tsx';

export function AboutView() {
  const name = meta.value?.name ?? '';
  const [told, setTold] = useKept<Told | null>('about.told', null);
  const [failed, setFailed] = useKept<string | null>('about.failed', null);
  const [tab, setTab] = useKept<AboutTab>('about.tab', 'about');
  const [guide, setGuide] = useKept<string | null>('about.guide', null);
  const answered = useKeptRef<number>('about.asked', 0);
  const row = useRef<HTMLDivElement>(null);
  // Read behind, always: what was kept is drawn at once, and put right by what is read now.
  useEffect(() => {
    let stale = false;
    readAbout()
      .then((read) => !stale && (setTold(read), setFailed(null)))
      .catch((err: unknown) => !stale && setFailed(err instanceof Error ? err.message : String(err)));
    return () => {
      stale = true;
    };
  }, []);
  // A tab somebody asked for by an address.
  const asking = aboutAsked.value;
  useEffect(() => {
    if (!asking || asking.asked <= answered.current) return;
    answered.current = asking.asked;
    const wanted = asked(asking.at);
    setTab(wanted.tab);
    if (wanted.guide) setGuide(wanted.guide);
  }, [asking?.asked]);
  const guides = guidesOf(told);
  const reading = guideShown(guides, guide);
  // What the address bar says of it after the `#`.
  useEffect(() => sayAboutShowing(atOf(tab, tab === 'docs' ? (guide ?? null) : null)), [tab, guide]);
  useEffect(() => reveal(row.current), [tab]);
  return (
    <div class="about-page">
      <div class="view-head">
        <Titled>
          <span class="about-mark">{signs.mark(22)}</span>
          <h1>{name}</h1>
          {meta.value?.version && <span class="faint">{meta.value.version}</span>}
        </Titled>
        <div class="view-tabs tabs-ruled about-tabs">
          <div class="view-tabs-list" role="tablist" aria-label={`About ${name}`} ref={row} onWheel={sideways}>
            {ABOUT_TABS.map((one) => (
              <button key={one} type="button" role="tab" class="view-tab about-tab" data-about-tab={one} aria-selected={one === tab} aria-controls={`about-panel-${one}`} onClick={() => setTab(one)}>
                {tabCalled(one, name)}
              </button>
            ))}
          </div>
        </div>
      </div>
      <div class="view-body about-body">
        {failed && <p class="error">{failed}</p>}
        <div role="tabpanel" id={`about-panel-${tab}`} aria-label={tabCalled(tab, name)} class="about-panel" data-about-tab={tab}>
          {tab === 'about' ? (
            told?.html ? (
              <WholePage html={told.html} name={tabCalled('about', name)} />
            ) : (
              <div class="empty">Opening the page</div>
            )
          ) : tab === 'docs' ? (
            <Docs guides={guides} reading={reading} choose={setGuide} />
          ) : (
            <Technical told={told} />
          )}
        </div>
      </div>
    </div>
  );
}

/** The guides marked public, listed, and the one chosen read in place. */
function Docs({ guides, reading, choose }: { guides: Guide[]; reading: Guide | null; choose: (name: string) => void }) {
  const [pages, setPages] = useKept<Record<string, string>>('about.pages', {});
  const [failed, setFailed] = useState<string | null>(null);
  const shown = reading?.name ?? '';
  useEffect(() => {
    if (!shown || pages[shown] !== undefined) return;
    let stale = false;
    setFailed(null);
    readGuide(shown)
      .then((read) => !stale && setPages((was) => ({ ...was, [shown]: read.html })))
      .catch((err: unknown) => !stale && setFailed(err instanceof Error ? err.message : String(err)));
    return () => {
      stale = true;
    };
  }, [shown]);
  // A link to another guide reads that one; one to a page no copy of the program carries leads nowhere.
  const follow = (href: string) => {
    const other = guideOfLink(href, guides);
    if (other) choose(other);
  };
  if (guides.length === 0) return <div class="empty">Opening the guides</div>;
  return (
    <div class="about-docs">
      <nav class="about-guides" aria-label="Guides">
        {guides.map((one) => (
          <button key={one.name} type="button" class="ghost about-guide" aria-current={one.name === shown ? 'page' : undefined} data-guide={one.name} onClick={() => choose(one.name)}>
            {one.title}
          </button>
        ))}
      </nav>
      <div class="about-read" data-reading={shown}>
        {failed ? <p class="error">{failed}</p> : pages[shown] !== undefined ? <WholePage html={pages[shown]!} name={reading?.title ?? shown} onLink={follow} /> : <div class="empty">Opening the guide</div>}
      </div>
    </div>
  );
}

/** What is technical of the program, as far as the reader may be told it, and what to say in a report. */
function Technical({ told }: { told: Told | null }) {
  const said = meta.value as Meta | null;
  const [system, setSystem] = useKept<{ os: string; arch: string } | null>('about.system', null);
  const [report, setReport] = useKept<string>('about.report', '');
  useEffect(() => {
    let stale = false;
    void readSystem().then((read) => !stale && read && setSystem(read));
    void reportText().then((text) => !stale && setReport(text));
    return () => {
      stale = true;
    };
  }, []);
  const lines = facts({ version: said?.version, build: said?.client_build, runsFrom: said?.runs_from, os: system?.os, arch: system?.arch, profile: said?.profile, data: told?.data });
  const packages = packagesOf(told);
  return (
    <div class="stack about-technical">
      <section class="panel about-part" data-part="program">
        <div class="panel-head">
          <h2>This program</h2>
        </div>
        <dl class="about-facts">
          {lines.map((one) => (
            <div key={one.label} class="about-fact" data-fact={one.label}>
              <dt class="muted">{one.label}</dt>
              <dd class={one.label === 'Data folder' || one.label === 'Client build' ? 'mono' : undefined}>{one.said}</dd>
            </div>
          ))}
        </dl>
        <div class="row">
          <button type="button" class="ghost" onClick={() => showRelease()}>
            What's new in each release
          </button>
        </div>
      </section>
      {packages !== null && (
        <section class="panel about-part" data-part="packages">
          <div class="panel-head">
            <h2>Packages installed</h2>
            <span class="muted">{packages.length === 0 ? 'none' : packages.length}</span>
          </div>
          {packages.length > 0 && (
            <ul class="about-packages">
              {packages.map((one) => (
                <li key={one.name} data-package={one.name}>
                  <span class="truncate">{one.title}</span>
                  <span class={one.stands === 'differs' ? 'about-differs' : 'faint'}>{packageSaid(one)}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
      <section class="panel about-part" data-part="report">
        <div class="panel-head">
          <h2>What to say in a report</h2>
        </div>
        <p class="muted">The program and its version, the build of the client, the daemon's system and your browser. Copied, it is put on your clipboard and sent nowhere.</p>
        {report && <pre class="about-report">{report}</pre>}
        <div class="row">
          <button type="button" onClick={() => void copyWhatToSay()}>
            {signs.copy()} Copy
          </button>
        </div>
      </section>
    </div>
  );
}
