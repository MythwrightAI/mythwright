//! Where somebody has read to in what the factory says, as text.
//!
//! Whoever follows events is handed a place to go on from (`cursor` of
//! what `/api/events/recent` and `/api/fleet` answer, the `id` of each
//! frame of the stream) and hands it back as it was (`since`, `after`,
//! `Last-Event-ID`). It is text, and what is in it is this program's to
//! know: a place in each journal the factory keeps, its own and each
//! project's (`journal::Places`). Nobody who kept the text unread has
//! anything to change as that comes to be more than one number.
//!
//! So it is written so that it is no number, and nobody adds to it. A
//! number is still taken, and means after every event numbered that or
//! below: what was handed out while there was one journal, and what an
//! event's own `seq` is. Of the journal that event is in, that is just
//! after it; of another it is what was numbered later, which is not the
//! same as what was said later. Whoever wants neither more nor less
//! than what was said after a place hands back a `cursor`.

use journal::Places;
use kernel::{Error, Result};

/// What every place written here begins with; what follows is the
/// journal's own.
const WRITTEN: &str = "at-";

/// A place among the events, as it is handed out.
pub(crate) fn at(place: &Places) -> String {
    format!("{WRITTEN}{}", place.text())
}

/// A place somebody handed back: one written here, or an event's number.
pub(crate) fn read(place: &str) -> Result<Places> {
    let place = place.trim();
    Places::parse(place.strip_prefix(WRITTEN).unwrap_or(place)).ok_or_else(|| {
        Error::invalid(format!(
            "`{place}` is not a place among the events: hand back a `cursor` or a stream's `id` as it was \
             given, or an event's number (`seq`) to read what was numbered after it"
        ))
    })
}

/// A place somebody may have handed back, where they handed one.
pub(crate) fn said(place: Option<&str>) -> Result<Option<Places>> {
    place.filter(|place| !place.trim().is_empty()).map(read).transpose()
}
