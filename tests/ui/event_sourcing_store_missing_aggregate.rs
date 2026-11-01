use ddd_toolkit_core::{EventStore, ExpectedRevision, LoadResult, AppendError, AppendResult};
struct NotAggregate;
struct Store;
impl EventStore for Store {
    type Aggregate = NotAggregate;
    type Error = ();
    fn load(&self, _: &()) -> Result<LoadResult<()>, ()> { unreachable!() }
    fn append(&mut self, _: &(), _: ExpectedRevision, _: Vec<()>) -> Result<AppendResult, AppendError<()>> { unreachable!() }
}
fn main() {}
