//! Objective weights for the soft (preference-level) balancing constraints.
//!
//! The balancing families encode clustering avoidance as sliding-window soft
//! constraints. Their penalties are combined into the objective via
//! [`crate::helpers::merge_objectified_weighted`], which emits a plain weighted
//! sum `Σ wᵢ·λᵢ` (no `1/n` normalization, no global `L∞` bound). Concentrating
//! the calibration constants here keeps the families consistent.

/// Reference magnitude for a soft balancing penalty.
///
/// Chosen well above `InterrogationCost` (added with coefficient `1.0`, see
/// [`crate::misc::interrogation_cost`]) so that clustering avoidance dominates
/// and `InterrogationCost` acts as a tie-breaker. Kept in the low hundreds for
/// ILP conditioning.
pub(crate) const BASE: f64 = 100.0;

/// Per-window weight for a sliding-window balancing violation.
///
/// The weight is `∝ 1 / (window_size · typical_periodicity)`, scaled by
/// `total_weeks` so it is normalized to roughly [`BASE`]:
///
/// - `1 / window_size` makes a *close* cluster (small window) cost more than a
///   far one, even for a subject with a long typical spacing;
/// - `1 / typical_periodicity` orders the families against one another
///   (avoid-twice ≫ teacher-rotation ≫ year-rotation).
///
/// `window_size` and `typical_periodicity` are clamped to `≥ 1` to avoid
/// division blow-ups on degenerate inputs.
pub(crate) fn window_weight(total_weeks: f64, ws_weeks: f64, typical_periodicity: f64) -> f64 {
    BASE * total_weeks / (ws_weeks.max(1.0) * typical_periodicity.max(1.0))
}
