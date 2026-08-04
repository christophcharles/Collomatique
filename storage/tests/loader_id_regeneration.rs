//! The loader's opt-in id regeneration (`DeserializeOptions::regenerate_ids`)
//!
//! This is the mirror of the writer's toggle (`writer_id_regeneration.rs`),
//! and it exists for one reason: before the writer learned to refuse ids
//! above the format's ceiling, it wrote them silently. Such a file is
//! unreadable by the rules of spec §3, yet it holds a perfectly ordinary
//! document — renumbering its ids from 0 gives it back.
//!
//! What the pass does *not* do matters as much: renumbering is injective,
//! so it repairs nothing else. A duplicated id and a dangling reference
//! are still rejected under the toggle, and the tests below pin that.

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

fn regenerating() -> DeserializeOptions {
    DeserializeOptions {
        regenerate_ids: true,
    }
}

/// Decodes with the toggle on, expecting success and no caveats
fn decode_regenerating(content: &str) -> collomatique_state_colloscopes::Data {
    let (data, caveats) = deserialize_data_with_options(content, &regenerating())
        .expect("The document should decode");
    assert!(caveats.is_empty());
    data
}

/// Decodes with the toggle on, expecting a decode error
fn expect_decode_error_regenerating(content: &str) -> DecodeError {
    let error = deserialize_data_with_options(content, &regenerating())
        .expect_err("Document should be rejected");
    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process, got {error:?}");
    };
    decode_error
}

/// A small document with references crossing three blocks
///
/// Same fixture as the writer's regeneration tests: the four ids are
/// parameters, so the very same document can be written with sparse ids
/// and with the dense ids regeneration is supposed to produce. The
/// teacher names the subject (§4.3) and the student is excluded from the
/// period (§4.4), so a defining id and a referencing one of each kind is
/// exercised.
fn cross_referencing_document(period: u64, subject: u64, teacher: u64, student: u64) -> String {
    document(&[
        entry(&format!(
            r#"{{ "GeneralPlanning": {{
                "first_week": null,
                "periods": [
                    {{ "id": {period}, "weeks": [ {{ "interrogations": true, "annotation": null }} ] }}
                ]
            }} }}"#
        )),
        entry(&format!(
            r#"{{ "Subjects": [
                {{
                    "id": {subject},
                    "name": "Mathématiques",
                    "interrogation_parameters": {{
                        "students_per_group": {{ "min": 1, "max": 2 }},
                        "groups_per_interrogation": {{ "min": 1, "max": 1 }},
                        "duration_minutes": 60,
                        "take_duration_into_account": true,
                        "periodicity": {{ "ExactlyPeriodic": {{ "periodicity_in_weeks": 2 }} }}
                    }},
                    "excluded_periods": []
                }}
            ] }}"#
        )),
        entry(&format!(
            r#"{{ "Teachers": [
                {{ "id": {teacher}, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [{subject}] }}
            ] }}"#
        )),
        entry(&format!(
            r#"{{ "Students": [
                {{ "id": {student}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [{period}] }}
            ] }}"#
        )),
    ])
}

/// A document whose single subject carries the given id
fn subject_document(subject: u64) -> String {
    document(&[entry(&format!(
        r#"{{ "Subjects": [
            {{ "id": {subject}, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }}
        ] }}"#
    ))])
}

#[test]
fn regeneration_rescues_a_file_above_the_id_ceiling() {
    // A file defining an id above 2^63 - 1 is invalid (§3) and the
    // decoder says so — see `object_id_above_the_id_space_is_rejected`.
    // The toggle renumbers before that sweep runs, so the very same file
    // opens, and what comes out is the document it always described: one
    // subject, now numbered 0.
    let content = subject_document(u64::MAX);

    let error = deserialize_data(&content).expect_err("The file is invalid by default");
    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process, got {error:?}");
    };
    assert_eq!(
        decode_error,
        DecodeError::IdAboveCeiling {
            block: "Subjects",
            id: u64::MAX
        }
    );

    let rescued = decode_regenerating(&content);
    let (dense, _caveats) =
        deserialize_data(&subject_document(0)).expect("The dense document should decode");
    assert_eq!(rescued, dense);
}

#[test]
fn sparse_ids_are_renumbered_densely_in_ascending_order() {
    // The four ids of the file are 5, 7, 40 and 100, so the map is
    // 5 -> 0, 7 -> 1, 40 -> 2, 100 -> 3 — and the references follow, or
    // the document would not even reconstruct. The equivalent file
    // written with those dense ids, read the plain way, is the
    // reference: this pins the ordering rule (ascending old value) as
    // well as the renumbering itself.
    let rescued = decode_regenerating(&cross_referencing_document(7, 40, 5, 100));

    let (dense, caveats) = deserialize_data(&cross_referencing_document(1, 2, 0, 3))
        .expect("The dense document should decode");
    assert!(caveats.is_empty());
    assert_eq!(rescued, dense);

    // Read the plain way the sparse file keeps its own ids, so the
    // toggle is what made the difference — nothing else.
    let (plain, _caveats) = deserialize_data(&cross_referencing_document(7, 40, 5, 100))
        .expect("The sparse document is perfectly valid");
    assert_ne!(plain, dense);
}

#[test]
fn a_file_already_numbered_from_zero_reads_the_same_either_way() {
    // On a document whose ids are already 0, 1, 2… the map is the
    // identity, so the toggle changes nothing at all.
    let content = cross_referencing_document(1, 2, 0, 3);

    let (plain, caveats) = deserialize_data(&content).expect("The document should decode");
    assert!(caveats.is_empty());
    assert_eq!(decode_regenerating(&content), plain);
}

#[test]
fn regeneration_does_not_repair_a_duplicated_id_inside_a_block() {
    // Two subjects sharing one id are one id, and renumbering — being
    // injective — leaves them sharing one id. The file is ambiguous, not
    // misnumbered, so it stays rejected. The reported id is the
    // renumbered one (0), which is the documented cost of the toggle.
    let content = document(&[entry(
        r#"{ "Subjects": [
            { "id": 5, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] },
            { "id": 5, "name": "Physique", "interrogation_parameters": null, "excluded_periods": [] }
        ] }"#,
    )]);

    assert_eq!(
        expect_decode_error_regenerating(&content),
        DecodeError::DuplicatedIdInBlock {
            block: "Subjects",
            id: 0
        }
    );
}

#[test]
fn regeneration_does_not_repair_a_duplicated_id_across_blocks() {
    // Same reasoning across two blocks: the subject and the teacher
    // still collide after renumbering.
    let content = document(&[
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
    ]);

    assert_eq!(
        expect_decode_error_regenerating(&content),
        DecodeError::DuplicatedIdAcrossBlocks {
            first: "Subjects",
            second: "Teachers",
            id: 0
        }
    );
}

#[test]
fn regeneration_does_not_repair_a_dangling_reference() {
    // The teacher names a subject no block defines. Renumbering maps
    // that lone id to a number of its own — an id no entity defines
    // either — so the reference still points nowhere and the file stays
    // rejected. Only the id in the message changes: the map is
    // 1 -> 0 (the teacher), 5 -> 1 (the subject), 99 -> 2 (the dangling
    // reference), so the error names 2 rather than the 99 written in the
    // file.
    let content = document(&[
        entry(
            r#"{ "Subjects": [
                { "id": 5, "name": "Mathématiques", "interrogation_parameters": null, "excluded_periods": [] }
            ] }"#,
        ),
        entry(
            r#"{ "Teachers": [
                { "id": 1, "surname": "Rogue", "firstname": "Severus", "tel": null, "email": null, "subjects": [99] }
            ] }"#,
        ),
    ]);

    assert_eq!(
        expect_decode_error_regenerating(&content),
        DecodeError::DanglingReference {
            block: "Teachers",
            row: RowKey::Id(0),
            referenced: IdKind::Subject,
            id: 2
        }
    );
}
