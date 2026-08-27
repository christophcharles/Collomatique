//! Placement state and every score computation.
//!
//! The scoring rule of the design's §8: a candidate's score is always a
//! **pure function of the current configuration**, recomputed from the
//! placement tables. No running accumulator — its float drift could make two
//! states each look better than the other, and the sweep of `pass.rs` relies
//! on a strict `>` over a pure function to be unable to cycle.

use crate::frozen::FrozenPlacements;
use crate::pairs::{pair_mass, plan_n_uses};
use crate::specs::{GenerationPlan, KeptList};
use crate::targets::balanced_targets;
use crate::vars::GroupListIdx;
use collomatique_state_colloscopes::group_lists::{
    GroupList, GroupListFilling, GroupListParameters, PrefilledGroup,
};
use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};
use std::collections::{BTreeMap, BTreeSet};

/// One rebuilt list under construction.
struct ListState {
    /// How many (period, subject) pairs the list serves — the multiplicity
    /// `k` of §2.1. Honestly `covered.len()`: a spec covering nothing (only
    /// reachable in hand-built plans) weighs nothing, its students are still
    /// placed.
    multiplicity: usize,
    /// Group targets, descending, fixed before any placement (§3).
    targets: Vec<u32>,
    /// Current members, indexed like `targets`.
    fills: Vec<BTreeSet<StudentId>>,
}

/// Everything the greedy reads and writes: the rebuilt lists' fills, the
/// constants derived from the plan, and the scoring.
pub(super) struct State<'a> {
    plan: &'a GenerationPlan,
    lists: Vec<ListState>,
    /// Spec indices whose student set contains the student. Only students
    /// appearing in at least one spec get an entry — a student known solely
    /// through a kept list is never placed.
    profiles: BTreeMap<StudentId, BTreeSet<usize>>,
    /// student -> (spec index -> group index), placed lists only.
    placements: BTreeMap<StudentId, BTreeMap<usize, usize>>,
    /// Prefill output: (student, spec) placements the greedy never revises.
    frozen: BTreeSet<(StudentId, usize)>,
    /// `N_s` — *all* of s's list-uses, rebuilt and kept alike (the fixed-N
    /// convention of §2.2). Its keys are the whole student universe.
    n_uses: BTreeMap<StudentId, usize>,
    /// The kept lists that weigh something. A zero-use list is inert and is
    /// dropped here, so it cannot split cohorts for nothing.
    kept: Vec<&'a KeptList>,
    /// student -> its (kept index, group index) memberships, ascending.
    kept_memberships: BTreeMap<StudentId, Vec<(usize, usize)>>,
}

impl<'a> State<'a> {
    /// Derives the constants of the plan — targets, multiplicities, profiles,
    /// kept-list masses, `N_s` — and starts with every list empty.
    pub(super) fn new(plan: &'a GenerationPlan) -> State<'a> {
        let lists: Vec<ListState> = plan
            .specs
            .iter()
            .map(|(spec, covered)| {
                let n = u32::try_from(spec.students().len()).unwrap_or(u32::MAX);
                let targets = balanced_targets(n, spec.students_per_group());
                let fills = vec![BTreeSet::new(); targets.len()];
                ListState {
                    multiplicity: covered.len(),
                    targets,
                    fills,
                }
            })
            .collect();

        let mut profiles: BTreeMap<StudentId, BTreeSet<usize>> = BTreeMap::new();
        for (list, (spec, _covered)) in plan.specs.iter().enumerate() {
            for &student in spec.students() {
                profiles.entry(student).or_default().insert(list);
            }
        }

        let kept: Vec<&KeptList> = plan
            .kept_lists
            .iter()
            .filter(|kept| kept.use_count > 0)
            .collect();
        let mut kept_memberships: BTreeMap<StudentId, Vec<(usize, usize)>> = BTreeMap::new();
        for (k, list) in kept.iter().enumerate() {
            for (g, group) in list.groups.iter().enumerate() {
                for &student in group {
                    // Pushed in ascending (k, g) order, hence already sorted:
                    // the cohort key of `cohorts.rs` compares these vectors.
                    kept_memberships.entry(student).or_default().push((k, g));
                }
            }
        }

        // Same student universe as the two tables above: the specs' students,
        // plus whoever a weighing kept list groups.
        let n_uses = plan_n_uses(plan);

        State {
            plan,
            lists,
            profiles,
            placements: BTreeMap::new(),
            frozen: BTreeSet::new(),
            n_uses,
            kept,
            kept_memberships,
        }
    }

    // --- structure -------------------------------------------------------

    /// The group targets of a list, descending.
    pub(super) fn targets(&self, list: usize) -> &[u32] {
        &self.lists[list].targets
    }

    /// Whether the group can still take one more student.
    pub(super) fn has_free_seat(&self, list: usize, group: usize) -> bool {
        let list = &self.lists[list];
        list.fills[group].len() < list.targets[group] as usize
    }

    /// Whether nobody sits in the group yet — prefill only claims empty ones.
    pub(super) fn is_empty_group(&self, list: usize, group: usize) -> bool {
        self.lists[list].fills[group].is_empty()
    }

    /// The spec indices the student must be placed in.
    pub(super) fn profile(&self, student: StudentId) -> &BTreeSet<usize> {
        static EMPTY: std::sync::LazyLock<BTreeSet<usize>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.profiles.get(&student).unwrap_or(&EMPTY)
    }

    /// The students of the plan, in ascending order: everyone in a spec, plus
    /// everyone a kept list groups.
    pub(super) fn universe(&self) -> impl Iterator<Item = StudentId> + '_ {
        self.n_uses.keys().copied()
    }

    /// `N_s`, the number of list-uses the student takes part in.
    pub(super) fn n_uses(&self, student: StudentId) -> usize {
        self.n_uses.get(&student).copied().unwrap_or(0)
    }

    /// The student's kept-list memberships, as ascending `(kept index, group
    /// index)` pairs — the second half of the cohort key.
    pub(super) fn kept_memberships(&self, student: StudentId) -> &[(usize, usize)] {
        self.kept_memberships
            .get(&student)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Whether every list of the student's profile already holds them.
    pub(super) fn fully_placed(&self, student: StudentId) -> bool {
        let placed = self.placements.get(&student).map_or(0, BTreeMap::len);
        placed == self.profile(student).len()
    }

    /// The student's lists that still need a group, in spec order.
    pub(super) fn unplaced_lists(&self, student: StudentId) -> Vec<usize> {
        let placed = self.placements.get(&student);
        self.profile(student)
            .iter()
            .copied()
            .filter(|list| !placed.is_some_and(|p| p.contains_key(list)))
            .collect()
    }

    /// The student's lists the revision sweep may re-choose: the profile
    /// minus what prefill froze.
    pub(super) fn movable_lists(&self, student: StudentId) -> Vec<usize> {
        self.profile(student)
            .iter()
            .copied()
            .filter(|&list| !self.frozen.contains(&(student, list)))
            .collect()
    }

    /// What prefill seated and the pass never touched, as model coordinates.
    ///
    /// The lookup cannot miss: `freeze` is only ever called right after
    /// `place` on the same pair, and nothing removes a frozen placement —
    /// the revision sweep only visits `movable_lists`, the profile *minus*
    /// `frozen`.
    pub(super) fn frozen_placements(&self) -> FrozenPlacements {
        FrozenPlacements::new(
            self.frozen
                .iter()
                .map(|&(student, list)| {
                    let group = self.placements[&student][&list];
                    ((GroupListIdx(list), student), group as u32)
                })
                .collect(),
        )
    }

    // --- mutation --------------------------------------------------------

    /// Seats the student. The group must have a free seat and the student
    /// must be out of that list.
    pub(super) fn place(&mut self, student: StudentId, list: usize, group: usize) {
        debug_assert!(
            self.has_free_seat(list, group),
            "a target is never exceeded",
        );
        self.lists[list].fills[group].insert(student);
        self.placements
            .entry(student)
            .or_default()
            .insert(list, group);
    }

    /// Takes the student out of a list, returning the group they left.
    pub(super) fn remove(&mut self, student: StudentId, list: usize) -> usize {
        let group = self
            .placements
            .get_mut(&student)
            .and_then(|placed| placed.remove(&list))
            .expect("the student is placed in that list");
        self.lists[list].fills[group].remove(&student);
        group
    }

    /// Marks a placement as prefill output — never revised afterwards.
    pub(super) fn freeze(&mut self, student: StudentId, list: usize) {
        self.frozen.insert((student, list));
    }

    // --- scoring ---------------------------------------------------------

    /// The mass one meeting in `list` puts on each partner, for a group of
    /// target `target`: `k / (N_s · (target − 1))` (§2.2), through the shared
    /// [`pair_mass`] the model's objective reads too.
    fn mass(&self, student: StudentId, list: usize, target: u32) -> f64 {
        pair_mass(
            self.lists[list].multiplicity,
            self.n_uses(student),
            target as usize,
        )
    }

    /// Same, for a kept list. The partner count comes from the *actual* group
    /// size (§2.1): prefilled lists are user-made and may be unbalanced.
    fn kept_mass(&self, student: StudentId, kept: usize, size: usize) -> f64 {
        pair_mass(self.kept[kept].use_count, self.n_uses(student), size)
    }

    /// `P_s(t)` — the mass the student's partner distribution puts on `t`,
    /// summed over the lists that currently group them together, kept lists
    /// included. `O(#lists of s)`.
    fn p_between(&self, student: StudentId, partner: StudentId) -> f64 {
        let mut p = 0.0;
        if let Some(placed) = self.placements.get(&student) {
            for (&list, &group) in placed {
                if self.lists[list].fills[group].contains(&partner) {
                    p += self.mass(student, list, self.lists[list].targets[group]);
                }
            }
        }
        for &(k, g) in self.kept_memberships(student) {
            let group = &self.kept[k].groups[g];
            if group.contains(&partner) {
                p += self.kept_mass(student, k, group.len());
            }
        }
        p
    }

    /// The **exact global delta** of seating an out-of-list student in a
    /// group (§7.4): only the student and the group's current occupants see
    /// their collision probability move.
    ///
    /// Without the occupants' terms, the newcomer could dilute an established
    /// pair's concentration for free. Prefilled and kept-list masses take
    /// part through `p_between`. An empty group — or a target-1 group, which
    /// can hold nobody else — has delta 0 and is still a legal candidate.
    pub(super) fn placement_delta(&self, student: StudentId, list: usize, group: usize) -> f64 {
        debug_assert!(
            !self
                .placements
                .get(&student)
                .is_some_and(|placed| placed.contains_key(&list)),
            "the delta is the score of *joining*: the student must be out",
        );
        let target = self.lists[list].targets[group];
        let m_s = self.mass(student, list, target);
        let mut delta = 0.0;
        for &occupant in &self.lists[list].fills[group] {
            let p_s = self.p_between(student, occupant);
            delta += (p_s + m_s).powi(2) - p_s.powi(2);
            let m_u = self.mass(occupant, list, target);
            let p_u = self.p_between(occupant, student);
            delta += (p_u + m_u).powi(2) - p_u.powi(2);
        }
        delta
    }

    /// The whole objective: `Σ_s Σ_t P_s(t)²` (§2.3). Not used by the search
    /// — the search works on deltas — but it is the instrument the objective
    /// tests measure with, and the diagnostic to compare a greedy solution
    /// with the ILP's optimum on small instances (§9). Deliberately wired
    /// nowhere else for now, hence the allow.
    #[allow(dead_code)]
    pub(super) fn objective_value(&self) -> f64 {
        let mut total = 0.0;
        for student in self.universe() {
            let mut p: BTreeMap<StudentId, f64> = BTreeMap::new();
            if let Some(placed) = self.placements.get(&student) {
                for (&list, &group) in placed {
                    let m = self.mass(student, list, self.lists[list].targets[group]);
                    for &partner in &self.lists[list].fills[group] {
                        if partner != student {
                            *p.entry(partner).or_default() += m;
                        }
                    }
                }
            }
            for &(k, g) in self.kept_memberships(student) {
                let group = &self.kept[k].groups[g];
                let m = self.kept_mass(student, k, group.len());
                for &partner in group {
                    if partner != student {
                        *p.entry(partner).or_default() += m;
                    }
                }
            }
            total += p.values().map(|mass| mass * mass).sum::<f64>();
        }
        total
    }

    // --- output ----------------------------------------------------------

    /// One prefilled `GroupList` per spec, in plan order, paired with the
    /// (period, subject) pairs it must be associated to.
    ///
    /// No compaction and no empty group, unlike the ILP conversion: every
    /// student was placed and the targets sum to the student count, so every
    /// group is exactly at its target.
    pub(super) fn into_group_lists(
        self,
        names: &[String],
    ) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
        let State { plan, lists, .. } = self;
        lists
            .into_iter()
            .zip(plan.specs.iter())
            .enumerate()
            .map(|(i, (list, (spec, covered)))| {
                debug_assert!(
                    list.fills
                        .iter()
                        .zip(list.targets.iter())
                        .all(|(fill, &target)| fill.len() == target as usize),
                    "the greedy places every student and never exceeds a target",
                );
                let groups: Vec<PrefilledGroup> = list
                    .fills
                    .into_iter()
                    .map(|students| PrefilledGroup { students })
                    .collect();
                let params = GroupListParameters {
                    name: names[i].clone(),
                    students_per_group: spec.students_per_group().clone(),
                    group_names: vec![None; groups.len()],
                };
                let group_list = GroupList::new(params, GroupListFilling::Prefilled { groups })
                    .expect("greedy placements satisfy the prefilled invariants by construction");
                (group_list, covered.clone())
            })
            .collect()
    }
}
