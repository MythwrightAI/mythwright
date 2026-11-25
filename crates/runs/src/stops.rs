//! The stops under way, by instance: what an apply that comes to an
//! instance while it is being stopped asks first.
//!
//! A stop says that it is under way from before it does anything until
//! it is written, and an apply that comes to such an instance waits
//! until it is over. It came out of the runner, which says what an apply
//! that meets one comes to and offers this under the name it always gave
//! (`crate::meets::Stops` there).

use std::collections::HashMap;
use std::sync::Mutex;

use kernel::Cancel;

/// The stops under way, by instance: how many of each there are, and
/// what is cancelled when the last of them is over.
#[derive(Default)]
pub struct Stops(Mutex<HashMap<String, (usize, Cancel)>>);

/// One stop under way, which is over when this is let go of: written,
/// given up on, or cut short with whoever asked for it.
pub struct Stopping<'a>(&'a Stops, &'a str);

impl Stops {
    pub fn begin<'a>(&'a self, id: &'a str) -> Stopping<'a> {
        self.ahead(id);
        Stopping(self, id)
    }

    /// One stop of `id` more under way, which whoever says so ends
    /// ([`Stops::end`]): a stop an instance asked of itself, which is
    /// held by whatever begins it and not by a [`Stopping`].
    pub fn ahead(&self, id: &str) {
        let mut stops = self.0.lock().expect("stops lock");
        stops.entry(id.to_string()).or_insert_with(|| (0, Cancel::new())).0 += 1;
    }

    /// Whether a stop of `id` is under way.
    pub fn under_way(&self, id: &str) -> bool {
        self.0.lock().expect("stops lock").contains_key(id)
    }

    /// What is cancelled when `id` has been stopped, while it is being.
    pub fn over(&self, id: &str) -> Option<Cancel> {
        self.0.lock().expect("stops lock").get(id).map(|(_, over)| over.clone())
    }

    /// One stop of `id` fewer under way: the last over cancels what is
    /// waited on for it.
    pub fn end(&self, id: &str) {
        let mut stops = self.0.lock().expect("stops lock");
        if let Some((going, over)) = stops.get_mut(id) {
            *going -= 1;
            if *going == 0 {
                over.cancel();
                stops.remove(id);
            }
        }
    }
}

impl Drop for Stopping<'_> {
    fn drop(&mut self) {
        self.0.end(self.1);
    }
}
