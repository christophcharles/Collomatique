# TODO: a cheaper proxy for group stability in `constraints-groups`

The stability objective (`constraints-groups/src/objective.rs`) minimizes
`w_groups · Σ GroupHasStudents + w_pairs · Σ SharedPair`. The second term keeps
the groupings of different lists close to each other, and it is what makes the
model big: one reified `SharedPair` variable, with its linking constraints, per
pair of students appearing in more than one list — quadratic in the class size.

A cheaper proxy is wanted, pushing solutions the same way with a variable count
closer to linear. Two directions to weigh: compare each list against a single
reference grouping (the largest list, or the previous solution when
regenerating) instead of all lists against each other; or aggregate per group
rather than per pair, accepting a coarser measure.

Whatever replaces it must keep what the pair term was built for: reusing an
existing grouping costs nothing, so an already-consistent document is left
alone. A before/after measurement on a real document decides whether the loss
of precision is worth it.
