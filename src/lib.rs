//! Signal contract for component version handover coordination.
//!
//! This crate is the private upgrade vocabulary shared by two versions of a
//! component daemon. It carries typed marker, readiness, completion, mirror,
//! divergence, and recovery records. It does not own runtime socket policy or
//! migration execution.

use nota_next::{Block, Delimiter, NotaBlock, NotaDecode, NotaDecodeError, NotaEncode};
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

    fn from_nota_literal(literal: &str) -> Result<Self, NotaDecodeError> {
        let parts = literal.split('-').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(NotaDecodeError::Parse(format!(
                "date literal must be YYYY-MM-DD: {literal}"
            )));
        }
        let year = parts[0]
            .parse::<u16>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid date year: {literal}")))?;
        let month = parts[1]
            .parse::<u8>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid date month: {literal}")))?;
        let day = parts[2]
            .parse::<u8>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid date day: {literal}")))?;
        let date = Self { year, month, day };
        if date.is_valid() {
            Ok(date)
        } else {
            Err(NotaDecodeError::Parse(format!(
                "invalid date literal: {literal}"
            )))
        }
    }

    fn is_valid(&self) -> bool {
        let maximum_day = match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(self.year) => 29,
            2 => 28,
            _ => return false,
        };
        (1..=maximum_day).contains(&self.day)
    }

    const fn is_leap_year(year: u16) -> bool {
        year % 400 == 0 || (year % 4 == 0 && year % 100 != 0)
    }
}

impl NotaDecode for Date {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let literal = block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom { type_name: "Date" })?;
        Self::from_nota_literal(literal)
    }
}

impl NotaEncode for Date {
    fn to_nota(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
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

    fn from_nota_literal(literal: &str) -> Result<Self, NotaDecodeError> {
        let parts = literal.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(NotaDecodeError::Parse(format!(
                "time literal must be HH:MM:SS: {literal}"
            )));
        }
        let hour = parts[0]
            .parse::<u8>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid time hour: {literal}")))?;
        let minute = parts[1]
            .parse::<u8>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid time minute: {literal}")))?;
        let second = parts[2]
            .parse::<u8>()
            .map_err(|_| NotaDecodeError::Parse(format!("invalid time second: {literal}")))?;
        let time = Self {
            hour,
            minute,
            second,
        };
        if time.is_valid() {
            Ok(time)
        } else {
            Err(NotaDecodeError::Parse(format!(
                "invalid time literal: {literal}"
            )))
        }
    }

    const fn is_valid(&self) -> bool {
        self.hour < 24 && self.minute < 60 && self.second < 60
    }
}

impl NotaDecode for Time {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let literal = block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom { type_name: "Time" })?;
        Self::from_nota_literal(literal)
    }
}

impl NotaEncode for Time {
    fn to_nota(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawPayload(Vec<u8>);

impl RawPayload {
    fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    fn decode_literal(literal: &str) -> Result<Vec<u8>, NotaDecodeError> {
        let hex = literal.strip_prefix('#').ok_or_else(|| {
            NotaDecodeError::Parse(format!("raw payload must be a #hex literal: {literal}"))
        })?;
        if hex.len() % 2 != 0 {
            return Err(NotaDecodeError::Parse(format!(
                "raw payload hex literal has odd length: {literal}"
            )));
        }
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| Self::decode_byte(pair, literal))
            .collect()
    }

    fn decode_byte(pair: &[u8], literal: &str) -> Result<u8, NotaDecodeError> {
        let high = Self::decode_digit(pair[0], literal)?;
        let low = Self::decode_digit(pair[1], literal)?;
        Ok((high << 4) | low)
    }

    fn decode_digit(byte: u8, literal: &str) -> Result<u8, NotaDecodeError> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err(NotaDecodeError::Parse(format!(
                "raw payload contains non-hex digit: {literal}"
            ))),
        }
    }
}

impl NotaDecode for RawPayload {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let literal = block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom {
                type_name: "RawPayload",
            })?;
        Self::decode_literal(literal).map(Self::new)
    }
}

impl NotaEncode for RawPayload {
    fn to_nota(&self) -> String {
        let mut literal = String::with_capacity(1 + self.0.len() * 2);
        literal.push('#');
        for byte in &self.0 {
            literal.push_str(&format!("{byte:02x}"));
        }
        literal
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct HandoverMarker {
    pub component: ComponentName,
    pub schema_hash: ContractVersion,
    pub commit_sequence: u64,
    pub write_counter: u64,
    pub last_record_identifier: Option<u64>,
    pub recorded_at_date: Date,
    pub recorded_at_time: Time,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct MarkerRequest {
    pub component: ComponentName,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct ReadinessReport {
    pub component: ComponentName,
    pub source_marker: HandoverMarker,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct CompletionReport {
    pub component: ComponentName,
    pub accepted_marker: HandoverMarker,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MirrorPayload {
    pub component: ComponentName,
    pub source_version: ContractVersion,
    pub target_version: ContractVersion,
    pub kind: RecordKind,
    pub payload: Vec<u8>,
}

impl NotaDecode for MirrorPayload {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let children =
            NotaBlock::new(block).expect_children(Delimiter::Parenthesis, "MirrorPayload", 5)?;
        Ok(Self {
            component: ComponentName::from_nota_block(&children[0])?,
            source_version: ContractVersion::from_nota_block(&children[1])?,
            target_version: ContractVersion::from_nota_block(&children[2])?,
            kind: RecordKind::from_nota_block(&children[3])?,
            payload: RawPayload::from_nota_block(&children[4])?.into_bytes(),
        })
    }
}

impl NotaEncode for MirrorPayload {
    fn to_nota(&self) -> String {
        Delimiter::Parenthesis.wrap([
            self.component.to_nota(),
            self.source_version.to_nota(),
            self.target_version.to_nota(),
            self.kind.to_nota(),
            RawPayload::new(self.payload.clone()).to_nota(),
        ])
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct DivergencePayload {
    pub component: ComponentName,
    pub source_version: ContractVersion,
    pub target_version: ContractVersion,
    pub reason: DivergenceReason,
    pub kind: RecordKind,
    pub payload: Vec<u8>,
}

impl NotaDecode for DivergencePayload {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let children = NotaBlock::new(block).expect_children(
            Delimiter::Parenthesis,
            "DivergencePayload",
            6,
        )?;
        Ok(Self {
            component: ComponentName::from_nota_block(&children[0])?,
            source_version: ContractVersion::from_nota_block(&children[1])?,
            target_version: ContractVersion::from_nota_block(&children[2])?,
            reason: DivergenceReason::from_nota_block(&children[3])?,
            kind: RecordKind::from_nota_block(&children[4])?,
            payload: RawPayload::from_nota_block(&children[5])?.into_bytes(),
        })
    }
}

impl NotaEncode for DivergencePayload {
    fn to_nota(&self) -> String {
        Delimiter::Parenthesis.wrap([
            self.component.to_nota(),
            self.source_version.to_nota(),
            self.target_version.to_nota(),
            self.reason.to_nota(),
            self.kind.to_nota(),
            RawPayload::new(self.payload.clone()).to_nota(),
        ])
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct RecoveryRequest {
    pub component: ComponentName,
    pub failure_identifier: u64,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct HandoverAcceptance {
    pub accepted_marker: HandoverMarker,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct HandoverFinalization {
    pub finalized_marker: HandoverMarker,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct MirrorAcknowledgement {
    pub component: ComponentName,
    pub write_counter: u64,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct DivergenceAcknowledgement {
    pub component: ComponentName,
    pub divergence_identifier: u64,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct RecoveryResult {
    pub component: ComponentName,
    pub recovered: bool,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEncode, NotaDecode, Debug, Clone, PartialEq, Eq,
)]
pub struct HandoverRejection {
    pub component: ComponentName,
    pub reason: HandoverRejectionReason,
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    NotaEncode,
    NotaDecode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum HandoverRejectionReason {
    SchemaMismatch,
    CommitSequenceAdvanced,
    AlreadyInHandover,
    NotReady,
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    NotaEncode,
    NotaDecode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
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
