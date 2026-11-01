use ddd_toolkit_core::{
    AggregateRoot, DomainEvent, DomainEventConversion, DomainEventMetadata, Entity,
};

struct MarkerOnly;
impl DomainEvent for MarkerOnly {}

struct MetadataEvent {
    value: u32,
    metadata: String,
}

impl DomainEvent for MetadataEvent {}
impl DomainEventMetadata for MetadataEvent {
    type Metadata = str;

    fn event_name() -> &'static str {
        "MetadataEvent"
    }

    fn metadata(&self) -> Option<&Self::Metadata> {
        Some(&self.metadata)
    }
}

struct VersionedEvent;
impl DomainEvent for VersionedEvent {}
impl DomainEventMetadata for VersionedEvent {
    type Metadata = ();

    fn event_name() -> &'static str {
        "VersionedEvent"
    }

    fn event_version() -> u32 {
        7
    }

    fn metadata(&self) -> Option<&Self::Metadata> {
        None
    }
}

#[derive(Debug, PartialEq)]
enum ConversionError {
    Rejected,
}

struct ConvertibleEvent {
    value: u32,
}

impl DomainEvent for ConvertibleEvent {}
impl DomainEventConversion for ConvertibleEvent {
    type Output = String;
    type Error = ConversionError;

    fn convert(&self) -> Result<Self::Output, Self::Error> {
        if self.value == 0 {
            Err(ConversionError::Rejected)
        } else {
            Ok(self.value.to_string())
        }
    }
}

struct NonSerializableMetadata {
    value: u8,
}

struct NonSerializableMetadataEvent {
    metadata: NonSerializableMetadata,
}

impl DomainEvent for NonSerializableMetadataEvent {}
impl DomainEventMetadata for NonSerializableMetadataEvent {
    type Metadata = NonSerializableMetadata;

    fn event_name() -> &'static str {
        "NonSerializableMetadataEvent"
    }

    fn metadata(&self) -> Option<&Self::Metadata> {
        Some(&self.metadata)
    }
}

#[test]
fn marker_only_events_remain_usable_without_new_contracts() {
    fn assert_event<E: DomainEvent>() {}

    assert_event::<MarkerOnly>();
}

#[test]
fn metadata_exposes_name_default_version_and_borrowed_value() {
    let event = MetadataEvent {
        value: 42,
        metadata: String::from("source-a"),
    };

    assert_eq!(MetadataEvent::event_name(), "MetadataEvent");
    assert_eq!(MetadataEvent::event_version(), 1);
    assert_eq!(event.metadata(), Some("source-a"));
    assert_eq!(event.value, 42);
}

#[test]
fn metadata_can_override_version_and_return_none() {
    assert_eq!(VersionedEvent::event_name(), "VersionedEvent");
    assert_eq!(VersionedEvent::event_version(), 7);
    assert!(VersionedEvent.metadata().is_none());
}

#[test]
fn conversion_returns_consumer_output_or_exact_error_without_consuming_source() {
    let accepted = ConvertibleEvent { value: 9 };
    assert_eq!(accepted.convert(), Ok(String::from("9")));
    assert_eq!(accepted.value, 9);

    let rejected = ConvertibleEvent { value: 0 };
    assert_eq!(rejected.convert(), Err(ConversionError::Rejected));
    assert_eq!(rejected.value, 0);
}

#[test]
fn metadata_and_conversion_calls_preserve_event_owned_state() {
    let event = ConvertibleEvent { value: 11 };
    let before = event.value;

    let _ = event.convert();

    assert_eq!(event.value, before)
}

struct LifecycleEvent {
    value: u32,
    metadata: String,
}

impl DomainEvent for LifecycleEvent {}
impl DomainEventMetadata for LifecycleEvent {
    type Metadata = str;

    fn event_name() -> &'static str {
        "LifecycleEvent"
    }

    fn metadata(&self) -> Option<&Self::Metadata> {
        Some(&self.metadata)
    }
}
impl DomainEventConversion for LifecycleEvent {
    type Output = u32;
    type Error = ();

    fn convert(&self) -> Result<Self::Output, Self::Error> {
        Ok(self.value)
    }
}

struct LifecycleAggregate {
    id: u8,
    events: Vec<LifecycleEvent>,
}

impl Entity for LifecycleAggregate {
    type Id = u8;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for LifecycleAggregate {
    type Event = LifecycleEvent;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[test]
fn metadata_and_conversion_do_not_change_lifecycle_queue_state() {
    let event = LifecycleEvent {
        value: 21,
        metadata: String::from("observed"),
    };

    assert_eq!(event.metadata(), Some("observed"));
    assert_eq!(event.metadata(), Some("observed"));
    assert_eq!(event.convert(), Ok(21));
    assert_eq!(event.convert(), Ok(21));

    let mut aggregate = LifecycleAggregate {
        id: 1,
        events: vec![],
    };
    aggregate.record_event(event);

    assert_eq!(aggregate.events.len(), 1);
    assert_eq!(aggregate.pull_events().len(), 1);
    assert!(aggregate.pull_events().is_empty());
}

#[test]
fn metadata_dose_not_require_serialization_or_library_marker_traits() {
    let event = NonSerializableMetadataEvent {
        metadata: NonSerializableMetadata { value: 3 },
    };

    assert_eq!(event.metadata().map(|metadata| metadata.value), Some(3))
}
