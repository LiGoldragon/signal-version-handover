use nota_codec::{NotaDecode, NotaEncode};
use signal_frame::{
    ExchangeFrame, ExchangeFrameBody, ExchangeIdentifier, ExchangeLane, LaneSequence,
    RequestPayload, SessionEpoch,
};
use signal_version_handover::{
    CompletionReport, Date, DivergenceReason, HandoverMarker, HandoverRejection,
    HandoverRejectionReason, MarkerRequest, MirrorPayload, Operation, OperationKind, Reply, Time,
};
use version_projection::{ComponentName, ContractVersion, RecordKind};

fn version(byte: u8) -> ContractVersion {
    ContractVersion::new([byte; 32])
}

fn marker() -> HandoverMarker {
    HandoverMarker {
        component: ComponentName::new("persona-spirit"),
        schema_hash: version(1),
        commit_sequence: 34,
        write_counter: 55,
        last_record_identifier: Some(103),
        recorded_at_date: Date::new(2026, 5, 22),
        recorded_at_time: Time::new(11, 42, 0),
    }
}

fn encode<T: NotaEncode>(value: &T) -> String {
    let mut encoder = nota_codec::Encoder::new();
    value.encode(&mut encoder).expect("encode");
    encoder.into_string()
}

#[test]
fn operation_heads_are_contract_local() {
    let operation = Operation::AskHandoverMarker(MarkerRequest {
        component: ComponentName::new("persona-spirit"),
    });

    assert_eq!(operation.kind(), OperationKind::AskHandoverMarker);
    assert_eq!(
        encode(&operation.into_request()),
        "(AskHandoverMarker (persona-spirit))"
    );
}

#[test]
fn marker_reply_round_trips_through_nota() {
    let reply = Reply::HandoverMarker(marker());
    let text = encode(&reply);

    assert!(text.starts_with("(HandoverMarker (persona-spirit "));

    let mut decoder = nota_codec::Decoder::new(&text);
    let decoded = Reply::decode(&mut decoder).expect("decode");
    assert_eq!(decoded, reply);
}

#[test]
fn mirror_payload_carries_source_target_and_bytes() {
    let payload = MirrorPayload {
        component: ComponentName::new("persona-spirit"),
        source_version: version(1),
        target_version: version(2),
        kind: RecordKind::new("Entry"),
        payload: vec![1, 2, 3],
    };
    let operation = Operation::Mirror(payload.clone());

    assert_eq!(operation.kind(), OperationKind::Mirror);
    assert_eq!(operation, Operation::Mirror(payload));
}

#[test]
fn divergence_reason_is_typed_not_a_string() {
    let reply = Reply::HandoverRejected(HandoverRejection {
        component: ComponentName::new("persona-spirit"),
        reason: HandoverRejectionReason::CommitSequenceAdvanced,
    });
    let text = encode(&reply);

    assert!(text.contains("CommitSequenceAdvanced"));
    let mut decoder = nota_codec::Decoder::new(&text);
    assert_eq!(Reply::decode(&mut decoder).expect("decode"), reply);
}

#[test]
fn completion_report_survives_frame_round_trip() {
    let exchange = ExchangeIdentifier::new(
        SessionEpoch::new(1),
        ExchangeLane::Connector,
        LaneSequence::first(),
    );
    let frame = ExchangeFrame::<Operation, Reply>::new(ExchangeFrameBody::Request {
        exchange,
        request: Operation::HandoverCompleted(CompletionReport {
            component: ComponentName::new("persona-spirit"),
            accepted_marker: marker(),
        })
        .into_request(),
    });
    let bytes = frame.encode().expect("encode frame");
    let decoded = ExchangeFrame::<Operation, Reply>::decode(&bytes).expect("decode frame");

    assert_eq!(decoded, frame);
}

#[test]
fn divergence_reason_encodes_as_unit_variant() {
    let reason = DivergenceReason::NotRepresentable;
    assert_eq!(encode(&reason), "NotRepresentable");
}
