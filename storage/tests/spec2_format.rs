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

    let content = serialize_data(&data, false);
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
    // `populated_round_trip::reserialize_is_stable_spec2`.
    let reserialized = serialize_data(&data, false);
    let expected: serde_json::Value = serde_json::from_str(SPEC_COMPLETE_EXAMPLE).unwrap();
    let actual: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(actual, expected);

    // Byte stability of the writer's own output
    let (decoded_again, _caveats) =
        deserialize_data(&reserialized).expect("Reserialized document should decode");
    assert_eq!(decoded_again, data);
    assert_eq!(serialize_data(&decoded_again, false), reserialized);
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
                    "teacher_rotation": { "soft": true },
                    "slot_rotation": null,
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
        DecodeError::MismatchedSpecRequirementInEntry
    );

    let not_needed = document(&[format!(
        r#"{{ "minimum_spec_version": 2, "needed_entry": false, "content": {} }}"#,
        SETTINGS_BLOCK
    )]);
    assert_eq!(
        expect_decode_error(&not_needed),
        DecodeError::MismatchedSpecRequirementInEntry
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
    // No Assignments block and no per-subject slots row: the decoder
    // must rebuild the full derived key sets (one assignments entry per
    // period x non-excluded subject, one slots entry per interrogation
    // subject)
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
    assert_eq!(params.assignments.period_map.len(), 1);
    let period_assignments = params.assignments.period_map.values().next().unwrap();
    assert_eq!(period_assignments.subject_map.len(), 1);
    assert!(
        period_assignments
            .subject_map
            .values()
            .next()
            .unwrap()
            .is_empty()
    );
    assert_eq!(params.slots.subject_map.len(), 1);
    assert!(
        params
            .slots
            .subject_map
            .values()
            .next()
            .unwrap()
            .ordered_slots
            .is_empty()
    );
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
    assert_eq!(
        serialize_data(&bare_data, false),
        serialize_data(&redundant_data, false)
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
    // The spec's id range rule (> 2^63 - 1 is invalid) is enforced at
    // layer 3 with its dedicated error, not by the format structs
    let content = document(&[entry(&format!(
        r#"{{ "Students": [
            {{ "id": {}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [] }}
        ] }}"#,
        u64::MAX
    ))]);

    assert_eq!(expect_decode_error(&content), DecodeError::EndOfTheUniverse);
}
