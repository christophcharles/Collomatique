//! The collections a document is read and written through
//!
//! `docs/python/new_api_design.md` §5: "the collection object *is* the module
//! grouping". `doc.periods` is the general-planning ops, `doc.subjects` will be
//! the subject ones, and so on — one collection per family, reached from the
//! document and never built by hand.
//!
//! A collection is a view, not a copy: it holds the document it came from and
//! reads through it every time, so a script can keep one in a variable and see
//! the writes it makes through it.

pub mod periods;
pub mod students;
pub mod subjects;
pub mod teachers;
pub mod week_patterns;
pub mod weeks;

pub use periods::{Period, Periods};
pub use students::{Student, Students};
pub use subjects::{Interrogation, Subject, Subjects};
pub use teachers::{Teacher, Teachers};
pub use week_patterns::{WeekPattern, WeekPatterns};
pub use weeks::{Week, Weeks};

use pyo3::prelude::*;

use collomatique_state_colloscopes::PersonWithContact;

/// The name of a person, the way the application writes it
///
/// First name then surname, which is what every screen of the gui shows
/// (`gtk4/src/editor/slots/slot_params.rs` and its neighbours) — so a repr
/// naming a teacher or a student names them the way the user is used to
/// reading. It lives here because the model keeps one card for both entities,
/// and both handles flatten it.
pub(crate) fn person_name(desc: &PersonWithContact) -> String {
    format!("{} {}", desc.firstname, desc.surname)
}

/// Adds the collection classes and their handles to the module
///
/// They are registered so `isinstance` and `repr` say something useful, not so
/// a script can build one: none of them has a constructor.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Periods>()?;
    m.add_class::<Period>()?;
    m.add_class::<Weeks>()?;
    m.add_class::<Week>()?;
    m.add_class::<Subjects>()?;
    m.add_class::<Subject>()?;
    m.add_class::<Interrogation>()?;
    m.add_class::<Teachers>()?;
    m.add_class::<Teacher>()?;
    m.add_class::<Students>()?;
    m.add_class::<Student>()?;
    m.add_class::<WeekPatterns>()?;
    m.add_class::<WeekPattern>()?;
    Ok(())
}
