import dataclasses
import datetime

import collomatique

# `source` is a synthetic document that exercises every section of the tree:
# both shapes of every optional field, stored rows in both junction tables, a
# filled colloscope, and non-default settings, balancing and export
# configuration. `other_source` is a real colloscope, for the foreign-document
# questions.
doc = collomatique.load(source)
other = collomatique.load(other_source)

tree = doc.snapshot()

# The one value class for the whole document, holding the matching value class
# in every section.
assert isinstance(tree, collomatique.DocumentData)
assert isinstance(tree.colloscope, collomatique.ColloscopeData)
assert isinstance(tree.export_config, collomatique.ExportConfigData)
assert isinstance(tree.global_limits, collomatique.LimitsData)
assert isinstance(tree.global_balancing, collomatique.BalancingData)


def all_kinds(mapping, kind):
    return all(isinstance(value, kind) for value in mapping.values())


assert all_kinds(tree.weeks, collomatique.WeekData)
assert all_kinds(tree.subjects, collomatique.SubjectData)
assert all_kinds(tree.teachers, collomatique.TeacherData)
assert all_kinds(tree.students, collomatique.StudentData)
assert all_kinds(tree.week_patterns, collomatique.WeekPatternData)
assert all_kinds(tree.slots, collomatique.SlotData)
assert all_kinds(tree.incompats, collomatique.IncompatData)
assert all_kinds(tree.group_lists, collomatique.GroupListData)
assert all_kinds(tree.pairings, collomatique.PairingRuleData)
assert all_kinds(tree.slot_pairings, collomatique.SlotPairingRuleData)
assert all_kinds(tree.student_limits, collomatique.LimitsData)
assert all_kinds(tree.subject_balancing, collomatique.BalancingData)

# A tree holds ids, never handles: it is detached, and a handle would carry
# the document with it.
assert all(isinstance(period, collomatique.PeriodId) for period in tree.periods)
assert all(isinstance(key, collomatique.WeekId) for key in tree.weeks)
assert all(isinstance(key, collomatique.SubjectId) for key in tree.subjects)
assert all(isinstance(key, collomatique.TeacherId) for key in tree.teachers)
assert all(isinstance(key, collomatique.StudentId) for key in tree.students)
assert all(isinstance(key, collomatique.WeekPatternId) for key in tree.week_patterns)
assert all(isinstance(key, collomatique.SlotId) for key in tree.slots)
assert all(isinstance(key, collomatique.IncompatId) for key in tree.incompats)
assert all(isinstance(key, collomatique.GroupListId) for key in tree.group_lists)
assert all(isinstance(key, collomatique.PairingRuleId) for key in tree.pairings)
assert all(isinstance(key, collomatique.SlotPairingRuleId) for key in tree.slot_pairings)
assert all(isinstance(key, collomatique.StudentId) for key in tree.student_limits)
assert all(isinstance(key, collomatique.SubjectId) for key in tree.subject_balancing)
assert all(
    isinstance(period, collomatique.PeriodId)
    and isinstance(subject, collomatique.SubjectId)
    for period, subject in tree.assignments
)
assert all(
    isinstance(period, collomatique.PeriodId)
    and isinstance(subject, collomatique.SubjectId)
    for period, subject in tree.group_list_associations
)

# The order lives in the containers: each ordered section's dict order is the
# document's own walk order, the one a script reads through the handles. A
# snapshot that stored an index instead could contradict itself.
assert list(tree.periods) == [p.id for p in doc.periods]
assert list(tree.weeks) == [w.id for w in doc.weeks]
assert list(tree.subjects) == [s.id for s in doc.subjects]
assert list(tree.slots) == [s.id for s in doc.slots]

# The sparse sections hold the stored rows only, and the rows agree with the
# handles' reads of the very same tables.
assert tree.assignments == {
    (period.id, subject.id): {s.id for s in students}
    for period, subject, students in doc.assignments
}
assert tree.group_list_associations == {
    (period.id, subject.id): group_list.id
    for period, subject, group_list in doc.group_lists.associations()
}

# The first week comes out as the date the periods view shows.
assert tree.first_week == doc.periods.first_week
assert isinstance(tree.first_week, datetime.date)

# Every section actually has something to say: a fixture with a section left
# empty would pass the class checks above by never exercising them.
assert len(tree.weeks) >= 4
assert len(tree.subjects) >= 2
assert len(tree.teachers) >= 2
assert len(tree.students) >= 4
assert len(tree.week_patterns) >= 1
assert len(tree.slots) >= 3
assert len(tree.incompats) >= 1
assert len(tree.group_lists) >= 2
assert len(tree.pairings) >= 1
assert len(tree.slot_pairings) >= 1
assert len(tree.assignments) >= 2
assert len(tree.group_list_associations) >= 2
assert len(tree.student_limits) >= 1
assert len(tree.subject_balancing) >= 1
assert len(tree.colloscope.interrogations) >= 3
assert len(tree.colloscope.group_lists) >= 1

# A fresh tree every call. Two of them are equal and share nothing, and
# editing one is invisible to the document.
again = doc.snapshot()
assert again == tree
assert again is not tree
first_subject = list(again.subjects)[0]
again.subjects[first_subject].name = "Renamed in the tree"
assert doc.subjects[first_subject].name != "Renamed in the tree"

# The empty tree is what an empty document holds.
assert collomatique.DocumentData() == collomatique.new_document().snapshot()
assert collomatique.new_document().snapshot().first_week is None

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the tree is used, not here.
scratch = collomatique.DocumentData()
scratch.periods = 3
scratch.weeks = "nope"

# And the untouched empty tree, for rust to pin against the model's own
# default.
defaults = collomatique.DocumentData()

# And a tree has no identity: an id names a place in a document, and a value
# has none.
assert not hasattr(tree, "id")

# The field order of the class, which is what a positional call depends on.
assert [f.name for f in dataclasses.fields(collomatique.DocumentData)] == [
    "first_week",
    "periods",
    "weeks",
    "subjects",
    "teachers",
    "students",
    "assignments",
    "week_patterns",
    "slots",
    "incompats",
    "group_lists",
    "group_list_associations",
    "pairings",
    "slot_pairings",
    "global_limits",
    "student_limits",
    "global_balancing",
    "subject_balancing",
    "colloscope",
    "export_config",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import DocumentData as _same_class  # noqa: E402

assert _same_class is collomatique.DocumentData
assert collomatique.DocumentData.__module__ == "collomatique"

# A tree keyed by handles and one keyed by ids name the same entities, and
# the two do not compare equal — the wart §2.3 records, same as every value
# class.
subject = list(doc.subjects)[0]
teacher = list(doc.teachers)[0]
student = list(doc.students)[0]
week = list(doc.weeks)[0]
first_period = list(doc.periods)[0]

by_handles = collomatique.DocumentData(
    periods=[first_period],
    weeks={week: collomatique.WeekData(first_period)},
    subjects={subject: collomatique.SubjectData("Spé maths")},
    teachers={teacher: collomatique.TeacherData("Emmy", "Noether")},
    students={student: collomatique.StudentData("Harry", "Potter")},
)
by_ids = collomatique.DocumentData(
    periods=[first_period.id],
    weeks={week.id: collomatique.WeekData(first_period.id)},
    subjects={subject.id: collomatique.SubjectData("Spé maths")},
    teachers={teacher.id: collomatique.TeacherData("Emmy", "Noether")},
    students={student.id: collomatique.StudentData("Harry", "Potter")},
)
assert by_handles != by_ids

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_tree = 3
not_a_monday = collomatique.DocumentData(first_week=datetime.date(2026, 8, 12))
not_a_section = collomatique.DocumentData(subjects=3)
foreign_week = list(other.weeks)[0]
with_a_foreign_week = collomatique.DocumentData(
    weeks={foreign_week: collomatique.WeekData(first_period)}
)

# The two spellings of one entity are different dict keys, so a section can
# name the same entity twice. Silence would keep only one entry, so the
# extraction refuses instead — for an entity section, and for the two
# junction tables.
group_list = list(doc.group_lists)[0]
with_a_doubled_teacher = collomatique.DocumentData(
    teachers={
        teacher: collomatique.TeacherData("Emmy", "Noether"),
        teacher.id: collomatique.TeacherData("Emmy", "Noether"),
    }
)
with_a_doubled_assignment = collomatique.DocumentData(
    assignments={
        (first_period, subject): set(),
        (first_period.id, subject.id): set(),
    }
)
with_a_doubled_association = collomatique.DocumentData(
    group_list_associations={
        (first_period, subject): group_list,
        (first_period.id, subject.id): group_list.id,
    }
)
