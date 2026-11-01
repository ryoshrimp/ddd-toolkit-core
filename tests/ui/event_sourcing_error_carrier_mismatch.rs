use ddd_toolkit_core::{AggregateRoot, DomainEvent, Entity, EventStore, ExpectedRevision};
struct Event; impl DomainEvent for Event {}
struct Aggregate { id: u64 }
impl Entity for Aggregate { type Id=u64; fn id(&self)->&u64{&self.id} }
impl AggregateRoot for Aggregate { type Event=Event; fn record_event(&mut self, _:Event){} fn pull_events(&mut self)->Vec<Event>{Vec::new()} }
struct Store;
impl EventStore for Store {
 type Aggregate=Aggregate; type Error=();
 fn load(&self, _: &u64)->Result<ddd_toolkit_core::LoadResult<Event>,()>{
     let result: Result<Result<(), ()>, ()> = Ok(Err(()));
     result
 }
 fn append(&mut self, _: &u64, _: ExpectedRevision, _: Vec<Event>)->Result<ddd_toolkit_core::AppendResult, ddd_toolkit_core::AppendError<()>>{unreachable!()}
}
fn main() {}
