// Stylesheets are bundled by the build; to the type checker they are just a side effect.
declare module '*.css';

interface Window {
  /** The desktop app's window, when the client is running inside it. */
  ipc?: { postMessage(message: string): void };
  /** How that window is framed; see `desktop.ts`. */
  desktop?: import('./desktop.ts').Desktop;
}

// The icons the client was built with, which the build hands over under
// this name (`build.mjs`): each by its name, its drawing and the words it
// is found by.
declare module 'icons:set' {
  export const icons: import('./icons.ts').IconSet;
}
