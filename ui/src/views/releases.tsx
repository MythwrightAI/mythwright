// The notes of each release, as a page in a tab.
//
// The daemon made the page from the one file the notes are written in, as
// it makes any document's page, and it is drawn by what draws a document
// whole (`WholePage`): boxed off, with nothing in it run. Opened from Help
// at a release, that release is brought into view.

import { useEffect } from 'preact/hooks';
import { notes, readReleases, shownRelease } from '../releases.ts';
import { WholePage } from './file.tsx';

export function Releases() {
  const page = notes.value;
  // Asked for again where an earlier reading did not come in.
  useEffect(() => {
    if (notes.peek() === null) void readReleases();
  }, []);
  return (
    <div class="releases">
      <div class="view-head">
        <h1>What's new</h1>
        <span class="faint">The notes of each release. They came with this program, and nothing is fetched to show them.</span>
      </div>
      <div class="view-body">{page === null ? <div class="empty">Opening the notes</div> : <WholePage html={page} name="Releases" shows={shownRelease.value} />}</div>
    </div>
  );
}
