use ddd_toolkit_core::DomainEventMetadata;

struct NotAnEvent;

impl DomainEventMetadata for NotAnEvent {
    type Metadata = ();

    fn event_name() -> &'static str {
        "NotAnEvent"
    }

    fn metadata(&self) -> Option<&Self::Metadata> {
        None
    }
}

fn main() {}
