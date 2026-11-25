# The icon

The product's mark, white on a black square, drawn for the app's icon. The mark itself, and the mark with the
product's name under it, are kept as drawings in `docs/brand/` (a dark
and a light version of each); this square is the mark set on its ground.

## Replacing it

Replace `icon.svg` and run, from the top of the repository:

```sh
node desktop/icon/make.mjs
```

That writes every other file here from the drawing, and puts the drawing
in the client's entry page (`ui/index.html`) as the page's own icon. It
draws them with the browser the client's check scripts use (`BROWSER`,
`BROWSER_LIBS` as at the top of `ui/scripts/walkthrough.mjs`), since no
other image tool is counted on. Then build: the window's icon is built
into the desktop program.

The drawing is square (any square view box), and has to read at 16 pixels, which is the size a browser's tab and a title bar give
it: few shapes, thick ones.

## What is here

| File | What it is | Used for |
|---|---|---|
| `icon.svg` | The mark as a drawing | The source of every other file; the page's icon in a browser; the menu entry on a desktop that draws icons at any size |
| `icon-512.png`, `icon-256.png`, `icon-48.png` | The drawing at those sizes, see-through outside its shape | The menu entry and task switcher on Linux |
| `icon-window.rgba` | The drawing at 128 by 128 as bare pixels: four bytes each, a row at a time | The window's own icon, built into the desktop program (`desktop::ICON`) |
| `make.mjs` | What makes all of the above from `icon.svg` | |

Not here yet: `icon.icns` for macOS (the application's icon there is its
bundle's, not the window's, and the package for macOS holds no bundle
yet) and `icon.ico` for Windows (16, 32, 48 and 256 in one file, which is
built into a program there as a resource; nothing builds one in yet).
The packages for those two systems carry no picture of the icon.

## Where it shows

- **The window**, on Linux and Windows: the program gives its window the
  pixels built into it. On macOS a window has no icon of its own.
- **The desktop's menu**, on Linux: an entry naming the program and the
  icon, under the person's own data folder (`~/.local/share`, or
  `$XDG_DATA_HOME`). `cargo xtask install --desktop` puts both there. A
  package for Linux (`cargo xtask package`) carries the pictures under
  `icon/` and a script, `menu.sh`, that puts them and the entry there
  from wherever the folder was unpacked, since an entry names its program by its whole path;
  `menu.sh remove` takes them away. What an entry says is in
  `xtask/src/menu.rs`.
- **A browser's tab**: the drawing, written into the client's entry page.

The name the entry and the icon are filed under is `app_id` in
[brand.toml](../../brand.toml).
