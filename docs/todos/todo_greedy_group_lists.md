# TODO: A greedy algorithm for group-list generation

Group-list generation is an ILP today: `constraints-groups/` builds one
assignment matrix per list, with a stability objective and staggered
incremental epochs, and the GUI drives it through the ordinary solver dialog.

We should **try writing a greedy algorithm** for it instead.

There is already a precedent inside the crate. The template grouping — the
partition of the whole student body the objective asks each list to resemble —
used to be solved and is now computed by a greedy clustering on an affinity
graph (`constraints-groups/src/ghost.rs`, which explains why it moved). The
lists themselves are the remaining half.

The old ILP design, for reference, is the retired roadmap:
`git show 5556784b:docs/plans/auto_group_lists_plan.md`.

Generation not being settled is also what keeps it out of the Python API: there
is no `group_lists.add_generated` door, and there should not be one until this
question is answered (`a7c434d0`).
