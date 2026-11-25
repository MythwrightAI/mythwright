//! The words a building block answers with.
//!
//! A block that decides something says what it came to as one word of a
//! few (`came`), and the blueprint that asked it does what the word says.
//! Which words there are is an enum in the block's own code, made with
//! [`came!`](crate::came), so that the block cannot answer a word that is
//! not listed, and its tests can ask for every one that is: a table of
//! cases that leaves a word unreached fails, by `ALL`.

/// Declares the words a block can come to: an enum of them, each written
/// `Variant => "word"` with the word a blueprint reads; `ALL` of them in the
/// order written; `word`, and `words` for a sentence that lists them.
#[macro_export]
macro_rules! came {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$of:meta])* $variant:ident => $word:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $name {
            $($(#[$of])* $variant),+
        }

        impl $name {
            /// Every word it can come to, in the order they are declared.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The word a blueprint reads.
            pub const fn word(self) -> &'static str {
                match self {
                    $($name::$variant => $word),+
                }
            }

            /// The words, as a sentence lists them: `` `a`, `b` or `c` ``.
            pub fn words() -> String {
                let words: Vec<String> = Self::ALL.iter().map(|one| format!("`{}`", one.word())).collect();
                match words.split_last() {
                    Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
                    _ => words.concat(),
                }
            }
        }
    };
}
