//! Test-only helpers shared by the family fixtures.
//!
//! The default base document of the `ops/` fixtures is the frozen hogwarts
//! copy (`tests/fixtures/hogwarts.collomatique`, deliberately decoupled from
//! the living `examples/` file so the example can evolve without touching the
//! tests). Decoding it would be the same five lines in every family that needs
//! a realistic document, so they live here instead; everything else — which
//! teacher, which subject, which slot a fixture reads — stays in the family
//! that reads it, where the name says what it means.

use crate::{CascadeWarning, Desc};
use collomatique_state::AppState;
use collomatique_state_colloscopes::{Data, Fix};

const HOGWARTS: &str = include_str!("../tests/fixtures/hogwarts.collomatique");

/// The frozen base document, decoded and wrapped in a blank-history state.
pub(crate) fn hogwarts() -> AppState<Data, Desc> {
    let (data, caveats) =
        collomatique_storage::deserialize_data(HOGWARTS).expect("the frozen fixture should decode");
    assert!(
        caveats.is_empty(),
        "the frozen fixture should decode cleanly, got {caveats:?}"
    );

    AppState::new(data)
}

/// The repairs a session logged, read back as the [Fix] values a fixture
/// writes down.
///
/// On the way, the parent links are held to their shape: a repair's parent is
/// always a *later* entry of the same list (children land before the repair
/// that needed them). Every composite fixture goes through here, so a
/// composite whose per-op parent indices were not shifted into the
/// composite-wide list would trip this rather than reach the dialog.
pub(crate) fn fixes(warnings: &[CascadeWarning]) -> Vec<Fix> {
    for (i, warning) in warnings.iter().enumerate() {
        if let Some(parent) = warning.parent() {
            assert!(
                i < parent && parent < warnings.len(),
                "a repair's parent must be a later entry of the same warning list \
                 (warning {i} claims parent {parent}, out of {})",
                warnings.len(),
            );
        }
    }
    warnings.iter().map(|w| w.fix().clone()).collect()
}
