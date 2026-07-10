use crate::{InternalVar, Model, Var};
use collomatique_ilp::UsableData;
use std::collections::HashMap;

/// Assign each base variable an epoch for the incremental (staggered) solve: every `StudentGroup`
/// variable is solved first (epoch 0), then each `GroupInInterrogation` variable in the epoch
/// matching its week (week + 1), so the schedule fills in week by week on top of the fixed group
/// assignment. Base variables absent from the map fall into the strategy's final epoch.
///
/// Generic over the extra-variable and constraint-description spaces so it applies equally to a
/// [`ColloscopeModel`] and a [`ConfiguredColloscopeModel`]: it only ever inspects base variables.
///
/// [`ColloscopeModel`]: crate::ColloscopeModel
/// [`ConfiguredColloscopeModel`]: crate::ConfiguredColloscopeModel
pub fn build_incremental_epochs<E: UsableData, C: UsableData>(
    model: &Model<Var, E, C>,
) -> HashMap<InternalVar<Var, E>, u32> {
    let mut epochs = HashMap::new();
    for v in model.problem().get_variables().keys() {
        if let InternalVar::Base(base) = v {
            let epoch = match base {
                Var::StudentGroup { .. } => 0u32,
                Var::GroupInInterrogation { week, .. } => week.0 as u32,
            };
            epochs.insert(v.clone(), epoch);
        }
    }
    epochs
}
