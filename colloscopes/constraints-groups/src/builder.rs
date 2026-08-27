//! Build the ILP model from a generation plan.

use crate::GroupListsModel;
use crate::frozen::FrozenPlacements;
use crate::pairs::{PairData, cross_tiers};
use crate::specs::GenerationPlan;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::{Modeler, ReifyError};
use std::time::Instant;

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

/// The size of the objective, one line at a time, for the build log.
///
/// The objective itself is not a decision any more — it is the greedy's score,
/// coefficient for coefficient — so what is worth reporting is how big it came
/// out: how many pairs of students can still be brought together, how many
/// terms that costs, and what the kept lists already scored before the solver
/// decided anything. Those numbers are also the model's own size, since every
/// term is one extra column.
fn plan_report(pairs: &PairData, frozen: &FrozenPlacements) -> Vec<String> {
    let mut sites = 0usize;
    let mut products = 0usize;
    for (_pair, table) in pairs.pairs() {
        for tier in table {
            sites += tier.groups.len();
        }
        products += cross_tiers(table).count();
    }

    let mut lines = vec![
        format!(
            "[build_model] Pairs that can meet in a rebuilt group: {}",
            pairs.pairs().count(),
        ),
        format!(
            "[build_model] Objective terms: {sites} site(s), {products} product(s), \
             constant {:.6}",
            pairs.constant_term(),
        ),
    ];

    lines.push(match frozen.len() {
        0 => {
            String::from("[build_model] Frozen placements: none (the polish may undo the prefill)")
        }
        n => format!("[build_model] Frozen placements: {n} seat(s) pinned"),
    });

    lines
}

/// An empty `frozen` is how a caller says "pin nothing" — there is no
/// `Option`, because "the user did not ask" and "the user asked, and prefill
/// froze nobody" build exactly the same model.
pub fn build_model(plan: &GenerationPlan, frozen: &FrozenPlacements) -> GroupListsModel {
    build_model_with_log(plan, frozen, &mut |_: &str| {})
}

pub fn build_model_with_log(
    plan: &GenerationPlan,
    frozen: &FrozenPlacements,
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

    // One enumeration, three readings: it declares the extras, weighs them in
    // the objective, and is what a warm start valuates them from
    // (`group_lists_to_warm_start`). The report is read off it too, so it
    // comes first — before any bundle, so the log opens on the size of what
    // is about to be built.
    let pairs = PairData::new(plan, &env);

    for line in plan_report(&pairs, frozen) {
        log(&line);
    }

    // The extras must be declared before the constraints and the objective
    // reference them.
    apply!("extras", crate::extras::build_extras(&pairs));
    apply!("constraints", crate::constraints::build(&env, frozen));
    apply!("objective", crate::objective::build(&pairs));

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
        // List 0: 4 students, sizes 2..=3 → ceil(4/3) = 2 groups, targets
        // 2 / 2. List 1: 3 students, sizes 1..=2 → ceil(3/2) = 2 groups,
        // targets 2 / 1. Both lists serve one (period, subject) pair, so
        // neither is filtered out of the pair enumeration.
        let plan = crate::vars::tests::plan_with_uses(
            &[(&[1, 2, 3, 4], (2, 3), 1), (&[5, 6, 7], (1, 2), 1)],
            &[],
        );
        let model = build_model(&plan, &FrozenPlacements::default());

        // The `match` is exhaustive on purpose: a new constraint family
        // must not slip in without this test growing to count it.
        let mut one_group = 0;
        let mut sizes = 0;
        for (_, source) in model.problem().get_constraints() {
            if let ConstraintSource::User(desc) = source {
                match desc {
                    ConstraintDesc::StudentInOneGroup { .. } => one_group += 1,
                    ConstraintDesc::GroupSize { .. } => sizes += 1,
                    // This plan pins nothing.
                    ConstraintDesc::FrozenPlacement { .. } => unreachable!(),
                }
            }
        }
        // One "exactly one group" row per (list, student): 4 + 3.
        assert_eq!(one_group, 7);
        // One size row per (list, group): 2 + 2.
        assert_eq!(sizes, 4);

        // The objective references every declared extra, so they are all
        // expanded. The lists are disjoint, so no pair is in two of them and
        // no product is declared at all.
        //
        // Sites: list 0 has one tier of two groups, so each of its C(4,2) = 6
        // pairs gets 2. List 1's targets are 2 / 1, and the lone seat is
        // filtered out (F1), so each of its C(3,2) = 3 pairs gets 1.
        let mut sites = 0;
        let mut products = 0;
        let mut helpers = 0;
        for v in model.problem().get_variables().keys() {
            // Exhaustive for the same reason as the `match` above.
            match v {
                InternalVar::Extra(ExtraVarName::Together { .. }) => sites += 1,
                InternalVar::Extra(ExtraVarName::Coincide { .. }) => products += 1,
                InternalVar::Helper { .. } => helpers += 1,
                InternalVar::Base(_) => {}
            }
        }
        assert_eq!(sites, 6 * 2 + 3);
        assert_eq!(products, 0);
        // The one-sided definitions reference nothing but base variables and
        // each other, so the extras of the model are exactly those columns —
        // no reification helper anywhere, which is what lets a warm start name
        // every variable of the model.
        assert_eq!(helpers, 0);
    }

    #[test]
    fn the_log_opens_on_the_size_of_the_objective() {
        // Two lists sharing two students, plus a kept list that already
        // groups two others.
        //
        // List 0: 4 students at 2..=3 → 2 groups of 2, one tier. List 1: 4
        // students at 2..=2 → 2 groups of 2, one tier. Each list gives its
        // C(4, 2) = 6 pairs 2 sites, so 24 in all; the pair (3, 4) is the only
        // one in both lists, so it is the only product — and the only pair
        // counted once instead of twice, leaving 11 distinct pairs.
        //
        // The kept list groups 1 and 2 for one use each way. Both take part in
        // two uses in all (their spec and it), so each puts a mass of
        // 1 / (2 · (2 − 1)) = 0.5 on the other, and the constant is 2 × 0.5².
        let plan = crate::vars::tests::plan_with_uses(
            &[(&[1, 2, 3, 4], (2, 3), 1), (&[3, 4, 5, 6], (2, 2), 1)],
            &[(&[&[1, 2]], 1)],
        );
        let mut lines: Vec<String> = Vec::new();
        build_model_with_log(&plan, &FrozenPlacements::default(), &mut |line| {
            lines.push(line.to_string())
        });

        // The report is the head of the log, before any bundle is applied.
        assert_eq!(
            lines[..3],
            [
                "[build_model] Pairs that can meet in a rebuilt group: 11",
                "[build_model] Objective terms: 24 site(s), 1 product(s), constant 0.500000",
                "[build_model] Frozen placements: none (the polish may undo the prefill)",
            ]
        );
    }

    #[test]
    fn the_log_counts_the_pinned_seats() {
        // The last line of the report tells the two silences apart: a
        // generation that was not asked to keep the prefill, and one whose
        // prefill claimed nobody, build the same model.
        let plan = crate::vars::tests::plan_with_uses(&[(&[1, 2, 3, 4], (2, 3), 1)], &[]);
        let env = VarEnv::new(&plan);
        let pairs = PairData::new(&plan, &env);

        let none = plan_report(&pairs, &FrozenPlacements::default());
        assert_eq!(
            none.last().expect("the report is never empty"),
            "[build_model] Frozen placements: none (the polish may undo the prefill)",
        );

        let frozen = FrozenPlacements::new(std::collections::BTreeMap::from([
            (
                (
                    crate::vars::GroupListIdx(0),
                    crate::specs::tests::student(1),
                ),
                0,
            ),
            (
                (
                    crate::vars::GroupListIdx(0),
                    crate::specs::tests::student(2),
                ),
                0,
            ),
        ]));
        let some = plan_report(&pairs, &frozen);
        assert_eq!(
            some.last().expect("the report is never empty"),
            "[build_model] Frozen placements: 2 seat(s) pinned",
        );
    }
}
