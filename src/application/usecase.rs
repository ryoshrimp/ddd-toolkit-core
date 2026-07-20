#[trait_variant::make(Send)]
pub trait UseCase: Send + Sync {
    type Input: Send;
    type Output: Send;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::testing::block_on;

    use super::*;

    #[derive(Debug, PartialEq)]
    struct FooError {
        message: String,
    }

    impl Display for FooError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::error::Error for FooError {}

    struct FooUseCase;

    impl UseCase for FooUseCase {
        type Input = String;
        type Output = String;
        type Error = FooError;

        async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
            Ok(format!("hello, {input}"))
        }
    }

    struct CountingUseCase {
        calls: std::sync::Mutex<usize>,
    }

    impl UseCase for CountingUseCase {
        type Input = ();
        type Output = usize;
        type Error = FooError;

        async fn execute(&self, _input: Self::Input) -> Result<Self::Output, Self::Error> {
            let mut calls = self.calls.lock().expect("lock should not be poisoned");
            *calls += 1;
            Ok(*calls)
        }
    }

    struct FailingUseCase;

    impl UseCase for FailingUseCase {
        type Input = String;
        type Output = String;
        type Error = FooError;

        async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
            Err(FooError {
                message: format!("failed to execute with {input}"),
            })
        }
    }

    struct UnitUseCase;

    impl UseCase for UnitUseCase {
        type Input = ();
        type Output = ();
        type Error = FooError;

        async fn execute(&self, _input: Self::Input) -> Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn execute_returns_ok_output() {
        let use_case = FooUseCase;

        let output =
            block_on(use_case.execute("world".to_string())).expect("execute should succeed");

        assert_eq!(output, "hello, world");
    }

    #[test]
    fn execute_with_shared_reference_allows_multiple_calls() {
        let use_case = CountingUseCase {
            calls: std::sync::Mutex::new(0),
        };

        let first = block_on(use_case.execute(())).expect("first call should succeed");
        let second = block_on(use_case.execute(())).expect("second call should succeed");

        assert_eq!(first, 1);
        assert_eq!(second, 2);
    }

    // Verifies the trait bounds are usable from generic code; the runtime
    // assertion is secondary to the fact that this compiles.
    #[test]
    fn use_case_is_usable_through_generic_bound() {
        fn run<U: UseCase>(use_case: &U, input: U::Input) -> Result<U::Output, U::Error> {
            block_on(use_case.execute(input))
        }

        let output =
            run(&FooUseCase, "generic".to_string()).expect("generic execute should succeed");

        assert_eq!(output, "hello, generic");
    }

    // Compile-time only (no runtime assertion): the generic bound accepts the
    // future as `Send` purely because `#[trait_variant::make(Send)]` promises
    // it at the trait level, not because of the concrete impl.
    #[test]
    fn execute_future_is_send() {
        fn assert_send(_: impl Send) {}

        fn check<U: UseCase>(use_case: &U, input: U::Input) {
            assert_send(use_case.execute(input));
        }

        check(&FooUseCase, "send".to_string());
    }

    #[test]
    fn execute_returns_err_from_failing_use_case() {
        let use_case = FailingUseCase;

        let error = block_on(use_case.execute("bad".to_string())).expect_err("execute should fail");

        assert_eq!(
            error,
            FooError {
                message: "failed to execute with bad".to_string()
            }
        );
        assert_eq!(error.to_string(), "failed to execute with bad");
    }

    // Verifies the `Error: std::error::Error + Send + Sync + 'static` bound is
    // strong enough for the usual boxed-error upcast.
    #[test]
    fn error_can_be_boxed_as_dyn_error() {
        fn boxed<U: UseCase>(error: U::Error) -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(error)
        }

        let error =
            block_on(FailingUseCase.execute("bad".to_string())).expect_err("execute should fail");

        let boxed = boxed::<FailingUseCase>(error);

        assert_eq!(boxed.to_string(), "failed to execute with bad");
    }

    #[test]
    fn execute_works_with_unit_input_and_output() {
        let () = block_on(UnitUseCase.execute(())).expect("execute should succeed");
    }
}
