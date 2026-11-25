// What a pill in a blueprint's head means, said when it is pointed at.
//
// The pills are a word each ("checks", "running"), which is all the head
// has room for; what each one rests on is a sentence. Functions of plain
// values, so that what is said is held by cases and not only seen.

/** What the pill that says how the text checks means. */
export function checkSays(problems: number): string {
  if (problems === 0) {
    return 'The text as it stands reads, and every action, event and name in it is one this factory knows. It can be applied.';
  }
  const what = problems === 1 ? 'One thing is' : `${problems} things are`;
  return `${what} wrong with the text as it stands, each marked where it is written. It cannot be applied until it checks.`;
}

/** What the pill says while the text is being checked. */
export function checkingSays(): string {
  return 'The text is being checked against what this factory knows.';
}

/** The word a running instance's pill says. */
export function instanceWord(status: string): string {
  return status === 'active' ? 'running' : status;
}

/** What the pill of what was applied from this blueprint means. */
export function instanceSays(status: string): string {
  switch (status) {
    case 'starting':
      return 'It has been applied and its steps are running for the first time.';
    case 'active':
      return 'It has been applied and is running: what it hears, it handles.';
    case 'done':
      return 'It was applied, did its steps and has ended by itself.';
    case 'failed':
      return 'It was applied and failed. Why is in its runs.';
    case 'stopped':
      return 'It was applied and has been stopped. It hears nothing until it is applied again.';
    default:
      return `It is ${status}.`;
  }
}
