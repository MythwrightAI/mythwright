//! The handshake and the frames: what two installations that hold each
//! other's keys can say, and what nobody else can do to it.

use std::path::PathBuf;

use gateway::own::{Keys, Own, SECRET_FILE};
use gateway::sealed::{self, Kind, MAX_FRAME, Session};

/// A folder of a test's own, under the build's target directory, and gone
/// when the test has done with it: a suite that left one behind each time
/// it ran had left thousands.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl AsRef<std::path::Path> for Scratch {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("sealed-{name}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

fn keys(name: &str) -> Keys {
    Own::open(&scratch(name)).unwrap().link().unwrap()
}

/// A device and its hub that have shaken hands: each one's end of the session.
fn shaken() -> (Session, Session) {
    let (device, hub) = (keys("device"), keys("hub"));
    let offer = sealed::offer(&device, &hub.public, b"may I come in").unwrap();
    let heard = sealed::hear(&hub, &offer.message).unwrap();
    assert_eq!(heard.from, device.public, "the hub learns which key the device proved");
    assert_eq!(heard.payload, b"may I come in");
    let (hubs, answer) = heard.answer(b"you may").unwrap();
    let (devices, said) = offer.accept(&answer).unwrap();
    assert_eq!(said, b"you may");
    (devices, hubs)
}

fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn an_installation_makes_its_secret_once_keeps_it_to_itself_and_is_another_in_another_folder() {
    let dir = scratch("own");
    let first = Own::open(&dir).unwrap();
    let again = Own::open(&dir).unwrap();
    assert_eq!(
        first.link().unwrap().public,
        again.link().unwrap().public,
        "the same installation each start"
    );
    let elsewhere = Own::open(&scratch("own-other")).unwrap();
    assert_ne!(first.link().unwrap().public, elsewhere.link().unwrap().public);

    let written = std::fs::read_to_string(dir.join(SECRET_FILE)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(SECRET_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "its owner's alone");
    }
    // Asked to print itself, it prints nothing of what it holds; and the
    // key that is shown is not the secret, nor made of it in a way that shows.
    assert!(!format!("{first:?}").contains(written.trim()));
    let shown = gateway::own::fingerprint(&first.link().unwrap().public);
    assert_eq!(shown.len(), 24, "{shown}");
    assert!(!written.contains(&shown.replace('-', "")));

    // One that cannot be read is said to be so, and is not made again:
    // that would be another installation in this one's place.
    std::fs::write(dir.join(SECRET_FILE), "not what was written\n").unwrap();
    let damaged = Own::open(&dir).unwrap_err();
    assert!(damaged.message.contains("does not hold this installation's secret"), "{}", damaged.message);
    assert_eq!(std::fs::read_to_string(dir.join(SECRET_FILE)).unwrap(), "not what was written\n");
}

#[test]
fn what_is_sealed_opens_for_the_other_side_and_shows_nothing_of_itself() {
    let (device, hub) = shaken();
    let said = b"the till is in the back room".to_vec();
    let (_, frame) = device.seal(Kind::Report, &said).unwrap();
    assert!(!holds(&frame, b"the till"), "what crosses does not show what was said");
    assert_eq!(hub.open_next(Kind::Report, &frame).unwrap().1, said);
    let (_, back) = hub.seal(Kind::Command, b"count it").unwrap();
    assert_eq!(device.open_next(Kind::Command, &back).unwrap().1, b"count it");

    // Something longer than one message of the handshake's kind can be is
    // several records, and comes out whole; nothing is as long as that.
    let long: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
    let (_, frame) = device.seal(Kind::Report, &long).unwrap();
    assert_eq!(hub.open_next(Kind::Report, &frame).unwrap().1, long);
    let (_, empty) = device.seal(Kind::Open, &[]).unwrap();
    assert_eq!(hub.open_next(Kind::Open, &empty).unwrap().1, Vec::<u8>::new());
    let too_long = device.seal(Kind::Report, &vec![0u8; MAX_FRAME + 1]).unwrap_err();
    assert!(too_long.0.contains("the most that is said at once"), "{too_long}");
}

#[test]
fn a_frame_changed_by_one_byte_anywhere_is_refused() {
    let (device, hub) = shaken();
    let (_, frame) = device.seal(Kind::Report, b"a report of some length, to have a middle").unwrap();
    for at in [0, 7, 8, 9, 10, frame.len() / 2, frame.len() - 1] {
        let mut changed = frame.clone();
        changed[at] ^= 1;
        assert!(hub.open_next(Kind::Report, &changed).is_err(), "byte {at} was changed and it was taken");
    }
    // Cut short, or with more after it, it is not the frame that was sealed.
    assert!(hub.open_next(Kind::Report, &frame[..frame.len() - 1]).is_err());
    assert!(hub.open_next(Kind::Report, &[frame.clone(), vec![0]].concat()).is_err());
    assert!(hub.open_next(Kind::Report, &[]).is_err());
    // As it was sealed, it still opens: nothing above was taken.
    assert!(hub.open_next(Kind::Report, &frame).is_ok());
}

#[test]
fn a_frame_is_taken_once_and_never_after_one_sealed_later() {
    let (device, hub) = shaken();
    let (_, first) = device.seal(Kind::Report, b"first").unwrap();
    let (_, second) = device.seal(Kind::Report, b"second").unwrap();
    assert_eq!(hub.open_next(Kind::Report, &second).unwrap().1, b"second");
    let again = hub.open_next(Kind::Report, &second).unwrap_err();
    assert!(again.0.contains("said before"), "{again}");
    assert!(hub.open_next(Kind::Report, &first).is_err(), "overtaken, it is too late");
    let (_, third) = device.seal(Kind::Report, b"third").unwrap();
    assert_eq!(hub.open_next(Kind::Report, &third).unwrap().1, b"third");
}

#[test]
fn every_frame_a_session_seals_has_numbers_no_other_has() {
    let (device, _hub) = shaken();
    // Sealed from many places at once, as reports and answers are.
    let device = std::sync::Arc::new(device);
    let sealers: Vec<_> = (0..8)
        .map(|_| {
            let device = device.clone();
            std::thread::spawn(move || {
                (0..50).map(|_| device.seal(Kind::Report, &[7u8; 70_000]).unwrap().0).collect::<Vec<u64>>()
            })
        })
        .collect();
    let mut firsts: Vec<u64> = sealers.into_iter().flat_map(|sealer| sealer.join().unwrap()).collect();
    firsts.sort_unstable();
    // Each frame was two records; no two frames began within two of each other.
    assert!(firsts.windows(2).all(|pair| pair[1] - pair[0] >= 2), "{firsts:?}");
}

#[test]
fn a_frame_is_taken_only_where_its_kind_is_and_only_whole() {
    let (device, hub) = shaken();
    let (_, report) = device.seal(Kind::Report, b"a report").unwrap();
    let misplaced = hub.open_next(Kind::Open, &report).unwrap_err();
    assert!(misplaced.0.contains("another kind"), "{misplaced}");

    // Three records: none can be left out, said twice, or moved.
    let long = vec![3u8; 150_000];
    let (_, frame) = device.seal(Kind::Report, &long).unwrap();
    let record = |from: usize| {
        let length = u16::from_be_bytes([frame[from + 8], frame[from + 9]]) as usize;
        from + 10 + length
    };
    let (one, two) = (record(0), record(record(0)));
    assert_eq!(record(two), frame.len(), "three records");
    let (a, b, c) = (&frame[..one], &frame[one..two], &frame[two..]);
    for (what, wrong) in [
        ("the last left out", [a, b].concat()),
        ("the middle left out", [a, c].concat()),
        ("the first left out", [b, c].concat()),
        ("two changed places", [a, c, b].concat()),
        ("one said twice", [a, b, b, c].concat()),
    ] {
        assert!(hub.open_next(Kind::Report, &wrong).is_err(), "{what}, and it was taken");
    }
    assert_eq!(hub.open_next(Kind::Report, &frame).unwrap().1, long);
}

#[test]
fn a_frame_of_another_session_does_not_open_in_this_one() {
    let (device, _hub) = shaken();
    let (_other_device, other_hub) = shaken();
    let (_, frame) = device.seal(Kind::Report, b"for my own hub").unwrap();
    let refused = other_hub.open_next(Kind::Report, &frame).unwrap_err();
    assert!(refused.0.contains("not of this session"), "{refused}");
    // Nor does what one side sealed open as if the other had said it.
    assert!(device.open_next(Kind::Report, &frame).is_err());
}

#[test]
fn whoever_answers_without_the_hubs_key_is_not_the_hub() {
    let (device, hub, impostor) = (keys("device"), keys("hub"), keys("impostor"));
    let offer = sealed::offer(&device, &hub.public, b"for the hub alone").unwrap();
    assert!(!holds(&offer.message, b"for the hub alone"));
    assert!(!holds(&offer.message, &device.public), "nor which device is asking");
    // It cannot read the first message, which was made for the hub's key...
    assert!(sealed::hear(&impostor, &offer.message).is_err());
    // ...and an answer it makes to some message it can read is no answer to this one.
    let astray = sealed::offer(&device, &impostor.public, b"").unwrap();
    let (_, its_answer) = sealed::hear(&impostor, &astray.message).unwrap().answer(b"I am the hub").unwrap();
    assert!(offer.accept(&its_answer).is_err());
    // A first message changed on the way is not one at all.
    let offer = sealed::offer(&device, &hub.public, b"again").unwrap();
    let mut changed = offer.message.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    assert!(sealed::hear(&hub, &changed).is_err());
}

#[test]
fn a_record_that_continues_a_frame_is_not_taken_for_a_frame_of_its_own() {
    let (device, hub) = shaken();
    // Long enough to be two records. The second, handed in by itself, says
    // (sealed with it) which frame it is of, and that is not itself.
    let long = vec![7u8; 100_000];
    let (_, frame) = device.seal(Kind::Report, &long).unwrap();
    let first_length = u16::from_be_bytes([frame[8], frame[9]]) as usize;
    let second = &frame[10 + first_length..];
    assert!(!second.is_empty(), "there is a second record");
    let refused = hub.open_any(Kind::Report, second).unwrap_err();
    assert!(refused.to_string().contains("not of the frame it was handed in with"), "{refused}");
    // The whole frame, as it was sealed, is taken.
    assert_eq!(hub.open_next(Kind::Report, &frame).unwrap().1, long);
}

#[cfg(unix)]
#[test]
fn a_secret_that_others_on_the_machine_may_read_is_not_used() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("own-open");
    Own::open(&dir).unwrap();
    let file = dir.join(SECRET_FILE);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let said = Own::open(&dir).unwrap_err().message;
    assert!(said.contains("can be read by others") && said.contains("chmod 600"), "{said}");
    // Its owner's alone again, it is the installation it was.
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(Own::open(&dir).is_ok());
}

#[test]
fn of_two_starts_at_once_in_one_folder_both_hold_the_one_secret() {
    let dir = scratch("own-twice");
    let starts: Vec<_> = (0..8)
        .map(|_| {
            let dir = dir.to_path_buf();
            std::thread::spawn(move || Own::open(&dir).unwrap().link().unwrap().public)
        })
        .collect();
    let keys: Vec<[u8; 32]> = starts.into_iter().map(|start| start.join().unwrap()).collect();
    assert!(keys.iter().all(|key| *key == keys[0]), "every start came to the same installation");
    assert_eq!(Own::open(&dir).unwrap().link().unwrap().public, keys[0]);
}
