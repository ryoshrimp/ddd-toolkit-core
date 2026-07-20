use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

/// This helper only drives futures that resolve synchronously (this crate's
/// own test doubles never actually wait); it is not a general-purpose
/// executor. A future still pending after this long is treated as a bug in
/// the test rather than something to wait out.
const TIMEOUT: Duration = Duration::from_millis(200);

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    let start = Instant::now();
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        assert!(
            start.elapsed() <= TIMEOUT,
            "block_on: future did not resolve within {TIMEOUT:?} - this test helper only polls \
             futures that resolve synchronously with a no-op waker, so a future that is \
             genuinely pending (real I/O, a timer, ...) will spin forever instead of being \
             woken; use a real executor for that case instead",
        );
        std::hint::spin_loop();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::Poll,
    };

    #[test]
    fn resolves_ready_future_immediately() {
        assert_eq!(block_on(async { 42 }), 42);
    }

    #[test]
    fn resolves_future_that_is_pending_on_first_poll() {
        struct PendingOnce(bool);

        impl Future for PendingOnce {
            type Output = &'static str;

            fn poll(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Self::Output> {
                if self.0 {
                    Poll::Ready("done")
                } else {
                    self.0 = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        assert_eq!(block_on(PendingOnce(false)), "done");
    }

    #[test]
    #[should_panic(expected = "did not resolve within")]
    fn panics_instead_of_spinning_forever_on_a_future_that_never_resolves() {
        struct NeverReady;

        impl Future for NeverReady {
            type Output = ();

            fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        block_on(NeverReady);
    }

    #[test]
    fn runs_to_completion_before_the_timeout_elapses() {
        let polled = Arc::new(AtomicBool::new(false));
        let polled_in_future = Arc::clone(&polled);

        struct ReadyAfterFlag(Arc<AtomicBool>);

        impl Future for ReadyAfterFlag {
            type Output = ();

            fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                self.0.store(true, Ordering::SeqCst);
                Poll::Ready(())
            }
        }

        block_on(ReadyAfterFlag(polled_in_future));

        assert!(polled.load(Ordering::SeqCst));
    }
}
