import collomatique

# `source` is a document rust built for this script: the example only ever uses
# two of the four periodicities, and none of the values below is its default.
doc = collomatique.load(source)

subjects = list(doc.subjects)
assert [subject.name for subject in subjects] == [
    "Périodique",
    "Par bloc",
    "Dans l'année",
    "Blocs sur mesure",
]

periodique, par_bloc, dans_l_annee, sur_mesure = subjects

# The four kinds, read back and compared as whole values. The numbers are the
# ones rust put in — the two sides are written out separately on purpose, so a
# conversion that quietly swapped two fields would show up here.
assert periodique.interrogation.periodicity == collomatique.EveryNWeeks(3)
assert par_bloc.interrogation.periodicity == collomatique.OncePerBlock(4, 2)
assert dans_l_annee.interrogation.periodicity == collomatique.CountInYear((2, 5), 0)
assert sur_mesure.interrogation.periodicity == collomatique.CustomBlocks(
    (collomatique.WeekBlock(0, 2, (1, 1)), collomatique.WeekBlock(3, 4, (0, 2))), 1
)

# And field by field, which is what says the whole-value comparison above is
# comparing something.
assert periodique.interrogation.periodicity.n == 3
assert par_bloc.interrogation.periodicity.weeks_per_block == 4
assert par_bloc.interrogation.periodicity.minimum_week_separation == 2
assert dans_l_annee.interrogation.periodicity.count == (2, 5)
assert dans_l_annee.interrogation.periodicity.minimum_week_separation == 0

blocks = sur_mesure.interrogation.periodicity.blocks
assert isinstance(blocks, tuple)
assert [block.delay_in_weeks for block in blocks] == [0, 3]
assert [block.size_in_weeks for block in blocks] == [2, 4]
assert [block.count for block in blocks] == [(1, 1), (0, 2)]
assert sur_mesure.interrogation.periodicity.minimum_week_separation == 1

# The rest of the interrogation parameters, likewise none of them the default.
assert periodique.interrogation.students_per_group == (1, 1)
assert periodique.interrogation.groups_per_interrogation == (2, 4)
assert periodique.interrogation.duration == 45
assert periodique.interrogation.take_duration_into_account is False

assert dans_l_annee.interrogation.students_per_group == (3, 3)
assert dans_l_annee.interrogation.groups_per_interrogation == (1, 2)
assert dans_l_annee.interrogation.duration == 30
assert dans_l_annee.interrogation.take_duration_into_account is True

# One subject skips the second period, and what comes back is a live handle.
periods = list(doc.periods)
assert par_bloc.excluded_periods == frozenset({periods[1]})
assert [period.index for period in par_bloc.excluded_periods] == [1]
assert periodique.excluded_periods == frozenset()

# A leaf value is a value: it compares, it hashes, and two of them made the same
# way are interchangeable.
assert collomatique.EveryNWeeks(3) == collomatique.EveryNWeeks(3)
assert collomatique.EveryNWeeks(3) != collomatique.EveryNWeeks(2)
assert len({collomatique.EveryNWeeks(3), collomatique.EveryNWeeks(3)}) == 1
assert collomatique.WeekBlock(0, 2, (1, 1)) == collomatique.WeekBlock(0, 2, (1, 1))

# Against another kind it is simply not equal — never an error, and never true
# because the numbers happen to line up.
assert collomatique.EveryNWeeks(3) != collomatique.OncePerBlock(3, 3)
assert collomatique.EveryNWeeks(3) != 3
assert collomatique.WeekBlock(0, 2, (1, 1)) != collomatique.EveryNWeeks(3)

# They match, so a script can take one apart by shape rather than by getter.
match periodique.interrogation.periodicity:
    case collomatique.EveryNWeeks(n):
        assert n == 3
    case _:
        raise AssertionError("EveryNWeeks should match on its one argument")

match blocks[1]:
    case collomatique.WeekBlock(delay, size, count):
        assert (delay, size, count) == (3, 4, (0, 2))
    case _:
        raise AssertionError("WeekBlock should match on its three arguments")

# Construction validates what the model validates, and says so with `ValueError`
# rather than building something the document could never hold.
refused = [
    lambda: collomatique.WeekBlock(0, 0, (1, 1)),
    lambda: collomatique.WeekBlock(0, 2, (3, 1)),
    lambda: collomatique.EveryNWeeks(0),
    lambda: collomatique.OncePerBlock(0, 1),
    lambda: collomatique.OncePerBlock(2, 0),
    lambda: collomatique.CountInYear((5, 2), 0),
]
for build in refused:
    try:
        build()
    except ValueError:
        pass
    else:
        raise AssertionError("a leaf value must refuse what the model refuses")

# What the model does allow, python allows too: a yearly count may start at zero,
# and so may the separation that goes with it.
assert collomatique.CountInYear((0, 3), 0).count == (0, 3)
assert collomatique.CustomBlocks((), 0).blocks == ()

# The reprs are pasteable, which is the point of a value having a constructor.
assert repr(collomatique.EveryNWeeks(3)) == "EveryNWeeks(n=3)"
assert repr(collomatique.WeekBlock(0, 2, (1, 1))) == (
    "WeekBlock(delay_in_weeks=0, size_in_weeks=2, count=(1, 1))"
)
scope = {
    name: getattr(collomatique, name)
    for name in (
        "WeekBlock",
        "EveryNWeeks",
        "OncePerBlock",
        "CountInYear",
        "CustomBlocks",
    )
}
for value in (
    collomatique.EveryNWeeks(3),
    collomatique.OncePerBlock(4, 2),
    collomatique.CountInYear((2, 5), 0),
    collomatique.CustomBlocks((), 1),
    collomatique.CustomBlocks((collomatique.WeekBlock(0, 2, (1, 1)),), 1),
    collomatique.CustomBlocks(
        (collomatique.WeekBlock(0, 2, (1, 1)), collomatique.WeekBlock(3, 4, (0, 2))), 1
    ),
    collomatique.WeekBlock(3, 4, (0, 2)),
):
    assert eval(repr(value), scope) == value
