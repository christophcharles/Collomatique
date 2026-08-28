import dataclasses
import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope. Its slots carry both
# shapes — some with a week pattern of their own and some without — and its
# patterns all switch some weeks off.
doc = collomatique.load(source)

pattern_list = list(doc.week_patterns)
slot_list = list(doc.slots)

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
pattern_values = [pattern.to_data() for pattern in pattern_list]
slot_values = [slot.to_data() for slot in slot_list]

assert all(isinstance(d, collomatique.WeekPatternData) for d in pattern_values)
assert all(isinstance(d, collomatique.SlotData) for d in slot_values)

# The fields as python sees them, so that a conversion wrong in both directions
# at once — the day and the time swapped, say — cannot pass rust's round-trip
# comparison by cancelling itself out. The entities are named by their place in
# the walk they belong to, since an id is opaque and means nothing written down.
weeks = list(doc.weeks)
week_positions = {week.id: position for position, week in enumerate(weeks)}
subject_positions = {
    subject.id: position for position, subject in enumerate(doc.subjects)
}
teacher_surnames = {teacher.id: teacher.surname for teacher in doc.teachers}
pattern_positions = {
    pattern.id: position for position, pattern in enumerate(pattern_list)
}

value_pattern_names = [d.name for d in pattern_values]
value_excluded_week_indices = [
    sorted(week_positions[week_id] for week_id in d.excluded_weeks)
    for d in pattern_values
]

value_subject_indices = [subject_positions[d.subject] for d in slot_values]
value_teacher_surnames = [teacher_surnames[d.teacher] for d in slot_values]
value_start_times = [d.start_time for d in slot_values]
value_extra_info = [d.extra_info for d in slot_values]
value_pattern_indices = [
    None if d.week_pattern is None else pattern_positions[d.week_pattern]
    for d in slot_values
]
value_costs = [d.cost for d in slot_values]

# The day is one of the seven members, which are class attributes rather than an
# `enum.Enum`, so the script names them itself and rust holds the other half of
# this dictionary.
weekday_names = {
    collomatique.Weekday.MONDAY: "monday",
    collomatique.Weekday.TUESDAY: "tuesday",
    collomatique.Weekday.WEDNESDAY: "wednesday",
    collomatique.Weekday.THURSDAY: "thursday",
    collomatique.Weekday.FRIDAY: "friday",
    collomatique.Weekday.SATURDAY: "saturday",
    collomatique.Weekday.SUNDAY: "sunday",
}
value_weekdays = [weekday_names[d.weekday] for d in slot_values]

# A value holds ids, never handles: it is detached, and a handle would carry the
# document with it.
assert all(isinstance(d.subject, collomatique.SubjectId) for d in slot_values)
assert all(isinstance(d.teacher, collomatique.TeacherId) for d in slot_values)
assert all(
    d.week_pattern is None or isinstance(d.week_pattern, collomatique.WeekPatternId)
    for d in slot_values
)
assert all(
    isinstance(week_id, collomatique.WeekId)
    for d in pattern_values
    for week_id in d.excluded_weeks
)
assert all(isinstance(d.excluded_weeks, set) for d in pattern_values)

# The subject in the value is the slot's own. That is what makes a
# read-modify-write never trip over the one field a slot cannot change.
assert [d.subject for d in slot_values] == [slot.subject.id for slot in slot_list]
assert [d.teacher for d in slot_values] == [slot.teacher.id for slot in slot_list]
assert [d.week_pattern for d in slot_values] == [
    None if slot.week_pattern is None else slot.week_pattern.id for slot in slot_list
]

# The example is worth reading here: both shapes of `week_pattern` are among the
# slots, so neither branch passes by never being taken.
assert any(d.week_pattern is not None for d in slot_values)
assert any(d.week_pattern is None for d in slot_values)

# The day and the time come out as the read surface's own vocabulary — the same
# leaf value and the same `datetime.time`, not a second spelling of them.
assert all(isinstance(d.weekday, collomatique.Weekday) for d in slot_values)
assert all(isinstance(d.start_time, datetime.time) for d in slot_values)
assert all(
    d.start_time.second == 0 and d.start_time.microsecond == 0 for d in slot_values
)
assert [d.weekday for d in slot_values] == [slot.weekday for slot in slot_list]
assert [d.start_time for d in slot_values] == [slot.start_time for slot in slot_list]

# A fresh object every call. Two of them are equal and share nothing, so writing
# to one is invisible to the other and to the document.
first = slot_list[0]
again = first.to_data()
assert again == slot_values[0]
assert again is not slot_values[0]
again.extra_info = "Salle 13"
assert slot_values[0].extra_info != again.extra_info
assert first.extra_info != again.extra_info

# The container is a real mutable one, which is the whole point of a value:
# adding a week to it writes where it looks like it writes.
pattern_again = pattern_list[0].to_data()
pattern_again.excluded_weeks.add(weeks[0].id)
assert pattern_again.excluded_weeks != pattern_values[0].excluded_weeks

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.WeekPatternData("")
scratch.excluded_weeks = None
scratch = collomatique.SlotData(3, "personne", "jeudi", "14h00")
scratch.cost = "cher"

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing slot will pass the id as the method's argument.
assert not hasattr(slot_values[0], "id")
assert not hasattr(pattern_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.WeekPatternData)] == [
    "name",
    "excluded_weeks",
]
assert [f.name for f in dataclasses.fields(collomatique.SlotData)] == [
    "subject",
    "teacher",
    "weekday",
    "start_time",
    "extra_info",
    "week_pattern",
    "cost",
]

# The two classes are the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import SlotData as _same_class  # noqa: E402

assert _same_class is collomatique.SlotData
assert collomatique.SlotData.__module__ == "collomatique"
assert collomatique.WeekPatternData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably, and
# anything iterable does where a set is wanted. The values below extract to the
# same pattern and — this is the wart — do not compare equal, because a
# dataclass stores what it was given.
first_week = weeks[0]
pattern_by_handle = collomatique.WeekPatternData(
    "Semaines paires", excluded_weeks={first_week}
)
pattern_by_id = collomatique.WeekPatternData(
    "Semaines paires", excluded_weeks={first_week.id}
)
pattern_by_list = collomatique.WeekPatternData(
    "Semaines paires", excluded_weeks=[first_week]
)
assert pattern_by_handle != pattern_by_id

# Both ends of what a pattern can be: one that excludes nothing, and one the
# user never named. Neither is an unfinished value.
bare_pattern = collomatique.WeekPatternData("")
assert bare_pattern.excluded_weeks == set()

# The same, for a slot: one written out from end to end, twice over, naming its
# three entities by handle and by id.
subject = list(doc.subjects)[0]
teacher = list(doc.teachers)[0]
pattern = pattern_list[0]

slot_by_handle = collomatique.SlotData(
    subject,
    teacher,
    collomatique.Weekday.THURSDAY,
    datetime.time(14, 0),
    extra_info="Salle 12",
    week_pattern=pattern,
    cost=-3,
)
slot_by_id = collomatique.SlotData(
    subject.id,
    teacher.id,
    collomatique.Weekday.THURSDAY,
    datetime.time(14, 0),
    extra_info="Salle 12",
    week_pattern=pattern.id,
    cost=-3,
)
assert slot_by_handle != slot_by_id

# Nothing but the four fields a slot cannot do without, so rust can pin what the
# defaulted three come out as: no extra info, every week, and no cost.
bare_slot = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0)
)
assert bare_slot.extra_info == ""
assert bare_slot.week_pattern is None
assert bare_slot.cost == 0

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_pattern_name = collomatique.WeekPatternData(3)
not_a_weekday = collomatique.SlotData(subject, teacher, 3, datetime.time(8, 0))
not_a_time = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, "8h00"
)
seconds_in_the_time = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0, 30)
)
microseconds_in_the_time = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0, 0, 500)
)
not_an_extra_info = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0), extra_info=3
)
not_a_cost = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0), cost="cher"
)
not_a_subject = collomatique.SlotData(
    3, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0)
)
not_a_pattern = collomatique.SlotData(
    subject,
    teacher,
    collomatique.Weekday.MONDAY,
    datetime.time(8, 0),
    week_pattern="Semaines paires",
)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_teacher = collomatique.SlotData(
    subject, list(other.teachers)[0], collomatique.Weekday.MONDAY, datetime.time(8, 0)
)
foreign_week = collomatique.WeekPatternData(
    "Ailleurs", excluded_weeks={list(other.weeks)[0]}
)
