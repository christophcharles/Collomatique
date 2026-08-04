//! The writer's id-ceiling check (spec §3)
//!
//! Ids are capped at 2^63 - 1 by the file format, not by the in-memory
//! model: the id issuer hands out numbers without an upper bound. So a
//! perfectly valid document can hold an id the format cannot express,
//! and the writer has to say so instead of writing a file no reader
//! would accept.

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

/// A document whose only entity is a student carrying the very last
/// legal id
fn document_at_the_ceiling() -> String {
    document(&[entry(&format!(
        r#"{{ "Students": [
            {{ "id": {}, "surname": "Potter", "firstname": "Harry", "tel": null, "email": null, "excluded_periods": [] }}
        ] }}"#,
        u64::MAX >> 1
    ))])
}

#[test]
fn a_document_at_the_ceiling_is_written_back() {
    // 2^63 - 1 is a legal id, so this document is writable — and its
    // file is the same one it came from. This is the writer-side twin of
    // the decoder's `object_id_exactly_at_the_ceiling_is_accepted`.
    let content = document_at_the_ceiling();
    let (data, caveats) = deserialize_data(&content).expect("A boundary document should decode");
    assert!(caveats.is_empty());

    let serialized = serialize_data(&data).expect("A document at the ceiling should be writable");
    assert!(serialized.contains(&(u64::MAX >> 1).to_string()));

    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("The written document should decode");
    assert_eq!(decoded, data);
}

#[test]
fn one_edit_past_the_ceiling_makes_the_document_unwritable() {
    // The id issuer resumes just above the largest id of the file, and
    // has no upper bound of its own: the very next entity created here
    // gets 2^63, one past what the format can hold. Before this check
    // the writer produced that file silently, and no reader would take
    // it back.
    let content = document_at_the_ceiling();
    let (data, _caveats) = deserialize_data(&content).expect("A boundary document should decode");

    let mut state = AppState::<_, String>::new(data);
    let result = state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add one student past the ceiling".to_string(),
    );
    assert!(matches!(result, Ok(Some(NewId::StudentId(_)))));

    assert_eq!(
        serialize_data(state.get_data()),
        Err(EncodeError::IdAboveCeiling { id: 1 << 63 })
    );
}
