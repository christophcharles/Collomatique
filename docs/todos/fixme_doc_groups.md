# FIXME: the `constraints-groups` crate doc is stale on epochs

The module doc at the top of `constraints-groups/src/lib.rs` calls the crate
"complete (end of phase B)" and describes its epochs as "the inclusion-based
incremental epochs of piece 10, which callers feed to the solver so the
inclusion-minimal lists are built first".

Pieces 12 and 12bis changed that (`constraints-groups/src/incremental.rs`,
commit `b536e657` and the piece-12 commits before it): inclusion still orders
the levels, but every spec now gets an epoch of its own, and inside a level the
least entangled, then smallest, lists solve first. Phase C has closed since, so
"end of phase B" is wrong too.

That paragraph wants a rewrite. The reference is the retired roadmap,
`git show 5556784b:docs/plans/auto_group_lists_plan.md`, §2.6 for the design
and §5 for the Done records.
