use super::*;

use pyo3::types::PyDict;

/// The interpreter these tests build their objects in
///
/// `Python::initialize` is idempotent and the tests of a file share a process,
/// so calling it here is enough — no module registration, since `get_type` and
/// the exception classes do not need one.
fn attach<T>(body: impl for<'py> FnOnce(Python<'py>) -> T) -> T {
    Python::initialize();
    Python::attach(body)
}

/// `{name: (payload,)}` — one wrapper variant, the way `payload` writes it
fn wrapping<'py>(py: Python<'py>, name: &str, inner: Bound<'py, PyAny>) -> Bound<'py, PyAny> {
    let dict = PyDict::new(py);
    dict.set_item(
        name,
        PyTuple::new(py, [inner]).expect("a one-element tuple"),
    )
    .expect("a dict takes a string key");
    dict.into_any()
}

/// A family `colloscopes/ops/` grows later still reaches the script, on the base class
///
/// The one case no script can produce, because producing it needs a family the
/// model does not have. It is also the whole of the structural promise: the
/// three levels are read off the shape, so only the *class* falls back — the
/// op, the case and the details come through unchanged.
#[test]
fn a_family_python_does_not_know_falls_back_to_the_base_class() {
    attach(|py| {
        let case = wrapping(py, "NoSuchRoom", 7u64.into_pyobject(py).unwrap().into_any());
        let op = wrapping(py, "AddRoom", case);
        let data = wrapping(py, "Rooms", op);

        let error = from_data(py, "Room ID 7 is invalid".to_string(), &data);
        let value = error.value(py);

        assert!(value.is_instance_of::<UpdateError>());
        assert_eq!(value.str().unwrap().to_string(), "Room ID 7 is invalid");
        assert_eq!(
            value
                .getattr("op")
                .unwrap()
                .extract::<Option<String>>()
                .unwrap(),
            Some("AddRoom".to_string())
        );
        assert_eq!(
            value
                .getattr("case")
                .unwrap()
                .extract::<Option<String>>()
                .unwrap(),
            Some("NoSuchRoom".to_string())
        );
        assert_eq!(
            value
                .getattr("details")
                .unwrap()
                .extract::<(u64,)>()
                .unwrap(),
            (7,)
        );
    });
}

/// A shape the walk cannot follow still carries the model's sentence
///
/// The walk stops where it stops and keeps what it reached: the deeper
/// attributes are `None` rather than a guess, and nothing panics.
#[test]
fn a_shape_the_walk_does_not_know_keeps_the_sentence() {
    attach(|py| {
        let data = 5u64.into_pyobject(py).unwrap().into_any();

        let error = from_data(py, "something went wrong".to_string(), &data);
        let value = error.value(py);

        assert!(value.is_instance_of::<UpdateError>());
        assert_eq!(value.str().unwrap().to_string(), "something went wrong");
        assert!(value.getattr("op").unwrap().is_none());
        assert!(value.getattr("case").unwrap().is_none());
        assert_eq!(
            value.getattr("details").unwrap().extract::<u64>().unwrap(),
            5
        );
    });
}
