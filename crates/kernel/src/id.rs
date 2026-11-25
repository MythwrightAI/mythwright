//! Identifiers and secrets.

const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// A new id such as `run_01jz8k3m9qx4t7wb`: a prefix naming the kind, then
/// time and randomness, so ids sort by creation time and read well in logs.
/// Those made in one millisecond by this program follow one another, so
/// they sort by creation there too.
pub fn new(prefix: &str, now_ms: i64) -> String {
    let mut out = String::with_capacity(prefix.len() + 17);
    out.push_str(prefix);
    out.push('_');
    // 45 bits of milliseconds lasts until the year 3084.
    for shift in (0..9).rev() {
        out.push(ALPHABET[((now_ms >> (shift * 5)) & 31) as usize] as char);
    }
    let tail = after();
    for shift in (0..7).rev() {
        out.push(ALPHABET[((tail >> (shift * 5)) & 31) as usize] as char);
    }
    out
}

/// The 35 bits after the time: one more than the last id this program
/// made, from a random start.
///
/// Ids are what rows are kept by (an instance, a run and its steps, a
/// turn), and a row is put where its id falls among its table's pages.
/// Random after the millisecond, each was put on a page of its own among
/// those already full, and how many pages a write touched went by where
/// it fell and where those pages split: a clock that stands still, as a
/// test's does, made every id of a factory one millisecond's, and what
/// a card wrote to its project's log came to as many as 140 pages more
/// in one run than in another, and some 7 in 100 more than with ids
/// that follow. Following, each is put after the last, where
/// the last page is. It is one run for the whole program, and not one
/// for each millisecond: a test's suite has many factories on many
/// clocks in one program, and one kept for each millisecond apart was
/// begun again at random whenever the others' had pushed it out. They
/// are no less told apart: what one program makes is a run from a random
/// start, and two programs' runs meet in one millisecond only as two
/// random ids would.
fn after() -> u64 {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: OnceLock<AtomicU64> = OnceLock::new();
    const ALL: u64 = (1 << 35) - 1;
    let start = || {
        let fresh = random_bytes::<7>().iter().fold(0u64, |tail, byte| (tail << 5) | u64::from(byte & 31));
        AtomicU64::new(fresh)
    };
    NEXT.get_or_init(start).fetch_add(1, Ordering::Relaxed) & ALL
}

/// A bearer secret with 256 bits of entropy, as lowercase base32.
pub fn secret() -> String {
    random_bytes::<32>()
        .chunks(5)
        .flat_map(|chunk| {
            let mut buffer = [0u8; 5];
            buffer[..chunk.len()].copy_from_slice(chunk);
            let bits = u64::from_be_bytes([0, 0, 0, buffer[0], buffer[1], buffer[2], buffer[3], buffer[4]]);
            (0..8).rev().map(move |index| ALPHABET[((bits >> (index * 5)) & 31) as usize] as char)
        })
        .take(52)
        .collect()
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    // The system random source failing is not something the program can work
    // around, and continuing with predictable ids or secrets would be worse.
    getrandom::fill(&mut bytes).expect("the system random source is unavailable");
    bytes
}

/// A uniformly random number in `0..bound`, for jitter. Not for secrets.
pub fn random_below(bound: u64) -> u64 {
    if bound == 0 {
        return 0;
    }
    u64::from_le_bytes(random_bytes::<8>()) % bound
}
