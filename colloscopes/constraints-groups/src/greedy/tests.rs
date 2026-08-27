use super::state::State;
use super::*;
use crate::specs::tests::{plan_with_uses as plan, set, student};
use collomatique_state_colloscopes::StudentId;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

fn run(plan: &GenerationPlan) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();
    greedy_group_lists(plan, &names)
}

/// The prefilled groups of one list, as student sets.
fn groups_of(list: &GroupList) -> Vec<BTreeSet<StudentId>> {
    match list.filling() {
        GroupListFilling::Prefilled { groups } => {
            groups.iter().map(|group| group.students.clone()).collect()
        }
        GroupListFilling::Automatic { .. } => panic!("the greedy only emits prefilled lists"),
    }
}

/// The comparable form of a whole output: `GroupList` also carries names.
fn memberships(
    lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)],
) -> Vec<Vec<BTreeSet<StudentId>>> {
    lists
        .iter()
        .map(|(list, _covered)| groups_of(list))
        .collect()
}

/// Every hard constraint of §3, checked without reusing `balanced_targets`:
/// one list per spec in plan order, the covered pairs carried through, the
/// minimal group count, every student in exactly one group, sizes descending,
/// inside the range and balanced within one.
fn assert_valid(plan: &GenerationPlan, lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)]) {
    assert_eq!(lists.len(), plan.specs.len(), "one list per spec");
    for ((list, covered), (spec, spec_covered)) in lists.iter().zip(plan.specs.iter()) {
        assert_eq!(
            covered, spec_covered,
            "the covered pairs are carried through"
        );
        assert_eq!(&list.params().students_per_group, spec.students_per_group());
        assert!(list.params().group_names.iter().all(Option::is_none));

        let groups = groups_of(list);
        assert_eq!(groups.len(), list.params().group_names.len());

        let n = spec.students().len() as u32;
        let min = spec.students_per_group().start().get();
        let max = spec.students_per_group().end().get();
        assert_eq!(groups.len() as u32, n.div_ceil(max), "minimal group count");

        let sizes: Vec<u32> = groups.iter().map(|group| group.len() as u32).collect();
        assert!(sizes.windows(2).all(|w| w[0] >= w[1]), "descending sizes");
        assert!(
            sizes.iter().all(|&s| s >= min && s <= max),
            "sizes in range"
        );
        let spread = sizes.first().expect("at least one group") - sizes.last().expect("idem");
        assert!(spread <= 1, "balanced sizes: {sizes:?}");

        let mut seen = BTreeSet::new();
        for group in &groups {
            for &s in group {
                assert!(seen.insert(s), "student {s:?} sits in two groups");
            }
        }
        assert_eq!(&seen, spec.students(), "every student placed exactly once");
    }
}

/// Phase one alone, on a fresh state: what prefill seated, before the pass
/// gets to decide anything. The prefill tests need the phase boundary as an
/// observable, and the finished lists do not show it.
fn prefilled(plan: &GenerationPlan) -> State<'_> {
    let mut state = State::new(plan);
    let cohorts = super::cohorts::ordered_cohorts(&state);
    super::prefill::prefill(&mut state, &cohorts);
    state
}

/// Every frozen seat names the group the *finished* lists hold that student
/// in: prefill's decisions are the ones the pass never revises, and the group
/// indices stay put because `into_group_lists` never compacts.
fn assert_frozen_agrees(plan: &GenerationPlan, seats: &[(StudentId, usize, usize)]) {
    let memberships = memberships(&run(plan));
    for &(student, list, group) in seats {
        let groups = &memberships[list];
        assert!(
            group < groups.len(),
            "frozen seat {list}/{student:?}/{group} names a group the list does not have",
        );
        assert!(
            groups[group].contains(&student),
            "frozen seat {list}/{student:?}/{group} is not where the list holds them",
        );
    }
}

/// The objective value of a hand-built configuration: `config[list][group]`
/// lists the students of that group.
fn score(plan: &GenerationPlan, config: &[&[&[u64]]]) -> f64 {
    let mut state = State::new(plan);
    for (list, groups) in config.iter().enumerate() {
        for (group, students) in groups.iter().enumerate() {
            for &s in *students {
                state.place(student(s), list, group);
            }
        }
    }
    state.objective_value()
}

/// The same, for a finished placement read back from the lists that carry it:
/// the quantity `greedy_group_lists` maximizes, evaluated on lists it did not
/// necessarily produce.
fn score_lists(
    plan: &GenerationPlan,
    lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)],
) -> f64 {
    assert_eq!(lists.len(), plan.specs.len(), "one list per spec");
    let mut state = State::new(plan);
    for (list, ((spec, _covered), (group_list, _covered_again))) in
        plan.specs.iter().zip(lists.iter()).enumerate()
    {
        for &s in spec.students() {
            let group = group_list
                .filling()
                .find_student_group(s)
                .expect("every student of a spec sits in a group of its list");
            state.place(s, list, group);
        }
    }
    state.objective_value()
}

#[test]
fn hard_constraints_hold() {
    // An uneven count: 7 students in groups of 2 to 3 → targets 3, 2, 2.
    let uneven = plan(&[(&[1, 2, 3, 4, 5, 6, 7], (2, 3), 1)], &[]);
    assert_valid(&uneven, &run(&uneven));

    // Several lists over overlapping student sets, different size classes.
    let several = plan(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], (3, 4), 2),
            (&[1, 2, 3, 4, 5, 6], (2, 3), 1),
            (&[7, 8, 9, 10, 11, 12], (2, 2), 3),
        ],
        &[],
    );
    assert_valid(&several, &run(&several));

    // With kept lists in the mix, one of them inert.
    let with_kept = plan(
        &[(&[1, 2, 3, 4, 5, 6, 7, 8], (2, 3), 1)],
        &[
            (&[&[1, 5], &[2, 6], &[3, 7], &[4, 8]], 4),
            (&[&[1, 2, 3]], 0),
        ],
    );
    assert_valid(&with_kept, &run(&with_kept));

    // A spec covering nothing: multiplicity 0, every student still placed.
    let weightless = plan(&[(&[1, 2, 3, 4, 5], (2, 3), 0)], &[]);
    assert_valid(&weightless, &run(&weightless));
}

#[test]
fn deterministic() {
    let plan = plan(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], (3, 4), 2),
            (&[1, 2, 3, 4, 5], (2, 3), 1),
            (&[4, 5, 6, 7, 8, 9], (3, 3), 1),
        ],
        &[(&[&[1, 6], &[2, 7], &[3, 8]], 2)],
    );
    assert_eq!(memberships(&run(&plan)), memberships(&run(&plan)));
}

#[test]
fn trio_travels() {
    // Twelve students share a tutorial split in three groups of four, and
    // each takes a colle as one trio. A trio cannot tile a four-seat
    // tutorial group, so no cohort claims anything there: the tutorial is
    // entirely the greedy's, and every trio faces genuinely empty
    // alternatives when it commits. The first trio must still end up in one
    // tutorial group, because its colle co-use already gives its members
    // mass with each other.
    let plan = plan(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], (4, 4), 1),
            (&[1, 2, 3], (3, 3), 1),
            (&[4, 5, 6], (3, 3), 1),
            (&[7, 8, 9], (3, 3), 1),
            (&[10, 11, 12], (3, 3), 1),
        ],
        &[],
    );
    let lists = run(&plan);
    assert_valid(&plan, &lists);

    let tutorial = &memberships(&lists)[0];
    let group_of = |s: u64| {
        tutorial
            .iter()
            .position(|group| group.contains(&student(s)))
            .expect("every student is placed")
    };
    assert_eq!(group_of(1), group_of(2), "the trio travels together");
    assert_eq!(group_of(1), group_of(3), "the trio travels together");
}

#[test]
fn license_case() {
    // The §2.4 scenario, scaled down: eight students, a tutorial in two
    // groups of four, two colles in pairs. Each student has N = 3 uses, so
    // a colle partner weighs 1/3 and a tutorial mate 1/9.
    let plan = plan(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8], (4, 4), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
        ],
        &[],
    );
    let tutorial: &[&[u64]] = &[&[1, 2, 3, 4], &[5, 6, 7, 8]];
    let pairs_inside: &[&[u64]] = &[&[1, 2], &[3, 4], &[5, 6], &[7, 8]];
    let pairs_across: &[&[u64]] = &[&[1, 5], &[2, 6], &[3, 7], &[4, 8]];
    let scattered: &[&[u64]] = &[&[1, 3], &[2, 4], &[5, 7], &[6, 8]];

    // (a) stable colle partners who are also tutorial mates, (b) stable
    // colle partners but in the other tutorial group, (c) colle partners
    // scattered among the tutorial mates.
    let a = score(&plan, &[tutorial, pairs_inside, pairs_inside]);
    let b = score(&plan, &[tutorial, pairs_across, pairs_across]);
    let c = score(&plan, &[tutorial, pairs_inside, scattered]);

    assert!(a > b, "stable colle partners belong in your tutorial group");
    assert!(
        b > c,
        "a big tutorial is no license to scatter colle partners: {b} vs {c}",
    );

    // And the search itself must not land below the middle configuration.
    let lists = run(&plan);
    assert_valid(&plan, &lists);
    let found = score_lists(&plan, &lists);
    assert!(found >= b, "the greedy scores {found}, below {b}");
}

#[test]
fn prefill_exact_fit() {
    // The §6.3 worked example: one list of five in groups of 2 to 3 →
    // targets {3, 2}. A second list holds only 1 and 2, which splits the
    // cohorts into {1, 2} and {3, 4, 5}. The pair cannot tile the 3-group
    // and claims the 2-group; the trio then claims the 3-group. The earlier
    // "purity + lowest index" design sent the pair into the 3-group and
    // doomed the trio.
    let plan = plan(&[(&[1, 2, 3, 4, 5], (2, 3), 1), (&[1, 2], (2, 2), 1)], &[]);
    let lists = run(&plan);
    assert_valid(&plan, &lists);

    assert_eq!(memberships(&lists)[0], vec![set(&[3, 4, 5]), set(&[1, 2])]);

    // Prefill covered the whole plan here, so every (list, student) seat is
    // frozen: five in the first list, two in the second.
    let seats: usize = plan
        .specs
        .iter()
        .map(|(spec, _covered)| spec.students().len())
        .sum();
    let state = prefilled(&plan);
    let frozen: Vec<(StudentId, usize, usize)> = state.frozen_seats().collect();
    assert_eq!(frozen.len(), seats);
    assert_frozen_agrees(&plan, &frozen);
}

#[test]
fn prefill_can_claim_nothing() {
    // Ten students in two groups of five, split into five cohorts of two by a
    // kept list that pairs them off. No cohort can tile a five-seat group, so
    // no list is a claiming list and prefill seats nobody: the pass decides
    // everything, and there is nothing to pin.
    let plan = plan(
        &[(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], (5, 5), 1)],
        &[(&[&[1, 2], &[3, 4], &[5, 6], &[7, 8], &[9, 10]], 1)],
    );
    let lists = run(&plan);
    assert_valid(&plan, &lists);

    let state = prefilled(&plan);
    assert_eq!(
        state.frozen_seats().count(),
        0,
        "prefill claimed nothing, so nothing is frozen",
    );
}

#[test]
fn kept_lists_steer() {
    // Four students, one list in two pairs — perfectly symmetric on its own,
    // so prefill would pair 1 with 2 and 3 with 4. A kept list grouping 1
    // with 3 and 2 with 4 makes those students non-interchangeable (their
    // frozen partners differ), which splits the cohorts and carries the kept
    // grouping over.
    let steered = plan(&[(&[1, 2, 3, 4], (2, 2), 1)], &[(&[&[1, 3], &[2, 4]], 5)]);
    let lists = run(&steered);
    assert_valid(&steered, &lists);
    assert_eq!(memberships(&lists)[0], vec![set(&[1, 3]), set(&[2, 4])]);

    // The very same kept list, associated to no (period, subject) pair, is
    // inert: it weighs nothing and must not split anything.
    let ignored = plan(&[(&[1, 2, 3, 4], (2, 2), 1)], &[(&[&[1, 3], &[2, 4]], 0)]);
    let lists = run(&ignored);
    assert_valid(&ignored, &lists);
    assert_eq!(memberships(&lists)[0], vec![set(&[1, 2]), set(&[3, 4])]);
}

#[test]
fn the_log_reaches_the_callback() {
    // Six students in groups of 2 to 3 → targets {3, 3}, and a second list
    // holding only 1 and 2. The pair cannot tile a 3-group, so prefill only
    // seats it in the small list and it stays the pass's business; the other
    // cohort claims one 3-group and defers its fourth member. Three students
    // seated, three left — a pass with something to do, and a prefill that
    // did not do everything.
    let plan = plan(
        &[(&[1, 2, 3, 4, 5, 6], (2, 3), 1), (&[1, 2], (2, 2), 1)],
        &[],
    );
    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();

    let mut lines: Vec<String> = Vec::new();
    let lists =
        greedy_group_lists_with_log(&plan, &names, &mut |line| lines.push(line.to_string()));

    assert_valid(&plan, &lists);
    assert_eq!(
        memberships(&lists),
        memberships(&run(&plan)),
        "logging must not change what the greedy produces",
    );

    assert!(
        lines.iter().all(|line| line.starts_with("[greedy] ")),
        "every line carries the prefix: {lines:?}",
    );
    assert_eq!(
        lines[0],
        "[greedy] 6 student(s) over 2 list(s), in 2 cohort(s)"
    );
    // The elapsed time closes each of the remaining lines, so they are
    // matched on their head only.
    let assert_line = |head: &str| {
        assert!(
            lines.iter().any(|line| line.starts_with(head)),
            "no line starting with {head:?}: {lines:?}",
        );
    };
    assert_line("[greedy] Prefill: 3 student(s) seated, 3 left to the pass (");
    assert_line("[greedy] Pass: 3 student(s) placed (");
    // The score line closes no timing, so it is matched whole: it must report
    // the value of the placement the run returned, not some intermediate one.
    assert!(
        lines.contains(&format!(
            "[greedy] Objective value: {:.6}",
            score_lists(&plan, &lists),
        )),
        "no line reporting the produced placement's score: {lines:?}",
    );
    assert!(
        lines
            .last()
            .expect("the log is not empty")
            .starts_with("[greedy] Done ("),
        "the total closes the log: {lines:?}",
    );
}

#[test]
fn size_one_corner() {
    // Three students in groups of 1 to 2 → targets {2, 1}: somebody sits
    // alone in every list. Two such lists, so a repeated meeting is possible.
    let plan = plan(&[(&[1, 2, 3], (1, 2), 2), (&[1, 2, 3], (1, 2), 1)], &[]);
    let lists = run(&plan);
    assert_valid(&plan, &lists);

    // Fixed N (§2.4): a further meeting with your best partner still beats
    // the configuration where you sit alone. Nothing is renormalized away
    // when a use produces no partner, so "with your partner" stays strictly
    // better than "alone" instead of tying with it.
    let pair_first: &[&[u64]] = &[&[1, 2], &[3]];
    let pair_again: &[&[u64]] = &[&[1, 2], &[3]];
    let sits_alone: &[&[u64]] = &[&[2, 3], &[1]];
    let repeat = score(&plan, &[pair_first, pair_again]);
    let alone = score(&plan, &[pair_first, sits_alone]);
    assert!(repeat > alone, "{repeat} must beat {alone}");
}
