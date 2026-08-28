//! Prefill: tile whole groups out of single cohorts, before the greedy pass.
//!
//! On the prefilled subset this is a minimal-energy state — cohort-mates
//! together everywhere, nothing to improve locally — and it typically covers
//! a large portion of the class. It is not guaranteed to be part of a global
//! optimum: freezing the pure groups can force contrived placements on the
//! rest. That is the greedy trade.

use super::cohorts::Cohort;
use super::state::State;

/// A set of empty groups a cohort takes over in one list.
struct Claim {
    /// The group indices, in descending target order.
    groups: Vec<usize>,
    /// How many members they seat — the claim's coverage.
    covered: usize,
}

/// Places what can be placed, cohort by cohort, in the global order.
pub(super) fn prefill(state: &mut State, cohorts: &[Cohort]) {
    for cohort in cohorts {
        prefill_cohort(state, cohort);
    }
}

/// The best claim: among the list's still-empty groups, take a set whose
/// targets sum as high as possible **without exceeding** `budget`.
///
/// Not a subset-sum search — groups of equal target are interchangeable, so
/// only the counts matter, and a list's targets take at most the two values
/// `q + 1` and `q`. With `a` empty groups of the larger target and `b`
/// of the smaller, the loop is `a + 1` iterations.
///
/// The ascending loop with strict improvement implements the tie convention
/// for free: when several claim sets place the same number of members
/// (`{3, 3}` vs `{2, 2, 2}` for six), the smaller targets win. That tie is
/// not objective-blind — putting a member's fixed claiming mass on one
/// constant partner scores twice what splitting it over two does — and the
/// leftover students inherit the big groups, exactly where the
/// `1 / (target − 1)` weight makes a weak pairing cheapest.
fn best_claim(state: &State, list: usize, budget: usize) -> Claim {
    let targets = state.targets(list);
    let big = targets[0];
    let small = *targets.last().expect("every list has at least one group");

    let empty = |wanted: u32| -> Vec<usize> {
        (0..targets.len())
            .filter(|&group| targets[group] == wanted && state.is_empty_group(list, group))
            .collect()
    };
    let empty_big = empty(big);
    // When all targets are equal there is no second value to claim from.
    let empty_small = if small == big {
        Vec::new()
    } else {
        empty(small)
    };

    let big = big as usize;
    let small = small as usize;
    let mut best = (0usize, 0usize, 0usize);
    for x in 0..=empty_big.len().min(budget / big) {
        let y = if empty_small.is_empty() {
            0
        } else {
            empty_small.len().min((budget - x * big) / small)
        };
        let covered = x * big + y * small;
        if covered > best.2 {
            best = (x, y, covered);
        }
    }

    let (x, y, covered) = best;
    let mut groups = empty_big[..x].to_vec();
    groups.extend_from_slice(&empty_small[..y]);
    Claim { groups, covered }
}

/// One cohort: claim, shrink to a fixpoint, then place.
///
/// A student is prefilled **only if the claims cover them in every claiming
/// list**, otherwise they are entirely deferred to the greedy pass —
/// which can then place them jointly instead of leaving single-use orphan
/// pairings behind. Two precisions make the rule workable:
///
/// - a list where the cohort can claim nothing never vetoes (a trio cannot
///   tile a 12-seat tutorial; that list is simply not a claiming list, and
///   the greedy seats the trio there — together, since the score already
///   sees their prefilled co-uses);
/// - deferring uncovered members shrinks the claims, which can shrink
///   coverage again (a `{3}`-target list places 3 members or 0), so the
///   budget is iterated down to a fixpoint. It strictly decreases, so this
///   terminates; in practice it settles immediately.
fn prefill_cohort(state: &mut State, cohort: &Cohort) {
    let mut budget = cohort.members.len();
    let claims = loop {
        let claims: Vec<(usize, Claim)> = cohort
            .profile
            .iter()
            .map(|&list| (list, best_claim(state, list, budget)))
            .filter(|(_list, claim)| claim.covered > 0)
            .collect();
        // No claiming list at all: this cohort prefills nothing.
        let Some(coverage) = claims.iter().map(|(_list, claim)| claim.covered).min() else {
            break Vec::new();
        };
        if coverage == budget {
            break claims;
        }
        budget = coverage;
    };
    if claims.is_empty() {
        return;
    }

    // Every claim covers exactly `budget` members now, so the same first
    // `budget` members fill every claiming list — the cross-list consistency
    // story. The rest of the cohort is entirely deferred.
    let members = &cohort.members[..budget];
    for (list, claim) in &claims {
        debug_assert_eq!(claim.covered, budget, "the fixpoint equalizes coverage");
        let mut members = members.iter();
        for &group in &claim.groups {
            for _ in 0..state.targets(*list)[group] {
                let student = *members.next().expect("the claim seats exactly the budget");
                state.place(student, *list, group);
                state.freeze(student, *list);
            }
        }
    }
}
