//! Signal contract for component version handover coordination.
//!
//! This crate is the private upgrade vocabulary shared by two versions of a
//! component daemon. It carries typed marker, readiness, completion, mirror,
//! divergence, and recovery records. It does not own runtime socket policy or
//! migration execution.

use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode, NotaEnum, NotaRecord};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use signal_frame::signal_channel;
use version_projection::{ComponentName, ContractVersion, RecordKind};

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl NotaEncode for Date {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        encoder.write_date(self.year, self.month, self.day)
    }
}

impl NotaDecode for Date {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let (year, month, day) = decoder.read_date()?;
        Ok(Self { year, month, day })
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    pub const fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }
}

impl NotaEncode for Time {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        encoder.write_time(self.hour, self.minute, self.second)
    }
}

impl NotaDecode for Time {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let (hour, minute, second) = decoder.read_time()?;
        Ok(Self {
            hour,
            minute,
            second,
        })
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoverMarker {
    pub component: ComponentName,
    pub schema_hash: ContractVersion,
    pub commit_sequence: u64,
    pub write_counter: u64,
    pub last_record_identifier: Option<u64>,
    pub recorded_at_date: Date,
    pub recorded_at_time: Time,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct MarkerRequest {
    pub component: ComponentName,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReport {
    pub component: ComponentName,
    pub source_marker: HandoverMarker,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct CompletionReport {
    pub component: ComponentName,
    pub accepted_marker: HandoverMarker,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct MirrorPayload {
    pub component: ComponentName,
    pub source_version: ContractVersion,
    pub target_version: ContractVersion,
    pub kind: RecordKind,
    pub payload: Vec<u8>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct DivergencePayload {
    pub component: ComponentName,
    pub source_version: ContractVersion,
    pub target_version: ContractVersion,
    pub reason: DivergenceReason,
    pub kind: RecordKind,
    pub payload: Vec<u8>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RecoveryRequest {
    pub component: ComponentName,
    pub failure_identifier: u64,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoverAcceptance {
    pub accepted_marker: HandoverMarker,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoverFinalization {
    pub finalized_marker: HandoverMarker,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct MirrorAcknowledgement {
    pub component: ComponentName,
    pub write_counter: u64,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct DivergenceAcknowledgement {
    pub component: ComponentName,
    pub divergence_identifier: u64,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RecoveryResult {
    pub component: ComponentName,
    pub recovered: bool,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoverRejection {
    pub component: ComponentName,
    pub reason: HandoverRejectionReason,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum HandoverRejectionReason {
    SchemaMismatch,
    CommitSequenceAdvanced,
    AlreadyInHandover,
    NotReady,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum DivergenceReason {
    NotRepresentable,
    TargetUnavailable,
    TargetRejected,
}

signal_channel! {
    channel VersionHandover {
        operation AskHandoverMarker(MarkerRequest),
        operation ReadyToHandover(ReadinessReport),
        operation HandoverCompleted(CompletionReport),
        operation Mirror(MirrorPayload),
        operation Divergence(DivergencePayload),
        operation RecoverFromFailure(RecoveryRequest),
    }
    reply Reply {
        HandoverMarker(HandoverMarker),
        HandoverAccepted(HandoverAcceptance),
        HandoverFinalized(HandoverFinalization),
        MirrorAcknowledged(MirrorAcknowledgement),
        DivergenceAcknowledged(DivergenceAcknowledgement),
        RecoveryCompleted(RecoveryResult),
        HandoverRejected(HandoverRejection),
    }
}
