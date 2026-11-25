// What every check's daemon is started with, beside what the check says.
//
// A daemon keeps the session of the owner its first start made (an owner
// called `Unnamed`, until a check's `register --name Ada` names her) in the
// settings of whoever started it, which for a check is no person. So a
// check's daemon is given settings of its own, under this checkout's
// `target/`, taken away as the check ends, and never those of whoever runs
// the check. A check's command line says settings of its own (`HOME`,
// `XDG_CONFIG_HOME`, beside `--url`), holds no session, and is signed in by
// proving it can read the data folder the daemon says it keeps: it needs
// nothing of these.

import { mkdirSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const settings = fileURLToPath(new URL(`../../target/check-settings/${process.pid}`, import.meta.url));
mkdirSync(settings, { recursive: true });
process.env.XDG_CONFIG_HOME = settings;
process.on('exit', () => rmSync(settings, { recursive: true, force: true }));
