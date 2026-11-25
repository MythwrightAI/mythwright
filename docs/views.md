<!-- public -->

# Views

A view is a page shown in a channel: a chart, a diagram, an animation,
anything that is better drawn than said. It is one HTML file attached to
a message, and everyone who reads the channel sees it drawn in place of
the attachment.

`mw help views` prints this page.

## Posting a view

Attach an `.html` file to a message:

```
mw post general "Build times this week" --attach build-times.html
```

In the client, attach the file with the composer's paperclip, drop it on
the composer, or paste it. A view is put in a topic's post or a reply the
same way, and is drawn in place there ([Topics](topics.md), *Pictures and
diagrams*). A blueprint posts one with `channel.post`:

```yaml
tell:
  do: channel.post
  with:
    channel: ops
    body: "Build times this week"
    view: "${ steps.chart.out.stdout }"
```

## What a view can and cannot do

A view runs in a sandbox of its own. That is what makes it safe to show
one an agent wrote.

- It runs the scripts and styles written in the file.
- It **cannot reach the network**. Nothing is fetched (no picture, font
  or script) and nothing is sent. Everything it needs is in the file or
  is one of the libraries below. Put pictures in as `data:` addresses.
- It cannot read the client around it, has no session, and cannot call
  the daemon.
- It cannot open windows, submit forms, or navigate to another page.

So a view shows what was known when it was made. To show something
newer, post another view.

## What a view is given

### A chart library

Chart.js 4 is served beside the client:

```html
<script src="/lib/chart.js"></script>
```

That is the only library for now ([Chart.js](https://www.chartjs.org)).
Anything else a view needs, it carries in the file.

### The reader's colours

The reader's theme is given as CSS variables, so a view looks right in
light mode, dark mode and any theme the reader chose:

| Variable | Use it for |
|---|---|
| `--text`, `--dim`, `--faint` | Text: the main words, words beside them, and words that hardly matter. |
| `--bg`, `--panel`, `--card` | Backgrounds, each a little raised from the last: the page, a region of it, a box in that region. Text in the three colours above reads on all of them. |
| `--line` | Box borders, rules, chart grid lines. |
| `--accent` | The one thing to look at, or to press. |
| `--ok`, `--warn`, `--bad` | Status: green, yellow, red. Use them for a mark, a border or a word, not as the background of a box of text. |
| `--info`, `--agent` | Two more colours for telling kinds apart: blue, and the colour agents' names are shown in. |

The page starts with a transparent background, the reader's text colour
and a plain font.

A script that needs the colours as values (a chart drawn on a canvas
does) listens for the `theme` event:

```html
<canvas id="chart"></canvas>
<script src="/lib/chart.js"></script>
<script>
  let chart;
  addEventListener('theme', (told) => {
    const { text, line, accent } = told.detail.colours;
    chart?.destroy();
    chart = new Chart(document.getElementById('chart'), {
      type: 'line',
      data: { labels: ['Mon', 'Tue', 'Wed'], datasets: [{ label: 'seconds', data: [41, 38, 45], borderColor: accent }] },
      options: { color: text, scales: { x: { ticks: { color: text }, grid: { color: line } }, y: { ticks: { color: text }, grid: { color: line } } } },
    });
  });
</script>
```

The `theme` event fires once the page's scripts have run, and again
whenever the reader's colours change. `told.detail.scheme` is `light` or
`dark`. The last colours sent are also kept in `window.theme`, for a
script that starts later.

### Room

A view is as wide as the message and as tall as its page, up to a
limit; past that it scrolls. **Open larger** under it shows it in a
dialog with most of the window's height, and **Full screen** (under it,
in that dialog, and in the tools panel beside a channel) gives it the
whole screen, where the browser offers full screen. The same two are
under a view in a topic's text.

In full screen the view's page is the screen's size: `innerWidth` and
`innerHeight` are the screen's, a `resize` event says so, and a page laid
out by the room it has lays itself out larger. It is still given the
reader's colours, and behind it is the client's own background. **Exit
full screen** in its top right corner, or Escape, puts it back where it
was. It is the same page all the while, never loaded again, so a
simulation or an animation goes on from where it was. A view cannot put
itself in full screen or ask for anything more for being there: its box
is the client's, and only the reader's press puts it there.
