//! Decode submodule
//!
//! This module contains the logic that builds a [Data] from a file
//! document via [spec2::decode], the spec-2 pipeline. (Spec 1, the
//! pre-alpha dump format, is permanently retired and rejected before
//! decoding — see the versioning notes in `docs/file_format.md`.)
//!
//! Decoding is never trusted for semantic integrity: it funnels through
//! [Data::from_inner_data], the single trust boundary that revalidates
//! any [InnerData](collomatique_state_colloscopes::InnerData) regardless
//! of provenance. A decoder that happens to catch a problem earlier is a
//! convenience, not a guarantee.
//!
//! Diagnostics ([DecodeError]) distinguish an *unrecognised* block
//! (handled by the forward-compatibility rules — a [Caveat] or
//! [DecodeError::UnknownNeededEntry]) from a *recognised block with a
//! bad payload* ([DecodeError::IllformedBlock], which carries the serde
//! diagnostics); the latter is never silently swallowed.

use super::*;
use crate::json::*;

/// Error type when decoding the JSON structure
///
/// This error type describes error that happen when interpreting the file content.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("Unknown file type - this might be from a more recent version of Collomatique")]
    UnknownFileType(Version),
    #[error("An unknown entry requires a newer version of Collomatique")]
    UnknownNeededEntry(Version),
    #[error("An entry has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry,
    #[error("An entry is probably ill-formed (and thus not recognized)")]
    ProbablyIllformedEntry,
    #[error("An entry's content should be an object with exactly one key (the block name)")]
    MalformedEntryContent,
    #[error("Block {0:?} appears more than once")]
    DuplicatedBlock(&'static str),
    #[error("Block {block:?} is ill-formed: {detail}")]
    IllformedBlock {
        block: &'static str,
        /// The rendered serde diagnostics (field name, expected type,
        /// position relative to the block's entry content)
        detail: String,
    },
    #[error("An incompatibility slot crosses midnight")]
    SlotCrossesMidnight,
    #[error("The colloscope references an unknown slot id ({0})")]
    UnknownSlotInColloscope(u64),
    #[error("The colloscope interrogation cell (slot id {slot_id}, week {week}) does not exist")]
    InvalidInterrogationCell { slot_id: u64, week: u32 },
    #[error("The colloscope fills group list id {0} which is not an automatic group list")]
    InvalidColloscopeGroupList(u64),
    #[error(
        "Group list id {0} has an internally inconsistent filling (prefill group count or duplicated student)"
    )]
    InconsistentGroupList(u64),
    #[error("Pairing rule id {0} has its antecedent and consequent on the same subject")]
    InconsistentPairingRule(u64),
    #[error("Slot pairing rule id {0} has its antecedent and consequent on the same slot")]
    InconsistentSlotPairingRule(u64),
    #[error("generating new IDs is not secure, half the usable IDs have been used already")]
    EndOfTheUniverse,
    #[error("Duplicated ID")]
    DuplicatedID,
    #[error("The assignments reference an unknown period (period id {0})")]
    UnknownPeriodInAssignments(u64),
    #[error("The assignments reference an unknown subject (subject id {0})")]
    UnknownSubjectInAssignments(u64),
    #[error(
        "The assignments have a row for subject id {subject_id} on period id {period_id}, but the subject is excluded from that period"
    )]
    AssignmentOnExcludedPeriod { period_id: u64, subject_id: u64 },
    #[error("The slots reference an unknown subject (subject id {0})")]
    UnknownSubjectInSlots(u64),
    #[error("The slots have a row for subject id {0} which has no interrogations")]
    SlotsForSubjectWithoutInterrogations(u64),
    #[error(
        "Week pattern id {week_pattern_id} has {found} week entries but the schedule has {expected} weeks"
    )]
    WrongWeekCountInWeekPattern {
        week_pattern_id: u64,
        expected: usize,
        found: usize,
    },
    #[error("The loaded data is logically impossible: {0:?}")]
    LogicError(BTreeSet<collomatique_state_colloscopes::LogicError>),
    #[error("The loaded data breaks an invariant: {0:?}")]
    BrokenInvariants(BTreeSet<collomatique_state_colloscopes::FixableInvariant>),
}

impl From<collomatique_state_colloscopes::FromInnerDataError> for DecodeError {
    fn from(value: collomatique_state_colloscopes::FromInnerDataError) -> Self {
        use collomatique_state::tools::IdError;
        use collomatique_state_colloscopes::FromInnerDataError;
        match value {
            FromInnerDataError::IdError(id_error) => match id_error {
                IdError::DuplicatedId => DecodeError::DuplicatedID,
                IdError::EndOfTheUniverse => DecodeError::EndOfTheUniverse,
            },
            FromInnerDataError::Logic(set) => DecodeError::LogicError(set),
            FromInnerDataError::BrokenInvariants(set) => DecodeError::BrokenInvariants(set),
        }
    }
}

/// Caveats type
///
/// A file can be successfully decoded though not all information was
/// decoded successfully. This can happen for instance if we try to
/// open a file from a more recent version of Collomatique that has
/// some extra structures.
///
/// This type enumerates possible caveats that were encountered while decoding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Caveat {
    /// The file was opened but it was created with a newer version
    /// of Collomatique
    CreatedWithNewerVersion(Version),
    /// Unknown entries
    ///
    /// Some entries are unknown. They are maarked as unneeded,
    /// so the file can be decoded without them. But some information
    /// might be missing and it is preferable to use a newer version
    /// of Collomatique.
    UnknownEntries,
}

pub(crate) fn check_header(
    header: &Header,
    caveats: &mut BTreeSet<Caveat>,
) -> Result<(), DecodeError> {
    if let FileContent::UnknownFileContent(_value) = &header.file_content {
        return Err(DecodeError::UnknownFileType(
            header.produced_with_version.clone(),
        ));
    }
    if header.produced_with_version > Version::current() {
        caveats.insert(Caveat::CreatedWithNewerVersion(
            header.produced_with_version.clone(),
        ));
    }
    Ok(())
}

pub(crate) mod spec2;
