use ddd_toolkit_core::{ExpectedRevision, StreamRevision};
fn main() {
    let _: StreamRevision = "0".into();
    let _: ExpectedRevision = ExpectedRevision::Revision(0);
}
