/// A source of the current time, so domain/application code doesn't call
/// [`chrono::Utc::now`] directly and become untestable.
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
