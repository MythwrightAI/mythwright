// Which language a file is written in, by its name.
//
// Only the name is looked at. The colouring for each language is fetched
// when a file in it is first opened (see `codemirror.ts`), so knowing many
// languages costs nothing until one is used.

export type Language =
  | 'blueprint'
  | 'markdown'
  | 'yaml'
  | 'toml'
  | 'json'
  | 'javascript'
  | 'typescript'
  | 'jsx'
  | 'tsx'
  | 'rust'
  | 'python'
  | 'go'
  | 'c'
  | 'cpp'
  | 'java'
  | 'ruby'
  | 'sql'
  | 'shell'
  | 'html'
  | 'css'
  | 'dockerfile'
  | 'text';

const BY_ENDING: Record<string, Language> = {
  md: 'markdown',
  markdown: 'markdown',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'toml',
  lock: 'toml',
  json: 'json',
  jsonl: 'json',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  ts: 'typescript',
  mts: 'typescript',
  jsx: 'jsx',
  tsx: 'tsx',
  rs: 'rust',
  py: 'python',
  go: 'go',
  c: 'c',
  h: 'c',
  cc: 'cpp',
  cpp: 'cpp',
  hpp: 'cpp',
  java: 'java',
  rb: 'ruby',
  sql: 'sql',
  sh: 'shell',
  bash: 'shell',
  zsh: 'shell',
  html: 'html',
  htm: 'html',
  svg: 'html',
  xml: 'html',
  css: 'css',
};

/** Files known by their whole name. */
const BY_NAME: Record<string, Language> = {
  dockerfile: 'dockerfile',
  containerfile: 'dockerfile',
  '.bashrc': 'shell',
  '.zshrc': 'shell',
  '.profile': 'shell',
};

export function languageOfPath(path: string): Language {
  const name = (path.split('/').pop() ?? path).toLowerCase();
  if (BY_NAME[name]) return BY_NAME[name];
  // A lock file is in whatever its ending before `.lock` says, when it says.
  const ending = name.includes('.') ? name.slice(name.lastIndexOf('.') + 1) : '';
  if (ending === 'lock' && name.endsWith('.json.lock')) return 'json';
  return BY_ENDING[ending] ?? 'text';
}

const PICTURES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'bmp', 'ico', 'svg']);

/** Whether a file is a picture, to be shown and not read as text. A drawing written as text is still shown. */
export function isPicture(path: string): boolean {
  const name = (path.split('/').pop() ?? path).toLowerCase();
  const dot = name.lastIndexOf('.');
  return dot > 0 && PICTURES.has(name.slice(dot + 1));
}
