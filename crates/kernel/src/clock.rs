//! Time, injected.
//!
//! Everything that waits or timestamps asks a [`Clock`]. In production that is
//! the system clock; in tests it is a [`ManualClock`] that only moves when the
//! test says so. That is what lets a test cover a five-hour wait or a retry
//! backoff in microseconds, and it is a hard rule here: on the previous
//! codebase real waits in tests were about 80% of every gate.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::oneshot;

use crate::BoxFut;

pub trait Clock: Send + Sync + 'static {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> i64;

    /// Completes after `duration` of this clock's time.
    fn sleep(&self, duration: Duration) -> BoxFut<'static, ()>;

    /// Completes once this clock reads `at_ms`, however it was read before:
    /// a wait kept as when it wakes wakes then, and not so long after it
    /// was last read.
    fn sleep_until(&self, at_ms: i64) -> BoxFut<'static, ()> {
        self.sleep(Duration::from_millis((at_ms - self.now_ms()).max(0) as u64))
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as i64)
    }

    fn sleep(&self, duration: Duration) -> BoxFut<'static, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// A clock that stands still until advanced.
#[derive(Clone)]
pub struct ManualClock {
    inner: Arc<Manual>,
}

struct Manual {
    now_ms: Mutex<i64>,
    sleepers: Mutex<Vec<Sleeper>>,
    /// How many sleeps have ever been started; lets a test wait for the code
    /// under test to reach its next sleep without polling.
    started: AtomicU64,
    started_notify: tokio::sync::Notify,
}

struct Sleeper {
    wake_at: i64,
    wake: oneshot::Sender<()>,
}

impl ManualClock {
    /// Starts at a fixed, recognisable instant (2026-01-01T00:00:00Z).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Manual {
                now_ms: Mutex::new(1_767_225_600_000),
                sleepers: Mutex::new(Vec::new()),
                started: AtomicU64::new(0),
                started_notify: tokio::sync::Notify::new(),
            }),
        }
    }

    /// Moves time forward and wakes every sleep that is now due.
    pub fn advance(&self, duration: Duration) {
        let now = {
            let mut now = self.inner.now_ms.lock().expect("clock lock");
            *now += duration.as_millis() as i64;
            *now
        };
        let due: Vec<Sleeper> = {
            let mut sleepers = self.inner.sleepers.lock().expect("clock lock");
            let (due, waiting) = std::mem::take(&mut *sleepers).into_iter().partition(|s| s.wake_at <= now);
            *sleepers = waiting;
            due
        };
        for sleeper in due {
            let _ = sleeper.wake.send(());
        }
    }

    /// How many sleeps are waiting right now.
    pub fn sleeping(&self) -> usize {
        let mut sleepers = self.inner.sleepers.lock().expect("clock lock");
        sleepers.retain(|sleeper| !sleeper.wake.is_closed());
        sleepers.len()
    }

    /// Waits until at least `count` sleeps have been started since the clock
    /// was made. Tests use this to know the code under test is parked on the
    /// clock before advancing it.
    pub async fn sleeps_started(&self, count: u64) {
        loop {
            let notified = self.inner.started_notify.notified();
            if self.inner.started.load(Ordering::SeqCst) >= count {
                return;
            }
            notified.await;
        }
    }

    /// Sleeps started so far.
    pub fn sleep_count(&self) -> u64 {
        self.inner.started.load(Ordering::SeqCst)
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> i64 {
        *self.inner.now_ms.lock().expect("clock lock")
    }

    fn sleep(&self, duration: Duration) -> BoxFut<'static, ()> {
        self.sleep_until(self.now_ms() + duration.as_millis() as i64)
    }

    fn sleep_until(&self, wake_at: i64) -> BoxFut<'static, ()> {
        let (wake, woken) = oneshot::channel();
        // The time is read holding the sleepers, which `advance` takes once
        // it has moved the time: moved past it meanwhile, it is woken here
        // or there.
        let mut sleepers = self.inner.sleepers.lock().expect("clock lock");
        if wake_at <= self.now_ms() {
            let _ = wake.send(());
        } else {
            sleepers.push(Sleeper { wake_at, wake });
        }
        drop(sleepers);
        self.inner.started.fetch_add(1, Ordering::SeqCst);
        self.inner.started_notify.notify_waiters();
        Box::pin(async move {
            // If the clock is dropped the sleep never ends, which is correct:
            // nothing can advance it.
            if woken.await.is_err() {
                std::future::pending::<()>().await;
            }
        })
    }
}

/// `2026-01-01T00:00:00.000Z` for a Unix timestamp in milliseconds.
pub fn rfc3339(ms: i64) -> String {
    let (seconds, millis) = (ms.div_euclid(1000), ms.rem_euclid(1000));
    let (days, second_of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    // Civil-from-days, after Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        second_of_day / 3600,
        second_of_day % 3600 / 60,
        second_of_day % 60
    )
}
