use std::collections::BTreeMap;

use collomatique_state_colloscopes::colloscope_params::Parameters;

use crate::ColloscopeModel;

/// The modelization-relevant half of a solve request: the problem-scoping refinements (which
/// periods and group lists to recompute, and whether to soften cross-period constraints). It is
/// deliberately independent of the problem [`Parameters`] (the "what to solve" half) and of the
/// conductor strategy (a solve-orchestration concern the caller carries alongside). Reconciled
/// against the current [`Parameters`] by [`sanitize`] and fed into [`build_model`].
///
/// [`sanitize`]: SolveConfig::sanitize
/// [`build_model`]: SolveConfig::build_model
#[derive(Debug, Clone)]
pub struct SolveConfig {
    pub periods: BTreeMap<collomatique_state_colloscopes::PeriodId, PeriodSolveData>,
    pub group_lists: BTreeMap<collomatique_state_colloscopes::GroupListId, GroupListSolveData>,
    pub objectify_cross_fixed_period: bool,
}

#[derive(Debug, Clone)]
pub struct PeriodSolveData {
    pub recompute: bool,
    pub use_current_values: bool,
}

impl Default for PeriodSolveData {
    fn default() -> Self {
        PeriodSolveData {
            recompute: true,
            use_current_values: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupListSolveData {
    pub recompute: bool,
    pub previous_values_as_objective: bool,
}

impl Default for GroupListSolveData {
    fn default() -> Self {
        GroupListSolveData {
            recompute: true,
            previous_values_as_objective: false,
        }
    }
}

impl Default for SolveConfig {
    fn default() -> Self {
        SolveConfig {
            periods: BTreeMap::new(),
            group_lists: BTreeMap::new(),
            objectify_cross_fixed_period: true,
        }
    }
}

impl SolveConfig {
    /// Reconcile this config against the parameters it will be solved against, dropping or
    /// adjusting any refinements that no longer apply.
    pub fn sanitize(self, params: &Parameters) -> Self {
        let new_periods: BTreeMap<_, _> = params
            .periods
            .ordered_period_list
            .iter()
            .map(|(id, _)| {
                (
                    id.clone(),
                    match self.periods.get(id) {
                        Some(data) => data.clone(),
                        None => PeriodSolveData::default(),
                    },
                )
            })
            .collect();
        let new_group_lists: BTreeMap<_, _> = params
            .group_lists
            .group_list_map
            .iter()
            .filter_map(|(id, group_list)| {
                if group_list.is_prefilled() {
                    return None;
                }
                Some((
                    id.clone(),
                    match self.group_lists.get(id) {
                        Some(data) => data.clone(),
                        None => GroupListSolveData::default(),
                    },
                ))
            })
            .collect();
        SolveConfig {
            periods: new_periods,
            group_lists: new_group_lists,
            objectify_cross_fixed_period: self.objectify_cross_fixed_period,
        }
    }

    /// Build the ILP model to be solved from `params` and the current `colloscope`, streaming build
    /// log lines through `log`. The caller supplies the real colloscope (rather than an empty one)
    /// so the build can take the current assignments into account.
    pub async fn build_model(
        &self,
        params: &Parameters,
        colloscope: &collomatique_state_colloscopes::colloscopes::Colloscope,
        log: &mut (dyn FnMut(&str) + Send),
    ) -> Result<ColloscopeModel, String> {
        let inner_data = collomatique_state_colloscopes::InnerData {
            params: params.clone(),
            colloscope: colloscope.clone(),
            ..Default::default()
        };
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .map_err(|e| e.to_string())?;
        collomatique_sqlite_state::create_schema(&pool)
            .await
            .map_err(|e| e.to_string())?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data)
            .await
            .map_err(|e| e.to_string())?;
        Ok(crate::build_model_with_log(&pool, log).await)
    }
}
