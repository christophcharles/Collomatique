import dataclasses

import collomatique

# `source` is a throwaway copy of a real colloscope. It carries both shapes:
# subjects that run colles, and two that only take up room in the timetable.
doc = collomatique.load(source)

subject_list = list(doc.subjects)
with_colles = [subject for subject in subject_list if subject.interrogation is not None]

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
subject_values = [subject.to_data() for subject in subject_list]
interrogation_values = [subject.interrogation.to_data() for subject in with_colles]

assert all(isinstance(d, collomatique.SubjectData) for d in subject_values)
assert all(isinstance(d, collomatique.InterrogationData) for d in interrogation_values)

# The sub-view's own value and the one nested in the subject's are the same
# thing, built twice.
assert [d.interrogation for d in subject_values if d.interrogation is not None] == (
    interrogation_values
)

# The fields as python sees them, so that a conversion wrong in both directions
# at once — the two ranges swapped, say — cannot pass rust's round-trip
# comparison by cancelling itself out.
value_names = [d.name for d in subject_values]
value_holds_colles = [d.interrogation is not None for d in subject_values]
value_students_per_group = [d.students_per_group for d in interrogation_values]
value_groups_per_interrogation = [
    d.groups_per_interrogation for d in interrogation_values
]
value_durations = [d.duration for d in interrogation_values]
value_take_duration = [d.take_duration_into_account for d in interrogation_values]

# A value holds ids, never handles: it is detached, and a handle would carry the
# document with it.
assert all(
    isinstance(period, collomatique.PeriodId)
    for d in subject_values
    for period in d.excluded_periods
)
assert all(isinstance(d.excluded_periods, set) for d in subject_values)

# A fresh object every call. Two of them are equal and share nothing, so writing
# to one is invisible to the other and to the document.
first = subject_list[0]
again = first.to_data()
assert again == subject_values[0]
assert again is not subject_values[0]
again.name = "Potions avancées"
assert subject_values[0].name != again.name
assert first.name != again.name

# The nested value is a real one, and this is the whole reason these classes are
# dataclasses rather than rust ones: assigning through it writes where it looks
# like it writes, instead of into a copy that is thrown away.
again.interrogation.duration = 15
assert again.interrogation.duration == 15
assert first.interrogation.duration != 15

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All three of these are answered for when the value is used, not here.
scratch = collomatique.SubjectData("")
scratch.interrogation = 3
scratch.excluded_periods = None
scratch = collomatique.InterrogationData()
scratch.duration = -1

# And it has no identity: an id names a place in a document, and a value has
# none. Updating an existing subject will pass the id as the method's argument.
assert not hasattr(subject_values[0], "id")
assert not hasattr(interrogation_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.SubjectData)] == [
    "name",
    "interrogation",
    "excluded_periods",
]
assert [f.name for f in dataclasses.fields(collomatique.InterrogationData)] == [
    "students_per_group",
    "groups_per_interrogation",
    "duration",
    "take_duration_into_account",
    "periodicity",
]

# The two classes are the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import SubjectData as _same_class  # noqa: E402

assert _same_class is collomatique.SubjectData
assert collomatique.SubjectData.__module__ == "collomatique"
assert collomatique.InterrogationData.__module__ == "collomatique"

# `SubjectData("Maths")` creates a subject that *holds* colles, with the
# application's own default parameters — that is what the model's own default
# does, and a subject exists to be interrogated in. The one that holds none is
# the exception, and it is spelled out.
plain = collomatique.SubjectData("Maths")
assert plain.interrogation == collomatique.InterrogationData()
assert plain.excluded_periods == set()
assert collomatique.SubjectData("Quidditch", interrogation=None).interrogation is None

# The default periodicity is a leaf value, built afresh for each value and equal
# across them.
assert collomatique.InterrogationData().periodicity == collomatique.EveryNWeeks(2)
assert collomatique.InterrogationData() == collomatique.InterrogationData()

# A field that names an entity takes a handle or an id, interchangeably, and
# anything iterable does on the way in. The two values below extract to the same
# subject and — this is the wart — do not compare equal, because a dataclass
# stores what it was given.
period = list(doc.periods)[0]
by_handle = collomatique.SubjectData("Spé maths", excluded_periods={period})
by_id = collomatique.SubjectData("Spé maths", excluded_periods={period.id})
by_list = collomatique.SubjectData("Spé maths", excluded_periods=[period])
assert by_handle != by_id

# One written out from end to end, none of its fields left at its default.
written_out = collomatique.SubjectData(
    "Options",
    interrogation=collomatique.InterrogationData(
        students_per_group=(1, 2),
        groups_per_interrogation=(2, 2),
        duration=90,
        take_duration_into_account=False,
        periodicity=collomatique.CountInYear((0, 4), 3),
    ),
    excluded_periods={period},
)
no_colles = collomatique.SubjectData("Quidditch", interrogation=None)

# Everything left at its default, so that rust can pin the defaults against the
# model's own.
bare_subject = collomatique.SubjectData("")
bare_interrogation = collomatique.InterrogationData()

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
zero_duration = collomatique.SubjectData(
    "Maths", interrogation=collomatique.InterrogationData(duration=0)
)
inverted_range = collomatique.SubjectData(
    "Maths", interrogation=collomatique.InterrogationData(students_per_group=(3, 2))
)
empty_group = collomatique.SubjectData(
    "Maths", interrogation=collomatique.InterrogationData(students_per_group=(0, 2))
)
not_a_periodicity = collomatique.SubjectData(
    "Maths", interrogation=collomatique.InterrogationData(periodicity=3)
)
not_a_name = collomatique.SubjectData(3)
not_an_interrogation = collomatique.SubjectData("Maths", interrogation=3)

# The same refusal on the class handed over whole, which is where the message
# names it rather than the subject it usually sits in.
bare_zero_duration = collomatique.InterrogationData(duration=0)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_period = collomatique.SubjectData(
    "Maths", excluded_periods={list(other.periods)[0]}
)
