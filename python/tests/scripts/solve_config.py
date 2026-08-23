import dataclasses

import collomatique

# `source` is a throwaway copy of the document the value commits built for the
# two filling shapes: two periods, an automatic group list and a prefilled one
# side by side. Both shapes are needed here — a solve config speaks about the
# automatic list, and naming the prefilled one is one of the refusals.
doc = collomatique.load(source)

first_period, second_period = list(doc.periods)
group_list_list = list(doc.group_lists)
automatic = next(gl for gl in group_list_list if gl.groups is None)
prefilled = next(gl for gl in group_list_list if gl.groups is not None)

# The field order of each class, which is what a positional call depends on.
assert [f.name for f in dataclasses.fields(collomatique.PeriodSolveConfig)] == [
    "recompute",
    "use_current_values",
]
assert [f.name for f in dataclasses.fields(collomatique.GroupListSolveConfig)] == [
    "recompute",
    "previous_values_as_objective",
]
assert [f.name for f in dataclasses.fields(collomatique.ColloscopeSolveConfig)] == [
    "periods",
    "group_lists",
    "objectify_cross_fixed_period",
    "l1_anchor_weight",
]

# The classes are the module's, not a private submodule's, like every other
# value class.
assert collomatique.ColloscopeSolveConfig.__module__ == "collomatique"
assert collomatique.PeriodSolveConfig.__module__ == "collomatique"
assert collomatique.GroupListSolveConfig.__module__ == "collomatique"

# The model's own defaults, all of them: recompute everything, freely, with
# the two weights the application solves with. Rust pins these against
# `SolveConfig::default()`, so the two sides cannot drift apart.
bare = collomatique.ColloscopeSolveConfig()
assert bare.periods == {}
assert bare.group_lists == {}
assert bare.objectify_cross_fixed_period == 1000.0
assert bare.l1_anchor_weight == 1000.0
assert collomatique.PeriodSolveConfig() == collomatique.PeriodSolveConfig(True, False)
assert collomatique.GroupListSolveConfig() == collomatique.GroupListSolveConfig(
    True, False
)

# A config that says something about everything it can: the first period
# pinned and believed, the second recomputed and anchored, the automatic list
# recomputed and anchored, the cross-period constraints kept hard, and an
# anchor that prices nothing.
spelled_out = collomatique.ColloscopeSolveConfig(
    periods={
        first_period: collomatique.PeriodSolveConfig(
            recompute=False, use_current_values=True
        ),
        second_period: collomatique.PeriodSolveConfig(
            recompute=True, use_current_values=True
        ),
    },
    group_lists={
        automatic: collomatique.GroupListSolveConfig(
            previous_values_as_objective=True
        ),
    },
    objectify_cross_fixed_period=None,
    l1_anchor_weight=0.0,
)

# A key names an entity, so a handle and an id are the same key here — and the
# two configs, like every other pair of values written both ways, do not
# compare equal, because a handle and an id hash differently.
by_handle = collomatique.ColloscopeSolveConfig(
    periods={first_period: collomatique.PeriodSolveConfig(recompute=False)},
    group_lists={automatic: collomatique.GroupListSolveConfig(recompute=False)},
)
by_id = collomatique.ColloscopeSolveConfig(
    periods={first_period.id: collomatique.PeriodSolveConfig(recompute=False)},
    group_lists={automatic.id: collomatique.GroupListSolveConfig(recompute=False)},
)
assert by_handle != by_id

# The configs the boundary must refuse. They are built without complaint —
# that is the point of a dumb value — and rust extracts each one and reads the
# message.

# One period, named twice, once in each spelling.
named_twice = collomatique.ColloscopeSolveConfig(
    periods={
        first_period: collomatique.PeriodSolveConfig(),
        first_period.id: collomatique.PeriodSolveConfig(recompute=False),
    },
)

# A prefilled list has no groups to work out, so it has no solve to configure.
prefilled_list = collomatique.ColloscopeSolveConfig(
    group_lists={prefilled: collomatique.GroupListSolveConfig()},
)

# The one combination that means nothing: a list that is not recomputed keeps
# its groups, so there is nothing left for an anchor to hold on to.
nothing_to_anchor = collomatique.ColloscopeSolveConfig(
    group_lists={
        automatic: collomatique.GroupListSolveConfig(
            recompute=False, previous_values_as_objective=True
        )
    },
)

# The weights: zero is a weight, a negative one and a non-finite one are not.
zero_weights = collomatique.ColloscopeSolveConfig(
    objectify_cross_fixed_period=0.0, l1_anchor_weight=0.0
)
negative_weight = collomatique.ColloscopeSolveConfig(l1_anchor_weight=-1.0)
infinite_weight = collomatique.ColloscopeSolveConfig(
    objectify_cross_fixed_period=float("inf")
)

# And the ordinary shapes of wrong: a flag that is not one, and a mapping that
# is not one.
not_a_flag = collomatique.ColloscopeSolveConfig(
    periods={first_period: collomatique.PeriodSolveConfig(recompute=3)},
)
not_a_mapping = collomatique.ColloscopeSolveConfig(periods=3)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_period = collomatique.ColloscopeSolveConfig(
    periods={list(other.periods)[0]: collomatique.PeriodSolveConfig()},
)

# And a handle to something this document no longer holds names nothing
# either. The period is this script's own, added and taken away again, so the
# fixture's own entities are all still there for everything above.
doomed = doc.periods.add(2).created
doc.periods.remove_with_weeks(doomed)
dead_period = collomatique.ColloscopeSolveConfig(
    periods={doomed: collomatique.PeriodSolveConfig()},
)
