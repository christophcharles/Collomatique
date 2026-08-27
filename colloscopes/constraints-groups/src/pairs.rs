//! The pair enumeration of the collision objective: one source of truth for
//! the extras, the objective coefficients and the warm-start valuation.
//!
//! The objective is the greedy's own (`docs/plans/greedy_algorithm.md` §2.3),
//! restated for a solver. For an ordered pair `(s, t)`,
//!
//! ```text
//! P_s(t) = c_{s→t} + Σ_sites m_s(site) · z_site
//! ```
//!
//! where a *site* is a group of a rebuilt list both students belong to,
//! `z_site` is the 0/1 fact that they end up sharing it, `m_s(l, g) =
//! k_l / (N_s · (τ_g − 1))` is the mass one such meeting puts on `t` — `k_l`
//! the list's multiplicity, `τ_g` the group's target size, `N_s` all of s's
//! list-uses — and `c_{s→t}` is the constant mass the kept lists already put
//! on `t`. Every one of those is a constant here, which is the whole point of
//! pinning the group sizes ([`crate::targets`]).
//!
//! Maximizing `Σ_s Σ_t P_s(t)²` therefore only asks for the square expanded:
//!
//! ```text
//! (c + Σ_i m_i z_i)² = c² + Σ_i (m_i² + 2·c·m_i) z_i + 2 Σ_{i<j} m_i m_j z_i z_j
//! ```
//!
//! — a constant, a linear part, and pairwise products of binaries. Nothing is
//! approximated: unlike the level-binary scheme the roadmap sketched, this
//! stays exact when a pair's sites carry *different* masses, which is exactly
//! the license case of §2.4 (a colle trio and a big tutorial in one profile).
//!
//! **Tiers.** `m_s(l, g)` depends on the group only through its target size,
//! and one group per list can be shared at most, so the products *inside* a
//! list are identically zero and the sites of a list collapse into at most two
//! [`Tier`]s — the two values the balanced targets take (§3). The sum of a
//! tier's site binaries is itself 0/1, so one product variable per (tier,
//! tier) of two *different* lists covers the whole quadratic part.
//!
//! **Filtering (F1).** A list of multiplicity 0 and a tier of target 1 carry
//! mass 0 in both directions, so they are dropped from the enumeration
//! outright. What survives has a strictly positive objective coefficient,
//! which is what keeps declaration, expansion and valuation in lockstep: the
//! model expands extras lazily, and a warm start naming a variable the model
//! did not expand is refused wholesale.

#[cfg(test)]
mod tests;

use crate::specs::{GenerationPlan, pairs_of};
use crate::vars::{GroupListIdx, VarEnv};
use collomatique_state_colloscopes::StudentId;
use std::collections::BTreeMap;

/// The mass one meeting in a group of `size` students puts on each partner,
/// for a list-use count of `uses` and a student taking part in `n_uses` uses
/// overall: `uses / (n_uses · (size − 1))` (§2.2).
///
/// Zero when the student sits alone there — nobody to put mass on — and zero
/// when `n_uses` is 0, the student whose every list serves no (period,
/// subject) pair: they are placed like anybody else, and score nothing.
///
/// `size` is the *target* size for a rebuilt list and the *actual* group size
/// for a kept one (§2.1): kept lists are user-made and may be unbalanced.
pub(crate) fn pair_mass(uses: usize, n_uses: usize, size: usize) -> f64 {
    if n_uses == 0 || size <= 1 {
        return 0.0;
    }
    uses as f64 / (n_uses as f64 * (size - 1) as f64)
}

/// `N_s` for every student of the plan: all of s's list-uses, rebuilt and kept
/// alike (the fixed-N convention of §2.2).
///
/// A kept list serving no (period, subject) pair is inert and is skipped, so
/// the keys are the plan's student universe: everyone a spec places, plus
/// everyone a *weighing* kept list groups.
pub(crate) fn plan_n_uses(plan: &GenerationPlan) -> BTreeMap<StudentId, usize> {
    let mut n_uses: BTreeMap<StudentId, usize> = BTreeMap::new();
    for (spec, covered) in &plan.specs {
        for &student in spec.students() {
            *n_uses.entry(student).or_default() += covered.len();
        }
    }
    for kept in &plan.kept_lists {
        if kept.use_count == 0 {
            continue;
        }
        for group in &kept.groups {
            for &student in group {
                *n_uses.entry(student).or_default() += kept.use_count;
            }
        }
    }
    n_uses
}

/// The groups of one list that share a target size — the sites a pair meets in
/// at a single mass. At most two per list (§3), and none of target 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tier {
    pub(crate) list: GroupListIdx,
    /// The common target size of the groups, at least 2.
    pub(crate) target: u32,
    /// The group indices, ascending.
    pub(crate) groups: Vec<u32>,
}

/// Everything the objective needs about the plan's student pairs: where they
/// can meet, at what mass, and what the kept lists already gave them.
///
/// The single source of truth of the three sides that must agree: what
/// [`crate::extras`] declares, what [`crate::objective`] weighs, and what
/// [`group_lists_to_warm_start`](crate::group_lists_to_warm_start) valuates.
#[derive(Debug, Clone)]
pub(crate) struct PairData {
    /// `N_s`, by student ([`plan_n_uses`]).
    n_uses: BTreeMap<StudentId, usize>,
    /// `k_l`, by list index: how many (period, subject) pairs the list serves.
    multiplicities: Vec<usize>,
    /// Per pair `(a, b)` with `a < b`, the tiers where they can meet, ordered
    /// by list then by target. A pair no surviving list holds both members of
    /// has no entry at all.
    tiers: BTreeMap<(StudentId, StudentId), Vec<Tier>>,
    /// Per pair the kept lists group together, the constants `(c_{a→b},
    /// c_{b→a})`. The two directions differ: they divide by `N_a` and `N_b`.
    kept: BTreeMap<(StudentId, StudentId), (f64, f64)>,
}

impl PairData {
    pub(crate) fn new(plan: &GenerationPlan, env: &VarEnv) -> PairData {
        let n_uses = plan_n_uses(plan);
        let multiplicities: Vec<usize> = plan
            .specs
            .iter()
            .map(|(_spec, covered)| covered.len())
            .collect();
        let n_of = |student: StudentId| n_uses.get(&student).copied().unwrap_or(0);

        let mut tiers: BTreeMap<(StudentId, StudentId), Vec<Tier>> = BTreeMap::new();
        for list in env.lists() {
            // F1: a list nothing uses puts mass 0 on every pair of it.
            if multiplicities[list.0] == 0 {
                continue;
            }
            let mut by_target: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for (group, &target) in env.targets(list).iter().enumerate() {
                // F1 again: nobody meets anybody in a group of one.
                if target > 1 {
                    by_target.entry(target).or_default().push(group as u32);
                }
            }
            if by_target.is_empty() {
                continue;
            }
            for pair in pairs_of(env.students(list)) {
                let table = tiers.entry(pair).or_default();
                for (&target, groups) in &by_target {
                    table.push(Tier {
                        list,
                        target,
                        groups: groups.clone(),
                    });
                }
            }
        }

        let mut kept: BTreeMap<(StudentId, StudentId), (f64, f64)> = BTreeMap::new();
        for list in &plan.kept_lists {
            if list.use_count == 0 {
                continue;
            }
            for group in &list.groups {
                for (a, b) in pairs_of(group) {
                    let entry = kept.entry((a, b)).or_insert((0.0, 0.0));
                    entry.0 += pair_mass(list.use_count, n_of(a), group.len());
                    entry.1 += pair_mass(list.use_count, n_of(b), group.len());
                }
            }
        }

        PairData {
            n_uses,
            multiplicities,
            tiers,
            kept,
        }
    }

    /// `N_s` — 0 for a student the plan does not know.
    pub(crate) fn n_uses(&self, student: StudentId) -> usize {
        self.n_uses.get(&student).copied().unwrap_or(0)
    }

    /// `m_s(l, τ)`, the mass one meeting in a group of target `target` of
    /// `list` puts on each partner of `student`. Panics on a stale list index,
    /// like [`VarEnv::group_count`](crate::vars::VarEnv::group_count).
    pub(crate) fn mass(&self, student: StudentId, list: GroupListIdx, target: u32) -> f64 {
        pair_mass(
            self.multiplicities[list.0],
            self.n_uses(student),
            target as usize,
        )
    }

    /// `c_{from→to}` — the kept-list mass already on `to` in `from`'s partner
    /// distribution. 0 for a pair no kept list groups together.
    pub(crate) fn kept_constant(&self, from: StudentId, to: StudentId) -> f64 {
        let (a, b) = if from < to { (from, to) } else { (to, from) };
        match self.kept.get(&(a, b)) {
            Some(&(a_to_b, b_to_a)) => {
                if from < to {
                    a_to_b
                } else {
                    b_to_a
                }
            }
            None => 0.0,
        }
    }

    /// The pairs that can meet in a rebuilt group, each with its tier table.
    pub(crate) fn pairs(&self) -> impl Iterator<Item = ((StudentId, StudentId), &[Tier])> {
        self.tiers
            .iter()
            .map(|(&pair, table)| (pair, table.as_slice()))
    }

    /// The tier table of one pair — empty for a pair that can never meet.
    /// Test-only: the three production readings all sweep the whole table
    /// through [`PairData::pairs`].
    #[cfg(test)]
    pub(crate) fn tiers(&self, a: StudentId, b: StudentId) -> &[Tier] {
        self.tiers.get(&(a, b)).map_or(&[], Vec::as_slice)
    }

    /// The `c²` part of the expansion, summed over *every* ordered pair: the
    /// score a solution starts from, whatever the solver decides. Pairs that
    /// only ever meet in kept lists are in here and nowhere else — they are
    /// the reason the constant cannot be dropped as a mere offset.
    pub(crate) fn constant_term(&self) -> f64 {
        self.kept
            .values()
            .map(|&(a_to_b, b_to_a)| a_to_b * a_to_b + b_to_a * b_to_a)
            .sum()
    }

    /// The objective coefficient of one site binary of a tier, both directions
    /// of the pair summed: `Σ_dir (m_dir² + 2·c_dir·m_dir)`.
    ///
    /// Strictly positive on anything the enumeration kept (F1): the masses of
    /// a surviving tier are non-zero in both directions.
    pub(crate) fn together_coefficient(
        &self,
        a: StudentId,
        b: StudentId,
        list: GroupListIdx,
        target: u32,
    ) -> f64 {
        let m_a = self.mass(a, list, target);
        let m_b = self.mass(b, list, target);
        m_a * m_a
            + 2.0 * self.kept_constant(a, b) * m_a
            + m_b * m_b
            + 2.0 * self.kept_constant(b, a) * m_b
    }

    /// The objective coefficient of a cross-list product, both directions
    /// summed: `Σ_dir 2·m_dir(l1, τ1)·m_dir(l2, τ2)`.
    pub(crate) fn coincide_coefficient(
        &self,
        a: StudentId,
        b: StudentId,
        first: &Tier,
        second: &Tier,
    ) -> f64 {
        debug_assert!(
            first.list != second.list,
            "products inside one list are identically zero and are never asked for",
        );
        2.0 * (self.mass(a, first.list, first.target) * self.mass(a, second.list, second.target)
            + self.mass(b, first.list, first.target) * self.mass(b, second.list, second.target))
    }
}

/// The tier products a pair's table contributes to the objective: every
/// unordered couple of tiers of two *different* lists, the first in list order.
///
/// Same-list couples are left out because one group per list means their
/// product is identically zero — the sites of a list are mutually exclusive.
pub(crate) fn cross_tiers(tiers: &[Tier]) -> impl Iterator<Item = (&Tier, &Tier)> {
    tiers.iter().enumerate().flat_map(move |(i, first)| {
        tiers[i + 1..]
            .iter()
            .filter(move |second| first.list != second.list)
            .map(move |second| (first, second))
    })
}
