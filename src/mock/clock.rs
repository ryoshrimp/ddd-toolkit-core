use crate::port::clock::Clock;

/// A [`Clock`] that always returns a configured time until [`FixedClock::set`]
/// changes it.
#[derive(Debug)]
pub struct FixedClock(std::sync::Mutex<chrono::DateTime<chrono::Utc>>);

impl FixedClock {
    /// Creates a `FixedClock` that returns `now` until [`FixedClock::set`]
    /// is called.
    pub fn new(now: chrono::DateTime<chrono::Utc>) -> Self {
        Self(std::sync::Mutex::new(now))
    }

    /// Changes the time this clock returns.
    pub fn set(&self, now: chrono::DateTime<chrono::Utc>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = now;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn utc(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn new_then_now_returns_configured_time() {
        let time = utc("2026-07-19T00:00:00Z");
        let clock = FixedClock::new(time);

        assert_eq!(clock.now(), time);
    }

    #[test]
    fn now_returns_equal_time_on_repeated_calls() {
        let time = utc("2026-07-19T00:00:00Z");
        let clock = FixedClock::new(time);

        assert_eq!(clock.now(), time);
        assert_eq!(clock.now(), time);
        assert_eq!(clock.now(), time);
    }

    #[test]
    fn set_changes_time_returned_by_now() {
        let clock = FixedClock::new(utc("2026-07-19T00:00:00Z"));

        let later = utc("2026-07-20T12:34:56Z");
        clock.set(later);

        assert_eq!(clock.now(), later);
    }

    #[test]
    fn set_from_another_thread_is_visible_to_now() {
        let clock = std::sync::Arc::new(FixedClock::new(utc("2026-07-19T00:00:00Z")));

        let later = utc("2026-07-20T12:34:56Z");
        let writer = std::sync::Arc::clone(&clock);
        std::thread::spawn(move || writer.set(later))
            .join()
            .unwrap();

        assert_eq!(clock.now(), later);
    }

    #[test]
    fn box_dyn_clock_is_usable() {
        let time = utc("2026-07-19T00:00:00Z");
        let clock: Box<dyn Clock> = Box::new(FixedClock::new(time));

        assert_eq!(clock.now(), time);
    }

    #[test]
    fn debug_format_contains_type_name() {
        let clock = FixedClock::new(utc("2026-07-19T00:00:00Z"));

        assert!(format!("{clock:?}").contains("FixedClock"));
    }

    #[test]
    fn now_returns_extreme_timestamps_as_is() {
        let clock = FixedClock::new(chrono::DateTime::<chrono::Utc>::MIN_UTC);
        assert_eq!(clock.now(), chrono::DateTime::<chrono::Utc>::MIN_UTC);

        clock.set(chrono::DateTime::<chrono::Utc>::MAX_UTC);
        assert_eq!(clock.now(), chrono::DateTime::<chrono::Utc>::MAX_UTC);
    }

    // A panic while the lock is held poisons the Mutex; now()/set() must
    // recover the guard instead of panicking forever.
    #[test]
    fn clock_recovers_from_poisoned_mutex() {
        let time = utc("2026-07-19T00:00:00Z");
        let clock = FixedClock::new(time);

        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = clock.0.lock().unwrap();
            panic!("simulated panic while holding the lock");
        }));
        assert!(poisoned.is_err());
        assert!(clock.0.is_poisoned());

        assert_eq!(clock.now(), time);
    }
}
