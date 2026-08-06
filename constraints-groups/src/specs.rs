//! Input side of the translation layer: from the document state and the
//! user's request to the deduplicated list specs the model is built from.

use crate::ghost::{GhostGrouping, build_ghost};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::group_lists::GroupListFilling;
use collomatique_state_colloscopes::{
    GroupListId, NonEmptyRangeInclusive, PeriodId, StudentId, SubjectId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// What one generated group list must satisfy. The solver knows nothing
/// about subjects: one list is built per distinct spec. `Ord` (via the
/// `NonEmptyRangeInclusive` ordering) makes the spec its own dedup key.
///
/// The fields are private and the constructor is fallible: an unsatisfiable
/// (student count, size range) combination is unrepresentable, so everything
/// downstream — the group count of `VarEnv`, the model, the conversion — may
/// assume a feasible spec.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupListSpec {
    students: BTreeSet<StudentId>,
    students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
}

/// Why a [`GroupListSpec`] cannot exist. Splitting `n` students into `k`
/// groups of `min` to `max` needs `k·min <= n <= k·max`, so the feasible
/// counts are the interval `⌈n/max⌉ ..= ⌊n/min⌋`, and the spec is
/// satisfiable exactly when that interval is non-empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GroupListSpecError {
    #[error("a group list spec needs at least one student")]
    NoStudents,
    #[error("{students} students cannot be split into groups of {min} to {max} students")]
    UnsatisfiableSize {
        students: u32,
        min: NonZeroU32,
        max: NonZeroU32,
    },
}

impl GroupListSpec {
    /// The single feasibility gate of the crate: at least one student, and
    /// `⌈n/max⌉ · min <= n` (the minimal group count must not undershoot the
    /// minimum size). The config dialog runs it before offering a pair for
    /// rebuild, so a failure downstream is a caller bug.
    pub fn new(
        students: BTreeSet<StudentId>,
        students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
    ) -> Result<GroupListSpec, GroupListSpecError> {
        let n = u32::try_from(students.len()).unwrap_or(u32::MAX);
        if n == 0 {
            return Err(GroupListSpecError::NoStudents);
        }
        let min = *students_per_group.start();
        let max = *students_per_group.end();
        // The minimal count is the only one worth testing: below it the
        // groups overflow `max`, above it they only get emptier.
        let count = n.div_ceil(max.get());
        if u64::from(count) * u64::from(min.get()) > u64::from(n) {
            return Err(GroupListSpecError::UnsatisfiableSize {
                students: n,
                min,
                max,
            });
        }
        Ok(GroupListSpec {
            students,
            students_per_group,
        })
    }

    /// Exactly the students that must be placed. Every one gets a group.
    pub fn students(&self) -> &BTreeSet<StudentId> {
        &self.students
    }

    /// The allowed group size range, from the subject's
    /// `SubjectInterrogationParameters.students_per_group`.
    pub fn students_per_group(&self) -> &NonEmptyRangeInclusive<NonZeroU32> {
        &self.students_per_group
    }
}

/// The user's selection, as assembled by the config dialog (piece 2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GenerationRequest {
    /// (period, subject) pairs to build new lists for.
    pub rebuild: BTreeSet<(PeriodId, SubjectId)>,
    /// Existing prefilled lists whose pairs are pinned as already-shared.
    pub kept_lists: BTreeSet<GroupListId>,
    /// Canonical group-size range override: `None` elects it automatically
    /// (a student-weighted vote among the rebuilt specs), `Some` fixes it.
    pub canonical_range: Option<NonEmptyRangeInclusive<NonZeroU32>>,
}

/// Where a plan's canonical range came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeSource {
    /// Elected by the student-weighted vote among the specs.
    Automatic,
    /// Fixed by the request's override.
    Manual,
}

/// The deduplicated, model-ready form of a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationPlan {
    /// Deduplicated specs, each with the (period, subject) pairs it covers.
    /// The order is deterministic (sorted by the dedup key); `GroupListIdx`
    /// indexes into this vector.
    pub specs: Vec<(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)>,
    /// (period, subject) pairs skipped because nobody is registered —
    /// reported so the UI can warn instead of silently dropping them.
    pub skipped: BTreeSet<(PeriodId, SubjectId)>,
    /// Pairs of students (a < b) fixed to "already shared" by the kept
    /// lists, keyed by the kept list's own size range: a pin only discounts
    /// the size class it was observed in — a kept tutorial must not make
    /// colle pairs free. Only contains pairs where both students co-occur in
    /// a spec of that same range; other pairs never get a variable there.
    pub pinned_pairs:
        BTreeMap<NonEmptyRangeInclusive<NonZeroU32>, BTreeSet<(StudentId, StudentId)>>,
    /// The canonical group size — the size the *typical* student is grouped
    /// at — and where it came from. Size classes further from it weigh less
    /// in the stability objective (see `VarEnv::class_weight`). `None` only
    /// when the plan has no specs at all.
    pub canonical_range: Option<(NonEmptyRangeInclusive<NonZeroU32>, RangeSource)>,
    /// The template ("ghost") grouping: every student of the plan, split at
    /// the canonical size. It is not a list of the plan — it is never
    /// converted to output ([`build_group_lists`](crate::build_group_lists)
    /// iterates `specs`) and gets no `SharedPair` variable — but a fixed
    /// grouping the objective asks the real lists to resemble. That is what
    /// tells "nine identical lists and one different" from "five and five",
    /// which the per-pair step term cannot see.
    ///
    /// Computed here rather than decided by the solver: see the
    /// [`ghost`](crate::ghost) module doc.
    ///
    /// `None` when the plan has no specs, and when the canonical size cannot
    /// split the whole student body at all: the template term is then simply
    /// absent.
    pub ghost: Option<GhostGrouping>,
}

/// A malformed request. These are caller bugs (the config dialog only
/// offers valid choices), as opposed to the legitimate `skipped` pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GenerationPlanError {
    #[error("rebuild pair references unknown period {0:?}")]
    UnknownPeriod(PeriodId),
    #[error("rebuild pair references unknown subject {0:?}")]
    UnknownSubject(SubjectId),
    #[error("rebuild pair references subject {0:?} which has no interrogations")]
    SubjectWithoutInterrogations(SubjectId),
    #[error("kept list {0:?} does not exist")]
    UnknownKeptList(GroupListId),
    #[error("kept list {0:?} is not prefilled")]
    KeptListNotPrefilled(GroupListId),
    #[error("rebuild pair ({0:?}, {1:?}) has an unusable spec: {2}")]
    InvalidSpec(PeriodId, SubjectId, GroupListSpecError),
}

/// The distinct group-size ranges of the specs, best canonical candidate
/// first. Each spec votes for its own range with weight `students × covering
/// (period, subject) pairs` — one vote per real grouping obligation,
/// weighted by how many students it concerns — so a range used by two
/// subjects of the whole class beats one used by a single subject with four
/// students registered.
///
/// Ties break toward the *tighter* range: smaller maximum first, then larger
/// minimum. A range is exactly its (start, end), so no two candidates can
/// share a full sort key and the order never depends on iteration order.
///
/// A spec covering nothing votes with weight `students`: that only happens
/// in hand-built test plans, and a plan whose specs all cover nothing would
/// otherwise be ranked on ties alone.
pub(crate) fn ranked_canonical_ranges(
    specs: &[(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)],
) -> Vec<NonEmptyRangeInclusive<NonZeroU32>> {
    let mut votes: BTreeMap<NonEmptyRangeInclusive<NonZeroU32>, u64> = BTreeMap::new();
    for (spec, covered) in specs {
        let weight = spec.students().len() as u64 * covered.len().max(1) as u64;
        *votes.entry(spec.students_per_group().clone()).or_default() += weight;
    }
    let mut ranked: Vec<(NonEmptyRangeInclusive<NonZeroU32>, u64)> = votes.into_iter().collect();
    ranked.sort_by_key(|(range, weight)| {
        std::cmp::Reverse((
            *weight,
            std::cmp::Reverse(range.end().get()),
            range.start().get(),
        ))
    });
    ranked.into_iter().map(|(range, _)| range).collect()
}

/// Elect the canonical group-size range: the winner of the vote described by
/// [`ranked_canonical_ranges`]. `None` only when there is no spec to vote.
pub(crate) fn elect_canonical_range(
    specs: &[(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)],
) -> Option<NonEmptyRangeInclusive<NonZeroU32>> {
    ranked_canonical_ranges(specs).into_iter().next()
}

/// Weight of a size class in the objective: how much a pair meeting in a
/// group of this range matters, relative to a meeting at the canonical size.
/// 1 for the canonical range and anything tighter, then decaying like
/// `(canonical_max − 1) / (class_max − 1)` — in a tutorial group of 20 every
/// student meets 19 others whatever the model does, so such a meeting is a
/// far weaker tie than one in a group of 3, and pricing them alike lets
/// tutorials pre-pay (and thereby free) every colle pair.
///
/// Without a canonical range (a plan with no specs) every class weighs 1.
///
/// Callers must skip ranges of maximum 1 — a group of one holds no pair, so
/// the divisor is never 0. A *canonical* maximum of 1 is read as 2 instead,
/// so a document whose typical subject takes students one at a time still
/// ranks its real groups by size rather than zeroing the objective.
///
/// Free rather than a [`VarEnv`](crate::vars::VarEnv) method because the
/// template grouping is computed from the raw specs, before any `VarEnv`
/// exists; `VarEnv::class_weight` delegates here.
pub(crate) fn class_weight(
    canonical_range: Option<&NonEmptyRangeInclusive<NonZeroU32>>,
    class_range: &NonEmptyRangeInclusive<NonZeroU32>,
) -> f64 {
    let Some(canonical) = canonical_range else {
        return 1.0;
    };
    let canon_max = canonical.end().get().max(2);
    let class_max = class_range.end().get();
    (f64::from(canon_max - 1) / f64::from(class_max - 1)).min(1.0)
}

/// The pairs `(a, b)` with `a < b` of a student set, in order. `BTreeSet`
/// iteration is sorted, so taking the members in order and pairing each with
/// its successors guarantees `a < b`.
pub(crate) fn pairs_of(students: &BTreeSet<StudentId>) -> Vec<(StudentId, StudentId)> {
    let members: Vec<StudentId> = students.iter().copied().collect();
    let mut pairs = Vec::new();
    for (i, &a) in members.iter().enumerate() {
        for &b in &members[i + 1..] {
            pairs.push((a, b));
        }
    }
    pairs
}

/// Turns a user request into the model's input: deduplicated specs, the
/// pairs nobody is registered for, the pinned student pairs the kept lists
/// impose, the canonical group size the objective weighs classes against,
/// and the template grouping it measures deviations from.
pub fn build_generation_plan(
    params: &Parameters,
    request: &GenerationRequest,
) -> Result<GenerationPlan, GenerationPlanError> {
    for list_id in &request.kept_lists {
        let list = params
            .group_lists
            .group_list_map
            .get(list_id)
            .ok_or(GenerationPlanError::UnknownKeptList(*list_id))?;
        if !list.is_prefilled() {
            return Err(GenerationPlanError::KeptListNotPrefilled(*list_id));
        }
    }

    let mut skipped = BTreeSet::new();
    // Dedup key: the spec itself. The `BTreeMap` makes the final spec
    // order deterministic (sorted by student set, then range).
    let mut spec_map: BTreeMap<GroupListSpec, BTreeSet<(PeriodId, SubjectId)>> = BTreeMap::new();

    for &(period, subject) in &request.rebuild {
        if params.periods.find_period_position(period).is_none() {
            return Err(GenerationPlanError::UnknownPeriod(period));
        }
        let subject_data = params
            .subjects
            .find_subject(subject)
            .ok_or(GenerationPlanError::UnknownSubject(subject))?;
        let interrogation_params = subject_data
            .parameters
            .interrogation_parameters
            .as_ref()
            .ok_or(GenerationPlanError::SubjectWithoutInterrogations(subject))?;

        // An absent row is the canonical form of "nobody assigned"; the
        // emptiness guard is defensive against a non-canonical one.
        let students = match params.assignments.students(period, subject) {
            Some(students) if !students.is_empty() => students.clone(),
            _ => {
                skipped.insert((period, subject));
                continue;
            }
        };

        // The config dialog gates on the very same constructor, so a
        // failure here is a caller bug like the variants above.
        let spec = GroupListSpec::new(students, interrogation_params.students_per_group.clone())
            .map_err(|e| GenerationPlanError::InvalidSpec(period, subject, e))?;
        spec_map.entry(spec).or_default().insert((period, subject));
    }

    let specs: Vec<(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)> =
        spec_map.into_iter().collect();

    let canonical_range = match &request.canonical_range {
        Some(range) => Some((range.clone(), RangeSource::Manual)),
        None => elect_canonical_range(&specs).map(|range| (range, RangeSource::Automatic)),
    };
    let mut pinned_pairs: BTreeMap<_, BTreeSet<(StudentId, StudentId)>> = BTreeMap::new();
    for list_id in &request.kept_lists {
        let list = params
            .group_lists
            .group_list_map
            .get(list_id)
            .expect("kept lists were validated above");
        // The kept list's own size range is the class its pairs were formed
        // in, and the only class they discount.
        let range = list.params().students_per_group.clone();
        let GroupListFilling::Prefilled { groups } = list.filling() else {
            unreachable!("kept lists were validated to be prefilled above");
        };
        for group in groups {
            // BTreeSet iteration is sorted, so i < j guarantees a < b.
            let members: Vec<StudentId> = group.students.iter().copied().collect();
            for (i, &a) in members.iter().enumerate() {
                for &b in &members[i + 1..] {
                    let coexist = specs.iter().any(|(spec, _)| {
                        *spec.students_per_group() == range
                            && spec.students().contains(&a)
                            && spec.students().contains(&b)
                    });
                    if coexist {
                        pinned_pairs
                            .entry(range.clone())
                            .or_default()
                            .insert((a, b));
                    }
                }
            }
        }
    }

    // After the pins: the template is clustered on an affinity graph that
    // folds them in, so a kept list must already be known here.
    let ghost = build_ghost(&specs, &pinned_pairs, canonical_range.as_ref());

    Ok(GenerationPlan {
        specs,
        skipped,
        pinned_pairs,
        canonical_range,
        ghost,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListParameters, PrefilledGroup,
    };
    use collomatique_state_colloscopes::ids::Id;
    use collomatique_state_colloscopes::{
        Subject, SubjectInterrogationParameters, SubjectParameters,
    };

    pub(crate) fn student(n: u64) -> StudentId {
        unsafe { StudentId::new(n) }
    }
    pub(crate) fn subject_id(n: u64) -> SubjectId {
        unsafe { SubjectId::new(n) }
    }
    pub(crate) fn period_id(n: u64) -> PeriodId {
        unsafe { PeriodId::new(n) }
    }
    pub(crate) fn group_list_id(n: u64) -> GroupListId {
        unsafe { GroupListId::new(n) }
    }

    pub(crate) fn set(ns: &[u64]) -> BTreeSet<StudentId> {
        ns.iter().map(|&n| student(n)).collect()
    }

    pub(crate) fn range(min: u32, max: u32) -> NonEmptyRangeInclusive<NonZeroU32> {
        NonEmptyRangeInclusive::new(
            NonZeroU32::new(min).expect("non-zero")..=NonZeroU32::new(max).expect("non-zero"),
        )
        .expect("non-empty")
    }

    fn subject_with_range(min: u32, max: u32) -> Subject {
        Subject {
            parameters: SubjectParameters {
                name: "S".to_string(),
                interrogation_parameters: Some(SubjectInterrogationParameters {
                    students_per_group: range(min, max),
                    ..Default::default()
                }),
            },
            excluded_periods: BTreeSet::new(),
        }
    }

    fn subject_without_interrogations() -> Subject {
        Subject {
            parameters: SubjectParameters {
                name: "S".to_string(),
                interrogation_parameters: None,
            },
            excluded_periods: BTreeSet::new(),
        }
    }

    /// A prefilled list with the given groups, unnamed, at the range of
    /// subjects 1 and 2 — so its pins land in the class those subjects build.
    pub(crate) fn prefilled_list(groups: &[&[u64]]) -> GroupList {
        prefilled_list_with_range(groups, range(2, 3))
    }

    /// Two periods; subjects 1 and 2 share the range 2..=3, subject 3 has
    /// 1..=2, subject 4 has no interrogations and subject 5 has the range
    /// 5..=6, unsatisfiable for the four students it is assigned; four
    /// assignment rows, all on period 1.
    pub(crate) fn base_params() -> Parameters {
        let mut params = Parameters::default();
        params.periods.ordered_period_list = vec![(period_id(1), ()), (period_id(2), ())]
            .try_into()
            .expect("distinct period ids");
        params.subjects.ordered_subject_list = vec![
            (subject_id(1), subject_with_range(2, 3)),
            (subject_id(2), subject_with_range(2, 3)),
            (subject_id(3), subject_with_range(1, 2)),
            (subject_id(4), subject_without_interrogations()),
            (subject_id(5), subject_with_range(5, 6)),
        ]
        .try_into()
        .expect("distinct subject ids");
        params.assignments.map = vec![
            ((period_id(1), subject_id(1)), set(&[1, 2, 3, 4])),
            ((period_id(1), subject_id(2)), set(&[1, 2, 3, 4])),
            ((period_id(1), subject_id(3)), set(&[1, 2, 3, 4])),
            ((period_id(1), subject_id(5)), set(&[1, 2, 3, 4])),
        ]
        .into_iter()
        .collect();
        params
    }

    fn request(rebuild: &[(u64, u64)], kept: &[u64]) -> GenerationRequest {
        GenerationRequest {
            rebuild: rebuild
                .iter()
                .map(|&(p, s)| (period_id(p), subject_id(s)))
                .collect(),
            kept_lists: kept.iter().map(|&id| group_list_id(id)).collect(),
            canonical_range: None,
        }
    }

    /// A prefilled list with the given groups and size range, unnamed.
    pub(crate) fn prefilled_list_with_range(
        groups: &[&[u64]],
        students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
    ) -> GroupList {
        let groups: Vec<PrefilledGroup> = groups
            .iter()
            .map(|students| PrefilledGroup {
                students: set(students),
            })
            .collect();
        GroupList::new(
            GroupListParameters {
                name: "Kept".to_string(),
                students_per_group,
                group_names: vec![None; groups.len()],
            },
            GroupListFilling::Prefilled { groups },
        )
        .expect("consistent prefilled list")
    }

    #[test]
    fn spec_needs_students() {
        assert_eq!(
            GroupListSpec::new(BTreeSet::new(), range(1, 2)),
            Err(GroupListSpecError::NoStudents)
        );
    }

    #[test]
    fn spec_rejects_unsatisfiable_sizes() {
        // 2 students in groups of 3 to 4: the minimal count is
        // ceil(2 / 4) = 1 group, which then needs 3 students.
        assert_eq!(
            GroupListSpec::new(set(&[1, 2]), range(3, 4)),
            Err(GroupListSpecError::UnsatisfiableSize {
                students: 2,
                min: NonZeroU32::new(3).expect("non-zero"),
                max: NonZeroU32::new(4).expect("non-zero"),
            })
        );
    }

    #[test]
    fn spec_feasibility_is_exact_at_the_boundary() {
        // Fixed size 2: ceil(5 / 2) = 3 groups need 6 students, so 5 is
        // rejected and 6 is accepted. A loose `n >= min` test would accept
        // both, and a `n % min == 0` test would reject sizes that a wider
        // range makes perfectly splittable (7 students in groups of 2 to 3).
        assert_eq!(
            GroupListSpec::new(set(&[1, 2, 3, 4, 5]), range(2, 2)),
            Err(GroupListSpecError::UnsatisfiableSize {
                students: 5,
                min: NonZeroU32::new(2).expect("non-zero"),
                max: NonZeroU32::new(2).expect("non-zero"),
            })
        );
        assert!(GroupListSpec::new(set(&[1, 2, 3, 4, 5, 6]), range(2, 2)).is_ok());
        assert!(GroupListSpec::new(set(&[1, 2, 3, 4, 5, 6, 7]), range(2, 3)).is_ok());
    }

    #[test]
    fn unsatisfiable_sizes_error() {
        let params = base_params();
        // Subject 5 wants groups of 5 to 6 students out of the four
        // registered ones.
        assert_eq!(
            build_generation_plan(&params, &request(&[(1, 5)], &[])),
            Err(GenerationPlanError::InvalidSpec(
                period_id(1),
                subject_id(5),
                GroupListSpecError::UnsatisfiableSize {
                    students: 4,
                    min: NonZeroU32::new(5).expect("non-zero"),
                    max: NonZeroU32::new(6).expect("non-zero"),
                }
            ))
        );
    }

    #[test]
    fn identical_pairs_dedup_into_one_spec() {
        let params = base_params();
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 2)], &[]))
            .expect("well-formed request");

        assert_eq!(plan.specs.len(), 1);
        let (spec, covered) = &plan.specs[0];
        assert_eq!(*spec.students(), set(&[1, 2, 3, 4]));
        assert_eq!(*spec.students_per_group(), range(2, 3));
        assert_eq!(
            *covered,
            BTreeSet::from([(period_id(1), subject_id(1)), (period_id(1), subject_id(2)),])
        );
        assert!(plan.skipped.is_empty());
        assert!(plan.pinned_pairs.is_empty());
    }

    #[test]
    fn different_ranges_stay_separate() {
        let params = base_params();
        // Same student set, different students-per-group range.
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[]))
            .expect("well-formed request");

        assert_eq!(plan.specs.len(), 2);
        assert_eq!(plan.specs[0].0.students(), plan.specs[1].0.students());
        assert_ne!(
            plan.specs[0].0.students_per_group(),
            plan.specs[1].0.students_per_group()
        );
    }

    #[test]
    fn missing_assignment_row_is_skipped() {
        let params = base_params();
        // Period 2 exists but carries no assignment row at all.
        let plan =
            build_generation_plan(&params, &request(&[(2, 1)], &[])).expect("well-formed request");

        assert!(plan.specs.is_empty());
        assert_eq!(
            plan.skipped,
            BTreeSet::from([(period_id(2), subject_id(1))])
        );
    }

    #[test]
    fn unknown_period_errors() {
        let params = base_params();
        assert_eq!(
            build_generation_plan(&params, &request(&[(9, 1)], &[])),
            Err(GenerationPlanError::UnknownPeriod(period_id(9)))
        );
    }

    #[test]
    fn unknown_subject_errors() {
        let params = base_params();
        assert_eq!(
            build_generation_plan(&params, &request(&[(1, 9)], &[])),
            Err(GenerationPlanError::UnknownSubject(subject_id(9)))
        );
    }

    #[test]
    fn no_interrogations_errors() {
        let params = base_params();
        assert_eq!(
            build_generation_plan(&params, &request(&[(1, 4)], &[])),
            Err(GenerationPlanError::SubjectWithoutInterrogations(
                subject_id(4)
            ))
        );
    }

    #[test]
    fn kept_list_validation_errors() {
        let mut params = base_params();
        assert_eq!(
            build_generation_plan(&params, &request(&[], &[7])),
            Err(GenerationPlanError::UnknownKeptList(group_list_id(7)))
        );

        // An automatic list is the default filling.
        params
            .group_lists
            .group_list_map
            .insert(group_list_id(7), GroupList::default());
        assert_eq!(
            build_generation_plan(&params, &request(&[], &[7])),
            Err(GenerationPlanError::KeptListNotPrefilled(group_list_id(7)))
        );
    }

    #[test]
    fn pinned_pairs_filtered_to_spec_coverage() {
        let mut params = base_params();
        params
            .group_lists
            .group_list_map
            .insert(group_list_id(7), prefilled_list(&[&[1, 2], &[3, 9]]));

        let plan =
            build_generation_plan(&params, &request(&[(1, 1)], &[7])).expect("well-formed request");

        // Student 9 belongs to no spec, so the pair (3, 9) never gets a
        // variable and must not be pinned. The kept list has the range
        // 2..=3, which is the range of subject 1's spec, so the pin lands
        // in that class.
        assert_eq!(
            plan.pinned_pairs,
            BTreeMap::from([(range(2, 3), BTreeSet::from([(student(1), student(2))]))])
        );
    }

    #[test]
    fn pins_only_discount_their_own_size_class() {
        let mut params = base_params();
        // The kept list groups its students two at a time, so it says
        // nothing about the 1..=2 class subject 3 builds — and nothing at
        // all about a class no rebuilt spec uses.
        params
            .group_lists
            .group_list_map
            .insert(group_list_id(7), prefilled_list(&[&[1, 2], &[3, 4]]));

        // Subject 1 is 2..=3, subject 3 is 1..=2: both are rebuilt, only
        // the first matches the kept list's range.
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[7]))
            .expect("well-formed request");

        assert_eq!(
            plan.pinned_pairs,
            BTreeMap::from([(
                range(2, 3),
                BTreeSet::from([(student(1), student(2)), (student(3), student(4))]),
            )])
        );

        // A kept list of a range nobody rebuilds pins nothing at all.
        params.group_lists.group_list_map.insert(
            group_list_id(8),
            prefilled_list_with_range(&[&[1, 2, 3, 4]], range(4, 4)),
        );
        let plan =
            build_generation_plan(&params, &request(&[(1, 3)], &[8])).expect("well-formed request");
        assert!(plan.pinned_pairs.is_empty());
    }

    #[test]
    fn canonical_range_is_the_student_weighted_mode() {
        let mut params = base_params();
        // Subject 3 (1..=2) covers the whole class of four; subjects 1 and 2
        // share the range 2..=3 and cover the same four students. Two
        // covering pairs against one, so 2..=3 wins 8 votes to 4.
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 2), (1, 3)], &[]))
            .expect("well-formed request");
        assert_eq!(
            plan.canonical_range,
            Some((range(2, 3), RangeSource::Automatic))
        );

        // Shrink subject 1 and 2's audience to a two-student oddball: 1..=2
        // now carries 4 votes against 2 + 2, and takes the election.
        params
            .assignments
            .map
            .insert((period_id(1), subject_id(1)), set(&[1, 2]));
        params
            .assignments
            .map
            .insert((period_id(1), subject_id(2)), set(&[3, 4]));
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 2), (1, 3)], &[]))
            .expect("well-formed request");
        assert_eq!(
            plan.canonical_range,
            Some((range(1, 2), RangeSource::Automatic))
        );
    }

    #[test]
    fn canonical_range_ties_break_toward_the_tighter_range() {
        let mut params = base_params();
        // One subject each, same four students: 2..=3 and 1..=2 both score
        // 4. The smaller maximum wins, and among equal maxima the larger
        // minimum would.
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[]))
            .expect("well-formed request");
        assert_eq!(
            plan.canonical_range,
            Some((range(1, 2), RangeSource::Automatic))
        );

        // Same maximum now: 2..=2 against 1..=2, still tied at 4 votes each.
        params.subjects.ordered_subject_list = vec![
            (subject_id(1), subject_with_range(2, 2)),
            (subject_id(3), subject_with_range(1, 2)),
        ]
        .try_into()
        .expect("distinct subject ids");
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[]))
            .expect("well-formed request");
        assert_eq!(
            plan.canonical_range,
            Some((range(2, 2), RangeSource::Automatic))
        );
    }

    #[test]
    fn canonical_range_override_wins_and_is_reported_as_manual() {
        let params = base_params();
        let mut req = request(&[(1, 1), (1, 2), (1, 3)], &[]);
        // The vote would elect 2..=3 (see above), and the override need not
        // even be a range any spec uses.
        req.canonical_range = Some(range(4, 5));
        let plan = build_generation_plan(&params, &req).expect("well-formed request");
        assert_eq!(
            plan.canonical_range,
            Some((range(4, 5), RangeSource::Manual))
        );
    }

    #[test]
    fn empty_plan_has_no_canonical_range() {
        let params = base_params();
        // Period 2 carries no assignment row, so every requested pair is
        // skipped and no spec survives to vote.
        let plan =
            build_generation_plan(&params, &request(&[(2, 1)], &[])).expect("well-formed request");
        assert!(plan.specs.is_empty());
        assert_eq!(plan.canonical_range, None);
        // And with no canonical size and no student, no template either.
        assert_eq!(plan.ghost, None);
    }

    #[test]
    fn ghost_is_the_canonical_partition_of_every_student() {
        let mut params = base_params();
        // Subjects 1 and 2 share the range 2..=3 and carry eight students
        // between them; subject 3's 1..=2 covers only four, so 2..=3 takes
        // the election 8 votes to 4.
        params
            .assignments
            .map
            .insert((period_id(1), subject_id(2)), set(&[5, 6, 7, 8]));
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 2), (1, 3)], &[]))
            .expect("well-formed request");

        // The template spans the *union* of the specs, which is wider than
        // any single one of them: it is the grouping every list is asked to
        // resemble, so every student needs a place in it.
        let ghost = plan.ghost.expect("the canonical range splits the union");
        assert_eq!(*ghost.spec().students(), set(&[1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(*ghost.spec().students_per_group(), range(2, 3));
    }

    #[test]
    fn a_kept_list_shapes_the_template() {
        // Why `build_ghost` runs *after* the pinned-pair loop and not before.
        // One rebuilt spec of four students at 2..=3, so the canonical range
        // is that same 2..=3 and the template is two groups of two. The spec
        // alone says nothing — every pair co-occurs exactly once — so with an
        // empty pin map the greedy falls back on its tie-break and groups
        // {1, 2} / {3, 4}.
        //
        // The kept list, of that same range, has already grouped 1 with 3.
        // Fed the pins, the greedy follows it.
        let mut params = base_params();
        params
            .group_lists
            .group_list_map
            .insert(group_list_id(7), prefilled_list(&[&[1, 3], &[2, 4]]));

        let plan =
            build_generation_plan(&params, &request(&[(1, 1)], &[7])).expect("well-formed request");
        let ghost = plan.ghost.expect("the canonical range splits the union");
        assert_eq!(ghost.groups(), [set(&[1, 3]), set(&[2, 4])]);

        // Without the kept list the very same document templates otherwise.
        let plan =
            build_generation_plan(&params, &request(&[(1, 1)], &[])).expect("well-formed request");
        let ghost = plan.ghost.expect("the canonical range splits the union");
        assert_eq!(ghost.groups(), [set(&[1, 2]), set(&[3, 4])]);
    }

    #[test]
    fn an_unsplittable_canonical_range_falls_back_to_the_next_voted_one() {
        let mut params = base_params();
        // Groups of exactly 3 for six students, groups of exactly 2 for two
        // others: 3..=3 wins the vote 6 to 2, but the union of eight
        // students cannot be split into groups of exactly 3 (three groups
        // would need nine). The runner-up 2..=2 splits it into four.
        params.subjects.ordered_subject_list = vec![
            (subject_id(1), subject_with_range(3, 3)),
            (subject_id(3), subject_with_range(2, 2)),
        ]
        .try_into()
        .expect("distinct subject ids");
        params.assignments.map = vec![
            ((period_id(1), subject_id(1)), set(&[1, 2, 3, 4, 5, 6])),
            ((period_id(1), subject_id(3)), set(&[7, 8])),
        ]
        .into_iter()
        .collect();

        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[]))
            .expect("well-formed request");

        // The election itself is untouched — the class weights still measure
        // against 3..=3. Only the template moves.
        assert_eq!(
            plan.canonical_range,
            Some((range(3, 3), RangeSource::Automatic))
        );
        let ghost = plan.ghost.expect("the runner-up splits the union");
        assert_eq!(*ghost.spec().students(), set(&[1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(*ghost.spec().students_per_group(), range(2, 2));
    }

    #[test]
    fn an_unsplittable_manual_range_leaves_no_ghost() {
        let params = base_params();
        // Four students in groups of 5 to 6: the single group is short of
        // the minimum, so no template exists at that size.
        let mut req = request(&[(1, 1), (1, 3)], &[]);
        req.canonical_range = Some(range(5, 6));
        let plan = build_generation_plan(&params, &req).expect("well-formed request");

        assert_eq!(
            plan.canonical_range,
            Some((range(5, 6), RangeSource::Manual))
        );
        assert_eq!(plan.ghost, None);

        // The very same union does get a template when the range was
        // elected rather than chosen: only an explicit choice suppresses
        // the fallback.
        let plan = build_generation_plan(&params, &request(&[(1, 1), (1, 3)], &[]))
            .expect("well-formed request");
        assert!(plan.ghost.is_some());
    }
}
