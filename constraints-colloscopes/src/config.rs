use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::Infallible;

use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::ids::GroupListId;
use ordered_float::OrderedFloat;

use collomatique_ilp::Constraint;
use collomatique_ilp_modeler::{
    ConstraintBundle, ConstraintSource, InternalVar, Modeler, Var as ModelerVar,
};

use crate::ConfiguredColloscopeModel;
use crate::ids::GlobalWeek;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::Var;

/// Weight of the soft L1 "keep the current value" anchor objectives. Configurable weights
/// are a future refinement; for now both weights are fixed module-level constants.
const L1_ANCHOR_WEIGHT: f64 = 1000.0;
/// Weight of the objectified cross-fixed-period constraints (soft, one independent penalty
/// per objectified constraint).
const CROSS_PERIOD_WEIGHT: f64 = 1000.0;

/// Extra-variable name space of a [`ConfiguredColloscopeModel`]: the base model's
/// [`ExtraVarName`]s plus the penalty variables introduced by the configuration.
///
/// The anchor penalties are deliberately split per week and per group list (rather than one
/// pooled penalty) because the incremental solver performs far better when the soft
/// objectives are independent across weeks/epochs.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConfiguredExtra {
    /// A base-model extra variable, unchanged.
    Inner(ExtraVarName),
    /// L1 penalty for the "keep current values" anchors of one week's `GroupInInterrogation`
    /// variables.
    AnchorWeek(GlobalWeek),
    /// L1 penalty for the "keep current values" anchors of one automatic group list's
    /// `StudentGroup` variables.
    AnchorGroupList(GroupListId),
    /// L1 penalty for one objectified cross-fixed-period constraint (indexed by its position
    /// in the stored set, so every such constraint is penalized independently).
    CrossPeriod(usize),
}

/// Constraint-description space of a [`ConfiguredColloscopeModel`]: the base model's
/// [`ConstraintDesc`]s plus the pin/anchor equalities introduced by the configuration.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConfiguredConstraintDesc {
    /// A base-model constraint, unchanged.
    Inner(ConstraintDesc),
    /// A `var == value` equality: either a hard pin of a non-recomputed variable or the
    /// (objectified) anchor of a recomputed opt-in variable to its current value.
    Fixed { var: Var, value: OrderedFloat<f64> },
}

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

    /// The [`PeriodSolveData`] governing a `GroupInInterrogation` variable of the given global
    /// week, defaulting to [`PeriodSolveData::default`] (recompute everything) when the week
    /// falls outside any period or the period carries no explicit config.
    fn period_data_for_week(&self, params: &Parameters, week: usize) -> PeriodSolveData {
        match crate::tools::week_to_period_id(params, week) {
            Some((period_id, _)) => self.periods.get(&period_id).cloned().unwrap_or_default(),
            None => PeriodSolveData::default(),
        }
    }

    /// The [`GroupListSolveData`] governing a `StudentGroup` variable of the given group list,
    /// defaulting to [`GroupListSolveData::default`] when the group list carries no explicit
    /// config.
    fn group_list_data(&self, group_list: &GroupListId) -> GroupListSolveData {
        self.group_lists
            .get(group_list)
            .cloned()
            .unwrap_or_default()
    }

    /// Whether a base variable belongs to the recomputed part of the problem. Non-recomputed
    /// variables are pinned to their current value; recomputed ones are free (possibly softly
    /// anchored).
    fn var_is_recompute(&self, params: &Parameters, v: &Var) -> bool {
        match v {
            Var::StudentGroup { group_list, .. } => self.group_list_data(group_list).recompute,
            Var::GroupInInterrogation { week, .. } => {
                self.period_data_for_week(params, week.0).recompute
            }
        }
    }

    /// Classify a user constraint from the set of base variables its footprint touches. See
    /// [`ConstraintClass`] for the meaning of each outcome.
    fn classify_constraint(
        &self,
        params: &Parameters,
        footprint: &HashSet<Var>,
    ) -> ConstraintClass {
        let touches_gii = footprint
            .iter()
            .any(|v| matches!(v, Var::GroupInInterrogation { .. }));

        if touches_gii {
            // Interrogation-touching constraint: the group-list scoping of any incidental
            // `StudentGroup` variable is irrelevant; only the periods matter.
            let mut has_fixed = false;
            let mut has_x = false;
            for v in footprint {
                if let Var::GroupInInterrogation { week, .. } = v {
                    let data = self.period_data_for_week(params, week.0);
                    if !data.recompute {
                        has_fixed = true;
                        if !data.use_current_values {
                            has_x = true;
                        }
                    }
                }
            }
            if !has_fixed {
                ConstraintClass::Keep
            } else if has_x {
                ConstraintClass::Drop
            } else if self.objectify_cross_fixed_period {
                ConstraintClass::Store
            } else {
                ConstraintClass::Keep
            }
        } else {
            // Pure `StudentGroup` (or empty) constraint: drop it as soon as any of its group
            // lists is fixed, otherwise keep it.
            let drop = footprint.iter().any(|v| match v {
                Var::StudentGroup { group_list, .. } => !self.group_list_data(group_list).recompute,
                _ => false,
            });
            if drop {
                ConstraintClass::Drop
            } else {
                ConstraintClass::Keep
            }
        }
    }

    /// Build the ILP model to be solved from the current database `pool`, streaming build log
    /// lines through `log`.
    ///
    /// The base ("initial") model is built in full, then filtered down to the constraints and
    /// objective terms that concern the recomputed part of the problem, non-recomputed
    /// variables are pinned to their current colloscope values, recomputed opt-in variables are
    /// softly anchored to those values, and cross-fixed-period constraints are optionally
    /// objectified rather than dropped.
    pub async fn build_model(
        &self,
        pool: &sqlx::SqlitePool,
        log: &mut (dyn FnMut(&str) + Send),
    ) -> Result<ConfiguredColloscopeModel, String> {
        log("--- Building initial model ---");
        log("");

        let base = crate::build_model_with_log(pool, log).await;

        let inner = collomatique_sqlite_state::sqlite_to_inner_data(pool)
            .await
            .map_err(|e| e.to_string())?;
        let params = inner.params;
        let colloscope = inner.colloscope;
        let complete = crate::convert::build_complete_config(&params, &colloscope);

        log("");
        log("--- Configuring reduced model ---");
        log("");

        // 1. Filter the base model down to the recomputed part, stashing the cross-fixed-period
        //    constraints to be objectified (in modeler-facing `Var<Var, ExtraVarName>` form).
        let graph = base.dependency_graph();
        let mut stored: Vec<(Constraint<ModelerVar<Var, ExtraVarName>>, ConstraintDesc)> =
            Vec::new();
        let mut kept = 0usize;
        let mut dropped = 0usize;
        let filtered = base
            .filter(
                |c, desc| {
                    let footprint = graph.constraint_footprint(c);
                    match self.classify_constraint(&params, &footprint) {
                        ConstraintClass::Keep => {
                            kept += 1;
                            true
                        }
                        ConstraintClass::Drop => {
                            dropped += 1;
                            false
                        }
                        ConstraintClass::Store => {
                            stored.push((c.clone(), desc.clone()));
                            false
                        }
                    }
                },
                // Keep every base variable: this preserves the full solution space (and keeps
                // `ConfigData` reconstruction happy); unreferenced extras are shed by `build`.
                |_b| true,
                |_e| true,
                // Drop objective terms that depend only on non-recomputed variables.
                |v| {
                    graph
                        .var_footprint(v)
                        .iter()
                        .any(|b| self.var_is_recompute(&params, b))
                },
            )
            .map_err(|e| format!("failed to filter configured model: {e:?}"))?;

        log(&format!(
            "  Constraints: {kept} kept, {dropped} dropped, {} stored for objectification",
            stored.len()
        ));

        // 2. Re-model in the wrapped variable/description space.
        let wrapped = filtered
            .transmute(wrap_var, wrap_source)
            .map_err(|e| format!("failed to wrap configured problem: {e:?}"))?;
        let mut modeler: Modeler<Var, ConfiguredExtra, ConfiguredConstraintDesc, (), Infallible> =
            Modeler::from_model_problem(&wrapped);

        // 3. Pin every non-recomputed variable to its current value (hard `var == value`).
        let pins: HashMap<Var, f64> = complete
            .get_values()
            .into_iter()
            .filter(|(v, _)| !self.var_is_recompute(&params, v))
            .collect();
        let pin_count = pins.len();
        if !pins.is_empty() {
            let bundle: ConstraintBundle<
                Var,
                ConfiguredExtra,
                ConfiguredConstraintDesc,
                (),
                Infallible,
            > = ConstraintBundle::from_config_data(&pins.into(), |v, value| {
                ConfiguredConstraintDesc::Fixed {
                    var: v.clone(),
                    value: OrderedFloat(value),
                }
            });
            modeler
                .apply_bundle(bundle)
                .map_err(|e| format!("failed to apply pin bundle: {e:?}"))?;
        }
        log(&format!("  Pinned {pin_count} non-recomputed variables"));

        // 4. Softly anchor recomputed opt-in variables to their current value, with penalties
        //    kept independent per week (GroupInInterrogation) and per group list (StudentGroup).
        let mut anchor_weeks: BTreeMap<GlobalWeek, HashMap<Var, f64>> = BTreeMap::new();
        let mut anchor_group_lists: BTreeMap<GroupListId, HashMap<Var, f64>> = BTreeMap::new();
        for (v, value) in complete.get_values() {
            match &v {
                Var::GroupInInterrogation { week, .. } => {
                    let data = self.period_data_for_week(&params, week.0);
                    if data.recompute && data.use_current_values {
                        anchor_weeks.entry(*week).or_default().insert(v, value);
                    }
                }
                Var::StudentGroup { group_list, .. } => {
                    let data = self.group_list_data(group_list);
                    if data.recompute && data.previous_values_as_objective {
                        anchor_group_lists
                            .entry(*group_list)
                            .or_default()
                            .insert(v, value);
                    }
                }
            }
        }

        let anchor_week_count = anchor_weeks.len();
        let anchor_group_list_count = anchor_group_lists.len();
        for (week, subset) in anchor_weeks {
            self.apply_anchor(&mut modeler, ConfiguredExtra::AnchorWeek(week), subset)?;
        }
        for (group_list, subset) in anchor_group_lists {
            self.apply_anchor(
                &mut modeler,
                ConfiguredExtra::AnchorGroupList(group_list),
                subset,
            )?;
        }
        log(&format!(
            "  Anchored current values across {anchor_week_count} week(s) and {anchor_group_list_count} group list(s)"
        ));

        // 5. Objectify the stored cross-fixed-period constraints, one independent penalty each.
        let stored_count = stored.len();
        for (index, (constraint, desc)) in stored.into_iter().enumerate() {
            let mapped = constraint.transmute(|v| match v {
                ModelerVar::Base(b) => ModelerVar::Base(b.clone()),
                ModelerVar::Extra(e) => ModelerVar::Extra(ConfiguredExtra::Inner(e.clone())),
            });
            let bundle: ConstraintBundle<
                Var,
                ConfiguredExtra,
                ConfiguredConstraintDesc,
                (),
                Infallible,
            > = ConstraintBundle::from_constraints([(
                mapped,
                ConfiguredConstraintDesc::Inner(desc),
            )]);
            let objectified = bundle
                .objectify_with_balance_and_coef(
                    ConfiguredExtra::CrossPeriod(index),
                    0.0,
                    CROSS_PERIOD_WEIGHT,
                )
                .map_err(|e| format!("failed to objectify cross-period constraint: {e:?}"))?
                .into_general();
            modeler
                .apply_bundle(objectified)
                .map_err(|e| format!("failed to apply cross-period bundle: {e:?}"))?;
        }
        log(&format!(
            "  Objectified {stored_count} cross-fixed-period constraint(s)"
        ));

        log("");
        log("--- Building final model ---");
        log("");

        modeler
            .build_with_log(&(), log)
            .map_err(|e| format!("failed to build configured model: {e:?}"))
    }

    /// Objectify a subset of `var == current_value` anchors into a single L1 penalty variable
    /// `penalty`, weighted so each anchored variable carries [`L1_ANCHOR_WEIGHT`].
    fn apply_anchor(
        &self,
        modeler: &mut Modeler<Var, ConfiguredExtra, ConfiguredConstraintDesc, (), Infallible>,
        penalty: ConfiguredExtra,
        subset: HashMap<Var, f64>,
    ) -> Result<(), String> {
        if subset.is_empty() {
            return Ok(());
        }
        // `objectify` with `alpha = 0` averages the per-constraint slacks (penalty = mean), so
        // scale the coefficient by the count to recover a unit-weight sum of `L1_ANCHOR_WEIGHT`
        // per anchored variable.
        let count = subset.len() as f64;
        let bundle: ConstraintBundle<
            Var,
            ConfiguredExtra,
            ConfiguredConstraintDesc,
            (),
            Infallible,
        > = ConstraintBundle::from_config_data(&subset.into(), |v, value| {
            ConfiguredConstraintDesc::Fixed {
                var: v.clone(),
                value: OrderedFloat(value),
            }
        });
        let objectified = bundle
            .objectify_with_balance_and_coef(penalty, 0.0, L1_ANCHOR_WEIGHT * count)
            .map_err(|e| format!("failed to objectify anchor bundle: {e:?}"))?
            .into_general();
        modeler
            .apply_bundle(objectified)
            .map_err(|e| format!("failed to apply anchor bundle: {e:?}"))
    }
}

/// Outcome of classifying a user constraint against a [`SolveConfig`].
enum ConstraintClass {
    /// Keep the constraint in the reduced model.
    Keep,
    /// Drop the constraint entirely.
    Drop,
    /// Drop the constraint from the hard model but stash it to be re-added as a soft
    /// (objectified) cross-fixed-period penalty.
    Store,
}

/// Wrap a flattened base-model variable into the configured model's variable space.
fn wrap_var(v: &InternalVar<Var, ExtraVarName>) -> InternalVar<Var, ConfiguredExtra> {
    match v {
        InternalVar::Base(b) => InternalVar::Base(b.clone()),
        InternalVar::Extra(e) => InternalVar::Extra(ConfiguredExtra::Inner(e.clone())),
        InternalVar::Helper { owner, id } => InternalVar::Helper {
            owner: ConfiguredExtra::Inner(owner.clone()),
            id: id.clone(),
        },
    }
}

/// Wrap a base-model constraint source into the configured model's description space.
fn wrap_source(
    src: &ConstraintSource<ExtraVarName, ConstraintDesc>,
) -> ConstraintSource<ConfiguredExtra, ConfiguredConstraintDesc> {
    match src {
        ConstraintSource::User(desc) => {
            ConstraintSource::User(ConfiguredConstraintDesc::Inner(desc.clone()))
        }
        ConstraintSource::DefiningExtra {
            extra,
            index,
            for_constraints,
        } => ConstraintSource::DefiningExtra {
            extra: ConfiguredExtra::Inner(extra.clone()),
            index: *index,
            for_constraints: *for_constraints,
        },
    }
}
