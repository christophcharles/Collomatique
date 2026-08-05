//! Build the (for now trivial) ILP model from a generation plan.

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

    // The extras are declarations only: nothing references them until the
    // constraints (piece 8) and the objective (piece 9), so this bundle
    // leaves the built model unchanged — an empty objective still folds to
    // a constant-0 minimize.
    apply!("extras", crate::extras::build_extras(&env));

    modeler
        .build_with_log(&env, log)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_model_has_only_base_vars() {
        let plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
        let model = build_model(&plan);

        // 7 base variables and nothing else: the extras of piece 7 are
        // declared but referenced by nothing, so `Modeler::build` expands
        // none of them and no helper variable can appear either.
        assert_eq!(model.problem().get_variables().len(), 7);
        assert_eq!(model.problem().get_constraints().len(), 0);
    }
}
