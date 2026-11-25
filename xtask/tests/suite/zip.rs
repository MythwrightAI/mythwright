//! The one file of a package for Windows: a zip written here, read back
//! by what the format's own notes say one is.

use xtask::zip::{self, Zip};

/// One file of a zip as it was read back.
#[derive(Debug, PartialEq)]
pub struct Unpacked {
    pub name: String,
    pub bytes: Vec<u8>,
    /// Whether it says it is a program, as Unix says that of a file.
    pub runs: bool,
}

fn two(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn four(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Reads a zip as a program that unpacks one does: from the end, where
/// the list of its files is said to be, through the list, to each file,
/// holding everything the two places say of a file to each other and
/// what is in it to its sum. It reads the plain form, stored, and fails
/// the test on anything else.
pub fn unpacked(zip: &[u8]) -> Vec<Unpacked> {
    let end = zip.len() - 22;
    assert_eq!(four(zip, end), 0x0605_4b50, "where the list is, is said at the very end");
    let (count, length, begins) =
        (two(zip, end + 10), four(zip, end + 12) as usize, four(zip, end + 16) as usize);
    assert_eq!(two(zip, end + 8), count);
    assert_eq!(begins + length, end, "the list ends where the end begins");
    let mut files = Vec::new();
    let mut at = begins;
    let mut next = 0;
    for _ in 0..count {
        assert_eq!(four(zip, at), 0x0201_4b50, "a line of the list");
        let (flags, method) = (two(zip, at + 8), two(zip, at + 10));
        let (sum, kept, size) =
            (four(zip, at + 16), four(zip, at + 20) as usize, four(zip, at + 24) as usize);
        let (named, more, said) = (two(zip, at + 28), two(zip, at + 30), two(zip, at + 32));
        let (kind, local) = (four(zip, at + 38), four(zip, at + 42) as usize);
        let name = std::str::from_utf8(&zip[at + 46..at + 46 + named]).unwrap().to_string();
        assert_eq!((method, kept, more, said), (0, size, 0, 0), "{name} is stored as it is");
        assert_eq!(flags, 1 << 11, "{name} is said to be UTF-8, and nothing else is said");
        assert_eq!(two(zip, at + 4) >> 8, 3, "permissions are said as Unix says them");
        assert_eq!(kind >> 28, 0o10, "{name} is a plain file");
        // The file itself, where the list says it is: one after another
        // with nothing between.
        assert_eq!(local, next, "{name} begins where the one before it ended");
        assert_eq!(four(zip, local), 0x0403_4b50, "{name} has its own header");
        assert_eq!(&zip[local + 4..local + 26], &zip[at + 6..at + 28], "{name}: both say the same");
        assert_eq!(two(zip, local + 26), named);
        assert_eq!(two(zip, local + 28), 0);
        assert_eq!(&zip[local + 30..local + 30 + named], name.as_bytes());
        let bytes = zip[local + 30 + named..local + 30 + named + size].to_vec();
        assert_eq!(zip::crc32(&bytes), sum, "{name} came out whole");
        next = local + 30 + named + size;
        files.push(Unpacked { name, bytes, runs: (kind >> 16) & 0o111 == 0o111 });
        at += 46 + named;
    }
    assert_eq!(at, end);
    assert_eq!(next, begins, "the list begins where the last file ended");
    files
}

#[test]
fn the_sum_a_zip_keeps_of_a_file_is_the_one_the_format_names() {
    // The check value every description of CRC-32 gives.
    assert_eq!(zip::crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(zip::crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    assert_eq!(zip::crc32(b""), 0);
}

#[test]
fn a_moment_is_written_as_a_zip_writes_one() {
    let cases: [(u64, &str, (u16, u16)); 7] = [
        (315_532_800, "the first moment a zip has: 1980-01-01 00:00:00", (33, 0)),
        (946_684_799, "1999-12-31 23:59:59", (10_143, 49_021)),
        (
            951_912_001,
            "2000-03-01 12:00:01, the day after a leap day in a year of a century",
            (10_337, 24_576),
        ),
        (1_709_251_199, "2024-02-29 23:59:59, a leap day", (22_621, 49_021)),
        (1_795_064_782, "2026-11-19 05:06:22", (23_923, 10_443)),
        (4_354_819_198, "2107-12-31 23:59:58, the last a zip has", (65_439, 49_021)),
        (0, "1970, before a zip has any: the first it has", (33, 0)),
    ];
    for (seconds, what, written) in cases {
        assert_eq!(zip::stamp(seconds), written, "{what}");
    }
    assert_eq!(zip::stamp(4_354_819_200), (65_439, 49_021), "2108, after the last: the last it has");
}

#[test]
fn a_zip_written_here_is_read_back_whole_file_by_file_in_the_order_given() {
    let large: Vec<u8> = (0..200_000u32).map(|at| (at % 251) as u8).collect();
    let files: [(&str, &[u8], bool); 5] = [
        ("sample/README.md", b"how to start it\n", false),
        ("sample/empty", b"", false),
        ("sample/sm.exe", &large, true),
        ("sample/ui/assets/app-ONE.js", b"part of one", false),
        ("sample/extensions/caf\u{e9}/notes.txt", "caf\u{e9}\n".as_bytes(), false),
    ];
    let mut zip = Zip::new(Vec::new(), 1_795_064_782);
    for (name, bytes, runs) in files {
        zip.file(name, bytes, runs).unwrap();
    }
    let written = zip.finish().unwrap();
    let read = unpacked(&written);
    assert_eq!(read.len(), files.len());
    for (read, (name, bytes, runs)) in read.iter().zip(files) {
        assert_eq!((read.name.as_str(), &read.bytes[..], read.runs), (name, bytes, runs));
    }
    // Every file says the moment the zip was told, in both places.
    assert_eq!(&written[10..14], &[0xCB, 0x28, 0x73, 0x5D], "10443 and 23923, low byte first");
    // And a zip of nothing is still one.
    assert!(unpacked(&Zip::new(Vec::new(), 0).finish().unwrap()).is_empty());
}
