//! Build the ILP model from a generation plan.

use crate::GroupListsModel;
use crate::frozen::FrozenPlacements;
use crate::objective::ObjectiveWeights;
use crate::specs::{GenerationPlan, RangeSource};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp_modeler::{Modeler, ReifyError};
use std::time::Instant;

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

/// What the stability objective was resolved to, one line at a time, for the
/// build log. Three decisions shape that objective and none of them is
/// visible in the model itself: the canonical group size, what each size
/// class then weighs, and whether there is a template grouping at all. Two of
/// them the user can steer from the advanced dialog, so a log that does not
/// name them cannot be acted on.
fn plan_report(plan: &GenerationPlan, env: &VarEnv, frozen: &FrozenPlacements) -> Vec<String> {
    let mut lines = Vec::new();

    match &plan.canonical_range {
        Some((range, source)) => {
            let source = match source {
                RangeSource::Automatic => "automatic, student-weighted vote",
                RangeSource::Manual => "manual, set in the advanced settings",
            };
            lines.push(format!(
                "[build_model] Canonical group size: {}-{} ({})",
                range.start().get(),
                range.end().get(),
                source,
            ));
        }
        None => lines.push(String::from(
            "[build_model] Canonical group size: none (the plan has no list to build)",
        )),
    }

    for class in env.classes() {
        let range = env.class_range(class);
        let lists = env
            .lists()
            .filter(|list| env.class_of(*list) == class)
            .count();
        lines.push(format!(
            "[build_model] Size class {}-{}: {} list(s), pair weight {:.3}",
            range.start().get(),
            range.end().get(),
            lists,
            env.class_weight(class),
        ));
    }

    match &plan.ghost {
        Some(ghost) => lines.push(format!(
            "[build_model] Template grouping: {} students in {} groups of {}-{}",
            ghost.spec().students().len(),
            env.ghost_group_count(),
            ghost.spec().students_per_group().start().get(),
            ghost.spec().students_per_group().end().get(),
        )),
        // The three ways to end up without one, told apart by the canonical
        // range beside it: no specs at all, a manual size that cannot split
        // the whole student body (a manual size is never fallen back from),
        // or — automatically — no group size of the document that can.
        None => lines.push(String::from(match &plan.canonical_range {
            None => "[build_model] Template grouping: none (the plan has no list to build)",
            Some((_, RangeSource::Manual)) => {
                "[build_model] Template grouping: none (the chosen size cannot split all the \
                 students; the deviation weight has no effect)"
            }
            Some((_, RangeSource::Automatic)) => {
                "[build_model] Template grouping: none (no group size of the document can split \
                 all the students; the deviation weight has no effect)"
            }
        })),
    }

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
pub fn build_model(
    plan: &GenerationPlan,
    weights: ObjectiveWeights,
    frozen: &FrozenPlacements,
) -> GroupListsModel {
    build_model_with_log(plan, weights, frozen, &mut |_: &str| {})
}

/// `_weights` is already dead: the collision objective has nothing to weigh.
/// The parameter survives one commit so that gtk4, which names
/// [`ObjectiveWeights`] in four files, can be simplified on its own before the
/// type is retired.
pub fn build_model_with_log(
    plan: &GenerationPlan,
    _weights: ObjectiveWeights,
    frozen: &FrozenPlacements,
    log: &mut (dyn FnMut(&str) + Send),
) -> GroupListsModel {
    let env = VarEnv::new(plan);

    for line in plan_report(plan, &env, frozen) {
        log(&line);
    }

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
    // (`group_lists_to_warm_start`).
    let pairs = crate::pairs::PairData::new(plan, &env);

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
        let model = build_model(
            &plan,
            crate::ObjectiveWeights::default(),
            &FrozenPlacements::default(),
        );

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
        // One size row per (list, group): 2 + 2. The template adds none: it
        // is plan data, not a grouping the model shapes.
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
    fn the_log_reports_the_objective_resolution() {
        // Two colle lists of four students (8 votes) against one tutorial
        // list of six (6 votes): 2..=3 is canonical, so a tutorial meeting
        // weighs (3 − 1) / (6 − 1) = 0.4 of a colle meeting. The template
        // spans the six students at the canonical size: ceil(6 / 3) = 2
        // groups.
        let plan = crate::vars::tests::plan_of(&[
            (&[1, 2, 3, 4], (2, 3)),
            (&[3, 4, 5, 6], (2, 3)),
            (&[1, 2, 3, 4, 5, 6], (6, 6)),
        ]);
        let lines = plan_report(&plan, &VarEnv::new(&plan), &FrozenPlacements::default());

        assert_eq!(
            lines,
            vec![
                "[build_model] Canonical group size: 2-3 (automatic, student-weighted vote)",
                "[build_model] Size class 2-3: 2 list(s), pair weight 1.000",
                "[build_model] Size class 6-6: 1 list(s), pair weight 0.400",
                "[build_model] Template grouping: 6 students in 2 groups of 2-3",
                "[build_model] Frozen placements: none (the polish may undo the prefill)",
            ]
        );
    }

    #[test]
    fn the_log_counts_the_pinned_seats() {
        // The last line of the report tells the two silences apart: a
        // generation that was not asked to keep the prefill, and one whose
        // prefill claimed nobody, build the same model.
        let plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 3))]);
        let env = VarEnv::new(&plan);

        let none = plan_report(&plan, &env, &FrozenPlacements::default());
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
        let some = plan_report(&plan, &env, &frozen);
        assert_eq!(
            some.last().expect("the report is never empty"),
            "[build_model] Frozen placements: 2 seat(s) pinned",
        );
    }

    #[test]
    fn the_log_says_why_there_is_no_template() {
        let mut plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 3))]);
        // What a manual 3..=3 over five students would give: the size stays
        // in force for the class weights, but nothing is templated.
        plan.canonical_range = Some((
            crate::specs::tests::range(3, 3),
            crate::specs::RangeSource::Manual,
        ));
        plan.ghost = None;

        let lines = plan_report(&plan, &VarEnv::new(&plan), &FrozenPlacements::default());
        assert_eq!(
            lines[0],
            "[build_model] Canonical group size: 3-3 (manual, set in the advanced settings)"
        );
        assert!(
            lines[2].contains("none (the chosen size cannot split all the students"),
            "unexpected template line: {}",
            lines[2]
        );
    }

    #[test]
    fn the_report_reaches_the_build_log() {
        let plan = crate::vars::tests::plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        let mut lines: Vec<String> = Vec::new();
        build_model_with_log(
            &plan,
            crate::ObjectiveWeights::default(),
            &FrozenPlacements::default(),
            &mut |line| lines.push(line.to_string()),
        );

        // The report is the head of the log, before any bundle is applied.
        assert_eq!(
            lines[0],
            "[build_model] Canonical group size: 2-2 (automatic, student-weighted vote)"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("[build_model] Template grouping: 4 students")),
            "the template line is missing from the build log"
        );
    }
}
