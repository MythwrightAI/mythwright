// What Linked from, at the end of a topic's article (`views/articleend.tsx`),
// lists: the pages that link to it as the daemon says them
// (`GET /api/topics/<topic>/linked`), those whose only link is a series box
// gathered apart, by the series' title. Every page of a series carries the
// same box, so a page in a series of twenty would otherwise list nineteen
// pages that say nothing of it, over those that do.

/** A page that links to the topic, as the daemon says it: where its only link is a series box, that box's title. */
export interface Linking {
  id: string;
  title: string;
  wiki: { id: string; title: string } | null;
  series?: string | null;
}

/** The pages listed one by one, in the order said, and those of each series, by the series' title in the order first met. */
export function gatheredLinks(linked: readonly Linking[]): { listed: Linking[]; series: { title: string; pages: Linking[] }[] } {
  const listed: Linking[] = [];
  const series: { title: string; pages: Linking[] }[] = [];
  for (const page of linked) {
    if (typeof page.series !== 'string') {
      listed.push(page);
      continue;
    }
    const one = series.find((each) => each.title === page.series);
    if (one) one.pages.push(page);
    else series.push({ title: page.series, pages: [page] });
  }
  return { listed, series };
}
