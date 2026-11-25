// Builds the client into dist/.
//
//   node build.mjs           one production build
//   node build.mjs --watch   rebuild on change, for development
//
// The daemon serves dist/ from disk and people have the client open while
// it is rebuilt, so a build replaces the client without taking anything
// from under them, and without the daemon being touched:
//
//   - new files are written beside the old ones (every name carries a hash
//     of its content);
//   - the entry page is replaced last and in one step. That is the moment
//     the new build exists: the daemon sees it and tells open pages, which
//     offer to reload;
//   - the builds before this one stay, so a page still on one of them can
//     fetch the parts it has not loaded yet: those replaced within a week,
//     twenty at most (`scripts/generations.ts`). Older ones are removed;
//   - one build at a time writes and tidies, holding `dist.lock`
//     (`scripts/publishing.ts`): a build beside it would otherwise take
//     away the files it had just written.
import { build, context } from 'esbuild';
import { createHash } from 'node:crypto';
import { access, mkdir, readFile, readdir, realpath, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { RECORD, hashOf, sourcesOf } from './scripts/built.ts';
import { generations, leftovers } from './scripts/generations.ts';
import { LOCK, whileHolding } from './scripts/publishing.ts';
import { built, moduleOf, reserved } from './scripts/iconset.ts';
import { called, differs, drawnIn, named, records, withinPrebuilt } from './scripts/notices.ts';

// The icons a table's column can be given are one set, read here and
// never fetched: the client asks for `icons:set` and is handed what of
// the set is built (`scripts/iconset.ts` says what is left out, and why),
// with the set's notice. It is asked for only where an icon is drawn or
// chosen, so it is a part of its own that most pages never load.
const ICONS = 'node_modules/lucide-static';
const PACKAGES = '../extensions';
let iconsBuilt = '';
const icons = {
  name: 'icons',
  setup(b) {
    b.onResolve({ filter: /^icons:set$/ }, () => ({ path: 'set', namespace: 'icons' }));
    b.onLoad({ filter: /.*/, namespace: 'icons' }, async () => {
      const read = (file) => readFile(file, 'utf8');
      // What may not be named is read where the gate reads it. A build
      // that cannot read it offers no icon it could not check.
      const unread = (err) => {
        throw new Error(`the icons are built only where the packages can be read, since an icon that names one is left out, and ${PACKAGES} could not be: ${err.message}`);
      };
      const folders = (await readdir(PACKAGES, { withFileTypes: true }).catch(unread)).filter((entry) => entry.isDirectory()).map((entry) => entry.name);
      const manifests = await Promise.all(folders.map((folder) => read(`${PACKAGES}/${folder}/extension.yaml`).catch(() => '')));
      const names = reserved(folders, manifests, await read(`${PACKAGES}/reserved`).catch(unread));
      const { version } = JSON.parse(await read(`${ICONS}/package.json`));
      const { icons, left } = built(JSON.parse(await read(`${ICONS}/icon-nodes.json`)), JSON.parse(await read(`${ICONS}/tags.json`)), names);
      iconsBuilt = `icons: ${Object.keys(icons).length} of lucide-static ${version}, ${left.length === 0 ? 'none left out' : `left out: ${left.join(', ')}`}`;
      const notice = `The icons a column can be given are Lucide's (lucide-static ${version}), under these terms:\n\n${(await read(`${ICONS}/LICENSE`)).trim()}`;
      return { contents: moduleOf(icons, notice), loader: 'js', watchFiles: [`${ICONS}/icon-nodes.json`, `${ICONS}/tags.json`, `${PACKAGES}/reserved`] };
    });
  },
};

// The diagram library (mermaid) lays diagrams out with ELK where asked, a
// library under the EPL or the GPL, which `about.toml` does not accept: it
// is never built in. What the diagram library loads of it is this, which
// says so: a diagram that asks for that layout is its code with that line
// under it, and every other is laid out by dagre (`diagramSettings`).
const withoutElk = {
  name: 'without-elk',
  setup(b) {
    b.onResolve({ filter: /^elkjs(\/|$)/ }, () => ({ path: 'elk', namespace: 'without-elk' }));
    b.onLoad({ filter: /.*/, namespace: 'without-elk' }, () => ({
      contents: `export default class { constructor() { throw new Error('The ELK layout is not in this client: take "layout: elk" (or "-elk") out of the diagram and it is laid out by the usual one.'); } }`,
      loader: 'js',
    }));
  },
};

const watch = process.argv.includes('--watch');
const options = {
  plugins: [icons, withoutElk],
  entryPoints: { app: 'src/main.tsx' },
  outdir: 'dist/assets',
  bundle: true,
  format: 'esm',
  // The editor and the Reference are large and not needed to start:
  // they are split out and loaded when first opened.
  splitting: true,
  // Hashed in development too: these files are served as never changing.
  entryNames: '[name]-[hash]',
  chunkNames: 'chunk-[hash]',
  assetNames: '[name]-[hash]',
  minify: !watch,
  sourcemap: watch ? 'inline' : false,
  target: 'es2022',
  jsx: 'automatic',
  jsxImportSource: 'preact',
  logLevel: 'info',
  metafile: true,
  // The files are written here, not by esbuild: when it rebuilds, esbuild
  // removes what its previous build wrote, which is exactly what a page
  // still on that build needs.
  write: false,
};

/**
 * Writes a file so that a reader sees the old one or the new one, never
 * half. The name it is written under first is this build's alone: two
 * builds writing one name at once would each rename the other's half.
 */
async function replace(path, content) {
  const aside = `${path}.${process.pid}.new`;
  await writeFile(aside, content);
  await rename(aside, path);
}

/**
 * Makes what esbuild just built the client the daemon serves, holding the
 * lock on dist/ throughout. The lock is beside the folder that is written,
 * wherever that is: a dist/ that is a link to another is that one.
 */
const publish = async (result) => {
  const lock = join(dirname(await realpath('dist')), LOCK);
  return whileHolding(lock, () => published(result));
};

async function published({ metafile, outputFiles }) {
  // A name carries the hash of its content, so a file already there is
  // already right.
  await Promise.all(
    outputFiles.map((file) =>
      access(file.path).then(
        () => {},
        () => replace(file.path, file.contents),
      ),
    ),
  );
  const outputs = Object.entries(metafile.outputs);
  const [entry, made] = outputs.find(([, output]) => output.entryPoint === 'src/main.tsx') ?? [];
  if (!entry || !made.cssBundle) throw new Error('the build has no entry script or no stylesheet');
  const served = (path) => path.replace(/^dist\//, '');
  const files = outputs.map(([path]) => served(path)).sort();
  // The names carry the content's hashes, so this changes exactly when
  // something the browser would load does.
  const id = createHash('sha256').update(files.join('\n')).digest('hex').slice(0, 12);

  const before = await readFile('dist/builds.json', 'utf8')
    .then((text) => JSON.parse(text))
    .catch(() => []);
  const kept = generations(Array.isArray(before) ? before : [], { id, at: Date.now(), files });
  await replace('dist/builds.json', JSON.stringify(kept));

  const template = await readFile('index.html', 'utf8');
  const html = template
    .replace('%APP_JS%', `/${served(entry)}`)
    .replace('%APP_CSS%', `/${served(made.cssBundle)}`)
    .replace('%BUILD%', id)
    .replace('%LIVE%', watch ? 'yes' : 'no');
  await replace('dist/index.html', html);
  // What it was built from, after the page that loads it: a check holds
  // the files beside it to this before it reads a page (`scripts/built.ts`).
  const sources = sourcesOf(Object.keys(metafile.inputs));
  const read = await Promise.all(sources.map((path) => readFile(path).then(hashOf)));
  await replace(RECORD, JSON.stringify({ id, files: Object.fromEntries(sources.map((path, at) => [path, read[at]])) }));

  const present = (await readdir('dist/assets')).map((name) => `assets/${name}`);
  await Promise.all(leftovers(kept, present).map((file) => rm(`dist/${file}`, { force: true })));
  return id;
}

// What a view shown in a channel may draw with. A view reaches nothing
// over the network, so what it is given is served from here, under a name
// that does not change: the pages that use it are written by other hands.
const CHARTS = 'node_modules/chart.js';

// Whose work is in what is served, by the folder of each package: what the
// bundler took a file of that is in what it wrote, and the two this build
// puts there itself, one as its authors built it and one read by the
// plugin above. A third thing served that the bundler does not read is
// named here, or it is handed on with no notice.
const bundled = async (metafile) => [...new Set([...drawnIn(metafile), ...(await withinPrebuilt(CHARTS)), ICONS])].sort();

// Asked by `cargo xtask notices`: the notice of each of those, written out
// for it to read. Nothing is built into dist/ and nothing is served anew.
if (process.argv.includes('--notices')) {
  const result = await build({ ...options, logLevel: 'silent' });
  const said = await records(await bundled(result.metafile), await readFile('../about.toml', 'utf8'));
  // All of it is handed over before this ends: what reads it reads a pipe.
  await new Promise((written) => process.stdout.write(said, written));
  process.exit(0);
}

await mkdir('dist/lib', { recursive: true });
await replace('dist/lib/chart.js', await readFile(`${CHARTS}/dist/chart.umd.min.js`));

await mkdir('dist/assets', { recursive: true });
if (watch) {
  const ctx = await context({
    ...options,
    plugins: [
      icons,
      withoutElk,
      {
        name: 'publish',
        setup: (b) =>
          b.onEnd((result) => {
            // A build that failed leaves the last good one being served.
            if (result.errors.length > 0 || !result.metafile) return;
            return publish(result).then(
              (id) => console.log(`client ${id}; open pages reload`),
              (err) => console.error(`could not publish the build: ${err.message}`),
            );
          }),
      },
    ],
  });
  await ctx.watch();
} else {
  const result = await build(options);
  // How the client's check builds: a package that is in what was built and
  // not in the notices a package of the product carries fails it, before
  // anything is served.
  if (process.argv.includes('--held')) {
    const drawn = await Promise.all((await bundled(result.metafile)).map(called));
    const wrong = differs(drawn.sort(), named(await readFile('../THIRD-PARTY-NOTICES.md', 'utf8').catch(() => '')));
    if (wrong) {
      console.error(wrong);
      process.exit(1);
    }
  }
  const id = await publish(result);
  const sizes = Object.entries(result.metafile.outputs)
    .filter(([name]) => name.endsWith('.js') || name.endsWith('.css'))
    .map(([name, out]) => `${name.replace('dist/assets/', '')} ${(out.bytes / 1024).toFixed(0)}kB`);
  console.log(`client ${id}  ${sizes.join('  ')}`);
  if (iconsBuilt) console.log(iconsBuilt);
}
