use crate::port::Clock;

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

    #[test]
    fn now_returns_time_within_before_after_window() {
        let before = chrono::Utc::now();
        let now = SystemClock.now();
        let after = chrono::Utc::now();

        assert!(before <= now);
        assert!(now <= after);
    }

    #[test]
    fn now_via_dyn_clock_trait_object_returns_time() {
        let clock: Box<dyn Clock> = Box::new(SystemClock);

        let before = chrono::Utc::now();
        let now = clock.now();
        let after = chrono::Utc::now();

        assert!(before <= now);
        assert!(now <= after);
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

        let before = chrono::Utc::now();
        let now_copied = copied.now();
        let now_cloned = cloned.now();
        let after = chrono::Utc::now();

        assert!(before <= now_copied);
        assert!(now_copied <= after);
        assert!(before <= now_cloned);
        assert!(now_cloned <= after);
    }

    #[test]
    fn default_constructs_usable_system_clock() {
        #[allow(clippy::default_constructed_unit_structs)]
        let clock = SystemClock::default();

        let before = chrono::Utc::now();
        let now = clock.now();
        let after = chrono::Utc::now();

        assert!(before <= now);
        assert!(now <= after);
    }

    #[test]
    fn system_clock_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<SystemClock>();
    }
}
