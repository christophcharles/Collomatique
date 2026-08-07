# TODO: reconstruct incumbents found in a restarted (subtree) search

CBC sometimes abandons the model it started branching on and restarts the search
on a smaller one. Incumbents found after that restart cannot be reconstructed
into original column space by `reconstruct_incumbent`
(collo-cbc/cpp/collo_cbc.cpp), so they are reported as
`COLLO_CBC_INCUMBENT_FAILED`. This note records how to reconstruct them
properly. Everything below was measured on a real failing run (Aug 2026), not
inferred.

## What this buys, and what it does not

Buys:

- **Kill-and-keep.** If the user kills a solver process, `CbcMain1` never
  returns, so the only solution that survives is the last one delivered through
  a progress event. Without this, that is the root incumbent, however long the
  search ran.
- **A fresher distance cutoff.** The cutoff (`good_enough` in the epoch callback,
  strategies/src/strategies/incremental.rs) compares the incumbent objective
  against the bound. A stale incumbent objective makes the measured gap look
  *larger* than it is, so the cutoff errs late, never early. Safe, but sluggish.

Does **not** affect:

- **The after-incumbent time limit.** `SolveDeadlines::check`
  (ilp/src/solvers/collo_cbc.rs) arms `incumbent_deadline` once, on the first
  event carrying an incumbent, and never re-arms it. Later incumbents cannot
  move it.
- **The solution a normal or callback-stopped solve returns.** That comes from
  `cbcModel.bestSolution()` after `CbcMain1` returns, which is already
  postprocessed into original space.

So this is a real improvement but not a correctness fix. The correctness fix is
the shape guard described under "Precondition" below.

## When the restart happens

CBC logs it:

```
Cbc0044I Reduced cost fixing - 758 rows, 398 columns - restarting search
```

Reduced-cost fixing proves that many columns cannot take their other value in
any improving solution, fixes them, and restarts branch and bound on the
resulting smaller model. It needs a large objective with a *tiny relative gap* —
that is the regime where reduced costs dominate. The failing group-list epoch
had a bound of `-115997.38` against an incumbent of `-115984.91`, on values of
magnitude 116000: a relative gap of about 0.01%.

This is why "a big tree" does not reproduce it. A synthetic market-split
instance with 40 binaries searched 1.5M nodes and never restarted; every event
came from the top-level model.

The restarted model is a **stack-local `CbcModel`** created inside `CbcMain1`,
whose `parentModel()` is the top-level one. Note that nesting depth does not
identify it — a heuristic sub-MIP sits at the same depth. What distinguishes it
is that it fires `treeStatus` events; heuristic sub-MIPs do not.

## Why the current code gets it wrong

`reconstruct_incumbent` postsolves through `cbcPreProcessPointer`, the
`CglPreProcess` object `CbcMain1` publishes as an unmangled global. That object
describes the **top-level** preprocessing only, and it is *not* republished for
the restart — measured identical before and after: same pointer, same
`numberSolvers() == 3`, same shapes.

So a 398-column incumbent gets `memcpy`'d into a map that expects 1056 columns.
Worse, the function still returns `true`: its only sanity check is
`originalModel()->getNumCols() == orig_num_cols`, and `originalModel()` is the
1107-column original either way. The result is a confidently wrong incumbent:

| event | nodes | reported objective | CBC's actual incumbent |
|---|---|---|---|
| root (top-level model) | 0 | -115984.91304347821 | -115984.91 ✓ |
| restarted search | 17 | 15015.00 | -115984.93 |
| restarted search | 124 | 12014.17 | -115984.96 |
| restarted search | 4273 | -3985.54 | -115985.07 |

A wrong incumbent is worse than a missing one. The distance cutoff would compare
a bound of `-115997` against `-3985` and never fire.

## Precondition: the shape guard

Before any of this, `reconstruct_incumbent` must refuse a model it cannot map.
The check is that the model's column count equals the preprocessed end of the
published chain:

```cpp
CglPreProcess* pp = cbcPreProcessPointer;
int ns = pp ? pp->numberSolvers() : 0;
const OsiSolverInterface* pre = (ns > 0) ? pp->modifiedModel(ns - 1) : nullptr;
if (!pre || model->getNumCols() != pre->getNumCols())
    return false;   // -> COLLO_CBC_INCUMBENT_FAILED
```

Note `CglPreProcess` has **no** `presolvedModel()` accessor. The available ones
are `originalModel()`, `startModel()`, `numberSolvers()`, `modelAtPass(i)` and
`modifiedModel(i)`; the preprocessed end is `modifiedModel(numberSolvers() - 1)`.

With the guard in place, restarted-search incumbents report
`ReconstructionFailed` honestly and everything downstream already handles that.
The work below then upgrades them from "failed" to "reconstructed".

## The reconstruction

Two mappings exist, and chaining them closes the gap exactly.

Measured on the failing epoch (original problem: 1107 columns, 971 rows):

| object | columns | `originalColumns()` | maps into |
|---|---|---|---|
| `cbcPreProcessPointer` | `startModel` 1107 → `modifiedModel(2)` 1056 | — | 1107 |
| top-level `CbcModel` (parent=0) | 1056 | 1056 entries: `0,1,2,3,4,…,1106` | 1107-space |
| restarted search model | 398 | 398 entries: `3,5,8,13,14,…,1054` | **1056-space** |

The search model's `originalColumns()` maps into the *parent's preprocessed*
space, not into the original. The 1056 − 398 = 658 columns it dropped are
exactly the reduced-cost-fixed ones, and they are fixed in
`parentModel()->solver()`:

```
parent_cols=1056  unmapped=658  fixed_in_solver=658  fixed_in_continuousSolver=0
```

Read the values from **`solver()`**, not `continuousSolver()` — the continuous
solver had zero fixed columns. `solver()` normally carries node-local bounds
during a search, which would be the wrong thing to read in general; it is safe
here specifically because those 658 columns do not exist in the child model, so
the child's branching cannot have touched them. Keep that reasoning in a comment
at the site — it is the whole justification.

### Algorithm

Expand the incumbent one level up the parent chain, repeatedly, until it lives
in the space `cbcPreProcessPointer` describes; then run the existing postsolve.

```cpp
// Expand `child_solution` (child->getNumCols() values) into the parent's column
// space. Returns false if the mapping is absent or does not account for every
// parent column.
static bool expand_to_parent(
    const CbcModel* child,
    const std::vector<double>& child_solution,
    std::vector<double>& out
) {
    const CbcModel* parent = child->parentModel();
    if (!parent)
        return false;
    const int* oc = child->originalColumns();
    if (!oc)
        return false;

    const OsiSolverInterface* ps = parent->solver();
    if (!ps)
        return false;
    const int n = parent->getNumCols();
    const double* lo = ps->getColLower();
    const double* up = ps->getColUpper();

    // Base: every parent column the child dropped must have been fixed, and the
    // fixed bound is its value.
    out.assign(n, 0.0);
    std::vector<bool> resolved(n, false);
    for (int j = 0; j < n; j++) {
        if (up[j] - lo[j] < 1e-9) {
            out[j] = lo[j];
            resolved[j] = true;
        }
    }

    // Overwrite the columns the child kept.
    for (int i = 0; i < child->getNumCols(); i++) {
        const int j = oc[i];
        if (j < 0 || j >= n)
            return false;
        out[j] = child_solution[i];
        resolved[j] = true;
    }

    // Anything neither mapped nor fixed would be a guess. Refuse instead.
    for (int j = 0; j < n; j++) {
        if (!resolved[j])
            return false;
    }
    return true;
}
```

Then, in `reconstruct_incumbent`, before the existing postsolve:

```cpp
    std::vector<double> current(incumbent, incumbent + model->getNumCols());
    const CbcModel* m = model;
    // Walk up until we reach the space cbcPreProcessPointer describes. One
    // restart was observed; nothing rules out two, so loop rather than assume.
    while (m->getNumCols() != pre->getNumCols()) {
        std::vector<double> up_one;
        if (!expand_to_parent(m, current, up_one))
            return false;
        current.swap(up_one);
        m = m->parentModel();
        if (!m)
            return false;
    }
    // `current` is now in preprocessed space; postsolve as today, but install it
    // into a clone of `m->continuousSolver()` (the top-level one), not the
    // search model's.
```

The rest is unchanged: clone the continuous solver, install the values, fix the
integer columns, `initialSolve()`, `postProcess` a throwaway clone, read
`originalModel()->getColSolution()`.

## Open risks

- **Only one instance, one restart level.** The walk is written as a loop for
  that reason, but a two-level chain has never been observed and never tested.
- **`originalColumns()` on a restarted model** was non-null here. Whether CBC
  always populates it on a restart is unverified; the `!oc` branch above is the
  fallback.
- **`solver()` bounds are read mid-search.** The argument for why that is safe is
  in the comment above, but it rests on the child not owning those columns. If a
  future CBC restarts with a *different* reduction mechanism that keeps the
  columns, this silently breaks — which is why `expand_to_parent` refuses when
  any column is neither mapped nor fixed, rather than defaulting it.
- **Objective check.** The reconstructed incumbent's objective is recomputed by
  `original_objective` from the original problem. Compare it against CBC's
  `Cbc0004I Integer solution of …` lines when verifying; an error of ~120000 on
  values of ~116000 is the signature of the bug this note is about.

## How to reproduce and verify

The reproducer is a model dump. Set `COLLO_CBC_DUMP_MODEL=<prefix>` (writes
`<prefix>-<pid>-<NNN>.collomodel`, see collo-cbc/src/lib.rs) on a run of
group-list generation, and pick the epoch whose dump is followed by a long gap.
The failing epoch was 1107 columns × 971 rows; each solve gets its own
subprocess, so every file is index `000` with a different pid.

Then:

```
COLLO_CBC_DEBUG_EVENTS=1 cargo run -p collo-cbc --release --example replay -- \
    <dump>.collomodel --log 1
```

Use `--release`; the debug build is far too slow for a 15000-node search. The
model never finishes — run it under `timeout 25`, which is well past the restart
at ~0.9 s. Confirm the restart with `grep Cbc0044I`, and confirm the model split
by grouping the `collo_cbc[dbg] event=` lines by `model=`/`parent=`.

The failing epoch's own dump is committed as
`collo-cbc/tests/data/restarted_search.collomodel` (57 KB), and
`collo-cbc/tests/restarted_search.rs` replays it. That is the reproducer — there
is no need to regenerate one. Other dumps stay outside the repo; a colloscope
epoch can be tens of megabytes.
