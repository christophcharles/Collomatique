//! Group targets: the shape of a solution, fixed before any placement.
//!
//! The greedy fills these targets group by group, and never revisits them:
//! how many groups a list has, and how big each of them is, is settled here
//! before anybody is placed.

use collomatique_state_colloscopes::NonEmptyRangeInclusive;
use std::num::NonZeroU32;

/// The imposed group targets for `n` students in `range`: the minimal count
/// `k = ⌈n / max⌉`, sizes balanced around `n / k` — `n % k` groups of
/// `⌈n / k⌉` then the rest at `⌊n / k⌋`, descending.
///
/// Feasible for every spec `GroupListSpec::new` accepts, with no extra
/// condition: `k ≥ n / max` gives `⌈n / k⌉ ≤ max`, and the spec feasibility
/// `k · min ≤ n` gives `⌊n / k⌋ ≥ min`. Note this balances around `n / k`,
/// *not* around `max`: packing at `max` and shaving students off can fail
/// (`n = 9, max = 8` would need `{8, 1}`; balanced gives `{5, 4}`).
///
/// The targets sum to `n` and never move afterwards, so a free seat always
/// exists for the next student — the greedy can never corner itself.
pub(crate) fn balanced_targets(n: u32, range: &NonEmptyRangeInclusive<NonZeroU32>) -> Vec<u32> {
    let max = range.end().get();
    let k = n.div_ceil(max);
    debug_assert!(k > 0, "a spec always has at least one student");
    let q = n / k;
    let r = n % k;
    debug_assert!(
        q >= range.start().get() && q + u32::from(r > 0) <= max,
        "the balanced targets stay inside the spec's size range",
    );
    (0..k).map(|i| if i < r { q + 1 } else { q }).collect()
}
