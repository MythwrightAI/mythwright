import { render } from 'preact';
import { App } from './app.tsx';
import { framed } from './desktop.ts';
import { boot } from './store.ts';
// What keeps the open tabs, a set for each workspace, says so to the store before anybody enters.
import './tabsof.ts';
// What reads and follows the themes of whoever signs in, which are kept with their account.
import './themed.ts';
// What reads and follows how a time is written for whoever signs in, which is kept with their account.
import './clockkept.ts';
// What reads and follows how what whoever signs in reads looks for them, which is kept with their account.
import './appearancekept.ts';
// What reads and follows each project's environments, which the side panel and an apply go by.
import './envsheld.ts';
import { sayTitles } from './tips.ts';
import './themes.css';
import './style.css';
import './context.css';
import './graph.css';
import './themepanel.css';
import './peoplegroups.css';
import './colourpick.css';
import './drawer.css';
import './talked.css';
import './external.css';
import './projectenvs.css';
import './flags.css';
import './gitpage.css';
import './about.css';
import './cardrows.css';
import './trash.css';
import './clocked.css';
import './appearance.css';
import './inbox.css';
import './reviewed.css';
import './conversed.css';
import './composer.css';
import './table.css';
import './ticks.css';
import './comments.css';
import './handle.css';
import './mockups.css';
import './page.css';
import './where.css';
import './grouped.css';
import './foot.css';
import './spacesettings.css';
import './secrets.css';
import './comparing.css';
import './ordered.css';
import './holds.css';
import './sits.css';
import './groups.css';
import './agentlist.css';
import './agentedit.css';
import './settingup.css';
import './channeldocs.css';
import './posted.css';
import './kept.css';
import './repomade.css';
import './archived.css';
import './topic.css';
import './topicpage.css';
import './contents.css';
import './topichistory.css';
import './pictures.css';
import './cited.css';
import './boxes.css';
import './prosetables.css';
import './mentioned.css';
import './wikis.css';
import './direct.css';
import './members.css';
import './agenthistory.css';
import './cardcost.css';
import './cardtabs.css';
import './recordfiles.css';
import './runs.css';
import './pipelines.css';
import './activity.css';
import './summary.css';
import './icons.css';
import './heads.css';
import './widths.css';
import './shaping.css';
import './takenaway.css';
import './keyrevoked.css';
import './head.css';
import './following.css';
import './turning.css';
import './code.css';
import './handed.css';
import './finder.css';
import './extensions.css';
// Last, so that what it says of a narrow screen is said over the rest.
import './panelseek.css';
import './folding.css';
import './phone.css';
import './reference.css';
import './blueprintlist.css';
import './sandboxed.css';
import './sidebyside.css';
import './scenario.css';
import './preview.css';

// In the desktop app the client draws the top of the window, and everything
// else starts below it.
if (framed) document.documentElement.classList.add('framed');
sayTitles();
const root = document.getElementById('app')!;
// What the page says where it could not start, and tries once more by itself first (`index.html`).
const couldNotStart = (window as { couldNotStart?: (said: string) => void }).couldNotStart;
boot()
  .then(() => {
    render(<App />, root);
    // Started: a page that fails to open later is tried again by itself once more.
    try {
      sessionStorage.removeItem('client.started-again');
    } catch {
      /* nothing was kept */
    }
  })
  .catch((err) => {
    const said = `The daemon is not answering: ${err instanceof Error ? err.message : err}.`;
    if (couldNotStart) return couldNotStart(said);
    root.textContent = `${said} Reload to try again.`;
    root.className = 'empty';
  });
