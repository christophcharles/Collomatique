//! Build the ILP model from a generation plan.

use crate::GroupListsModel;
use crate::objective::ObjectiveWeights;
use crate::specs::GenerationPlan;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::{Modeler, ReifyError};
use std::time::Instant;

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

pub fn build_model(plan: &GenerationPlan, weights: ObjectiveWeights) -> GroupListsModel {
    build_model_with_log(plan, weights, &mut |_: &str| {})
}

pub fn build_model_with_log(
    plan: &GenerationPlan,
    weights: ObjectiveWeights,
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

    // The extras must be declared before the constraints and the objective
    // reference them.
    apply!("extras", crate::extras::build_extras(&env));
    apply!("constraints", crate::constraints::build(&env));
    apply!("objective", crate::objective::build(&env, weights));

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
        // List 0: 4 students, sizes 2..=3 → ceil(4/3) = 2 groups. List 1:
        // 3 students, sizes 1..=2 → ceil(3/2) = 2 groups.
        let plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
        let model = build_model(&plan, crate::ObjectiveWeights::default());

        // The `match` is exhaustive on purpose: a new constraint family
        // must not slip in without this test growing to count it.
        let mut one_group = 0;
        let mut max = 0;
        let mut min = 0;
        let mut ghost_one_group = 0;
        let mut ghost_max = 0;
        let mut ghost_min = 0;
        for (_, source) in model.problem().get_constraints() {
            if let ConstraintSource::User(desc) = source {
                match desc {
                    ConstraintDesc::StudentInOneGroup { .. } => one_group += 1,
                    ConstraintDesc::StudentsPerGroupMax { .. } => max += 1,
                    ConstraintDesc::StudentsPerGroupMin { .. } => min += 1,
                    ConstraintDesc::GhostStudentInOneGroup { .. } => ghost_one_group += 1,
                    ConstraintDesc::GhostStudentsPerGroupMax { .. } => ghost_max += 1,
                    ConstraintDesc::GhostStudentsPerGroupMin { .. } => ghost_min += 1,
                }
            }
        }
        // One "exactly one group" row per (list, student): 4 + 3.
        assert_eq!(one_group, 7);
        // One size constraint of each kind per (list, group): 2 + 2.
        assert_eq!(max, 4);
        assert_eq!(min, 4);
        // The template spans the union of the two lists — 7 students at the
        // canonical 2..=3 (list 0's range wins the vote 4 to 3), so
        // ceil(7 / 3) = 3 groups — and carries the same two families.
        assert_eq!(ghost_one_group, 7);
        assert_eq!(ghost_max, 3);
        assert_eq!(ghost_min, 3);

        // The objective references every `SharedPair`, so they are all
        // expanded. The lists are disjoint, so the co-occurring pairs are
        // C(4,2) + C(3,2) = 6 + 3. Their one-sided definitions reference
        // nothing but base variables, so the extras of the model are
        // exactly those nine columns — the whole `PairInGroup` block, and
        // the helper columns of its reification, are gone.
        //
        // The two lists have different ranges, so they are two size classes,
        // sorted: class 0 is list 1's 1..=2 and class 1 is list 0's 2..=3.
        // Every pair belongs to exactly one of them here.
        let mut per_class = [0, 0];
        let mut canonical = 0;
        let mut deviation = 0;
        let mut helpers = 0;
        for v in model.problem().get_variables().keys() {
            // Exhaustive for the same reason as the `match` above.
            match v {
                InternalVar::Extra(ExtraVarName::SharedPair { class, .. }) => {
                    per_class[class.0] += 1
                }
                InternalVar::Extra(ExtraVarName::CanonicalPair { .. }) => canonical += 1,
                InternalVar::Extra(ExtraVarName::Deviation { .. }) => deviation += 1,
                InternalVar::Helper { .. } => helpers += 1,
                InternalVar::Base(_) => {}
            }
        }
        assert_eq!(per_class, [3, 6]);
        // The template families ignore the size class — one `CanonicalPair`
        // per co-occurring pair, and one `Deviation` per (pair, list) site.
        // The lists are disjoint, so each of the nine pairs meets in exactly
        // one of them and the two counts coincide here.
        assert_eq!(canonical, 9);
        assert_eq!(deviation, 9);
        assert_eq!(helpers, 0);
    }
}
