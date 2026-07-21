use crate::port::clock::Clock;

/// A [`Clock`] backed by [`chrono::Utc::now`].
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::adapter::clock::SystemClock;
/// use ddd_toolkit_core::port::clock::Clock;
///
/// let clock = SystemClock;
///
/// assert!(clock.now() <= chrono::Utc::now());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Wall-clock time (chrono::Utc::now(), which SystemClock wraps) is not
    // guaranteed monotonic: NTP slew/step corrections can move it backward
    // between two calls. Assert closeness to a reference instant instead of
    // strict before/after ordering, so the test isn't flaky under clock
    // adjustments while still catching a grossly wrong (e.g. fixed/mocked)
    // implementation.
    fn assert_close_to_now(
        now: chrono::DateTime<chrono::Utc>,
        reference: chrono::DateTime<chrono::Utc>,
    ) {
        let diff = (now - reference).num_milliseconds().abs();
        assert!(
            diff < 5_000,
            "expected {now} to be within 5s of {reference}, was {diff}ms apart"
        );
    }

    #[test]
    fn now_returns_time_close_to_wall_clock() {
        let reference = chrono::Utc::now();
        let now = SystemClock.now();

        assert_close_to_now(now, reference);
    }

    #[test]
    fn now_via_dyn_clock_trait_object_returns_time() {
        let clock: Box<dyn Clock> = Box::new(SystemClock);

        let reference = chrono::Utc::now();
        let now = clock.now();

        assert_close_to_now(now, reference);
    }

    #[test]
    fn debug_formats_struct_name() {
        assert_eq!(format!("{:?}", SystemClock), "SystemClock");
    }

    #[test]
    fn copy_and_clone_produce_usable_instances() {
        let original = SystemClock;
        let copied = original;
        #[allow(clippy::clone_on_copy)]
        let cloned = original.clone();

        let reference = chrono::Utc::now();
        let now_copied = copied.now();
        let now_cloned = cloned.now();

        assert_close_to_now(now_copied, reference);
        assert_close_to_now(now_cloned, reference);
    }

    #[test]
    fn default_constructs_usable_system_clock() {
        #[allow(clippy::default_constructed_unit_structs)]
        let clock = SystemClock::default();

        let reference = chrono::Utc::now();
        let now = clock.now();

        assert_close_to_now(now, reference);
    }

    #[test]
    fn system_clock_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<SystemClock>();
    }
}
