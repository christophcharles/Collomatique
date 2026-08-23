//! The words the application speaks about its own data.
//!
//! The GUI and the scripting api describe the same entity with the same
//! sentence: a warning's text, a panel title and a `repr` all name a week the
//! way [`rendering::render_week`] does. This crate holds that shared
//! vocabulary — the entity renderers that moved out of `collomatique_ops`
//! (whose warning texts are their first caller) and the load-caveat sentences
//! of [`caveats::caveat_text`], which gtk4's caveat dialog and the python
//! module both render. [`solver::conductor_warning_text`] does the same for
//! the sentences the solve dialog writes about a strategy's configuration.

pub mod caveats;
pub mod rendering;
pub mod solver;
