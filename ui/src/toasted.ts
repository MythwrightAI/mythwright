// What is said in the corner of the page, as a function of what is showing: a
// sentence said again while it is showing takes the place of the one there,
// so that the same words are never there twice. Two of them tell nobody more,
// and of two that each offer Undo the newest is what was done last.

export interface Toast {
  id: number;
  text: string;
  bad: boolean;
  undo?: () => void;
}

/** What is showing once `said` is said too: what was there, but for what says the same, and then it. */
export function sayToast(showing: Toast[], said: Toast): Toast[] {
  return [...showing.filter((entry) => entry.text !== said.text), said];
}
