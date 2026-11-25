// Whose work is in what the client's build writes, and under what terms.
//
// A package of the product carries the notices of what it bundles
// (THIRD-PARTY-NOTICES.md, made by `cargo xtask notices`). The client's
// half is said from here, by the build itself, since only the build knows
// which packages are in what it serves: those the bundler took a file of
// that is in what it wrote, and those the build puts there whole, already
// built. What only builds or tests the client is in neither.
//
// `node build.mjs --notices` writes each such package as a record: its
// licence, and the text of its licence as the package carries it. And
// `node build.mjs --held`, which is how the client's check builds, fails
// where the packages in what was just built are not the ones the notices
// name: a package newly drawn in has no notice until the file is made
// again.
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { readFile, readdir } from 'node:fs/promises';

/** The folder of the package a file of the bundle is of, or nothing for a file of the client's own. */
export function packageOf(file: string): string | null {
  const marker = 'node_modules/';
  const at = file.lastIndexOf(marker);
  if (at < 0) return null;
  const parts = file.slice(at + marker.length).split('/');
  const name = parts[0]?.startsWith('@') ? parts.slice(0, 2).join('/') : parts[0];
  return name ? file.slice(0, at + marker.length) + name : null;
}

/** What the bundler says of what it wrote: for each file written, how much of each file it read is in it. */
export interface Written {
  outputs: Record<string, { inputs: Record<string, { bytesInOutput: number }> }>;
}

/** The folders of the packages that have anything in what the bundler wrote. */
export function drawnIn(written: Written): string[] {
  const folders = new Set<string>();
  for (const output of Object.values(written.outputs)) {
    for (const [file, part] of Object.entries(output.inputs)) {
      const folder = part.bytesInOutput > 0 ? packageOf(file) : null;
      if (folder) folders.add(folder);
    }
  }
  return [...folders].sort();
}

/**
 * A package that is served as it was built by its authors, and what that
 * build holds: the packages it depends on, which are built into it.
 */
export async function withinPrebuilt(folder: string, seen = new Set<string>()): Promise<string[]> {
  if (seen.has(folder)) return [];
  seen.add(folder);
  const manifest = JSON.parse(await readFile(`${folder}/package.json`, 'utf8'));
  for (const name of Object.keys(manifest.dependencies ?? {})) {
    const nested = `${folder}/node_modules/${name}`;
    await withinPrebuilt(existsSync(nested) ? nested : `node_modules/${name}`, seen);
  }
  return [...seen].sort();
}

/** The licences a library may be under, as `about.toml` at the top of the repository lists them. */
export function accepted(config: string): string[] {
  const at = config.search(/^accepted\s*=\s*\[/m);
  if (at < 0) return [];
  const list = config.slice(at, config.indexOf(']', at));
  return [...list.matchAll(/^\s*"([^"]+)"/gm)].map((match) => match[1] ?? '');
}

/**
 * The licence a package is taken under, of what its manifest says: the one
 * it names, or of several it offers (`A OR B`) the first that is accepted.
 */
export function taken(said: unknown, accepted: string[]): string | null {
  if (typeof said !== 'string') return null;
  const offered = said.replace(/^\(|\)$/g, '').split(/\s+OR\s+/);
  if (offered.some((one) => /\s/.test(one.trim()))) return null;
  return accepted.find((one) => offered.includes(one)) ?? null;
}

/** What each licence is called, of those a package of the client's has been under. */
const CALLED: Record<string, string> = {
  'MIT': 'MIT License',
  'ISC': 'ISC License',
  'BSD-2-Clause': 'BSD 2-Clause "Simplified" License',
  'BSD-3-Clause': 'BSD 3-Clause "New" or "Revised" License',
  'Apache-2.0': 'Apache License 2.0',
};

/**
 * The text of a licence as it is written for anybody's work, for a package
 * that carries no file of its own: said with who the package's manifest
 * names as having written it. The two a package without a file is nearly
 * always under; another is given its text here when one comes.
 */
const STANDARD: Record<string, string> = {
  'MIT': `MIT License

Copyright (c) <year> <copyright holders>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`,
  'ISC': `ISC License

Copyright (c) <year> <copyright holders>

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.`,
};

/** Who a package's manifest names as having written it. */
export function by(manifest: { author?: unknown; contributors?: unknown }): string[] {
  const one = (said: unknown): string => {
    if (typeof said === 'string') return said;
    const { name, email } = (said ?? {}) as { name?: unknown; email?: unknown };
    return typeof name !== 'string' ? '' : typeof email === 'string' ? `${name} <${email}>` : name;
  };
  const others = Array.isArray(manifest.contributors) ? manifest.contributors : [];
  return [manifest.author, ...others].map(one).filter((name) => name !== '');
}

/**
 * The packages whose manifest names no licence, with the licence the file
 * of it they carry is, and that file's SHA-256: it was read before it was
 * put here, and an update of the package that changes it is refused until
 * it is read again. The client's half of what `about.toml` says of crates
 * (a `clarify`).
 */
export const CLARIFIED: Record<string, { licence: string; file: string; sha256: string }> = {
  // The MIT licence, naming who wrote it; its manifest says nothing of it (2.1.0).
  khroma: { licence: 'MIT', file: 'license', sha256: '66b333b0f66759a0b710459e03f7029abe17f4358114a128d2c972e642961b49' },
};

/**
 * What a package's manifest says it is under: what it names, or for one
 * that names nothing and is clarified, what `CLARIFIED` says, once the
 * file's SHA-256 (`sha256Of`) is the one that was read.
 */
export function saidUnder(manifest: { name?: unknown; license?: unknown }, sha256Of: (file: string) => string | null): { said: unknown; file?: string } | { changed: string } {
  const clarified = manifest.license === undefined && typeof manifest.name === 'string' ? CLARIFIED[manifest.name] : undefined;
  if (!clarified) return { said: manifest.license };
  const now = sha256Of(clarified.file);
  if (now !== clarified.sha256) {
    return {
      changed: `${manifest.name}'s ${clarified.file} is not the file that was read for it in CLARIFIED in ui/scripts/notices.ts (its SHA-256 is now ${now ?? 'nothing: it is not there'}): read it again, and say there what licence it is`,
    };
  }
  return { said: clarified.licence, file: clarified.file };
}

/**
 * The files of its licence among a package's files: those called so (a
 * program is not, whatever it is called), and of several, those called
 * after the licence it is taken under, where any is.
 */
export function licenceFiles(files: string[], licence: string): string[] {
  const all = files.filter((file) => /^(licen[sc]e|copying)([-._].*)?$/i.test(file) && !/\.(c|m)?(j|t)s$/i.test(file)).sort();
  const word = (licence.split('-')[0] ?? licence).toLowerCase();
  const its = all.filter((file) => file.toLowerCase().includes(word));
  return all.length > 1 && its.length > 0 ? its : all;
}

/** One package's record, or why none can be written. */
async function record(folder: string, licences: string[]): Promise<string> {
  const manifest = JSON.parse(await readFile(`${folder}/package.json`, 'utf8'));
  const called = `${manifest.name} ${manifest.version}`;
  // Only a clarified package's file is read for its SHA-256.
  const clarified = manifest.license === undefined ? CLARIFIED[manifest.name] : undefined;
  const sha256 = clarified
    ? await readFile(`${folder}/${clarified.file}`).then(
        (bytes) => createHash('sha256').update(bytes).digest('hex'),
        () => null,
      )
    : null;
  const under = saidUnder(manifest, () => sha256);
  if ('changed' in under) throw new Error(under.changed);
  const licence = taken(under.said, licences);
  if (!licence) {
    throw new Error(
      `${called} is in what the client's build writes, and its manifest says it is under ${JSON.stringify(manifest.license)}, which is no licence about.toml accepts. One that asks no more than that its notice goes with a copy is added to \`accepted\` there; one that asks more is the maintainer's to accept, or the package is not used`,
    );
  }
  // The file of its licence: the one there is, or of several the ones
  // called after the licence it is taken under.
  const files = under.file ? [under.file] : licenceFiles(await readdir(folder), licence);
  const standard = files.length === 0 ? STANDARD[licence] : undefined;
  if (files.length === 0 && !standard) {
    throw new Error(
      `${called} is in what the client's build writes and carries no file of its licence (${licence}), so there is no notice of it to hand on. Its text is to be found where the package is kept, and given to STANDARD in ui/scripts/notices.ts`,
    );
  }
  const texts = await Promise.all(files.map((file) => readFile(`${folder}/${file}`, 'utf8')));
  const lines = [
    `%%licence%% ${licence}`,
    `%%name%% ${CALLED[licence] ?? licence}`,
    `%%from%% ${standard ? 'standard' : 'file'}`,
    `%%library%% ${called}`,
    `%%where%% https://www.npmjs.com/package/${manifest.name}`,
    ...(standard ? by(manifest).map((one) => `%%by%% ${one}`) : []),
    '%%text%%',
    standard ?? texts.map((text) => text.trim()).join('\n\n'),
    '%%end%%',
  ];
  // What it says besides, which a licence may ask to be handed on too.
  for (const file of (await readdir(folder)).filter((file) => /^notice([-._].*)?$/i.test(file)).sort()) {
    lines.push(`%%besides%% ${file}`, `%%library%% ${called}`, '%%text%%', (await readFile(`${folder}/${file}`, 'utf8')).trim(), '%%end%%');
  }
  return `${lines.join('\n')}\n`;
}

/** The records of the packages in these folders, as the tooling reads them. */
export async function records(folders: string[], config: string): Promise<string> {
  const licences = accepted(config);
  if (licences.length === 0) throw new Error('about.toml at the top of the repository lists no accepted licence');
  return (await Promise.all(folders.map((folder) => record(folder, licences)))).join('');
}

/** A package's name and version, as the notices and a build are held to each other by. */
export async function called(folder: string): Promise<string> {
  const manifest = JSON.parse(await readFile(`${folder}/package.json`, 'utf8'));
  return `${manifest.name} ${manifest.version}`;
}

/** The client's packages a notices file names, each as its name and version. */
export function named(notices: string): string[] {
  const names = new Set<string>();
  let within = false;
  let fence: string | null = null;
  for (const line of notices.split('\n')) {
    // Nothing inside a licence's text is read as anything.
    if (fence !== null) {
      if (line === fence) fence = null;
    } else if (/^````+text$/.test(line)) {
      fence = line.slice(0, -'text'.length);
    } else if (line.startsWith('## ')) {
      within = line === '## The client: packages';
    } else if (within) {
      const library = /^- `([^`]+)` ([^\s,]+)/.exec(line);
      if (library) names.add(`${library[1]} ${library[2]}`);
    }
  }
  return [...names].sort();
}

/** What is wrong where the packages just built are not those the notices name, or nothing. */
export function differs(built: string[], named: string[]): string | null {
  const missing = built.filter((one) => !named.includes(one));
  const gone = named.filter((one) => !built.includes(one));
  if (missing.length === 0 && gone.length === 0) return null;
  const said = [
    missing.length > 0 ? `in what was built and not in the notices: ${missing.join(', ')}` : '',
    gone.length > 0 ? `in the notices and no longer in what is built: ${gone.join(', ')}` : '',
  ].filter((part) => part !== '');
  return `THIRD-PARTY-NOTICES.md does not name the packages the client is built from (${said.join('; ')}). Run \`cargo xtask notices\` at the top of the repository and commit what it writes`;
}
