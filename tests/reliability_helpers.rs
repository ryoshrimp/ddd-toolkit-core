use ddd_toolkit_core::{DeliveryMetadata, OutboxMessageId, StreamHistory, StreamRevision};

#[test]
fn delivery_metadata_replaces_labels_without_changing_event_identity() {
    let metadata = DeliveryMetadata::new("event-1");
    assert_eq!(metadata.correlation_id(), None);
    assert_eq!(metadata.causation_id(), None);

    let metadata = metadata
        .with_correlation_id("old-correlation")
        .with_causation_id("old-cause")
        .with_correlation_id(String::from("new-correlation"))
        .with_causation_id(String::from("new-cause"));
    assert_eq!(metadata.event_id(), "event-1");
    assert_eq!(metadata.correlation_id(), Some("new-correlation"));
    assert_eq!(metadata.causation_id(), Some("new-cause"));
}

#[test]
fn outbox_message_id_transfers_its_string_including_an_empty_id() {
    for value in ["message-1", ""] {
        let id = OutboxMessageId::new(value.to_owned());
        assert_eq!(id.as_str(), value);
        assert_eq!(id.into_inner(), value);
    }
}

#[test]
fn stream_revision_stops_at_max_and_history_transfers_events() {
    assert_eq!(StreamRevision::new(0).next(), Some(StreamRevision::new(1)));
    assert_eq!(StreamRevision::new(u64::MAX).next(), None);

    let history = StreamHistory::try_new(
        vec![String::from("created"), String::from("updated")],
        StreamRevision::new(1),
    )
    .unwrap();
    assert_eq!(history.into_events(), ["created", "updated"]);
}
