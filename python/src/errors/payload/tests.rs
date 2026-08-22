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
