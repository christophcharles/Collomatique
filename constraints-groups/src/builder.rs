//! Build the ILP model from a generation plan.

use crate::GroupListsModel;
use crate::specs::GenerationPlan;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::{Modeler, ReifyError};
use std::time::Instant;

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

pub fn build_model(plan: &GenerationPlan) -> GroupListsModel {
    build_model_with_log(plan, &mut |_: &str| {})
}

pub fn build_model_with_log(
    plan: &GenerationPlan,
    log: &mut (dyn FnMut(&str) + Send),
) -> GroupListsModel {
    let env = VarEnv::new(plan);

    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);

    macro_rules! apply {
        ($name:expr, $bundle:expr) => {{
            let t = Instant::now();
            log(&format!("[build_model] Applying bundle: {}...", $name));
            modeler
                .apply_bundle($bundle.into_general())
                .unwrap_or_else(|_| panic!("no duplicate extras from {}", $name));
            log(&format!(
                "[build_model] Bundle applied ({:.2?})",
                t.elapsed()
            ));
        }};
    }

    // The extras must be declared before the constraints reference them.
    // The pair extras (`PairInGroup`, `SharedPair`) stay unreferenced until
    // the objective (piece 9), so lazy expansion keeps them out of the
    // built model.
    apply!("extras", crate::extras::build_extras(&env));
    apply!("constraints", crate::constraints::build(&env));

    modeler
        .build_with_log(&env, log)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_ilp_modeler::{ConstraintSource, InternalVar};

    #[test]
    fn shape_constraints_are_emitted() {
        // List 0: 4 students, sizes 2..=3 → 2 slots. List 1: 3 students,
        // sizes 1..=2 → 3 slots.
        let plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
        let model = build_model(&plan);

        // The `match` is exhaustive on purpose: a new constraint family
        // must not slip in without this test growing to count it.
        let mut max = 0;
        let mut min = 0;
        for (_, source) in model.problem().get_constraints() {
            if let ConstraintSource::User(desc) = source {
                match desc {
                    ConstraintDesc::StudentsPerGroupMax { .. } => max += 1,
                    ConstraintDesc::StudentsPerGroupMin { .. } => min += 1,
                }
            }
        }
        // One size constraint of each kind per (list, group): 2 + 3.
        assert_eq!(max, 5);
        assert_eq!(min, 5);

        // The pair extras are referenced by nothing until the piece-9
        // objective: lazy expansion must keep them out of the problem.
        assert!(model.problem().get_variables().keys().all(|v| {
            !matches!(
                v,
                InternalVar::Extra(
                    ExtraVarName::PairInGroup { .. } | ExtraVarName::SharedPair { .. }
                )
            )
        }));
    }
}
