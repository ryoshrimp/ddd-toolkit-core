use ddd_toolkit_core::DomainEventConversion;

struct NotAnEvent;

impl DomainEventConversion for NotAnEvent {
    type Output = ();
    type Error = ();

    fn convert(&self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

fn main() {}
