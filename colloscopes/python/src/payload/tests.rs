use super::*;

use collomatique_ops::{
    CutPeriodError, GeneralPlanningUpdateError, MergeWithPreviousPeriodError, UpdateError,
};
use collomatique_state::ids::Id;

/// The interpreter these tests build their objects in
///
/// `Python::initialize` is idempotent and the tests of a file share a process,
/// so calling it here is enough — the walk mints id classes, which `Bound::new`
/// does without the module being registered anywhere.
fn attach<T>(body: impl for<'py> FnOnce(Python<'py>) -> T) -> T {
    Python::initialize();
    Python::attach(body)
}

/// What `{name: payload}` holds, for a name the test expects
fn under<'py>(data: &Bound<'py, PyAny>, name: &str) -> Bound<'py, PyAny> {
    let dict = data
        .cast::<PyDict>()
        .unwrap_or_else(|_| panic!("a variant is a one-key dict, not {data}"));

    assert_eq!(dict.len(), 1, "a variant carries one key");
    dict.get_item(name)
        .expect("looking a key up should not fail")
        .unwrap_or_else(|| panic!("the variant should be `{name}`, not {dict}"))
}

/// The one element of a wrapper variant's payload
fn only<'py>(payload: &Bound<'py, PyAny>) -> Bound<'py, PyAny> {
    let tuple = payload
        .cast::<PyTuple>()
        .unwrap_or_else(|_| panic!("a payload is a tuple, not {payload}"));

    assert_eq!(tuple.len(), 1, "a wrapper variant wraps one thing");
    tuple.get_item(0).expect("a one-element tuple has an item")
}

/// An id in a refusal reaches python as an id, not as a number
///
/// The whole reason the walk is a serializer of its own: `serde_json` would
/// have written `4` here, and a script cannot look anything up with a `4`.
#[test]
fn an_id_keeps_its_class_through_the_walk() {
    attach(|py| {
        // Not an id any document handed out — the walk never reaches one, it
        // only rebuilds what the model named.
        let period = unsafe { <collomatique_state_colloscopes::PeriodId as Id>::new(4) };
        let error = UpdateError::GeneralPlanning(GeneralPlanningUpdateError::CutPeriod(
            CutPeriodError::InvalidPeriodId(period),
        ));

        let data = to_py(py, &error).expect("the model's errors serialize");

        let family = only(&under(&data, "GeneralPlanning"));
        let op = only(&under(&family, "CutPeriod"));
        let case = under(&op, "InvalidPeriodId");

        let id = only(&case);
        assert!(id.is_instance_of::<crate::ids::PeriodId>());
        assert_eq!(id.repr().unwrap().to_string(), "<PeriodId 4>");
    });
}

/// A case that carries nothing is its own name, and nothing else
#[test]
fn a_case_that_carries_nothing_is_its_name() {
    attach(|py| {
        let error =
            UpdateError::GeneralPlanning(GeneralPlanningUpdateError::MergeWithPreviousPeriod(
                MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith,
            ));

        let data = to_py(py, &error).expect("the model's errors serialize");

        let family = only(&under(&data, "GeneralPlanning"));
        let op = only(&under(&family, "MergeWithPreviousPeriod"));

        assert_eq!(
            op.extract::<String>().expect("a unit case is a string"),
            "NoPreviousPeriodToMergeWith"
        );
    });
}

/// The whole-value argument a repair carries for the op does not reach python
///
/// Thirteen `Fix` variants carry a `rebuilt` entity beside their coordinates,
/// so `FixOp::to_annotated_op` can stay a pure translation. It is the model's
/// storage shape, and none of the twenty-five french sentences reads it — a
/// script gets the coordinates and reads the entity itself off the document the
/// write just left.
#[test]
fn a_rebuilt_payload_does_not_reach_python() {
    attach(|py| {
        let teacher_id = unsafe { <collomatique_state_colloscopes::TeacherId as Id>::new(2) };
        let subject_id = unsafe { <collomatique_state_colloscopes::SubjectId as Id>::new(3) };
        let fix = collomatique_state_colloscopes::Fix::RemoveTeacherSubject {
            teacher: teacher_id,
            subject: subject_id,
            rebuilt: collomatique_state_colloscopes::teachers::Teacher::default(),
        };

        let (kind, details) = repair(py, &fix).expect("the model's repairs serialize");

        assert_eq!(kind.as_deref(), Some("RemoveTeacherSubject"));

        let details = details
            .cast::<PyDict>()
            .expect("a repair names its coordinates by field name");
        assert!(
            !details.contains(REBUILT).expect("a lookup should not fail"),
            "the rebuilt teacher should not be among the coordinates, but got {details}"
        );
        assert_eq!(details.len(), 2, "the two coordinates, and nothing else");
        assert_eq!(
            details
                .get_item("teacher")
                .expect("a lookup should not fail")
                .expect("the repair names the teacher")
                .repr()
                .unwrap()
                .to_string(),
            "<TeacherId 2>"
        );
        assert_eq!(
            details
                .get_item("subject")
                .expect("a lookup should not fail")
                .expect("the repair names the subject")
                .repr()
                .unwrap()
                .to_string(),
            "<SubjectId 3>"
        );
    });
}

/// A repair that carries only coordinates comes through as it is
///
/// The other half of the rule: dropping `rebuilt` takes nothing else with it,
/// and an id among the coordinates keeps its class the way a refusal's does.
#[test]
fn a_repair_that_carries_only_coordinates_is_unchanged() {
    attach(|py| {
        let slot_id = unsafe { <collomatique_state_colloscopes::SlotId as Id>::new(7) };
        let fix = collomatique_state_colloscopes::Fix::DeleteSlot { slot: slot_id };

        let (kind, details) = repair(py, &fix).expect("the model's repairs serialize");

        assert_eq!(kind.as_deref(), Some("DeleteSlot"));

        let details = details
            .cast::<PyDict>()
            .expect("a repair names its coordinates by field name");
        assert_eq!(details.len(), 1, "the one coordinate this repair has");

        let slot = details
            .get_item("slot")
            .expect("a lookup should not fail")
            .expect("the repair names the slot");
        assert!(slot.is_instance_of::<crate::ids::SlotId>());
        assert_eq!(slot.repr().unwrap().to_string(), "<SlotId 7>");
    });
}
