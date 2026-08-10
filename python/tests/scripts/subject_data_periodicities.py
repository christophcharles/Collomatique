import collomatique

# `source` is a document rust built for this script: the example only ever uses
# two of the four periodicities, and no subject of it skips a period.
doc = collomatique.load(source)

subjects = list(doc.subjects)
subject_values = [subject.to_data() for subject in subjects]

# The four kinds come out of a value as the very leaf values the read surface
# hands out, in the order rust wrote them.
assert [
    type(d.interrogation.periodicity).__name__ for d in subject_values
] == ["EveryNWeeks", "OncePerBlock", "CountInYear", "CustomBlocks"]
assert all(
    isinstance(d.interrogation.periodicity, collomatique.Periodicity)
    for d in subject_values
)

# The one subject that skips a period carries it — as an id, since a value is
# detached — and the ones that skip none carry an empty set rather than a
# `None`.
periods = list(doc.periods)
assert subject_values[1].excluded_periods == {periods[1].id}
assert subject_values[0].excluded_periods == set()
assert subject_values[2].excluded_periods == set()

# The same four, written out by hand rather than read back: this is the half
# that says a script can *build* a periodicity, and rust compares them with the
# ones the document holds.
hand_built = [
    collomatique.InterrogationData(periodicity=collomatique.EveryNWeeks(3)),
    collomatique.InterrogationData(periodicity=collomatique.OncePerBlock(4, 2)),
    collomatique.InterrogationData(periodicity=collomatique.CountInYear((2, 5), 0)),
    collomatique.InterrogationData(
        periodicity=collomatique.CustomBlocks(
            (
                collomatique.WeekBlock(0, 2, (1, 1)),
                collomatique.WeekBlock(3, 4, (0, 2)),
            ),
            1,
        )
    ),
]

# A periodicity with no block at all is one the model allows, so the boundary
# does too — it is a subject nobody is ever interrogated in, which is odd but
# not wrong.
no_block = collomatique.InterrogationData(
    periodicity=collomatique.CustomBlocks((), 0)
)
