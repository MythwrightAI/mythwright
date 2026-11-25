# Reporting a security problem

Report it privately, through this repository's own private reporting: on
its page, **Security**, then **Report a vulnerability**. Please do not open
a public issue for it, and do not send it to anyone's address.

Say what you did, what happened and what you expected, with the version
(`mw version`) and the system. If you have a way to show it, include it.

What the product protects and what it does not is written down, so that a
report can tell a hole from a limit that is already known:

- [docs/protected.md](docs/protected.md), *What is protected, and what is not*.
- [docs/architecture.md](docs/architecture.md), *Identity*, with the reason
  for each.
- [docs/roadmap.md](docs/roadmap.md), for what is known and not built: an
  agent's program is not held in on the machine, and the client and the
  command line are not encrypted by the daemon.

This is a preview. A fix goes into the next version; there are no older
versions that are kept up.
