//! Spec-2 format tests
//!
//! These tests exercise the spec-2 pipeline against hand-written
//! documents: the complete example of the spec (§6), the entry-level
//! validity rules, the derived-key-set completion, and the placement
//! errors of the sparse colloscope encoding.

use collomatique_storage::*;

fn document(entries: &[String]) -> String {
    format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": {{ "major": 0, "minor": 1, "patch": 0 }},
        "file_content": "Colloscope"
    }},
    "entries": [{}]
}}"#,
        entries.join(",\n")
    )
}

fn entry(content: &str) -> String {
    format!(
        r#"{{ "minimum_spec_version": 2, "needed_entry": true, "content": {} }}"#,
        content
    )
}

fn expect_decode_error(content: &str) -> DecodeError {
    let error = deserialize_data(content).expect_err("Document should be rejected");
    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process, got {error:?}");
    };
    decode_error
}

const SETTINGS_BLOCK: &str = r#"{ "Settings": {
    "global": {
        "interrogations_per_week_min": null,
        "interrogations_per_week_max": { "soft": true, "value": 3 },
        "max_interrogations_per_day": null
    },
    "students": []
} }"#;

/// A minimal scheduling setup: one period of two weeks, one subject
/// with interrogations, one teacher, one slot (id 7)
///
/// The second week's `interrogations` flag and the slot's week pattern
/// (`[true, false]` when enabled) are the two knobs that make weeks
/// inactive for the colloscope placement tests.
fn scheduling_entries(week1_interrogations: bool, with_pattern: bool) -> Vec<String> {
    let mut entries = vec![
        entry(&format!(
            r#"{{ "GeneralPlanning": {{
                "first_week": null,
                "periods": [
                    {{ "id": 1, "weeks": [
                        {{ "interrogations": true, "annotation": null }},
                        {{ "interrogations": {week1_interrogations}, "annotation": null }}
                    ] }}
                ]
            }} }}"#
        )),
        entry(
            r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": {
                        "students_per_group": { "min": 1, "max": 2 },
                        "groups_per_interrogation": { "min": 1, "max": 1 },
                        "duration_minutes": 60,
                        "take_duration_into_account": true,
                        "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                    },
                    "excluded_periods": []
                }
            ] }"#,
        ),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [2] }
            ] }"#,
        ),
    ];
    if with_pattern {
        entries.push(entry(
            r#"{ "WeekPatterns": [ { "id": 6, "name": "Quinzaine", "weeks": [true, false] } ] }"#,
        ));
    }
    let week_pattern_id = if with_pattern { "6" } else { "null" };
    entries.push(entry(&format!(
        r#"{{ "Slots": [
            {{ "subject_id": 2, "slots": [
                {{ "id": 7, "teacher_id": 3, "start": {{ "day": "monday", "time": "14:00" }}, "extra_info": "", "week_pattern_id": {week_pattern_id}, "cost": 0 }}
            ] }}
        ] }}"#
    )));
    entries
}

fn scheduling_document(week1_interrogations: bool, with_pattern: bool, colloscope: &str) -> String {
    let mut entries = scheduling_entries(week1_interrogations, with_pattern);
    entries.push(entry(colloscope));
    document(&entries)
}

#[test]
fn blank_data_serializes_to_zero_blocks() {
    // Every block of a blank document is in default state, so the
    // canonical form omits them all. In particular this pins the
    // equivalence of the in-memory defaults and the frozen format
    // defaults for the blocks with non-trivial ones (Balancing,
    // ExportConfig).
    let data = collomatique_state_colloscopes::Data::new();

    let content = serialize_data(&data);
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("Serialized data should be valid JSON");
    assert_eq!(value["entries"], serde_json::json!([]));

    let (decoded, caveats) = deserialize_data(&content).expect("Blank document should decode");
    assert_eq!(decoded, data);
    assert!(caveats.is_empty());
}

/// The complete example of the spec (§6), verbatim
const SPEC_COMPLETE_EXAMPLE: &str = r#"{
  "header": {
    "file_type": "Collomatique",
    "produced_with_version": { "major": 0, "minor": 1, "patch": 0 },
    "file_content": "Colloscope"
  },
  "entries": [
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GeneralPlanning": {
          "first_week": "2026-08-31",
          "periods": [
            {
              "id": 1,
              "weeks": [
                { "interrogations": true, "annotation": "Rentrée" },
                { "interrogations": true, "annotation": null }
              ]
            }
          ]
        }
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Subjects": [
          {
            "id": 2,
            "name": "Mathématiques",
            "interrogation_parameters": {
              "students_per_group": { "min": 1, "max": 2 },
              "groups_per_interrogation": { "min": 1, "max": 1 },
              "duration_minutes": 60,
              "take_duration_into_account": true,
              "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
            },
            "excluded_periods": []
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Teachers": [
          {
            "id": 3,
            "surname": "Rogue",
            "firstname": "Severus",
            "tel": null,
            "email": "rogue@poudlard.fr",
            "subjects": [2]
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Students": [
          {
            "id": 4,
            "surname": "Potter",
            "firstname": "Harry",
            "tel": "0601020304",
            "email": null,
            "excluded_periods": []
          },
          {
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "tel": null,
            "email": "hermione@poudlard.fr",
            "excluded_periods": []
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Assignments": [
          { "period_id": 1, "subject_id": 2, "students": [4, 5] }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "WeekPatterns": [
          { "id": 6, "name": "Toutes les semaines", "weeks": [true, true] }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Slots": [
          {
            "subject_id": 2,
            "slots": [
              {
                "id": 7,
                "teacher_id": 3,
                "start": { "day": "monday", "time": "14:00" },
                "extra_info": "Salle 101",
                "week_pattern_id": 6,
                "cost": 0
              }
            ]
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GroupLists": [
          {
            "id": 8,
            "name": "Groupes de maths",
            "students_per_group": { "min": 1, "max": 2 },
            "group_names": ["Groupe 1", null],
            "filling": { "Automatic": { "excluded_students": [] } }
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GroupListAssociations": [
          { "period_id": 1, "subject_id": 2, "group_list_id": 8 }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Settings": {
          "global": {
            "interrogations_per_week_min": null,
            "interrogations_per_week_max": { "soft": true, "value": 3 },
            "max_interrogations_per_day": null
          },
          "students": []
        }
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Colloscope": {
          "interrogations": [
            { "slot_id": 7, "week": 0, "assigned_groups": [0] }
          ],
          "group_lists": [
            {
              "group_list_id": 8,
              "students": [
                { "student_id": 4, "group": 0 },
                { "student_id": 5, "group": 0 }
              ]
            }
          ]
        }
      }
    }
  ]
}"#;

#[test]
fn spec_complete_example_decodes_and_reserializes_identically() {
    let (data, caveats) =
        deserialize_data(SPEC_COMPLETE_EXAMPLE).expect("The spec §6 example should decode");
    assert!(caveats.is_empty());

    // The example is in canonical form, so re-serializing must produce
    // the same document. The comparison is on JSON values (the doc
    // displays records more compactly than our pretty-printer does);
    // byte determinism itself is pinned just below and by
    // `populated_round_trip::reserialize_is_stable`.
    let reserialized = serialize_data(&data);
    let expected: serde_json::Value = serde_json::from_str(SPEC_COMPLETE_EXAMPLE).unwrap();
    let actual: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(actual, expected);

    // Byte stability of the writer's own output
    let (decoded_again, _caveats) =
        deserialize_data(&reserialized).expect("Reserialized document should decode");
    assert_eq!(decoded_again, data);
    assert_eq!(serialize_data(&decoded_again), reserialized);
}

#[test]
fn known_block_with_bad_payload_fails_with_serde_detail() {
    let content = document(&[entry(r#"{ "Teachers": [ { "id": 4 } ] }"#)]);

    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Teachers");
    assert!(
        detail.contains("missing field"),
        "The serde diagnostics should be surfaced, got {detail:?}"
    );
}

#[test]
fn subject_with_inverted_range_is_rejected() {
    // An empty (min > max) range in a file must be a hard decode error, never
    // silently repaired: `format::scalars::Range` rejects `min > max` at the
    // serde layer, which is what makes the in-memory `NonEmptyRangeInclusive`
    // non-empty by construction. This pins that contract at the document level.
    let content = document(&[entry(
        r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": {
                        "students_per_group": { "min": 3, "max": 2 },
                        "groups_per_interrogation": { "min": 1, "max": 1 },
                        "duration_minutes": 60,
                        "take_duration_into_account": true,
                        "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                    },
                    "excluded_periods": []
                }
            ] }"#,
    )]);

    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Subjects");
    assert!(
        detail.contains("invalid range"),
        "The serde diagnostics should surface the inverted range, got {detail:?}"
    );
}

#[test]
fn duplicated_block_is_rejected() {
    let content = document(&[entry(SETTINGS_BLOCK), entry(SETTINGS_BLOCK)]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::DuplicatedBlock("Settings")
    );
}

#[test]
fn entry_content_must_be_an_object_with_exactly_one_key() {
    let two_keys = document(&[entry(
        r#"{
            "Colloscope": { "interrogations": [], "group_lists": [] },
            "Balancing": {
                "global": {
                    "teacher_rotation": false,
                    "slot_rotation": false,
                    "avoid_twice_in_a_row": true,
                    "year_teacher_rotation": false,
                    "period_teacher_rotation": false
                },
                "subjects": []
            }
        }"#,
    )]);
    assert_eq!(
        expect_decode_error(&two_keys),
        DecodeError::MalformedEntryContent
    );

    let not_an_object = document(&[entry("42")]);
    assert_eq!(
        expect_decode_error(&not_an_object),
        DecodeError::MalformedEntryContent
    );
}

#[test]
fn known_block_with_non_canonical_envelope_values_is_rejected() {
    let wrong_spec = document(&[format!(
        r#"{{ "minimum_spec_version": 3, "needed_entry": true, "content": {} }}"#,
        SETTINGS_BLOCK
    )]);
    assert_eq!(
        expect_decode_error(&wrong_spec),
        DecodeError::MismatchedSpecRequirementInEntry("Settings")
    );

    let not_needed = document(&[format!(
        r#"{{ "minimum_spec_version": 2, "needed_entry": false, "content": {} }}"#,
        SETTINGS_BLOCK
    )]);
    assert_eq!(
        expect_decode_error(&not_needed),
        DecodeError::MismatchedSpecRequirementInEntry("Settings")
    );
}

#[test]
fn unknown_block_within_supported_spec_is_probably_illformed() {
    let content = document(&[format!(
        r#"{{ "minimum_spec_version": 2, "needed_entry": false, "content": {{ "NotABlock": {{}} }} }}"#
    )]);
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::ProbablyIllformedEntry
    );
}

#[test]
fn colloscope_row_on_unknown_slot_is_rejected() {
    let content = scheduling_document(
        true,
        false,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 99, "week": 0, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::UnknownSlotInColloscope(99)
    );
}

#[test]
fn colloscope_row_on_out_of_range_week_is_rejected() {
    let content = scheduling_document(
        true,
        false,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 7, "week": 5, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::InvalidInterrogationCell {
            slot_id: 7,
            week: 5
        }
    );
}

#[test]
fn colloscope_row_on_non_interrogation_week_is_rejected() {
    // Week 1 has its `interrogations` flag off
    let content = scheduling_document(
        false,
        false,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 7, "week": 1, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::InvalidInterrogationCell {
            slot_id: 7,
            week: 1
        }
    );
}

#[test]
fn colloscope_row_on_week_pattern_off_week_is_rejected() {
    // Week 1 has interrogations, but the slot's week pattern is off
    let content = scheduling_document(
        true,
        true,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 7, "week": 1, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::InvalidInterrogationCell {
            slot_id: 7,
            week: 1
        }
    );
}

#[test]
fn colloscope_row_on_active_cell_decodes() {
    let content = scheduling_document(
        true,
        true,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 7, "week": 0, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );
    let (_data, caveats) = deserialize_data(&content).expect("Active cell should decode");
    assert!(caveats.is_empty());
}

#[test]
fn colloscope_group_list_row_on_unknown_list_is_rejected() {
    let content = document(&[entry(
        r#"{ "Colloscope": {
            "interrogations": [],
            "group_lists": [ { "group_list_id": 8, "students": [] } ]
        } }"#,
    )]);
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::InvalidColloscopeGroupList(8)
    );
}

#[test]
fn colloscope_group_list_row_on_prefilled_list_is_rejected() {
    // A prefilled list carries its composition in GroupLists and never
    // appears in the colloscope
    let content = document(&[
        entry(
            r#"{ "GroupLists": [
                {
                    "id": 8,
                    "name": "Groupes",
                    "students_per_group": { "min": 1, "max": 2 },
                    "group_names": [null],
                    "filling": { "Prefilled": { "groups": [ { "students": [] } ] } }
                }
            ] }"#,
        ),
        entry(
            r#"{ "Colloscope": {
                "interrogations": [],
                "group_lists": [ { "group_list_id": 8, "students": [] } ]
            } }"#,
        ),
    ]);
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::InvalidColloscopeGroupList(8)
    );
}

#[test]
fn derived_key_sets_are_completed() {
    // No Assignments block and no per-subject slots row. Both are sparse, so
    // the skeleton file yields zero assignment rows and zero slots ordering
    // rows: a subject with interrogations but no slots gets no ordering entry
    // (canonical absent).
    let entries = vec![
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [ { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] } ]
            } }"#,
        ),
        entry(
            r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": {
                        "students_per_group": { "min": 1, "max": 2 },
                        "groups_per_interrogation": { "min": 1, "max": 1 },
                        "duration_minutes": 60,
                        "take_duration_into_account": true,
                        "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                    },
                    "excluded_periods": []
                }
            ] }"#,
        ),
    ];
    let content = document(&entries);

    let (data, caveats) = deserialize_data(&content).expect("Document should decode");
    assert!(caveats.is_empty());

    let params = &data.get_inner_data().params;
    assert_eq!(params.assignments.map.len(), 0);
    assert_eq!(params.slots.subjects_with_slots().count(), 0);
}

#[test]
fn neutral_rows_decode_identically_to_their_absence() {
    // Redundant neutral entries in derived-key-set collections are
    // valid and encode the same state as their absence (spec §3)
    let bare = document(&scheduling_entries(true, false));

    let mut entries = scheduling_entries(true, false);
    entries.push(entry(
        r#"{ "Assignments": [ { "period_id": 1, "subject_id": 2, "students": [] } ] }"#,
    ));
    entries.push(entry(
        r#"{ "Colloscope": { "interrogations": [], "group_lists": [] } }"#,
    ));
    let redundant = document(&entries);

    let (bare_data, _caveats) = deserialize_data(&bare).expect("Bare document should decode");
    let (redundant_data, _caveats) =
        deserialize_data(&redundant).expect("Redundant document should decode");
    assert_eq!(bare_data, redundant_data);

    // And the canonical form of both omits the neutral rows
    assert_eq!(serialize_data(&bare_data), serialize_data(&redundant_data));
}

// The four tests below are the counterpart of the one above: a neutral
// entry is the redundant spelling of an absent row only when its key is
// *inside* the derived key set. "Keys outside that set are invalid"
// (§3) whatever the row content, so an empty row must not launder an
// invalid key past decode.

#[test]
fn neutral_slots_row_on_unknown_subject_is_rejected() {
    // The derived key set of §4.7 is the subjects with interrogations;
    // subject 9999 does not exist at all.
    let content = document(&[entry(
        r#"{ "Slots": [ { "subject_id": 9999, "slots": [] } ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::UnknownSubjectInSlots(9999)
    );
}

#[test]
fn neutral_slots_row_on_subject_without_interrogations_is_rejected() {
    // Subject 2 exists but has no interrogations, so it is outside the
    // §4.7 key set just as firmly as an unknown id.
    let content = document(&[
        entry(
            r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": null,
                    "excluded_periods": []
                }
            ] }"#,
        ),
        entry(r#"{ "Slots": [ { "subject_id": 2, "slots": [] } ] }"#),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::SlotsForSubjectWithoutInterrogations(2)
    );
}

#[test]
fn neutral_assignments_row_on_unknown_subject_is_rejected() {
    // The §4.5 key set is (period × subject not excluded from it). The
    // period half of the key is already validated before the neutral-row
    // drop; the subject half is what this pins.
    let content = document(&[
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [ { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] } ]
            } }"#,
        ),
        entry(r#"{ "Assignments": [ { "period_id": 1, "subject_id": 9999, "students": [] } ] }"#),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::UnknownSubjectInAssignments(9999)
    );
}

#[test]
fn neutral_assignments_row_on_excluded_subject_is_rejected() {
    // Subject 2 exists but excludes period 1, so the pair (1, 2) is
    // outside the §4.5 key set.
    let content = document(&[
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [
                    { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] },
                    { "id": 4, "weeks": [ { "interrogations": true, "annotation": null } ] }
                ]
            } }"#,
        ),
        entry(
            r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": null,
                    "excluded_periods": [1]
                }
            ] }"#,
        ),
        entry(r#"{ "Assignments": [ { "period_id": 1, "subject_id": 2, "students": [] } ] }"#),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::AssignmentOnExcludedPeriod {
            period_id: 1,
            subject_id: 2
        }
    );
}

/// Subjects for the §4.11 pairing constraints: 2 has interrogations, 3 and 4
/// do not. Each test below stops at Subjects + Pairings on purpose: its error
/// is raised in `reconstruct`, so the document never has to be complete enough
/// to satisfy the invariant gate.
const PAIRING_SUBJECTS: &str = r#"{ "Subjects": [
    {
        "id": 2,
        "name": "Mathématiques",
        "interrogation_parameters": {
            "students_per_group": { "min": 1, "max": 2 },
            "groups_per_interrogation": { "min": 1, "max": 1 },
            "duration_minutes": 60,
            "take_duration_into_account": true,
            "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
        },
        "excluded_periods": []
    },
    { "id": 3, "name": "Quidditch", "interrogation_parameters": null, "excluded_periods": [] },
    { "id": 4, "name": "Déjeuner", "interrogation_parameters": null, "excluded_periods": [] }
] }"#;

fn pairing_document(rule: &str) -> String {
    document(&[
        entry(PAIRING_SUBJECTS),
        entry(&format!(r#"{{ "Pairings": [ {rule} ] }}"#)),
    ])
}

fn pairing_rule(antecedent: u64, consequent: u64) -> String {
    format!(
        r#"{{ "id": 12,
              "antecedent": {{ "subject_id": {antecedent}, "should_have": true }},
              "consequent": {{ "subject_id": {consequent}, "should_have": true }},
              "excluded_periods": [], "soft": false }}"#
    )
}

/// §4.11 requires both subjects to have interrogations. Subject 3 exists but
/// runs none, so the rule is vacuous and the decoder refuses the file by name,
/// instead of letting the invariant gate report it in model vocabulary.
#[test]
fn pairing_rule_on_a_subject_without_interrogations_is_rejected() {
    assert_eq!(
        expect_decode_error(&pairing_document(&pairing_rule(3, 2))),
        DecodeError::PairingRuleForSubjectWithoutInterrogations {
            rule_id: 12,
            subject_id: 3
        }
    );
}

/// The consequent is checked too, and the subject reported is the offending
/// one, not simply the first part.
#[test]
fn pairing_rule_with_an_uninterrogated_consequent_is_rejected() {
    assert_eq!(
        expect_decode_error(&pairing_document(&pairing_rule(2, 3))),
        DecodeError::PairingRuleForSubjectWithoutInterrogations {
            rule_id: 12,
            subject_id: 3
        }
    );
}

/// §4.11 also requires both subjects to exist. Subject 99 does not.
#[test]
fn pairing_rule_on_an_unknown_subject_is_rejected() {
    assert_eq!(
        expect_decode_error(&pairing_document(&pairing_rule(99, 2))),
        DecodeError::DanglingReference {
            block: "Pairings",
            row: RowKey::Id(12),
            referenced: IdKind::Subject,
            id: 99
        }
    );
}

/// Both parts offend: the antecedent is reported, matching the scan order the
/// `ops` pairing errors already publish, so the two layers blame the same part
/// of the same rule.
#[test]
fn pairing_rule_with_both_parts_uninterrogated_reports_the_antecedent() {
    assert_eq!(
        expect_decode_error(&pairing_document(&pairing_rule(3, 4))),
        DecodeError::PairingRuleForSubjectWithoutInterrogations {
            rule_id: 12,
            subject_id: 3
        }
    );
}

/// The internal seal still runs first: a rule naming one subject twice is
/// incoherent on its own terms, whatever that subject is.
#[test]
fn a_rule_naming_one_uninterrogated_subject_twice_is_still_inconsistent() {
    assert_eq!(
        expect_decode_error(&pairing_document(&pairing_rule(3, 3))),
        DecodeError::InconsistentPairingRule(12)
    );
}

/// A document with a single period of seven weeks and one week pattern
/// whose bitmask has `week_count` entries
///
/// The spec (§4.6) requires exactly one entry per week of the schedule,
/// so only `week_count == 7` is well-formed here.
fn seven_week_document_with_pattern_of_length(week_count: usize) -> String {
    let weeks = vec![r#"{ "interrogations": true, "annotation": null }"#; 7].join(", ");
    let bits = vec!["true"; week_count].join(", ");
    document(&[
        entry(&format!(
            r#"{{ "GeneralPlanning": {{
                "first_week": null,
                "periods": [ {{ "id": 1, "weeks": [{weeks}] }} ]
            }} }}"#
        )),
        entry(&format!(
            r#"{{ "WeekPatterns": [ {{ "id": 6, "name": "Quinzaine", "weeks": [{bits}] }} ] }}"#
        )),
    ])
}

#[test]
fn week_pattern_matching_the_schedule_decodes() {
    // The control: the exact-length bitmask is the well-formed shape.
    let content = seven_week_document_with_pattern_of_length(7);

    let (_data, caveats) = deserialize_data(&content).expect("Exact-length pattern should decode");
    assert!(caveats.is_empty());
}

#[test]
fn week_pattern_shorter_than_the_schedule_is_rejected() {
    // Decode zips the bitmask against the walk order, so the missing
    // bits silently default to active.
    let content = seven_week_document_with_pattern_of_length(1);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::WrongWeekCountInWeekPattern {
            week_pattern_id: 6,
            expected: 7,
            found: 1
        }
    );
}

#[test]
fn week_pattern_longer_than_the_schedule_is_rejected() {
    // The surplus bits vanish in the zip. Since the in-memory type keeps
    // only the exclusion set (no length), nothing downstream can catch
    // this — decode is the only place it can be seen.
    let content = seven_week_document_with_pattern_of_length(8);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::WrongWeekCountInWeekPattern {
            week_pattern_id: 6,
            expected: 7,
            found: 8
        }
    );
}

#[test]
fn incompatibility_slot_crossing_midnight_is_rejected() {
    let entries = vec![
        entry(
            r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": {
                        "students_per_group": { "min": 1, "max": 2 },
                        "groups_per_interrogation": { "min": 1, "max": 1 },
                        "duration_minutes": 60,
                        "take_duration_into_account": true,
                        "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                    },
                    "excluded_periods": []
                }
            ] }"#,
        ),
        entry(
            r#"{ "Incompatibilities": [
                {
                    "id": 9,
                    "subject_id": 2,
                    "name": "Option latin",
                    "slots": [ { "day": "monday", "time": "23:30", "duration_minutes": 60 } ],
                    "minimum_free_slots": 1,
                    "week_pattern_id": null
                }
            ] }"#,
        ),
    ];
    let content = document(&entries);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::SlotCrossesMidnight
    );
}

#[test]
fn object_id_above_the_id_space_is_rejected() {
    // The spec's id range rule (§3: ids above 2^63 - 1 make the file
    // invalid) is enforced by the decoder's id sweep, naming the block
    let content = document(&[entry(&format!(
        r#"{{ "Students": [
            {{ "id": {}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [] }}
        ] }}"#,
        u64::MAX
    ))]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::IdAboveCeiling {
            block: "Students",
            id: u64::MAX
        }
    );
}

#[test]
fn object_id_exactly_at_the_ceiling_is_accepted() {
    // 2^63 - 1 is the last legal id (§3): the ceiling check is strict
    let content = document(&[entry(&format!(
        r#"{{ "Students": [
            {{ "id": {}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [] }}
        ] }}"#,
        u64::MAX >> 1
    ))]);

    let (_data, caveats) = deserialize_data(&content).expect("boundary id should decode");
    assert!(caveats.is_empty());
}

#[test]
fn duplicate_id_across_blocks_is_rejected() {
    // §3: an id value is defined at most once across the whole file, not
    // just within its own block. The error names both defining blocks, in
    // canonical block order.
    let entries = vec![
        entry(
            r#"{ "Subjects": [
                { "id": 3, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [] }
            ] }"#,
        ),
    ];
    let content = document(&entries);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::DuplicatedIdAcrossBlocks {
            first: "Subjects",
            second: "Teachers",
            id: 3
        }
    );
}

#[test]
fn duplicate_slot_id_across_subjects_is_rejected() {
    // A slot id shared by two subjects must be rejected explicitly. Since
    // the flat slot-table restructure (phase B commit 3) the slots are
    // keyed by id, so collapsing two rows onto the same id would silently
    // drop a slot; decode detects the duplicate instead.
    let subject = |id: u64, name: &str| {
        format!(
            r#"{{ "id": {id}, "name": "{name}", "interrogation_parameters": {{
                "students_per_group": {{ "min": 1, "max": 2 }},
                "groups_per_interrogation": {{ "min": 1, "max": 1 }},
                "duration_minutes": 60,
                "take_duration_into_account": true,
                "periodicity": {{ "ExactlyPeriodic": {{ "periodicity_in_weeks": 2 }} }}
            }}, "excluded_periods": [] }}"#
        )
    };
    let entries = vec![
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [
                    { "id": 1, "weeks": [
                        { "interrogations": true, "annotation": null },
                        { "interrogations": true, "annotation": null }
                    ] }
                ]
            } }"#,
        ),
        entry(&format!(
            r#"{{ "Subjects": [ {}, {} ] }}"#,
            subject(2, "Mathématiques"),
            subject(4, "Physique")
        )),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [2, 4] }
            ] }"#,
        ),
        // Both subject rows reference the same slot id 8.
        entry(
            r#"{ "Slots": [
                { "subject_id": 2, "slots": [
                    { "id": 8, "teacher_id": 3, "start": { "day": "monday", "time": "14:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] },
                { "subject_id": 4, "slots": [
                    { "id": 8, "teacher_id": 3, "start": { "day": "tuesday", "time": "15:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] }
            ] }"#,
        ),
    ];
    let content = document(&entries);
    assert_eq!(
        expect_decode_error(&content),
        DecodeError::DuplicatedIdInBlock {
            block: "Slots",
            id: 8
        }
    );
}

// The two tests below are the siblings of the one above for the other
// blocks whose row keys are read straight from the file: the diagnostic
// names the block and the offending id, not just "duplicated ID".

#[test]
fn duplicate_period_id_names_its_block() {
    let content = document(&[entry(
        r#"{ "GeneralPlanning": {
            "first_week": null,
            "periods": [
                { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] },
                { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] }
            ]
        } }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::DuplicatedIdInBlock {
            block: "GeneralPlanning",
            id: 1
        }
    );
}

#[test]
fn duplicate_subject_id_names_its_block() {
    let content = document(&[entry(
        r#"{ "Subjects": [
            { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] },
            { "id": 2, "name": "Physique", "interrogation_parameters": null, "excluded_periods": [] }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        DecodeError::DuplicatedIdInBlock {
            block: "Subjects",
            id: 2
        }
    );
}

// Integer widths (§3 of the spec). Apart from ids, every integer field
// of the format is 32 bits wide: unsigned 0..=2^32 - 1 (or 1..=2^32 - 1
// where a minimum of 1 is stated), and the one signed field — a slot's
// `cost` — is -2^31..=2^31 - 1. The format structs carry those widths,
// so an out-of-width value is a plain serde failure and comes out as
// `IllformedBlock` naming its block, like an inverted range does. The
// four tests below pin one value per width family.

/// The scheduling setup of [scheduling_entries], with the slot's `cost`
/// written verbatim (so a test can put an out-of-width literal there)
fn document_with_slot_cost(cost: &str) -> String {
    let mut entries = scheduling_entries(true, false);
    entries.pop().expect("The Slots entry is pushed last");
    entries.push(entry(&format!(
        r#"{{ "Slots": [
            {{ "subject_id": 2, "slots": [
                {{ "id": 7, "teacher_id": 3, "start": {{ "day": "monday", "time": "14:00" }}, "extra_info": "", "week_pattern_id": null, "cost": {cost} }}
            ] }}
        ] }}"#
    )));
    document(&entries)
}

#[test]
fn slot_cost_out_of_signed_32_bits_is_rejected() {
    // Both boundaries of the signed 32-bit range decode...
    for cost in ["-2147483648", "2147483647"] {
        let content = document_with_slot_cost(cost);
        let (_data, caveats) = deserialize_data(&content)
            .unwrap_or_else(|error| panic!("Cost {cost} should decode, got {error:?}"));
        assert!(caveats.is_empty());
    }

    // ... and one step past the top does not.
    let content = document_with_slot_cost("2147483648");
    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Slots");
    assert!(
        detail.contains("2147483648"),
        "The serde diagnostics should surface the out-of-width cost, got {detail:?}"
    );
}

#[test]
fn duration_out_of_32_bits_is_rejected() {
    // The `0` end of the duration range is pinned by the `scalars` unit
    // test `duration_is_a_positive_number_of_minutes`; this is the top end.
    let content = document(&[entry(
        r#"{ "Subjects": [
                {
                    "id": 2,
                    "name": "Mathématiques",
                    "interrogation_parameters": {
                        "students_per_group": { "min": 1, "max": 2 },
                        "groups_per_interrogation": { "min": 1, "max": 1 },
                        "duration_minutes": 4294967296,
                        "take_duration_into_account": true,
                        "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                    },
                    "excluded_periods": []
                }
            ] }"#,
    )]);

    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Subjects");
    assert!(
        detail.contains("4294967296"),
        "The serde diagnostics should surface the out-of-width duration, got {detail:?}"
    );
}

#[test]
fn settings_limit_value_out_of_32_bits_is_rejected() {
    let content = document(&[entry(
        r#"{ "Settings": {
            "global": {
                "interrogations_per_week_min": null,
                "interrogations_per_week_max": { "soft": true, "value": 4294967296 },
                "max_interrogations_per_day": null
            },
            "students": []
        } }"#,
    )]);

    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Settings");
    assert!(
        detail.contains("4294967296"),
        "The serde diagnostics should surface the out-of-width limit, got {detail:?}"
    );
}

// Dangling references (spec §4). Every "id in this field must be an
// existing X" constraint is checked while decoding and reported through
// the shared `DanglingReference` variant, naming the block, the row and
// the kind of entity the id failed to name. Each fixture below is the
// smallest document that reaches its check: the error is raised in
// `reconstruct`, so the document never has to be complete enough to
// satisfy the invariant gate.

/// A subject row with interrogation parameters, for the fixtures that
/// need a subject inside a derived key set
fn subject_with_interrogations(id: u64, name: &str) -> String {
    format!(
        r#"{{ "id": {id}, "name": "{name}", "interrogation_parameters": {{
            "students_per_group": {{ "min": 1, "max": 2 }},
            "groups_per_interrogation": {{ "min": 1, "max": 1 }},
            "duration_minutes": 60,
            "take_duration_into_account": true,
            "periodicity": {{ "ExactlyPeriodic": {{ "periodicity_in_weeks": 2 }} }}
        }}, "excluded_periods": [] }}"#
    )
}

fn dangling(block: &'static str, row: RowKey, referenced: IdKind, id: u64) -> DecodeError {
    DecodeError::DanglingReference {
        block,
        row,
        referenced,
        id,
    }
}

#[test]
fn subject_excluded_period_must_exist() {
    let content = document(&[entry(
        r#"{ "Subjects": [
            { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [99] }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Subjects", RowKey::Id(2), IdKind::Period, 99)
    );
}

#[test]
fn teacher_subject_must_exist() {
    let content = document(&[entry(
        r#"{ "Teachers": [
            { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [99] }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Teachers", RowKey::Id(3), IdKind::Subject, 99)
    );
}

#[test]
fn student_excluded_period_must_exist() {
    let content = document(&[entry(
        r#"{ "Students": [
            { "id": 4, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [99] }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Students", RowKey::Id(4), IdKind::Period, 99)
    );
}

#[test]
fn assigned_student_must_exist() {
    let content = document(&[
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [ { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] } ]
            } }"#,
        ),
        entry(
            r#"{ "Subjects": [
                { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(r#"{ "Assignments": [ { "period_id": 1, "subject_id": 2, "students": [99] } ] }"#),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling(
            "Assignments",
            RowKey::PeriodSubject {
                period_id: 1,
                subject_id: 2
            },
            IdKind::Student,
            99
        )
    );
}

#[test]
fn slot_teacher_must_exist() {
    let content = document(&[
        entry(&format!(
            r#"{{ "Subjects": [ {} ] }}"#,
            subject_with_interrogations(2, "Mathématiques")
        )),
        entry(
            r#"{ "Slots": [
                { "subject_id": 2, "slots": [
                    { "id": 7, "teacher_id": 99, "start": { "day": "monday", "time": "14:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Slots", RowKey::Id(7), IdKind::Teacher, 99)
    );
}

#[test]
fn slot_week_pattern_must_exist() {
    let content = document(&[
        entry(&format!(
            r#"{{ "Subjects": [ {} ] }}"#,
            subject_with_interrogations(2, "Mathématiques")
        )),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [2] }
            ] }"#,
        ),
        entry(
            r#"{ "Slots": [
                { "subject_id": 2, "slots": [
                    { "id": 7, "teacher_id": 3, "start": { "day": "monday", "time": "14:00" }, "extra_info": "", "week_pattern_id": 99, "cost": 0 }
                ] }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Slots", RowKey::Id(7), IdKind::WeekPattern, 99)
    );
}

#[test]
fn incompatibility_subject_must_exist() {
    let content = document(&[entry(
        r#"{ "Incompatibilities": [
            {
                "id": 9,
                "subject_id": 99,
                "name": "Option latin",
                "slots": [],
                "minimum_free_slots": 1,
                "week_pattern_id": null
            }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Incompatibilities", RowKey::Id(9), IdKind::Subject, 99)
    );
}

#[test]
fn incompatibility_week_pattern_must_exist() {
    let content = document(&[
        entry(
            r#"{ "Subjects": [
                { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(
            r#"{ "Incompatibilities": [
                {
                    "id": 9,
                    "subject_id": 2,
                    "name": "Option latin",
                    "slots": [],
                    "minimum_free_slots": 1,
                    "week_pattern_id": 99
                }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Incompatibilities", RowKey::Id(9), IdKind::WeekPattern, 99)
    );
}

#[test]
fn prefilled_group_student_must_exist() {
    let content = document(&[entry(
        r#"{ "GroupLists": [
            {
                "id": 8,
                "name": "Groupes",
                "students_per_group": { "min": 1, "max": 2 },
                "group_names": [null],
                "filling": { "Prefilled": { "groups": [ { "students": [99] } ] } }
            }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("GroupLists", RowKey::Id(8), IdKind::Student, 99)
    );
}

#[test]
fn automatic_excluded_student_must_exist() {
    let content = document(&[entry(
        r#"{ "GroupLists": [
            {
                "id": 8,
                "name": "Groupes",
                "students_per_group": { "min": 1, "max": 2 },
                "group_names": [null],
                "filling": { "Automatic": { "excluded_students": [99] } }
            }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("GroupLists", RowKey::Id(8), IdKind::Student, 99)
    );
}

/// A one-entry GroupLists block for the association fixtures: automatic,
/// no exclusions, a single unnamed group
const SIMPLE_GROUP_LIST: &str = r#"{ "GroupLists": [
    {
        "id": 8,
        "name": "Groupes",
        "students_per_group": { "min": 1, "max": 2 },
        "group_names": [null],
        "filling": { "Automatic": { "excluded_students": [] } }
    }
] }"#;

#[test]
fn association_period_must_exist() {
    let content = document(&[
        entry(
            r#"{ "Subjects": [
                { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(SIMPLE_GROUP_LIST),
        entry(
            r#"{ "GroupListAssociations": [ { "period_id": 99, "subject_id": 2, "group_list_id": 8 } ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling(
            "GroupListAssociations",
            RowKey::PeriodSubject {
                period_id: 99,
                subject_id: 2
            },
            IdKind::Period,
            99
        )
    );
}

#[test]
fn association_subject_must_exist() {
    let content = document(&[
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [ { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] } ]
            } }"#,
        ),
        entry(SIMPLE_GROUP_LIST),
        entry(
            r#"{ "GroupListAssociations": [ { "period_id": 1, "subject_id": 99, "group_list_id": 8 } ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling(
            "GroupListAssociations",
            RowKey::PeriodSubject {
                period_id: 1,
                subject_id: 99
            },
            IdKind::Subject,
            99
        )
    );
}

#[test]
fn association_group_list_must_exist() {
    let content = document(&[
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [ { "id": 1, "weeks": [ { "interrogations": true, "annotation": null } ] } ]
            } }"#,
        ),
        entry(
            r#"{ "Subjects": [
                { "id": 2, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(
            r#"{ "GroupListAssociations": [ { "period_id": 1, "subject_id": 2, "group_list_id": 99 } ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling(
            "GroupListAssociations",
            RowKey::PeriodSubject {
                period_id: 1,
                subject_id: 2
            },
            IdKind::GroupList,
            99
        )
    );
}

#[test]
fn pairing_rule_excluded_period_must_exist() {
    let content = document(&[
        entry(&format!(
            r#"{{ "Subjects": [ {}, {} ] }}"#,
            subject_with_interrogations(2, "Mathématiques"),
            subject_with_interrogations(20, "Physique")
        )),
        entry(
            r#"{ "Pairings": [
                { "id": 12,
                  "antecedent": { "subject_id": 2, "should_have": true },
                  "consequent": { "subject_id": 20, "should_have": true },
                  "excluded_periods": [99], "soft": false }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Pairings", RowKey::Id(12), IdKind::Period, 99)
    );
}

#[test]
fn slot_pairing_slot_must_exist() {
    let content = document(&[entry(
        r#"{ "SlotPairings": [
            { "id": 14,
              "antecedent": { "slot_id": 99, "should_have": true },
              "consequent": { "slot_id": 98, "should_have": true },
              "excluded_periods": [], "soft": false }
        ] }"#,
    )]);

    // The antecedent is checked first, matching the pairings convention
    assert_eq!(
        expect_decode_error(&content),
        dangling("SlotPairings", RowKey::Id(14), IdKind::Slot, 99)
    );
}

#[test]
fn slot_pairing_excluded_period_must_exist() {
    let content = document(&[
        entry(&format!(
            r#"{{ "Subjects": [ {} ] }}"#,
            subject_with_interrogations(2, "Mathématiques")
        )),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [2] }
            ] }"#,
        ),
        entry(
            r#"{ "Slots": [
                { "subject_id": 2, "slots": [
                    { "id": 7, "teacher_id": 3, "start": { "day": "monday", "time": "14:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 },
                    { "id": 8, "teacher_id": 3, "start": { "day": "tuesday", "time": "14:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] }
            ] }"#,
        ),
        entry(
            r#"{ "SlotPairings": [
                { "id": 14,
                  "antecedent": { "slot_id": 7, "should_have": true },
                  "consequent": { "slot_id": 8, "should_have": true },
                  "excluded_periods": [99], "soft": false }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("SlotPairings", RowKey::Id(14), IdKind::Period, 99)
    );
}

#[test]
fn settings_override_student_must_exist() {
    let content = document(&[entry(
        r#"{ "Settings": {
            "global": {
                "interrogations_per_week_min": null,
                "interrogations_per_week_max": null,
                "max_interrogations_per_day": null
            },
            "students": [
                { "student_id": 99, "limits": {
                    "interrogations_per_week_min": null,
                    "interrogations_per_week_max": null,
                    "max_interrogations_per_day": null
                } }
            ]
        } }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Settings", RowKey::Id(99), IdKind::Student, 99)
    );
}

#[test]
fn balancing_override_subject_must_exist() {
    let content = document(&[entry(
        r#"{ "Balancing": {
            "global": {
                "teacher_rotation": false,
                "slot_rotation": false,
                "avoid_twice_in_a_row": true,
                "year_teacher_rotation": false,
                "period_teacher_rotation": false
            },
            "subjects": [
                { "subject_id": 99, "options": {
                    "teacher_rotation": true,
                    "slot_rotation": false,
                    "avoid_twice_in_a_row": true,
                    "year_teacher_rotation": false,
                    "period_teacher_rotation": false
                } }
            ]
        } }"#,
    )]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Balancing", RowKey::Id(99), IdKind::Subject, 99)
    );
}

#[test]
fn colloscope_placed_student_must_exist() {
    let content = document(&[
        entry(SIMPLE_GROUP_LIST),
        entry(
            r#"{ "Colloscope": {
                "interrogations": [],
                "group_lists": [
                    { "group_list_id": 8, "students": [ { "student_id": 99, "group": 0 } ] }
                ]
            } }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error(&content),
        dangling("Colloscope", RowKey::Id(8), IdKind::Student, 99)
    );
}

/// The control for the whole family: a document exercising every
/// reference field checked above, with every id resolving, decodes with
/// no errors and no caveats.
#[test]
fn document_with_all_references_resolving_decodes() {
    let entries = vec![
        entry(
            r#"{ "GeneralPlanning": {
                "first_week": null,
                "periods": [
                    { "id": 1, "weeks": [
                        { "interrogations": true, "annotation": null },
                        { "interrogations": true, "annotation": null }
                    ] },
                    { "id": 10, "weeks": [ { "interrogations": true, "annotation": null } ] }
                ]
            } }"#,
        ),
        entry(&format!(
            r#"{{ "Subjects": [
                {{ "id": 2, "name": "Mathématiques", "interrogation_parameters": {{
                    "students_per_group": {{ "min": 1, "max": 2 }},
                    "groups_per_interrogation": {{ "min": 1, "max": 1 }},
                    "duration_minutes": 60,
                    "take_duration_into_account": true,
                    "periodicity": {{ "ExactlyPeriodic": {{ "periodicity_in_weeks": 2 }} }}
                }}, "excluded_periods": [10] }},
                {}
            ] }}"#,
            subject_with_interrogations(20, "Physique")
        )),
        entry(
            r#"{ "Teachers": [
                { "id": 3, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [2, 20] }
            ] }"#,
        ),
        entry(
            r#"{ "Students": [
                { "id": 4, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [10] },
                { "id": 5, "surname": "Granger", "firstname": "Hermione", "tel": null, "email": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(r#"{ "Assignments": [ { "period_id": 1, "subject_id": 2, "students": [4, 5] } ] }"#),
        entry(
            r#"{ "WeekPatterns": [ { "id": 6, "name": "Toutes les semaines", "weeks": [true, true, true] } ] }"#,
        ),
        entry(
            r#"{ "Slots": [
                { "subject_id": 2, "slots": [
                    { "id": 7, "teacher_id": 3, "start": { "day": "monday", "time": "14:00" }, "extra_info": "", "week_pattern_id": 6, "cost": 0 },
                    { "id": 8, "teacher_id": 3, "start": { "day": "tuesday", "time": "14:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] },
                { "subject_id": 20, "slots": [
                    { "id": 21, "teacher_id": 3, "start": { "day": "friday", "time": "10:00" }, "extra_info": "", "week_pattern_id": null, "cost": 0 }
                ] }
            ] }"#,
        ),
        entry(
            r#"{ "Incompatibilities": [
                {
                    "id": 9,
                    "subject_id": 2,
                    "name": "Option latin",
                    "slots": [ { "day": "monday", "time": "08:00", "duration_minutes": 60 } ],
                    "minimum_free_slots": 1,
                    "week_pattern_id": 6
                }
            ] }"#,
        ),
        entry(
            r#"{ "GroupLists": [
                {
                    "id": 11,
                    "name": "Groupes de maths",
                    "students_per_group": { "min": 1, "max": 2 },
                    "group_names": ["Groupe 1", null],
                    "filling": { "Automatic": { "excluded_students": [5] } }
                },
                {
                    "id": 13,
                    "name": "Groupes fixes",
                    "students_per_group": { "min": 1, "max": 2 },
                    "group_names": [null, null],
                    "filling": { "Prefilled": { "groups": [ { "students": [4] }, { "students": [5] } ] } }
                }
            ] }"#,
        ),
        entry(
            r#"{ "GroupListAssociations": [ { "period_id": 1, "subject_id": 2, "group_list_id": 11 } ] }"#,
        ),
        entry(
            r#"{ "Pairings": [
                { "id": 12,
                  "antecedent": { "subject_id": 2, "should_have": true },
                  "consequent": { "subject_id": 20, "should_have": true },
                  "excluded_periods": [10], "soft": true }
            ] }"#,
        ),
        entry(
            r#"{ "SlotPairings": [
                { "id": 14,
                  "antecedent": { "slot_id": 7, "should_have": true },
                  "consequent": { "slot_id": 8, "should_have": true },
                  "excluded_periods": [10], "soft": true }
            ] }"#,
        ),
        entry(
            r#"{ "Settings": {
                "global": {
                    "interrogations_per_week_min": null,
                    "interrogations_per_week_max": null,
                    "max_interrogations_per_day": null
                },
                "students": [
                    { "student_id": 4, "limits": {
                        "interrogations_per_week_min": null,
                        "interrogations_per_week_max": null,
                        "max_interrogations_per_day": null
                    } }
                ]
            } }"#,
        ),
        entry(
            r#"{ "Balancing": {
                "global": {
                    "teacher_rotation": false,
                    "slot_rotation": false,
                    "avoid_twice_in_a_row": true,
                    "year_teacher_rotation": false,
                    "period_teacher_rotation": false
                },
                "subjects": [
                    { "subject_id": 2, "options": {
                        "teacher_rotation": true,
                        "slot_rotation": false,
                        "avoid_twice_in_a_row": true,
                        "year_teacher_rotation": false,
                        "period_teacher_rotation": false
                    } }
                ]
            } }"#,
        ),
        entry(
            r#"{ "Colloscope": {
                "interrogations": [ { "slot_id": 7, "week": 0, "assigned_groups": [0] } ],
                "group_lists": [
                    { "group_list_id": 11, "students": [ { "student_id": 4, "group": 0 } ] }
                ]
            } }"#,
        ),
    ];
    let content = document(&entries);

    let (_data, caveats) =
        deserialize_data(&content).expect("Document with resolving references should decode");
    assert!(caveats.is_empty());
}

#[test]
fn colloscope_week_out_of_32_bits_is_rejected() {
    // Distinct from `colloscope_row_on_out_of_range_week_is_rejected`,
    // which uses an in-width week past the end of the schedule and is
    // caught later, by the placement check.
    let content = scheduling_document(
        true,
        false,
        r#"{ "Colloscope": {
            "interrogations": [ { "slot_id": 7, "week": 4294967296, "assigned_groups": [] } ],
            "group_lists": []
        } }"#,
    );

    let error = expect_decode_error(&content);
    let DecodeError::IllformedBlock { block, detail } = error else {
        panic!("The error should be IllformedBlock, got {error:?}");
    };
    assert_eq!(block, "Colloscope");
    assert!(
        detail.contains("4294967296"),
        "The serde diagnostics should surface the out-of-width week, got {detail:?}"
    );
}
