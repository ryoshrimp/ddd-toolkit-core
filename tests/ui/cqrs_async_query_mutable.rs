use ddd_toolkit_core::AsyncQueryHandler;
use std::future::Future;
use std::pin::Pin;

struct Query;

impl AsyncQueryHandler for Query {
    type Query = ();
    type Result = ();
    type Error = ();

    fn handle<'a>(
        &'a mut self,
        _: (),
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

fn main() {}
