use ddd_toolkit_core::{AggregateSnapshot, SnapshotLoadError, SnapshotSaveError, SnapshotStore};
struct NotAggregate;
struct Store;
impl SnapshotStore for Store {
 type Aggregate = NotAggregate; type State = (); type Error = ();
 fn load(&self, _: &())->Result<Option<AggregateSnapshot<()>>, SnapshotLoadError<()>>{unreachable!()}
 fn save(&mut self, _: &(), _: AggregateSnapshot<()>) -> Result<(), SnapshotSaveError<()>>{unreachable!()}
}
fn main() {}
