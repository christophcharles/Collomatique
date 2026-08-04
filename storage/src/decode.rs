//! Decode submodule
//!
//! This module contains the logic that builds an
//! [InnerData](collomatique_state_colloscopes::InnerData) from a file
//! document via [spec2::decode], the spec-2 pipeline. (Spec 1, the
//! pre-alpha dump format, is permanently retired and rejected before
//! decoding — see the versioning notes in `docs/file_format/file_format.md`.)
//!
//! **Every constraint of the file format is diagnosed here**, while
//! decoding, with a [DecodeError] that names the offending block, row and
//! field in the vocabulary of the file: the id-space rules of spec §3, and
//! every `Constraints:` line of spec §4 — referential ("this id must
//! exist", [DecodeError::DanglingReference]) as well as semantic ("and
//! that subject must have interrogations", one variant per constraint).
//! That is what a user can act on; the in-memory invariant checker, which
//! speaks of the model as a whole rather than of a row, is not a reporter
//! a user could use.
//!
//! Because of that, whatever this module decodes should also pass
//! `collomatique_state_colloscopes::Data::from_inner_data`, the in-memory
//! invariant gate. That is a **contract, not a defence**: no file can
//! reach the gate in a broken state, so a rejection there means this
//! crate built an `InnerData` it had no business building. Decoding no
//! longer runs the gate itself — the callers that need a `Data` do, and
//! the storage test suite runs it on everything it decodes so the two
//! stay in step. Keeping that true is a maintenance obligation, recorded
//! in the module docs of `collomatique_state_colloscopes::invariants`: a
//! new invariant needs a decode-time counterpart here and a rejection
//! test in `storage/tests/spec2_format.rs`.
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
    #[error(
        "Unknown file type - this might be from a more recent version of Collomatique (file written by version {0})"
    )]
    UnknownFileType(Version),
    #[error(
        "Unknown file content - this might be from a more recent version of Collomatique (file written by version {0})"
    )]
    UnknownFileContent(Version),
    #[error("An unknown entry requires a newer version of Collomatique")]
    UnknownNeededEntry(Version),
    #[error("Entry for block {0:?} has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry(&'static str),
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
    #[error("A slot of incompatibility id {incompat_id} crosses midnight")]
    IncompatibilitySlotCrossesMidnight { incompat_id: u64 },
    #[error("The colloscope references an unknown slot id ({0})")]
    UnknownSlotInColloscope(u64),
    #[error("The colloscope interrogation cell (slot id {slot_id}, week {week}) does not exist")]
    InvalidInterrogationCell { slot_id: u64, week: u32 },
    #[error(
        "The colloscope cell (slot id {slot_id}, week {week}) assigns group number {group}, but the associated group list has {group_count} groups"
    )]
    InterrogationGroupOutOfBounds {
        slot_id: u64,
        week: u32,
        group: u32,
        group_count: u32,
    },
    #[error("The colloscope fills group list id {0} which is not an automatic group list")]
    InvalidColloscopeGroupList(u64),
    #[error(
        "The colloscope places student id {student_id} in group list id {group_list_id}, but the list excludes that student"
    )]
    ColloscopeStudentExcluded { group_list_id: u64, student_id: u64 },
    #[error(
        "The colloscope places student id {student_id} of group list id {group_list_id} in group number {group}, but the list has {group_count} groups"
    )]
    ColloscopeStudentGroupOutOfBounds {
        group_list_id: u64,
        student_id: u64,
        group: u32,
        group_count: u32,
    },
    #[error(
        "Group list id {0} has an internally inconsistent filling (prefill group count or duplicated student)"
    )]
    InconsistentGroupList(u64),
    #[error("Pairing rule id {0} has its antecedent and consequent on the same subject")]
    InconsistentPairingRule(u64),
    #[error("Slot pairing rule id {0} has its antecedent and consequent on the same slot")]
    InconsistentSlotPairingRule(u64),
    #[error("Duplicated ID {id} in block {block:?}")]
    DuplicatedIdInBlock { block: &'static str, id: u64 },
    #[error("Block {block:?} defines id {id}, which is above the id ceiling (2^63 - 1)")]
    IdAboveCeiling { block: &'static str, id: u64 },
    #[error("Id {id} is defined in both block {first:?} and block {second:?}")]
    DuplicatedIdAcrossBlocks {
        first: &'static str,
        second: &'static str,
        id: u64,
    },
    #[error(
        "Teacher id {teacher_id} references subject id {subject_id}, which has no interrogations"
    )]
    TeacherSubjectWithoutInterrogations { teacher_id: u64, subject_id: u64 },
    #[error("The assignments reference an unknown period (period id {0})")]
    UnknownPeriodInAssignments(u64),
    #[error("The assignments reference an unknown subject (subject id {0})")]
    UnknownSubjectInAssignments(u64),
    #[error(
        "The assignments have a row for subject id {subject_id} on period id {period_id}, but the subject is excluded from that period"
    )]
    AssignmentOnExcludedPeriod { period_id: u64, subject_id: u64 },
    #[error(
        "Student id {student_id}, assigned in row (period {period_id}, subject {subject_id}), is excluded from that period"
    )]
    AssignedStudentExcludedFromPeriod {
        period_id: u64,
        subject_id: u64,
        student_id: u64,
    },
    #[error("The slots reference an unknown subject (subject id {0})")]
    UnknownSubjectInSlots(u64),
    #[error("The slots have a row for subject id {0} which has no interrogations")]
    SlotsForSubjectWithoutInterrogations(u64),
    #[error(
        "Slot id {slot_id} names teacher id {teacher_id}, who does not teach subject id {subject_id}"
    )]
    SlotTeacherDoesNotTeachSubject {
        slot_id: u64,
        teacher_id: u64,
        subject_id: u64,
    },
    #[error("Slot id {slot_id} plus its subject's interrogation duration crosses midnight")]
    SlotOverflowsDay { slot_id: u64 },
    /// The row named by `row` in block `block` references an id that no
    /// entity of kind `referenced` defines anywhere in the document.
    ///
    /// This is the shared variant for every spec §4 constraint of the
    /// form "every id in X is an existing Y" — the referential half.
    /// Constraints about the *state* of the referenced entity (e.g. "and
    /// that subject has interrogations") have their own per-constraint
    /// variants.
    #[error("Block {block:?}, {row}: references an unknown {referenced} (id {id})")]
    DanglingReference {
        block: &'static str,
        row: RowKey,
        referenced: IdKind,
        id: u64,
    },
    #[error(
        "The group-list association (period {period_id}, subject {subject_id}) names a subject with no interrogations"
    )]
    AssociationForSubjectWithoutInterrogations { period_id: u64, subject_id: u64 },
    #[error(
        "The group-list association (period {period_id}, subject {subject_id}) names a subject excluded from that period"
    )]
    AssociationOnExcludedPeriod { period_id: u64, subject_id: u64 },
    #[error("Pairing rule id {rule_id} names subject id {subject_id}, which has no interrogations")]
    PairingRuleForSubjectWithoutInterrogations { rule_id: u64, subject_id: u64 },
    #[error(
        "Slot pairing rule id {rule_id} pairs slot id {antecedent_slot_id} and slot id {consequent_slot_id}, which belong to different subjects"
    )]
    SlotPairingAcrossSubjects {
        rule_id: u64,
        antecedent_slot_id: u64,
        consequent_slot_id: u64,
    },
    #[error("The balancing options name subject id {subject_id}, which has no interrogations")]
    BalancingForSubjectWithoutInterrogations { subject_id: u64 },
    #[error(
        "Week pattern id {week_pattern_id} has {found} week entries but the schedule has {expected} weeks"
    )]
    WrongWeekCountInWeekPattern {
        week_pattern_id: u64,
        expected: usize,
        found: usize,
    },
}

/// File-vocabulary coordinates of a row inside a block
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKey {
    /// A row keyed by its own id (teachers, students, subjects, slots,
    /// incompatibilities, group lists, pairing rules, settings/balancing
    /// override rows, colloscope group-list rows…)
    Id(u64),
    /// An association row keyed by (period, subject)
    PeriodSubject { period_id: u64, subject_id: u64 },
}

impl std::fmt::Display for RowKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RowKey::Id(id) => write!(f, "row id {id}"),
            RowKey::PeriodSubject {
                period_id,
                subject_id,
            } => write!(f, "row (period {period_id}, subject {subject_id})"),
        }
    }
}

/// The kind of entity a dangling id was supposed to name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    Period,
    Subject,
    Teacher,
    Student,
    WeekPattern,
    Slot,
    GroupList,
}

impl std::fmt::Display for IdKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            IdKind::Period => "period",
            IdKind::Subject => "subject",
            IdKind::Teacher => "teacher",
            IdKind::Student => "student",
            IdKind::WeekPattern => "week pattern",
            IdKind::Slot => "slot",
            IdKind::GroupList => "group list",
        })
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
    // The two header discriminants are tolerated at parse (untagged
    // unknown-value arms) so that an unrecognized one is reported here as
    // itself, rather than as a generic serde failure on the envelope.
    if let FileType::UnknownFileType(_value) = &header.file_type {
        return Err(DecodeError::UnknownFileType(
            header.produced_with_version.clone(),
        ));
    }
    if let FileContent::UnknownFileContent(_value) = &header.file_content {
        return Err(DecodeError::UnknownFileContent(
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
