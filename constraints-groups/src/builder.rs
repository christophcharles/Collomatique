//! Build the ILP model from a generation plan.

use crate::GroupListsModel;
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
fn plan_report(plan: &GenerationPlan, env: &VarEnv) -> Vec<String> {
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

    lines
}

pub fn build_model(plan: &GenerationPlan, weights: ObjectiveWeights) -> GroupListsModel {
    build_model_with_log(plan, weights, &mut |_: &str| {})
}

pub fn build_model_with_log(
    plan: &GenerationPlan,
    weights: ObjectiveWeights,
    log: &mut (dyn FnMut(&str) + Send),
) -> GroupListsModel {
    let env = VarEnv::new(plan);

    for line in plan_report(plan, &env) {
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
        for (_, source) in model.problem().get_constraints() {
            if let ConstraintSource::User(desc) = source {
                match desc {
                    ConstraintDesc::StudentInOneGroup { .. } => one_group += 1,
                    ConstraintDesc::StudentsPerGroupMax { .. } => max += 1,
                    ConstraintDesc::StudentsPerGroupMin { .. } => min += 1,
                }
            }
        }
        // One "exactly one group" row per (list, student): 4 + 3.
        assert_eq!(one_group, 7);
        // One size constraint of each kind per (list, group): 2 + 2. The
        // template adds none: it is plan data, not a grouping the model
        // shapes.
        assert_eq!(max, 4);
        assert_eq!(min, 4);

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
        let mut pieces = 0;
        let mut helpers = 0;
        for v in model.problem().get_variables().keys() {
            // Exhaustive for the same reason as the `match` above.
            match v {
                InternalVar::Extra(ExtraVarName::SharedPair { class, .. }) => {
                    per_class[class.0] += 1
                }
                InternalVar::Extra(ExtraVarName::RefGroupInGroup { .. }) => pieces += 1,
                InternalVar::Helper { .. } => helpers += 1,
                InternalVar::Base(_) => {}
            }
        }
        assert_eq!(per_class, [3, 6]);
        // The template spans the union at the canonical 2..=3, so its three
        // reference groups are {1, 2, 3}, {4, 5} and {6, 7}: the clustering
        // sees no signal beyond "same spec", and breaks ties by student id.
        // List 0 ({1, 2, 3, 4}) meets the first two, list 1 ({5, 6, 7}) the
        // last two, so there are four sites — and one variable per site and
        // group of its list, 2 groups each.
        assert_eq!(pieces, 4 * 2);
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
        let lines = plan_report(&plan, &VarEnv::new(&plan));

        assert_eq!(
            lines,
            vec![
                "[build_model] Canonical group size: 2-3 (automatic, student-weighted vote)",
                "[build_model] Size class 2-3: 2 list(s), pair weight 1.000",
                "[build_model] Size class 6-6: 1 list(s), pair weight 0.400",
                "[build_model] Template grouping: 6 students in 2 groups of 2-3",
            ]
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

        let lines = plan_report(&plan, &VarEnv::new(&plan));
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
        build_model_with_log(&plan, crate::ObjectiveWeights::default(), &mut |line| {
            lines.push(line.to_string())
        });

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
