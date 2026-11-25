//! The sum beside a package, held to the examples of the standard that
//! says what a SHA-256 is.

use xtask::sha256::{self, Sha256};

/// What the standard's own notes give for each message (FIPS 180-4, and
/// the examples NIST publishes with it).
const EXAMPLES: [(&str, &str); 4] = [
    ("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    ("abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
    (
        "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    ),
    (
        "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    ),
];

/// A million of the letter `a`, the long one of those examples.
const A_MILLION_AS: &str = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";

#[test]
fn the_sum_of_each_of_the_standards_examples_is_the_one_the_standard_gives() {
    for (message, sum) in EXAMPLES {
        assert_eq!(sha256::hex(message.as_bytes()), sum, "of `{message}`");
    }
    assert_eq!(sha256::hex(&vec![b'a'; 1_000_000]), A_MILLION_AS);
}

#[test]
fn a_sum_is_the_same_however_what_is_summed_is_handed_over() {
    let whole = vec![b'a'; 1_000_000];
    // Pieces that end before a block, on one and after one, and one byte
    // at a time across the first blocks.
    for piece in [1, 3, 55, 56, 63, 64, 65, 127, 128, 1000] {
        let mut sum = Sha256::new();
        for part in whole.chunks(piece).take(if piece == 1 { 200 } else { usize::MAX }) {
            sum.update(part);
        }
        if piece == 1 {
            sum.update(&whole[200..]);
        }
        assert_eq!(sum.finish(), A_MILLION_AS, "in pieces of {piece}");
    }
    // Read from something that gives it out, as a file does.
    assert_eq!(sha256::of(&whole[..]).unwrap(), A_MILLION_AS);
    for (message, sum) in EXAMPLES {
        assert_eq!(sha256::of(message.as_bytes()).unwrap(), sum);
    }
}

#[test]
fn the_line_beside_a_file_is_the_one_the_systems_own_programs_read() {
    assert_eq!(
        sha256::line(EXAMPLES[1].1, "sample-1.2.3-windows-x86_64.zip"),
        format!("{}  sample-1.2.3-windows-x86_64.zip\n", EXAMPLES[1].1)
    );
}
