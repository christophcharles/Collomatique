//! The template ("ghost") grouping: one partition of the whole student body
//! at the canonical group size, which the objective then asks every generated
//! list to resemble.
//!
//! It used to be a grouping the solver decided, with its own assignment
//! matrix. That was the wrong place for it. A template variable turns the
//! model into max-weight k-way graph partitioning, whose LP relaxation is
//! vacuous (spread every student evenly over the template groups and every
//! defining row holds), whose `G!` group symmetry nothing breaks, and whose
//! primal heuristics therefore round from the worst possible starting point.
//! On a real document it ran for minutes without a first incumbent.
//!
//! So it is computed here instead, by a greedy clustering on an affinity
//! graph read off the document. That costs the model nothing that matters:
//! the softness the user cares about is `w_template` letting each *list*
//! ignore the reference grouping, which is the by-hand workflow — you do not
//! re-derive your generic groups subject by subject.

use crate::specs::{GroupListSpec, RangeSource, class_weight, pairs_of, ranked_canonical_ranges};
use collomatique_state_colloscopes::{NonEmptyRangeInclusive, PeriodId, StudentId, SubjectId};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// The template grouping: a concrete partition of every student of the plan,
/// at the canonical group size. Computed, not solved — see the module doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostGrouping {
    spec: GroupListSpec,
    /// The groups, in build order. Every size lies in the spec's range and
    /// the union is exactly the spec's students.
    groups: Vec<BTreeSet<StudentId>>,
    by_student: BTreeMap<StudentId, usize>,
}

impl GhostGrouping {
    /// Assemble a template from its spec and its groups. The caller owns the
    /// invariant: the groups must partition the spec's students, at sizes the
    /// spec's range allows. `build_ghost` is the production caller;
    /// hand-built plans (the smoke test) use this directly.
    pub fn new(spec: GroupListSpec, groups: Vec<BTreeSet<StudentId>>) -> GhostGrouping {
        let by_student: BTreeMap<StudentId, usize> = groups
            .iter()
            .enumerate()
            .flat_map(|(i, group)| group.iter().map(move |&s| (s, i)))
            .collect();
        debug_assert_eq!(
            by_student.len(),
            groups.iter().map(BTreeSet::len).sum::<usize>(),
            "the template groups must be disjoint",
        );
        debug_assert!(
            by_student.keys().copied().collect::<BTreeSet<_>>() == *spec.students(),
            "the template groups must cover exactly the spec's students",
        );
        GhostGrouping {
            spec,
            groups,
            by_student,
        }
    }

    /// The spec the template was built at: the whole student body of the plan
    /// and the canonical size range. The shape of the grouping, as opposed to
    /// the grouping itself.
    pub fn spec(&self) -> &GroupListSpec {
        &self.spec
    }

    /// The reference groups, in build order.
    pub fn groups(&self) -> &[BTreeSet<StudentId>] {
        &self.groups
    }

    /// The reference group of a student, or `None` for a student the template
    /// does not cover. In a real plan there is none: the template spans the
    /// union of the specs' students.
    pub fn group_of(&self, student: StudentId) -> Option<usize> {
        self.by_student.get(&student).copied()
    }
}

/// The attraction between each pair of students: how much of the document
/// they share. Every spec they both belong to contributes its multiplicity —
/// the number of (period, subject) slots it covers — weighted by its size
/// class, since a whole-class tutorial is shared by *everyone* and must not
/// drown the small lists' signal. Specs whose groups hold one student are
/// skipped: nobody ever meets there.
///
/// The maximum is scaled to 1, which fixes the scale for the second half: a
/// pair a kept list already groups gets a bonus of that list's class weight
/// on top — at the canonical size, a full unit, i.e. as attractive as the
/// most-affine pair of the whole document. A grouping the user has already
/// decided must outrank any grouping we merely infer.
fn attractions(
    specs: &[(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)],
    pinned_pairs: &BTreeMap<NonEmptyRangeInclusive<NonZeroU32>, BTreeSet<(StudentId, StudentId)>>,
    canonical_range: Option<&NonEmptyRangeInclusive<NonZeroU32>>,
) -> BTreeMap<(StudentId, StudentId), f64> {
    let mut points: BTreeMap<(StudentId, StudentId), f64> = BTreeMap::new();
    for (spec, covered) in specs {
        let range = spec.students_per_group();
        if range.end().get() == 1 {
            continue;
        }
        let weight = class_weight(canonical_range, range) * covered.len().max(1) as f64;
        for pair in pairs_of(spec.students()) {
            *points.entry(pair).or_insert(0.0) += weight;
        }
    }
    let max = points.values().copied().fold(0.0, f64::max);
    if max > 0.0 {
        for value in points.values_mut() {
            *value /= max;
        }
    }
    for (range, pairs) in pinned_pairs {
        if range.end().get() == 1 {
            continue;
        }
        let bonus = class_weight(canonical_range, range);
        for pair in pairs {
            *points.entry(*pair).or_insert(0.0) += bonus;
        }
    }
    points
}

/// The candidate with the greatest score, ties going to the smallest id.
/// `>` is strict and `BTreeSet` iterates in ascending order, so the first
/// maximum wins. Panics on an empty candidate set, which [`cluster`] never
/// produces: the group targets sum to exactly the student count.
fn pick(remaining: &BTreeSet<StudentId>, score: impl Fn(StudentId) -> f64) -> StudentId {
    let mut best: Option<(StudentId, f64)> = None;
    for &s in remaining {
        let value = score(s);
        match best {
            Some((_, top)) if value <= top => {}
            _ => best = Some((s, value)),
        }
    }
    best.expect("the greedy never asks for a student out of an empty set")
        .0
}

/// Split the students into `group_count` groups, most-attracted students
/// together. One group at a time: seed with the most-connected student left,
/// then repeatedly take the student most attracted to the group so far.
///
/// The sizes are decided before any student is placed — `n % g` groups of
/// `n / g + 1`, then groups of `n / g`. [`GroupListSpec::new`] guarantees
/// `g · min <= n <= g · max`, so `n / g >= min` and (when there is a
/// remainder) `n / g + 1 <= max`: every target lies in the range, and the
/// greedy can never paint itself into a corner.
///
/// Deterministic: `remaining` is a `BTreeSet`, so candidates are scanned in
/// ascending id order, and [`pick`]'s comparison is strict, so the smallest
/// id wins every tie.
///
/// `O(n²)` for the connectivity table plus `O(n² · max)` for the growth,
/// which is nothing next to one solver call.
fn cluster(
    students: &BTreeSet<StudentId>,
    group_count: u32,
    attraction: &BTreeMap<(StudentId, StudentId), f64>,
) -> Vec<BTreeSet<StudentId>> {
    let pull = |s: StudentId, t: StudentId| -> f64 {
        let key = if s < t { (s, t) } else { (t, s) };
        attraction.get(&key).copied().unwrap_or(0.0)
    };
    // Computed once over the whole student body, not per group: the seeds
    // are then the students with the most obligations overall, and the last
    // group is left to those who share the least with anybody — which is
    // exactly who should be placed last.
    let total_pull: BTreeMap<StudentId, f64> = students
        .iter()
        .map(|&s| (s, students.iter().map(|&t| pull(s, t)).sum()))
        .collect();

    let n = students.len();
    let g = group_count as usize;
    let (base, remainder) = (n / g, n % g);

    let mut remaining = students.clone();
    let mut groups = Vec::with_capacity(g);
    for i in 0..g {
        let target = base + usize::from(i < remainder);
        let seed = pick(&remaining, |s| total_pull[&s]);
        remaining.remove(&seed);
        let mut group = BTreeSet::from([seed]);
        while group.len() < target {
            let next = pick(&remaining, |s| group.iter().map(|&m| pull(s, m)).sum());
            remaining.remove(&next);
            group.insert(next);
        }
        groups.push(group);
    }
    debug_assert!(remaining.is_empty(), "the targets sum to the student count");
    groups
}

/// The template grouping of a plan: every student of every spec, clustered at
/// the canonical size. The shape is a [`GroupListSpec`] like any other, so
/// the rest of the crate reads the size range exactly as it does a real
/// list's — it is only the *use* that differs (no output list, no
/// `SharedPair`).
///
/// The canonical range need not be able to split the whole student body: it
/// was elected as the typical size of a *subject*, and the union of the
/// subjects is larger than any of them. When it cannot, an automatic range
/// falls through to the runners-up of the vote — the other real group sizes
/// of the document, in vote order — and takes the first that partitions the
/// union. A *manual* range does not: the user fixed it on purpose, and
/// templating at a size they did not ask for would be worse than having no
/// template at all.
///
/// The class weights the attraction graph uses always measure against the
/// *elected* range, even when the template itself falls back to another one:
/// the election says what a typical group of the document looks like, and
/// that does not change because the union happens not to divide.
pub(crate) fn build_ghost(
    specs: &[(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)],
    pinned_pairs: &BTreeMap<NonEmptyRangeInclusive<NonZeroU32>, BTreeSet<(StudentId, StudentId)>>,
    canonical_range: Option<&(NonEmptyRangeInclusive<NonZeroU32>, RangeSource)>,
) -> Option<GhostGrouping> {
    let (range, source) = canonical_range?;
    let students: BTreeSet<StudentId> = specs
        .iter()
        .flat_map(|(spec, _covered)| spec.students().iter().copied())
        .collect();
    let spec = match GroupListSpec::new(students.clone(), range.clone()) {
        Ok(spec) => Some(spec),
        Err(_) => match source {
            RangeSource::Manual => None,
            // The elected range is retried first and fails again — one wasted
            // constructor call for a search that reads as the plain fallback
            // it is.
            RangeSource::Automatic => ranked_canonical_ranges(specs)
                .into_iter()
                .find_map(|range| GroupListSpec::new(students.clone(), range).ok()),
        },
    }?;

    let attraction = attractions(specs, pinned_pairs, Some(range));
    let n = spec.students().len() as u32;
    let group_count = n.div_ceil(spec.students_per_group().end().get());
    let groups = cluster(spec.students(), group_count, &attraction);
    Some(GhostGrouping::new(spec, groups))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::tests::{period_id, range, set, student, subject_id};

    /// Bare specs with the (period, subject) pairs they cover, in the shape
    /// [`build_ghost`] takes. Every spec must be feasible on its own.
    fn specs_of(
        specs: &[(&[u64], (u32, u32), &[(u64, u64)])],
    ) -> Vec<(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)> {
        specs
            .iter()
            .map(|(students, (min, max), covered)| {
                (
                    GroupListSpec::new(set(students), range(*min, *max))
                        .expect("feasible test spec"),
                    covered
                        .iter()
                        .map(|&(p, s)| (period_id(p), subject_id(s)))
                        .collect(),
                )
            })
            .collect()
    }

    /// The template of a plan whose canonical range is elected from the specs
    /// themselves, exactly as `build_generation_plan` does it.
    fn ghost_of(
        specs: &[(&[u64], (u32, u32), &[(u64, u64)])],
        pins: &[((u32, u32), &[(u64, u64)])],
    ) -> GhostGrouping {
        let specs = specs_of(specs);
        let pinned_pairs: BTreeMap<_, BTreeSet<_>> = pins
            .iter()
            .map(|((min, max), pairs)| {
                (
                    range(*min, *max),
                    pairs
                        .iter()
                        .map(|&(a, b)| (student(a), student(b)))
                        .collect(),
                )
            })
            .collect();
        let canonical = crate::specs::elect_canonical_range(&specs)
            .map(|range| (range, RangeSource::Automatic));
        build_ghost(&specs, &pinned_pairs, canonical.as_ref())
            .expect("these instances all have a template")
    }

    /// The group of a student, as a sorted vector for readable assertions.
    fn group_with(ghost: &GhostGrouping, s: u64) -> BTreeSet<StudentId> {
        let index = ghost
            .group_of(student(s))
            .expect("the template covers every student");
        ghost.groups()[index].clone()
    }

    #[test]
    fn every_group_lands_in_the_size_range() {
        // Three shapes: an exact division, a remainder that must go into the
        // larger groups, and a single group.
        for (students, min, max, count) in [
            (vec![1, 2, 3, 4, 5, 6], 2, 2, 3),
            (vec![1, 2, 3, 4, 5, 6, 7], 2, 3, 3),
            (vec![1, 2, 3], 3, 4, 1),
        ] {
            let ghost = ghost_of(&[(&students, (min, max), &[(1, 1)])], &[]);
            assert_eq!(ghost.groups().len(), count as usize);

            let mut union = BTreeSet::new();
            for group in ghost.groups() {
                assert!(
                    group.len() >= min as usize && group.len() <= max as usize,
                    "group of {} outside {min}..={max}",
                    group.len(),
                );
                union.extend(group.iter().copied());
            }
            assert_eq!(union, set(&students));
            // And the index agrees with the groups it indexes.
            for &s in &students {
                assert!(group_with(&ghost, s).contains(&student(s)));
            }
        }
    }

    #[test]
    fn the_most_attracted_students_are_grouped() {
        // Four students at 2..=2, so two groups of two. Subject 1 covers all
        // four; subject 2 covers 1 and 2 alone, so that pair shares strictly
        // more of the document than any other and must end up together.
        let ghost = ghost_of(
            &[
                (&[1, 2, 3, 4], (2, 2), &[(1, 1)]),
                (&[1, 2], (2, 2), &[(1, 2)]),
            ],
            &[],
        );

        assert_eq!(group_with(&ghost, 1), set(&[1, 2]));
        assert_eq!(group_with(&ghost, 3), set(&[3, 4]));
    }

    #[test]
    fn a_kept_list_grouping_wins_over_affinity() {
        // Same shape, but now the co-occurrence and the pin disagree. The
        // extra spec makes (1, 2) the most affine pair; a kept list of the
        // canonical range has already grouped 1 with 3. The pin is worth a
        // full unit — the value of the *most* affine pair of the document —
        // so it outranks a grouping we merely inferred.
        let ghost = ghost_of(
            &[
                (&[1, 2, 3, 4], (2, 2), &[(1, 1)]),
                (&[1, 2], (2, 2), &[(1, 2)]),
            ],
            &[((2, 2), &[(1, 3)])],
        );

        assert_eq!(group_with(&ghost, 1), set(&[1, 3]));
        assert_eq!(group_with(&ghost, 2), set(&[2, 4]));
    }

    #[test]
    fn the_partition_is_deterministic() {
        // Six students with no distinguishing signal at all: every pair
        // co-occurs exactly once, so the greedy runs entirely on ties and
        // must still answer the same thing twice.
        let instance: &[(&[u64], (u32, u32), &[(u64, u64)])] =
            &[(&[1, 2, 3, 4, 5, 6], (2, 3), &[(1, 1)])];
        let first = ghost_of(instance, &[]);
        let second = ghost_of(instance, &[]);
        assert_eq!(first, second);
    }
}
