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
pub mod weeks;

pub use periods::{Period, Periods};
pub use weeks::{Week, Weeks};

use pyo3::prelude::*;

/// Adds the collection classes and their handles to the module
///
/// They are registered so `isinstance` and `repr` say something useful, not so
/// a script can build one: none of them has a constructor.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Periods>()?;
    m.add_class::<Period>()?;
    m.add_class::<Weeks>()?;
    m.add_class::<Week>()?;
    Ok(())
}
