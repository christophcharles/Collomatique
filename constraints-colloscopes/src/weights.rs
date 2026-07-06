//! Objective weights for the soft (preference-level) balancing constraints.
//!
//! The soft balancing families (teacher / slot rotation) encode "spread evenly"
//! as an L1 cumulative-deviation objective, combined into the global objective via
//! [`crate::helpers::merge_objectified_weighted`] (a plain weighted sum `Σ wᵢ·λᵢ`,
//! no `1/n` normalization, no global `L∞` bound). The soft limits/pairings families
//! use [`crate::helpers::merge_objectified`] and are scaled to [`BASE`] as well.
//! Concentrating the calibration constant here keeps the families consistent.

/// Reference magnitude for a soft balancing penalty.
///
/// Chosen well above `InterrogationCost` (added with coefficient `1.0`, see
/// [`crate::misc::interrogation_cost`]) so that balancing dominates and
/// `InterrogationCost` acts as a tie-breaker. Kept in the low hundreds for ILP
/// conditioning. The rotation families weight each `λᵢ` by `BASE / n` (where `n`
/// is the subject's active-week count) so a subject contributes `BASE·Σ|dᵢ|`
/// independent of the year length.
pub(crate) const BASE: f64 = 100.0;
