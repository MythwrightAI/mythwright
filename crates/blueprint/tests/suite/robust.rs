//! Texts nobody wrote. Whatever the reader is handed, it answers; whatever
//! it reads, the formatter lays out to a text that reads and means the
//! same; and one laying out settles it.
//!
//! The texts are made here, the same every run: every text that ships with
//! a piece cut off, taken out, said twice, moved, or a mark of the
//! language's put in the middle of it; a few of its lines alone; words of
//! the language in no order; and bytes that mean nothing. Nothing outside
//! makes them. A failure says which text, and running the test again
//! meets it again.

use std::panic::{AssertUnwindSafe, catch_unwind};

use blueprint::vocabulary::SHAPES;
use blueprint::{Settings, Unformatted, format, meaning};

use crate::check;
use crate::format::blueprints;

/// Numbers that look like chance and are the same every time.
struct Dice(u64);

impl Dice {
    fn roll(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, most: usize) -> usize {
        if most == 0 { 0 } else { (self.roll() % most as u64) as usize }
    }

    fn of<'a>(&mut self, some: &[&'a str]) -> &'a str {
        some[self.below(some.len())]
    }
}

/// What the language is written with, besides its words.
const MARKS: &[&str] = &[
    "${",
    "}",
    "${ }",
    "${ steps. }",
    "${ a ? b : }",
    "{",
    "[",
    "]",
    ":",
    ": ",
    ",",
    "- ",
    "#",
    "\"",
    "'",
    "\n",
    "\t",
    "  ",
    " ",
    "|",
    ">",
    "&",
    "*",
    "!",
    "%",
    "@",
    "`",
    "\\",
    "---",
    "...",
    "\r\n",
    "\u{0}",
    "\u{feff}",
    "é",
    "~",
    "?",
    "1",
    "1m",
    "true",
    "null",
    "a.b",
    "||",
    "&&",
    "==",
    "(",
    ")",
    "<<",
    "=",
    "-1",
    "1e9",
    "0x1f",
];

/// The few bytes most of the language is written with.
const NARROW: &[u8] = b" \n:-{}[]$#\"'ab1,|>";

/// A place in `text` that is the start of a character.
fn somewhere(text: &str, dice: &mut Dice) -> usize {
    let mut at = dice.below(text.len() + 1);
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// `text` with one thing done to it that nobody meant.
fn spoiled(text: &str, dice: &mut Dice) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let line = |dice: &mut Dice| dice.below(lines.len().max(1));
    match dice.below(8) {
        0 => text[..somewhere(text, dice)].to_string(),
        1 => {
            let (one, other) = (somewhere(text, dice), somewhere(text, dice));
            let (from, to) = (one.min(other), one.max(other).min(one.min(other) + 60));
            let to = (to..=text.len()).find(|at| text.is_char_boundary(*at)).unwrap_or(text.len());
            format!("{}{}", &text[..from], &text[to..])
        }
        2 => {
            let at = somewhere(text, dice);
            format!("{}{}{}", &text[..at], dice.of(MARKS), &text[at..])
        }
        3 => {
            let twice = line(dice);
            lines
                .iter()
                .enumerate()
                .flat_map(|(at, line)| [*line].repeat(1 + usize::from(at == twice)))
                .collect()
        }
        4 => {
            let (one, other) = (line(dice), line(dice));
            let mut moved = lines.clone();
            if !moved.is_empty() {
                moved.swap(one, other);
            }
            moved.concat()
        }
        5 => {
            let pushed = line(dice);
            let by = dice.of(&[" ", "  ", "   ", "\t", "        "]);
            lines
                .iter()
                .enumerate()
                .map(|(at, line)| format!("{}{line}", if at == pushed { by } else { "" }))
                .collect()
        }
        6 => {
            let pulled = line(dice);
            lines
                .iter()
                .enumerate()
                .map(|(at, line)| if at == pulled { line.trim_start_matches(' ') } else { line })
                .collect()
        }
        _ => {
            // A few of its lines, as a text of their own.
            let from = line(dice);
            let to = (from + 2 + dice.below(40)).min(lines.len());
            lines[from.min(to)..to].concat()
        }
    }
}

/// Every text made up, each with a word for what it is.
fn made_up() -> Vec<(String, String)> {
    let mut dice = Dice(0x9E37_79B9_7F4A_7C15);
    let mut texts = Vec::new();
    let mut shipped = Vec::new();
    for folder in ["blueprints", "examples", "crates/blueprint/tests/fixtures", "crates/studio/src"] {
        for file in blueprints(folder) {
            shipped.push((file.display().to_string(), std::fs::read_to_string(&file).unwrap()));
        }
    }
    assert!(shipped.len() >= 15, "the blueprints that ship were not found: only {}", shipped.len());
    for (name, text) in &shipped {
        // Fewer of a long text: it costs as much again each time.
        let times = if text.len() > 20_000 { 4 } else { 14 };
        for n in 0..times {
            texts.push((format!("{name}, spoiled ({n})"), spoiled(text, &mut dice)));
        }
        let once = spoiled(text, &mut dice);
        texts.push((format!("{name}, spoiled twice"), spoiled(&once, &mut dice)));
    }
    let words: Vec<&str> = SHAPES.iter().flat_map(|shape| shape.names()).collect();
    for n in 0..250 {
        let mut text = String::new();
        for _ in 0..dice.below(40) {
            text.push_str(if dice.below(3) == 0 { dice.of(MARKS) } else { dice.of(&words) });
            text.push_str(dice.of(&["", "", " ", ": ", ":\n  ", "\n", "\n  - ", "\n    "]));
        }
        texts.push((format!("words in no order ({n})"), text));
    }
    for n in 0..250 {
        let bytes: Vec<u8> = (0..dice.below(120)).map(|_| dice.roll() as u8).collect();
        texts.push((format!("bytes ({n})"), String::from_utf8_lossy(&bytes).into_owned()));
        let narrow: Vec<u8> = (0..dice.below(120)).map(|_| NARROW[dice.below(NARROW.len())]).collect();
        texts.push((format!("marks ({n})"), String::from_utf8_lossy(&narrow).into_owned()));
    }
    texts
}

/// The ways a text is laid out that a project may ask for.
fn every_way() -> Vec<Settings> {
    ["", "width: 60\nindent: 4\nexpressions: spaced", "one_line: never", "order: vocabulary"]
        .iter()
        .map(|said| Settings::read(said).expect("the settings read"))
        .collect()
}

#[test]
fn the_reader_and_the_formatter_answer_whatever_they_are_handed() {
    let ways = every_way();
    let texts = made_up();
    assert!(texts.len() > 1200, "only {} texts were made", texts.len());
    for (what, text) in &texts {
        let answered = catch_unwind(AssertUnwindSafe(|| {
            let _ = check(text);
            for way in &ways {
                let _ = format(text, way);
            }
        }));
        assert!(
            answered.is_ok(),
            "{what} was not answered: reading or laying it out panicked. It is:\n{text}"
        );
    }
}

#[test]
fn a_text_that_reads_is_laid_out_to_one_that_reads_and_means_the_same_and_once_is_enough() {
    let ways = every_way();
    let (mut read, mut checked) = (0, 0);
    for (what, text) in made_up() {
        let Some(meant) = meaning(&text) else { continue };
        read += 1;
        let sound = !check(&text).has_errors();
        for (way, settings) in ways.iter().enumerate() {
            let laid = match format(&text, settings) {
                Ok(laid) => laid,
                Err(Unformatted::Fault) => panic!(
                    "{what} reads, and laid out (way {way}) it would have meant something else. It is:\n{text}"
                ),
                Err(Unformatted::Unreadable(err)) => {
                    panic!("{what} reads and was said not to: {}", err.message)
                }
            };
            // In the order it was written it says the same things in the
            // same order; in the language's order, the formatter holds it
            // to saying the same things.
            if way < 3 {
                assert_eq!(
                    meaning(&laid).as_ref(),
                    Some(&meant),
                    "{what} (way {way}) is:\n{text}\nand came to:\n{laid}"
                );
            }
            let again =
                format(&laid, settings).unwrap_or_else(|left| panic!("{what}: {}\n{laid}", left.message()));
            assert_eq!(again, laid, "{what} (way {way}) is not settled by one laying out. It is:\n{text}");
            if sound {
                let after = check(&laid);
                let wrong: Vec<_> = after.errors().map(|found| found.message.clone()).collect();
                assert!(
                    wrong.is_empty(),
                    "{what} (way {way}) checked, and laid out does not: {wrong:?}\n{laid}"
                );
            }
        }
        checked += usize::from(sound);
    }
    assert!(read > 150, "only {read} of the texts made up read at all: they hold nothing");
    assert!(checked > 20, "only {checked} of the texts made up are blueprints with nothing wrong");
}
