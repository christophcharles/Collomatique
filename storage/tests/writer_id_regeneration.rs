//! The writer's opt-in id regeneration (`SerializeOptions::regenerate_ids`)
//!
//! Ids are the only thing that ties two blocks of a file together, and
//! their values carry no meaning of their own. So a document can always
//! be written with different ids, as long as every reference follows —
//! which is what regeneration does: renumber densely from 0, in
//! ascending order of the old values.
//!
//! Two properties are worth pinning, and both are here: the pass changes
//! nothing but the numbers (a document already numbered 0, 1, 2… comes
//! out byte for byte identical), and it is the way out of the format's
//! id ceiling.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{NewId, Op, StudentOp, students::Student};
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

fn regenerating() -> SerializeOptions {
    SerializeOptions {
        regenerate_ids: true,
    }
}

/// A small document with references crossing three blocks
///
/// The four ids are given as parameters so the very same document can be
/// written with sparse ids and with the dense ids regeneration is
/// supposed to produce. The teacher names the subject (§4.3) and the
/// student is excluded from the period (§4.4), so both a defining id and
/// a referencing one of each kind is exercised.
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

fn decode(content: &str) -> collomatique_state_colloscopes::Data {
    let (data, caveats) = deserialize_data(content).expect("The document should decode");
    assert!(caveats.is_empty());
    data
}

#[test]
fn a_document_already_numbered_from_zero_is_written_unchanged() {
    // Regeneration renumbers in ascending order of the old ids, so on a
    // document whose ids are already 0, 1, 2… the map is the identity
    // and the file must come out byte for byte the same. This is what
    // makes the toggle safe to switch on: it never rewrites a document
    // that has nothing to gain from it.
    let data = decode(&cross_referencing_document(1, 2, 0, 3));

    let plain = serialize_data(&data).expect("The document should be writable");
    let regenerated =
        serialize_data_with_options(&data, &regenerating()).expect("Regeneration cannot fail");

    assert_eq!(plain, regenerated);
}

#[test]
fn sparse_ids_are_renumbered_densely_in_ascending_order() {
    // The four ids of the document are 5, 7, 40 and 100, so the map is
    // 5 -> 0, 7 -> 1, 40 -> 2, 100 -> 3 — and the references follow, or
    // the file would not even decode. The dense document written the
    // plain way is the reference: the regenerated file must be exactly
    // it, which pins the ordering rule (ascending old value) as well as
    // the renumbering itself.
    let sparse = decode(&cross_referencing_document(7, 40, 5, 100));
    let dense = decode(&cross_referencing_document(1, 2, 0, 3));

    let regenerated =
        serialize_data_with_options(&sparse, &regenerating()).expect("Regeneration cannot fail");
    assert_eq!(
        regenerated,
        serialize_data(&dense).expect("The dense document should be writable")
    );

    // ...and what comes back is the same document as the dense one, ids
    // included: the file is the only thing regeneration touches, so
    // reloading it is how the new ids reach memory.
    assert_eq!(decode(&regenerated), dense);

    // The in-memory document is untouched — written the plain way it
    // still yields its own sparse ids.
    let plain = serialize_data(&sparse).expect("The sparse document should still be writable");
    assert_eq!(decode(&plain), sparse);
    assert_ne!(plain, regenerated);
}

#[test]
fn regeneration_rescues_a_document_past_the_id_ceiling() {
    // A document loaded at the ceiling grows an id above it at the very
    // first entity created (see `writer_id_ceiling.rs`), and is then
    // unwritable. Regeneration is the way out: the document holds two
    // entities, so densely numbered it needs the ids 0 and 1.
    let content = document(&[entry(&format!(
        r#"{{ "Students": [
            {{ "id": {}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [] }}
        ] }}"#,
        u64::MAX >> 1
    ))]);
    let mut state = AppState::<_, String>::new(decode(&content));
    let result = state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add one student past the ceiling".to_string(),
    );
    assert!(matches!(result, Ok(Some(NewId::StudentId(_)))));

    assert_eq!(
        serialize_data(state.get_data()),
        Err(EncodeError::IdAboveCeiling { id: 1 << 63 })
    );

    let rescued = serialize_data_with_options(state.get_data(), &regenerating())
        .expect("Regenerated ids should be back under the ceiling");

    let value: serde_json::Value =
        serde_json::from_str(&rescued).expect("The rescued file should be valid JSON");
    let students = value["entries"]
        .as_array()
        .expect("The entries should be an array")
        .iter()
        .find_map(|entry| entry["content"].get("Students"))
        .expect("The rescued file should still hold its students");
    let ids: Vec<_> = students
        .as_array()
        .expect("The Students block is an array")
        .iter()
        .map(|student| student["id"].as_u64().expect("Ids are numbers"))
        .collect();
    assert_eq!(ids, vec![0, 1]);

    // The rescued file is an ordinary document: it decodes with no
    // caveats and can be written again without any option.
    let reloaded = decode(&rescued);
    assert_eq!(serialize_data(&reloaded).expect("Writable again"), rescued);
}
