/// A source of the current time, so domain/application code doesn't call
/// [`chrono::Utc::now`] directly and become untestable.
///
/// [`crate::adapter::clock::SystemClock`] wraps the real wall clock;
/// [`crate::mock::clock::FixedClock`] is a test double that returns a
/// configured time.
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::port::clock::Clock;
///
/// struct FixedClock(chrono::DateTime<chrono::Utc>);
///
/// impl Clock for FixedClock {
///     fn now(&self) -> chrono::DateTime<chrono::Utc> {
///         self.0
///     }
/// }
///
/// fn is_in_the_future(clock: &dyn Clock, deadline: chrono::DateTime<chrono::Utc>) -> bool {
///     clock.now() < deadline
/// }
///
/// let noon = "2026-07-21T12:00:00Z".parse().unwrap();
/// let clock = FixedClock(noon);
///
/// assert!(is_in_the_future(&clock, "2026-07-21T13:00:00Z".parse().unwrap()));
/// assert!(!is_in_the_future(&clock, "2026-07-21T11:00:00Z".parse().unwrap()));
/// ```
pub trait Clock: Send + Sync {
    /// The current time.
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}

#[cfg(test)]
mod test {
    use super::*;

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    #[test]
    fn fixed_clock_test_double_returns_configured_time() {
        let time = chrono::DateTime::parse_from_rfc3339("2026-07-19T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let clock: &dyn Clock = &FixedClock(time);

        assert_eq!(clock.now(), time);
    }

    #[test]
    fn clock_implementors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}

        assert_send_sync::<FixedClock>();
        assert_send_sync::<dyn Clock>();
    }
}
