//! Input side of the translation layer: from the document state and the
//! user's request to the deduplicated list specs the model is built from.

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
    /// lists. Only contains pairs where both students co-occur in at least
    /// one spec — other pairs never get a variable. Unused until piece 7.
    pub pinned_pairs: BTreeSet<(StudentId, StudentId)>,
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

/// Turns a user request into the model's input: deduplicated specs, the
/// pairs nobody is registered for, and the pinned student pairs the kept
/// lists impose.
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

    let mut pinned_pairs = BTreeSet::new();
    for list_id in &request.kept_lists {
        let list = params
            .group_lists
            .group_list_map
            .get(list_id)
            .expect("kept lists were validated above");
        let GroupListFilling::Prefilled { groups } = list.filling() else {
            unreachable!("kept lists were validated to be prefilled above");
        };
        for group in groups {
            // BTreeSet iteration is sorted, so i < j guarantees a < b.
            let members: Vec<StudentId> = group.students.iter().copied().collect();
            for (i, &a) in members.iter().enumerate() {
                for &b in &members[i + 1..] {
                    let coexist = specs.iter().any(|(spec, _)| {
                        spec.students().contains(&a) && spec.students().contains(&b)
                    });
                    if coexist {
                        pinned_pairs.insert((a, b));
                    }
                }
            }
        }
    }

    Ok(GenerationPlan {
        specs,
        skipped,
        pinned_pairs,
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

    /// A prefilled list with the given groups, unnamed.
    pub(crate) fn prefilled_list(groups: &[&[u64]]) -> GroupList {
        let groups: Vec<PrefilledGroup> = groups
            .iter()
            .map(|students| PrefilledGroup {
                students: set(students),
            })
            .collect();
        GroupList::new(
            GroupListParameters {
                name: "Kept".to_string(),
                students_per_group: range(2, 3),
                group_names: vec![None; groups.len()],
            },
            GroupListFilling::Prefilled { groups },
        )
        .expect("consistent prefilled list")
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
        }
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
        // variable and must not be pinned.
        assert_eq!(
            plan.pinned_pairs,
            BTreeSet::from([(student(1), student(2))])
        );
    }
}
