# The greedy group-list generator (roadmap point 1)

*Detailed design for point 1 of `greedy_roadmap.md`. Every section carries a
status label:*

- **Settled** — decided during the design discussion; implement as written.
- **Proposed** — the current best proposal; the user has not signed off on the
  details and may still change them. Implement as written unless the choice is
  cheap to isolate, in which case isolate it.
- **Open** — genuinely undecided; a recommendation is given.

## 1. Scope — Settled

A greedy algorithm that builds all requested group lists at once, fast, always
succeeding, with teacher-quality stability. It does not replace the ILP; the
ILP becomes the optional polish (roadmap point 3). No GUI work here (point 2).

- Code: `colloscopes/constraints-groups/src/greedy.rs`, submodules as needed,
  unit tests in `greedy/tests.rs` (never a `#[cfg(test)]` mod at the bottom of
  the source file).
- Input: the existing `GenerationPlan` (see §5 for the one extension).
- Output and API, mirroring `build_group_lists` in `convert.rs`:

```rust
pub fn greedy_group_lists(
    plan: &GenerationPlan,
    names: &[String],
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>
```

One `GroupList` per spec, in plan order; `names.len()` must equal
`plan.specs.len()`; `group_names` all `None`. The return value is exactly the
payload of `GroupListsUpdateOp::AddGeneratedGroupLists`.

The greedy reads only `plan.specs` (with their covered pairs) and the new
kept-list field. It ignores `ghost`, `canonical_range` and `pinned_pairs`
entirely (those are ILP-era machinery, retired at roadmap point 3).

## 2. The objective — Settled

### 2.1 Vocabulary

- A **list-use** is one (period, subject) pair served by a list. A list
  covering Math and Physics is *two* uses of the same grouping. Kept prefilled
  lists contribute uses too (as many as the document associates to them).
- For a student `s`, `U_s` is the set of s's list-uses (rebuilt specs
  containing s, plus kept lists containing s), and `N_s = |U_s|`.
- **Group sizes are targets**, fixed before any placement (§3). For a group
  with target size `t`, each member has `m = t - 1` **partners** there. "Size"
  always means the target, never the current fill.

### 2.2 The partner distribution

For each student `s`, define a random variable `X_s` valued in
*students ∪ {nobody}*:

> Pick one of s's list-uses uniformly at random. If s is alone in its group
> there (target size 1), the outcome is "nobody". Otherwise pick one of s's
> partners in that group, uniformly.

So a partner `t` sitting with `s` in a group of target `t_size` inside a list
with multiplicity `k` (number of uses) receives mass

```text
mass(s, t, that list) = k / (N_s * (t_size - 1))
```

and `P_s(t)` is the sum of those masses over the lists where they are grouped
together. All masses are constants known before placement starts: `N_s` counts
*all* of s's uses (**fixed-N** convention — a use where s sits alone
contributes no mass and nothing is renormalized), and the divisor uses the
target size even while the group is filling up.

### 2.3 The score

Maximize the total **collision probability** — the probability that two
independent draws of `X_s` land on the same actual person ("nobody" never
counts as a repeat):

```text
C_s = sum over students t of P_s(t)^2        objective = maximize sum over s of C_s
```

### 2.4 Why this objective (rationale, kept short)

- **Counts, not steps.** The old `SharedPair` objective pays only for a pair's
  first meeting, so it cannot tell "the same partner nine times" from "two
  partners, five and four times". Squared masses reward repetition
  superadditively: 9² + 1² > 5² + 5².
- **The 1/(t_size − 1) weight is the anti-license mechanism.** A meeting in a
  12-student tutorial is worth 1/11 of that use's mass; a meeting in a trio is
  worth 1/2. Canonical test (student with three colles of 3 and one tutorial
  of 12): stable colle partners who are also tutorial-mates beats stable colle
  partners elsewhere in the tutorial, which beats scattering colle partners
  among tutorial-mates. An unweighted count gets that last comparison wrong —
  the tutorial would become a license to mix colle groups.
- **Fixed N rather than renormalizing** away alone-uses: renormalization would
  make sitting alone as good as a further meeting with your best partner (a
  point mass is a point mass), and would reward isolating well-established
  students. Under fixed N, "with your partner > with a stranger > alone".
  The two conventions coincide whenever no size-1 target exists, i.e. almost
  always. Fixed N is also the only version that stays linearizable for the
  ILP (roadmap point 3): renormalizing puts a solver-dependent variable in
  every denominator.
- **No logs.** Shannon entropy of `P_s` was considered and gives the same
  rankings on all the test cases; collision probability is chosen because it
  is log-free, exactly computable in rational arithmetic, and lands the design
  in the standard Mirkin/Rand family (this objective is a weighted Mirkin with
  a principled weighting).
- Because every co-membership adds strictly positive squared mass, the score
  alone already prefers joining any occupied group over opening an empty one;
  empty groups are entered only when forced (or during prefill).

## 3. Shape of a solution — Settled

Per list of `n` students with size range `min..=max`:

- Group count `k = ceil(n / max)`, the minimum, imposed (as in the ILP).
- Sizes balanced around `n / k`: with `q = floor(n / k)` and `r = n mod k`,
  the targets are `r` groups of `q + 1` then `k - r` groups of `q`, in that
  (descending) order — monotone descent by construction.
- Always feasible, with no extra condition: `k >= n / max` gives
  `ceil(n / k) <= max`, and the spec feasibility `k * min <= n` (guaranteed by
  `GroupListSpec::new`) gives `floor(n / k) >= min`. Note this balances around
  `n / k`, *not* around `max` — packing groups at `max` and shaving one
  student off some of them can fail (`n = 9, max = 8` would need `{8, 1}`;
  balanced gives `{5, 4}`).
- Consequence embraced as a design choice: we never produce `{3, 3, 1}` when
  `{3, 2, 2}` exists, matching what a teacher does by hand.

Targets never move during construction. A list whose targets sum to `n` means
some group always has a free seat for the next student, so **the greedy can
never corner itself**: hard constraints are satisfied unconditionally.

## 4. Pipeline — Settled

```text
prefill  ->  greedy pass  ->  (optional, later: ILP with prefilled variables fixed)
```

Prefill (§6) places the obvious students first: whole groups exactly tiled
from a single profile. On the prefilled subset this is a minimal-energy
state — cohort-mates together everywhere, nothing to improve locally — and
since it typically covers a large portion of the class, it should sit close
to the global optimum. It is *not* guaranteed to be part of one: freezing the
pure groups can force contrived placements on the remaining students, and
adjusting one or two pure groups might be better overall. That is the greedy
trade accepted here; the escape is the ILP run over the *whole* model
(nothing fixed, the complete prefill + greedy solution as warm start —
roadmap point 3 keeps both modes). The greedy pass (§7) places everyone and everything else. The ILP leg
is roadmap point 3, not this session.

## 5. Input extension: kept lists — Settled (approved)

`GenerationPlan` gains a field describing the kept lists so they can enter the
objective as real, immutable uses:

- per kept list: its groups (student sets), its size range (for the target
  sizes / partner counts), and its **use count** — the number of (period,
  subject) pairs the document currently associates to it. A kept list
  associated to zero pairs contributes zero uses and is naturally inert.
- Populated by `build_generation_plan` from `Parameters`. The ILP path
  ignores the field for now; at roadmap point 3 it becomes the ILP's input
  too and `pinned_pairs` retires.

Kept-list masses are constants: they seed `P_s` before any placement and are
never modified.

## 6. Prefill — Proposed

### 6.1 Cohorts

A **profile** is the set of lists containing a student (multiplicity does not
affect membership). A **cohort** is a maximal set of students with an
identical profile. Within a cohort, students are interchangeable — this is
what makes prefill simple and is used throughout.

### 6.2 Claim rule

Process cohorts in the global student order (§7.1). For a cohort `C`, in each
list of its profile independently: among that list's still-empty groups,
claim a set of groups whose targets sum as high as possible **without
exceeding** `|C|` — place the maximum number of members in fully-tiled pure
groups. This is not a subset-sum search: groups of equal target are
interchangeable, so only the counts matter. With `a` empty groups of target
`q + 1` and `b` of target `q` (a list's targets take at most these two
values, §3), loop `x` from `min(a, |C| div (q + 1))` down to 0, set
`y = min(b, (|C| − x·(q + 1)) div q)`, and keep `(x, y)` when
`x·(q + 1) + y·q` strictly beats the best so far. At most `a + 1` iterations.

Tie convention when several claim sets place the same number of members (e.g.
6 members, `{3, 3}` vs `{2, 2, 2}` both available): prefer **larger targets**
(`{3, 3}`). The descending loop with strict improvement implements this for
free. Flagged as arbitrary — this is the choice the greedy is blind to;
either convention is defensible, this one is simply pinned for determinism.
Note the "4 members: one 3-group + 1 leftover, or two 2-groups?" question is
*not* a tie: two 2-groups place 4 > 3 members, so maximize-placed already
chooses them.

### 6.3 Coverage: all-or-nothing per student — Proposed (explicitly not settled)

*The user is unsure about this rule and its sub-choices; treat the whole
subsection as a current best proposal, likely to be revisited, and keep its
implementation easy to change.*

A student is prefilled **only if the claims cover them in every claiming
list**; otherwise they are entirely deferred to the greedy pass. The
alternative it replaced (place each student wherever claims cover them, leave
them unplaced elsewhere) creates single-use orphan pairings for the tail
members of a cohort; all-or-nothing gives the greedy more work but lets it
place those students jointly. Two precisions that make the rule workable:

- **Lists where the cohort can claim nothing never veto.** A trio cannot tile
  the 12-seat tutorial; that list simply isn't a claiming list, the whole
  cohort is free there, and the greedy pass seats them (together — the score
  sees their prefilled co-uses). Without this rule, no cohort smaller than
  the largest group would ever prefill anything.
- **Shrink to a fixpoint.** Deferring uncovered members shrinks the claims,
  which can shrink coverage again (a `{3}`-target list places 3 members or 0,
  nothing between). Iterate: `p` = minimum coverage over claiming lists;
  recompute each list's claims with at most `p` members; a list dropping to
  zero leaves the claiming set; repeat until stable. Terminates immediately
  in practice.

Membership of claimed groups: take the cohort's members in canonical order
(ascending `StudentId`), fill claimed groups in descending target order. The
same canonical order in every list makes the blocks prefix-align across
lists, which is the whole cross-list consistency story.

**Invariant**: after prefill, every student is either completely placed (in
every list of their profile that holds claims, and *those* placements are
frozen) or completely untouched. No half-placed students.

Worked example (the case that killed two earlier designs): one list, targets
`{3, 2}`, cohorts `{a, b}` and `{c, d, e}`. `{a, b}` cannot claim the 3-group
(3 > 2) and claims the 2-group; `{c, d, e}` claims the 3-group. Perfect
answer — the earlier "purity + lowest-index" design sent `{a, b}` into the
3-group and doomed the trio.

## 7. The greedy pass — Proposed

### 7.1 Student order — Proposed (explicitly not settled)

Process cohorts rarest first: ascending cohort size, ties toward students
with more list-uses, then ascending `StudentId`. Rationale: rare profiles
have the fewest options for consistent partners and must commit while the
space is empty; the "takes everything standard" students come last and are
exactly the flexible ones. The same order drives prefill (§6.2) and the
greedy pass. *The user wants to think more about this order; treat it as a
parameter of the implementation, easy to swap.*

### 7.2 The loop

```text
for each student s (global order), skipping fully-prefilled students:
    place_student(state, s)     # joint placement over all of s's unplaced lists
    s is frozen — never revisited
```

`place_student` is an isolated subroutine with the contract: "given the
current state, choose s's group in every list of s's profile not already
fixed by prefill, deterministically". The boundary is settled (so the
strategy can be swapped if slow); the first implementation below is proposed.

### 7.3 First implementation of `place_student`: sweep to fixpoint

- First pass over s's unplaced lists in a fixed order (spec order is
  acceptable; the epoch-style ordering adds nothing decisive here): in each
  list, try every group with a free seat, take the best score delta.
- Revision sweeps: repeatedly re-visit each of s's lists, take s out, re-choose
  the best group given every other current placement; stop when a full sweep
  changes nothing. Terminates: a move requires a strict score increase and the
  score takes finitely many values. A debug assertion on sweep count as a bug
  detector, not a limiter.

Joint placement is not optional under this objective: a first meeting
contributes a negligible squared mass, so per-list placement would decide
almost everything on ties; only the joint view makes repetition visible.

### 7.4 The score of a candidate — Settled (consequence of §2)

The **exact global delta**: `ΔC_s` plus `ΔC_t` for every student `t` already
in the candidate group (only they are affected). Without the `ΔC_t` terms, s
could dilute an established pair's concentration for free. Prefilled and
kept-list masses participate in every delta.

### 7.5 Tie-break — Proposed

Scan groups in ascending index, require strict improvement to switch: the
lowest-indexed candidate wins exact ties. Targets being sorted descending,
ties fill the big groups first. (The wrong-size hazard this rule once caused
is gone: exact-fit claims happen in prefill, before this rule ever runs.)

### 7.6 What is deliberately absent — Settled

No lookahead (the student order is the lookahead budget), no cross-student
repair or swap pass, no revisiting of frozen students. Unhappy with the
result → send it to the ILP (roadmap point 3).

Known accepted weakness: the first member of a cohort still seeds on weak
information in the *non-claimable* lists (which tutorial group?), and a
cohort can be split when arithmetic forces it (three pairs into `{3, 3}`).
Both match what happens by hand.

## 8. Arithmetic — Open

Score deltas should compare exactly. Masses are rationals with small
denominators (`N_s * m * ...`), so exact comparison is cheap; recommendation:
rational arithmetic (`num-rational` if already in the tree, else a small
hand-rolled fraction on `i128` or `BigInt`), decided at implementation time.
The user considers this low-stakes; determinism of the *result* is what
matters, and the fixed scan orders plus strict-improvement rules provide it
even under `f64` — exact arithmetic just removes the near-tie fragility.

## 9. Tests — Agreed

In `greedy/tests.rs`, on hand-built plans:

- hard constraints always hold: every student in exactly one group per list,
  every group at its target size, targets within the range, descending sizes;
- determinism: same plan, same output, twice;
- the trio travels: a cohort's colle trio lands in one tutorial group;
- the license case: stable colle partners beat mixing among tutorial-mates
  (the (a)/(b)/(c) scenario of §2.4);
- prefill exact-fit: the `{3, 2}` / `{a,b}` + `{c,d,e}` example of §6.3;
- kept lists steer placement, weighted by their use count;
- the size-1 corner: alone-slot handling under fixed N;
- a fuzz-style test over `collomatique-testgen-colloscopes` documents: the
  greedy never panics and always outputs valid lists (same spirit as the
  constraints-colloscopes fuzz-build net).

A useful diagnostic (not a test): compute the objective value of the greedy's
output, and later compare with the ILP's optimum on small instances — to
judge the greedy *or* to judge the objective.

## 10. Open questions (recap)

1. §7.1 student order — proposed, user explicitly still thinking.
2. §6.3 all-or-nothing coverage and its sub-choices (never-veto lists,
   shrink-to-fixpoint) — proposed, user explicitly unsure; keep the
   implementation easy to change.
3. §6.2 tie convention (larger targets first) — pinned but arbitrary.
4. §7.3 sweep performance — acceptable until proven otherwise; the
   `place_student` boundary is the insurance.
5. §8 arithmetic — rationals recommended, decided at implementation time.
