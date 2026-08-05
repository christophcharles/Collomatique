//! Build the (for now trivial) ILP model from a generation plan.

use crate::GroupListsModel;
use crate::specs::GenerationPlan;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::{Modeler, ReifyError};

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

    let modeler: MyModeler<'_> = Modeler::from_described(&env);
    // Phase A: no extras, no constraints, no objective. `Modeler::build`
    // explicitly supports this — an empty objective folds to a constant-0
    // minimize — and the later pieces grow the model here bundle by bundle.

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

        // 7 base variables and nothing else: no extra was declared, so no
        // helper variable could have been introduced either.
        assert_eq!(model.problem().get_variables().len(), 7);
    }
}
