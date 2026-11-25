/** Names a workspace might be given, shown one at a time where none has been typed. They say by example what a name is; none is ever filled in for anybody. */
export const NAMES = [
  'Operation Sandwich',
  'The Department of Vibes',
  'Snacks & Infrastructure',
  'Secret Volcano Lair',
  'Goblin Accounting',
  'Rubber Duck Consultancy',
  'Bread Research Division',
  'Definitely Not a Side Project',
  'Moon Base, Phase One',
  'The Spreadsheet Graveyard',
  'Tuesday Experiments',
  'Raccoon Logistics',
];

/** Which name a showing begins at, from a throw between nought and one: each time the dialog is opened it begins somewhere else. */
export const beginsAt = (thrown: number, names = NAMES.length) => Math.max(0, Math.min(names - 1, Math.floor(thrown * names)));
