//! A zip file, written here: the one file of a package for Windows.
//!
//! Windows opens a zip by itself and nothing else, so its package is one.
//! No program that writes one can be counted on wherever a package is
//! made (that system's own `tar` does on Windows; a package for it made on
//! another system for trying has neither that nor a `zip`), and the
//! tooling stands on the standard library alone. So the file is written
//! here, the same on every system, and what is in it is stored as it is:
//! compressing is a program of its own, and is not written. A package
//! kept so is more than twice what a compressed one would be to download
//! (31 MB where the same compressed came to 13, the one time it was
//! measured), and is opened by everything that opens a zip.
//!
//! What is written is the format's plain form (PKWARE's APPNOTE): each
//! file with a header before it, then the list of them all, then where
//! the list is. Names are written with `/` between their parts and said
//! to be UTF-8. Nothing here is the larger form for more than 65,535
//! files or four gigabytes: a package is nowhere near either, and one
//! that came to be is refused in a sentence.

use std::io::{self, Write};

/// The sum a zip keeps of each file (CRC-32, as IEEE 802.3 has it), by
/// which whoever unpacks one knows the file came out whole.
pub fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (at, entry) in table.iter_mut().enumerate() {
            let mut value = at as u32;
            for _ in 0..8 {
                value = if value & 1 == 1 { 0xEDB8_8320 ^ (value >> 1) } else { value >> 1 };
            }
            *entry = value;
        }
        table
    });
    !bytes.iter().fold(!0u32, |sum, byte| table[((sum ^ u32::from(*byte)) & 0xFF) as usize] ^ (sum >> 8))
}

/// A moment as a zip writes one: a date and a time of day, each in sixteen
/// bits, to two seconds. `seconds` is counted from the start of 1970, and
/// what is written is that moment as the clock in Greenwich has it: a zip
/// says no zone, and the tooling knows none. A zip has no year before
/// 1980 or after 2107, and a moment outside those is written as the
/// nearest it has.
pub fn stamp(seconds: u64) -> (u16, u16) {
    let (days, of_day) = (seconds / 86_400, seconds % 86_400);
    // The date of a count of days, by the calendar's own arithmetic (eras
    // of four hundred years, each beginning on a first of March).
    let shifted = days + 719_468;
    let (era, day_of_era) = (shifted / 146_097, shifted % 146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    if year < 1980 {
        return ((1 << 5) | 1, 0);
    }
    if year > 2107 {
        return ((127 << 9) | (12 << 5) | 31, (23 << 11) | (59 << 5) | 29);
    }
    let date = ((year - 1980) << 9) | (month << 5) | day;
    let time = ((of_day / 3_600) << 11) | ((of_day % 3_600 / 60) << 5) | (of_day % 60 / 2);
    (date as u16, time as u16)
}

/// One file as the list at the end of a zip says it.
struct Listed {
    name: String,
    sum: u32,
    size: u32,
    at: u32,
    runs: bool,
}

/// A zip being written to `out`, a file at a time.
pub struct Zip<W: Write> {
    out: W,
    written: u64,
    listed: Vec<Listed>,
    stamp: (u16, u16),
}

fn too_much(what: &str) -> io::Error {
    io::Error::other(format!(
        "{what}, which is more than the plain form of a zip holds: what is written here is that form alone"
    ))
}

/// What says the names are UTF-8 (bit 11 of a file's flags).
const NAMES_ARE_UTF8: u16 = 1 << 11;
/// The version of the format a reader needs for a file that is stored: 1.0.
const NEEDS: u16 = 10;
/// Who is said to have written it: the format's version 2.0, with a file's
/// kind and permissions said as Unix has them, so that whoever unpacks one
/// where permissions mean something has a program that can be run.
const WRITTEN_AS: u16 = (3 << 8) | 20;

impl<W: Write> Zip<W> {
    /// A zip that says every file in it was written at `seconds` from the
    /// start of 1970.
    pub fn new(out: W, seconds: u64) -> Self {
        Zip { out, written: 0, listed: Vec::new(), stamp: stamp(seconds) }
    }

    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.out.write_all(bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    /// A file of the zip: its name from the top of what is unpacked, with
    /// `/` between the parts, what is in it, and whether it is a program.
    pub fn file(&mut self, name: &str, bytes: &[u8], runs: bool) -> io::Result<()> {
        let size = u32::try_from(bytes.len()).ok().filter(|size| *size < u32::MAX);
        let size = size.ok_or_else(|| too_much(&format!("{name} is four gigabytes or more")))?;
        let at = u32::try_from(self.written).ok().filter(|at| *at < u32::MAX);
        let at = at.ok_or_else(|| too_much("the files before this one come to four gigabytes or more"))?;
        let length =
            u16::try_from(name.len()).map_err(|_| too_much(&format!("the name {name} is too long")))?;
        if self.listed.len() >= usize::from(u16::MAX) - 1 {
            return Err(too_much("there are more than 65,534 files"));
        }
        let sum = crc32(bytes);
        let (date, time) = self.stamp;
        let mut head = Vec::with_capacity(30 + name.len());
        head.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        head.extend_from_slice(&NEEDS.to_le_bytes());
        head.extend_from_slice(&NAMES_ARE_UTF8.to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes()); // stored
        head.extend_from_slice(&time.to_le_bytes());
        head.extend_from_slice(&date.to_le_bytes());
        head.extend_from_slice(&sum.to_le_bytes());
        head.extend_from_slice(&size.to_le_bytes()); // as it is kept
        head.extend_from_slice(&size.to_le_bytes()); // as it is unpacked
        head.extend_from_slice(&length.to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes()); // nothing more of it
        head.extend_from_slice(name.as_bytes());
        self.put(&head)?;
        self.put(bytes)?;
        self.listed.push(Listed { name: name.to_string(), sum, size, at, runs });
        Ok(())
    }

    /// The list of the files and where it is, which is what makes it a
    /// zip. Gives back what it was written to.
    pub fn finish(mut self) -> io::Result<W> {
        let begins = u32::try_from(self.written).ok().filter(|at| *at < u32::MAX);
        let begins = begins.ok_or_else(|| too_much("the files come to four gigabytes or more"))?;
        let (date, time) = self.stamp;
        let mut list = Vec::new();
        for file in &self.listed {
            // A plain file, and what whoever owns it, their group and
            // everybody else may do with it.
            let kind: u32 = 0o100_000 | if file.runs { 0o755 } else { 0o644 };
            list.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            list.extend_from_slice(&WRITTEN_AS.to_le_bytes());
            list.extend_from_slice(&NEEDS.to_le_bytes());
            list.extend_from_slice(&NAMES_ARE_UTF8.to_le_bytes());
            list.extend_from_slice(&0u16.to_le_bytes()); // stored
            list.extend_from_slice(&time.to_le_bytes());
            list.extend_from_slice(&date.to_le_bytes());
            list.extend_from_slice(&file.sum.to_le_bytes());
            list.extend_from_slice(&file.size.to_le_bytes());
            list.extend_from_slice(&file.size.to_le_bytes());
            list.extend_from_slice(&(file.name.len() as u16).to_le_bytes());
            list.extend_from_slice(&0u16.to_le_bytes()); // nothing more of it
            list.extend_from_slice(&0u16.to_le_bytes()); // nothing said of it
            list.extend_from_slice(&0u16.to_le_bytes()); // all in this one file
            list.extend_from_slice(&0u16.to_le_bytes()); // not said to be text
            list.extend_from_slice(&(kind << 16).to_le_bytes());
            list.extend_from_slice(&file.at.to_le_bytes());
            list.extend_from_slice(file.name.as_bytes());
        }
        let length = u32::try_from(list.len()).map_err(|_| too_much("the list of files is too long"))?;
        let count = self.listed.len() as u16;
        list.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        list.extend_from_slice(&0u16.to_le_bytes()); // this file is the whole of it
        list.extend_from_slice(&0u16.to_le_bytes());
        list.extend_from_slice(&count.to_le_bytes());
        list.extend_from_slice(&count.to_le_bytes());
        list.extend_from_slice(&length.to_le_bytes());
        list.extend_from_slice(&begins.to_le_bytes());
        list.extend_from_slice(&0u16.to_le_bytes()); // nothing said of the whole
        self.put(&list)?;
        self.out.flush()?;
        Ok(self.out)
    }
}
