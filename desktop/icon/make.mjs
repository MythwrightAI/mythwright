// Makes every picture of the icon from the one drawing, icon.svg:
//
//   node desktop/icon/make.mjs
//
// It writes, beside the drawing: icon-512.png, icon-256.png and icon-48.png
// (the menu entry, the task switcher, title bars) and icon-window.rgba, the
// window's own icon as the bare pixels the desktop program is built with.
// It also puts the drawing in the client's entry page (ui/index.html) as
// the page's icon. Replacing the icon is replacing icon.svg and running
// this.
//
// The pictures are drawn by the browser the client's check scripts use,
// since there is no other image tool to count on: BROWSER and BROWSER_LIBS
// mean what they mean at the top of ui/scripts/walkthrough.mjs.
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const ui = join(here, '../../ui');
// The browser's driver is the client's, not a second one kept here.
const { chromium } = createRequire(join(ui, 'package.json'))('playwright-core');

/** The sizes kept as pictures, and the side of the window's own. */
const PICTURES = [512, 256, 48];
const WINDOW = 128;

async function findBrowser() {
  if (process.env.BROWSER) return process.env.BROWSER;
  const cache = join(homedir(), '.cache/ms-playwright');
  const shells = (await readdir(cache)).filter((name) => name.startsWith('chromium_headless_shell')).sort();
  if (shells.length === 0) throw new Error('no browser found; set BROWSER to a Chromium binary');
  return join(cache, shells.at(-1), 'chrome-headless-shell-linux64/chrome-headless-shell');
}

const drawing = await readFile(join(here, 'icon.svg'), 'utf8');
const source = `data:image/svg+xml;base64,${Buffer.from(drawing).toString('base64')}`;

const browser = await chromium.launch({
  executablePath: await findBrowser(),
  env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
});
try {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  for (const side of PICTURES) {
    await page.setViewportSize({ width: side, height: side });
    await page.setContent(
      `<body style="margin:0;background:transparent"><img src="${source}" width="${side}" height="${side}" style="display:block">`,
    );
    await page.locator('img').evaluate((img) => img.decode());
    await page.screenshot({ path: join(here, `icon-${side}.png`), omitBackground: true });
    console.log(`icon-${side}.png`);
  }
  // The window is told its icon as pixels, four bytes each, a row at a
  // time: read back from a canvas the drawing was put on.
  const pixels = await page.evaluate(
    async ({ source, side }) => {
      const img = new Image();
      img.src = source;
      await img.decode();
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = side;
      const context = canvas.getContext('2d');
      context.drawImage(img, 0, 0, side, side);
      const { data } = context.getImageData(0, 0, side, side);
      let text = '';
      for (const byte of data) text += String.fromCharCode(byte);
      return btoa(text);
    },
    { source, side: WINDOW },
  );
  const raw = Buffer.from(pixels, 'base64');
  if (raw.length !== WINDOW * WINDOW * 4) throw new Error(`the window's icon came out as ${raw.length} bytes`);
  await writeFile(join(here, 'icon-window.rgba'), raw);
  console.log(`icon-window.rgba  (${WINDOW} by ${WINDOW})`);
} finally {
  await browser.close();
}

// The page's own icon: the drawing itself, written into the entry page so
// that nothing more is fetched for it.
const entry = join(ui, 'index.html');
const html = await readFile(entry, 'utf8');
const inline = drawing
  .replace(/<!--[\s\S]*?-->/g, '')
  .replace(/\s*\n\s*/g, '')
  .replace(/"/g, "'")
  .replace(/%/g, '%25')
  .replace(/#/g, '%23')
  .replace(/</g, '%3C')
  .replace(/>/g, '%3E');
const link = /<link rel="icon" href="[^"]*" \/>/;
if (!link.test(html)) throw new Error('ui/index.html has no <link rel="icon" ... /> to put the icon in');
await writeFile(entry, html.replace(link, `<link rel="icon" href="data:image/svg+xml,${inline}" />`));
console.log('ui/index.html  (the page\'s icon)');
