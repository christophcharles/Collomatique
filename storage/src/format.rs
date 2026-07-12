//! Spec-2 format structs
//!
//! This module mirrors, field for field, the JSON shapes specified in
//! `docs/file_format.md` (spec version 2). It is layer 1 of the decoding
//! pipeline: serde derives (plus a few custom scalar parsers) enforce the
//! purely *local* validity rules — shapes, scalar encodings, key uniqueness
//! within a collection. Referential rules (dangling ids, derived key sets,
//! global id uniqueness) belong to the later layers.
//!
//! Records deny unknown fields and have no serde defaults: a missing field
//! is a hard error and optional values are an explicit `null`. Keyed
//! collections are sparse and reject duplicate keys (see [keyed]).
//!
//! The `Default` impl of every block type encodes the block's default state
//! (the meaning of the block's absence) as specified in the spec. These
//! defaults are frozen forever; the in-module tests pin them.
#![allow(dead_code)] // temporary: removed when the spec-2 read/write paths land

pub mod keyed;
pub mod scalars;

pub mod assignments;
pub mod balancing;
pub mod colloscope;
pub mod export_config;
pub mod general_planning;
pub mod group_list_associations;
pub mod group_lists;
pub mod incompatibilities;
pub mod pairings;
pub mod settings;
pub mod slot_pairings;
pub mod slots;
pub mod students;
pub mod subjects;
pub mod teachers;
pub mod week_patterns;

use serde::{Deserialize, Serialize};

/// One block payload, externally tagged by its block name
///
/// Serde's external tagging produces exactly the spec encoding for an
/// entry `content`: an object with exactly one key — the block name —
/// whose value is the block payload.
///
/// The variant declaration order is the canonical block order of the
/// spec (§2); the writer relies on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Block {
    GeneralPlanning(general_planning::GeneralPlanning),
    Subjects(subjects::Subjects),
    Teachers(teachers::Teachers),
    Students(students::Students),
    Assignments(assignments::Assignments),
    WeekPatterns(week_patterns::WeekPatterns),
    Slots(slots::Slots),
    Incompatibilities(incompatibilities::Incompatibilities),
    GroupLists(group_lists::GroupLists),
    GroupListAssociations(group_list_associations::GroupListAssociations),
    Pairings(pairings::Pairings),
    SlotPairings(slot_pairings::SlotPairings),
    Settings(settings::Settings),
    Balancing(balancing::Balancing),
    Colloscope(colloscope::Colloscope),
    ExportConfig(export_config::ExportConfig),
}

/// The name of a spec-2 block, without its payload
///
/// The variant declaration order is the canonical block order of the
/// spec (§2), like in [Block].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockName {
    GeneralPlanning,
    Subjects,
    Teachers,
    Students,
    Assignments,
    WeekPatterns,
    Slots,
    Incompatibilities,
    GroupLists,
    GroupListAssociations,
    Pairings,
    SlotPairings,
    Settings,
    Balancing,
    Colloscope,
    ExportConfig,
}

impl BlockName {
    /// All block names in canonical order
    pub const ALL: [BlockName; 16] = [
        BlockName::GeneralPlanning,
        BlockName::Subjects,
        BlockName::Teachers,
        BlockName::Students,
        BlockName::Assignments,
        BlockName::WeekPatterns,
        BlockName::Slots,
        BlockName::Incompatibilities,
        BlockName::GroupLists,
        BlockName::GroupListAssociations,
        BlockName::Pairings,
        BlockName::SlotPairings,
        BlockName::Settings,
        BlockName::Balancing,
        BlockName::Colloscope,
        BlockName::ExportConfig,
    ];

    /// The block name as it appears in the file
    pub fn as_str(self) -> &'static str {
        match self {
            BlockName::GeneralPlanning => "GeneralPlanning",
            BlockName::Subjects => "Subjects",
            BlockName::Teachers => "Teachers",
            BlockName::Students => "Students",
            BlockName::Assignments => "Assignments",
            BlockName::WeekPatterns => "WeekPatterns",
            BlockName::Slots => "Slots",
            BlockName::Incompatibilities => "Incompatibilities",
            BlockName::GroupLists => "GroupLists",
            BlockName::GroupListAssociations => "GroupListAssociations",
            BlockName::Pairings => "Pairings",
            BlockName::SlotPairings => "SlotPairings",
            BlockName::Settings => "Settings",
            BlockName::Balancing => "Balancing",
            BlockName::Colloscope => "Colloscope",
            BlockName::ExportConfig => "ExportConfig",
        }
    }

    /// Recognize a block name as it appears in the file
    ///
    /// Returns `None` for unrecognized names, which are subject to the
    /// forward-compatibility rules (spec §5).
    pub fn from_name(name: &str) -> Option<BlockName> {
        BlockName::ALL.into_iter().find(|b| b.as_str() == name)
    }
}

impl Block {
    /// The name of this block
    pub fn name(&self) -> BlockName {
        match self {
            Block::GeneralPlanning(_) => BlockName::GeneralPlanning,
            Block::Subjects(_) => BlockName::Subjects,
            Block::Teachers(_) => BlockName::Teachers,
            Block::Students(_) => BlockName::Students,
            Block::Assignments(_) => BlockName::Assignments,
            Block::WeekPatterns(_) => BlockName::WeekPatterns,
            Block::Slots(_) => BlockName::Slots,
            Block::Incompatibilities(_) => BlockName::Incompatibilities,
            Block::GroupLists(_) => BlockName::GroupLists,
            Block::GroupListAssociations(_) => BlockName::GroupListAssociations,
            Block::Pairings(_) => BlockName::Pairings,
            Block::SlotPairings(_) => BlockName::SlotPairings,
            Block::Settings(_) => BlockName::Settings,
            Block::Balancing(_) => BlockName::Balancing,
            Block::Colloscope(_) => BlockName::Colloscope,
            Block::ExportConfig(_) => BlockName::ExportConfig,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn block_is_externally_tagged() {
        let block = Block::Settings(settings::Settings::default());

        let value = serde_json::to_value(&block).unwrap();
        let expected = json!({
            "Settings": {
                "global": {
                    "interrogations_per_week_min": null,
                    "interrogations_per_week_max": null,
                    "max_interrogations_per_day": null
                },
                "students": []
            }
        });
        assert_eq!(value, expected);

        let round_tripped: Block = serde_json::from_value(value).unwrap();
        assert_eq!(round_tripped, block);
    }

    #[test]
    fn unknown_block_name_is_rejected() {
        let value = json!({ "NotABlock": {} });
        assert!(serde_json::from_value::<Block>(value).is_err());
    }

    #[test]
    fn block_name_round_trips_through_str() {
        for name in BlockName::ALL {
            assert_eq!(BlockName::from_name(name.as_str()), Some(name));
        }
        assert_eq!(BlockName::from_name("NotABlock"), None);
    }

    #[test]
    fn block_serialized_tag_matches_block_name() {
        let block = Block::Colloscope(colloscope::Colloscope::default());

        let value = serde_json::to_value(&block).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.len(), 1);
        assert!(object.contains_key(block.name().as_str()));
    }
}
