// A project's files: its folder, browsed a folder at a time. The folder's
// entries, and under them its README, where it has one, as the page it
// would be published as.
//
// Nothing here knows about any kind of source control, or what the files
// are of: it is the folder as it is, for a project that is code and for
// one that is not. It is a page of its own, off the project's front page,
// which is about how the project stands and not about what is in its folder.
//
// Whoever may write the project's files makes a file or a folder here, in
// the folder the page is showing (`repomade.tsx`): by the buttons in its
// head, which a library's New document is beside its own, and by lines of
// its gear.

import { useEffect } from 'preact/hooks';
import { api } from '../api.ts';
import type { Entry } from '../api.ts';
import { attempt, fileVersions, folders, isBlueprint, lookAt, openFolder, openTab, projects, trees } from '../store.ts';
import { Empty, NotTrusted } from '../ui.tsx';
import { ago, fileSize, parentOf } from '../util.ts';
import { PagePreview } from './file.tsx';
import { PageActs, Titled } from './pagehead.tsx';
import { askToMake, makeLines, mayMake } from './repomade.tsx';
import { useKept } from './returned.tsx';
import { headWord, ThingWas } from './thingwas.tsx';

/** The file that says what a folder is for. */
export const README = /^readme\.(md|markdown)$/i;

export function RepoView({ project: id }: { project: string }) {
  const project = projects.value.find((one) => one.id === id);
  // `undefined` while it is being read; `null` when there is none.
  // As it was last read, while it is read again.
  const [readme, setReadme] = useKept<string | null | undefined>(`readme:${id}`, undefined);
  // The folder being looked at, while there is such a folder; else the top.
  const entries = trees.value[id] ?? [];
  const asked = folders.value[id] ?? '';
  const folder = asked !== '' && entries.some((entry) => entry.dir && entry.path === asked) ? asked : '';
  const here = entries
    .filter((entry) => parentOf(entry.path) === folder)
    .sort((a, b) => Number(b.dir) - Number(a.dir) || a.name.localeCompare(b.name));
  const about = here.find((entry) => !entry.dir && README.test(entry.name));

  useEffect(() => lookAt(id), [id]);

  const version = about ? (fileVersions.value[`${id}:${about.path}`] ?? 0) : 0;
  useEffect(() => {
    if (!about) {
      setReadme(null);
      return;
    }
    // What was read before stands until what is there now has been read: nothing is drawn as being read again.
    let stale = false;
    api
      .readFile(id, about.path)
      .then((content) => !stale && setReadme(content))
      .catch(() => !stale && setReadme(null));
    return () => {
      stale = true;
    };
  }, [id, about?.path, version]);

  const making = mayMake();
  if (!project) return <Empty title="This project is gone" />;
  return (
    <div>
      <div class="view-head">
        <Titled>
          <h1>
            {project.name} {headWord(id, 'repo', 'repo')}
          </h1>
          <ThingWas project={id} thing="repo" />
          <NotTrusted project={id} />
          <code class="faint truncate">{project.path}</code>
        </Titled>
        <span class="grow" />
        <PageActs tab={{ kind: 'repo', project: id }} menu={() => makeLines(id, folder)} last={making.length === 0} />
        {making.includes('file') && (
          <button class="primary" onClick={() => askToMake('file', id, folder)}>
            New file
          </button>
        )}
        {making.includes('folder') && <button onClick={() => askToMake('folder', id, folder)}>New folder</button>}
      </div>
      <div class="view-body stack" style="gap:20px">
        <Files project={id} name={project.name} folder={folder} here={here} />
        {about && (
          <div class="panel">
            <div class="panel-head">
              <h2>{about.name}</h2>
              <span class="muted">as the page it would be published as</span>
              <span class="grow" />
              <button class="ghost link" onClick={() => openTab({ kind: 'file', project: id, path: about.path })}>
                Write in it
              </button>
            </div>
            {readme == null ? (
              <div class="empty">Reading {about.name}</div>
            ) : (
              <div class="project-readme whole">
                <PagePreview
                  source={readme}
                  name={about.name}
                  whole
                  onLink={(href) => {
                    // A link to another file of the project, from where this one is.
                    const at = about.path.split('/').slice(0, -1);
                    for (const part of href.split('#')[0]!.split('?')[0]!.split('/')) {
                      if (part === '..') at.pop();
                      else if (part !== '.' && part !== '') at.push(decodeURIComponent(part));
                    }
                    if (at.length > 0) openTab({ kind: 'file', project: id, path: at.join('/') });
                  }}
                />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * A folder of the project: where it is, and what is in it, folders first.
 * A folder is opened here; a file opens in a tab of its own.
 */
function Files({ project, name, folder, here }: { project: string; name: string; folder: string; here: Entry[] }) {
  const parts = folder === '' ? [] : folder.split('/');
  return (
    <div class="panel files">
      <div class="panel-head">
        <span class="crumbs truncate">
          {parts.length === 0 ? (
            <strong>{name}</strong>
          ) : (
            <button class="ghost link" onClick={() => openFolder(project, '')}>
              {name}
            </button>
          )}
          {parts.map((part, index) => (
            <span key={index}>
              {' / '}
              {index === parts.length - 1 ? (
                <strong>{part}</strong>
              ) : (
                <button class="ghost link" onClick={() => openFolder(project, parts.slice(0, index + 1).join('/'))}>
                  {part}
                </button>
              )}
            </span>
          ))}
        </span>
        <span class="grow" />
        <span class="muted">{here.length === 1 ? '1 entry' : `${here.length} entries`}</span>
        <button class="ghost" title="Look for files that something else has changed" onClick={() => void attempt(() => api.scanProject(project))}>
          Look again
        </button>
      </div>
      <table>
        <tbody>
          {folder !== '' && (
            <tr class="clickable" onClick={() => openFolder(project, parentOf(folder))}>
              <td colSpan={3}>
                <span class="glyph">▴</span> ..
              </td>
            </tr>
          )}
          {here.map((entry) => (
            <tr
              key={entry.path}
              class="clickable"
              onClick={() => (entry.dir ? openFolder(project, entry.path) : openTab({ kind: 'file', project, path: entry.path }))}
            >
              <td>
                <span class="glyph">{entry.dir ? '▸' : isBlueprint(entry.path) ? '◆' : '·'}</span> {entry.dir ? <strong>{entry.name}</strong> : entry.name}
              </td>
              <td class="num muted">{entry.dir ? '' : fileSize(entry.size)}</td>
              <td class="num muted">{ago(entry.modified)}</td>
            </tr>
          ))}
          {here.length === 0 && (
            <tr>
              <td class="faint">This folder is empty.</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
