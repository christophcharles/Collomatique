"""The values a script builds, and what a read hands back detached.

These are python dataclasses rather than rust classes because a value nests
and holds real mutable containers, and a pyo3 getter hands back a *clone* of
the struct it holds — so `value.nested.field = x` would quietly write to a
temporary that is thrown away. A dataclass has no such trap.

A value is dumb. It stores what it is given and checks nothing, so
`TeacherData("", "")` and `d.tel = 42` both simply work here. The checking
happens when the value is used — when a mutator extracts it — because that is
the last moment at which a message can still name the field that was wrong.

This file is never imported from disk. It is compiled from a string while
`collomatique` initializes, registered in `sys.modules` as `collomatique._data`,
and every name in `__all__` is re-exported into `collomatique` itself. So a
script writes `clm.TeacherData` and never names this module.

`from __future__ import annotations` is what makes the hints below legal. They
are strings that are never evaluated (PEP 563), so they may name `Subject`,
`SubjectId` and the other rust classes — which cannot be imported at runtime
here, since `collomatique` is still initializing while this file is compiled.
The `TYPE_CHECKING` block below is what makes those names resolvable to a type
checker and to a linter, and it runs on neither python's part nor ours.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import datetime

    from collomatique import (
        AutomaticGroups,
        Color,
        ConductorWarning,
        Enforcement,
        Filling,
        GroupList,
        GroupListId,
        IncompatId,
        Limit,
        Orientation,
        PairingRuleId,
        Period,
        PeriodId,
        Periodicity,
        Slot,
        SlotId,
        SlotPairingRuleId,
        Student,
        StudentId,
        Subject,
        SubjectId,
        Teacher,
        TeacherId,
        TimeSlot,
        Week,
        WeekId,
        Weekday,
        WeekPattern,
        WeekPatternId,
    )

__all__ = [
    "TeacherData",
    "StudentData",
    "SubjectData",
    "InterrogationData",
    "WeekPatternData",
    "SlotData",
    "IncompatData",
    "GroupListData",
    "PairingRuleSideData",
    "PairingRuleData",
    "SlotPairingRuleSideData",
    "SlotPairingRuleData",
    "LimitsData",
    "BalancingData",
    "ExportGlobalConfigData",
    "ExportColloscopeConfigData",
    "ExportStudentGroupsConfigData",
    "ExportGroupListConfigData",
    "ExportConfigData",
    "ColloscopeData",
    "WeekData",
    "DocumentData",
    "PeriodSolveConfig",
    "GroupListSolveConfig",
    "ColloscopeSolveConfig",
    "DefaultConfig",
    "WarmStartConfig",
    "IncrementalConfig",
    "FuzzyConfig",
    "ConductorStrategy",
    "GroupListsGenerationRequest",
]


def _every_other_week() -> Periodicity:
    """The periodicity a new `InterrogationData` comes with.

    A function rather than a value, because there is no value to write here:
    `EveryNWeeks` is one of the rust classes, and this file is compiled while
    `collomatique` is still initializing, so nothing of it can be imported at
    that moment. A `default_factory` runs when a value is built instead, which
    is long after the module is whole.
    """

    import collomatique

    return collomatique.EveryNWeeks(2)


@dataclass
class TeacherData:
    """A teacher, detached from the document.

    `doc.teachers[...].to_data()` hands one back, and `doc.teachers.add` and
    `doc.teachers.update` take one:

        clm.TeacherData("Emmy", "Noether", email="noether@lycee.fr",
                        subjects={maths})

    `firstname` and `surname` are plain strings, the empty one included: the
    model types them that way, and python mirrors it rather than editorializing.

    `tel` and `email` are a non-empty string or `None`. Somebody who shared no
    number has none, rather than having an empty one, so `""` is refused when
    the value is used.

    `subjects` is the set of subjects this teacher interrogates in. It takes
    `Subject` handles and `SubjectId`s interchangeably, in any mix, like every
    other place in this API that names an entity; `to_data()` always fills it
    with ids, so that a value carries no document around with it. Two values
    naming the same subject, one by handle and one by id, therefore do *not*
    compare equal — a dataclass stores what it was given, and a handle and an id
    hash differently.

    A teacher may only be declared in a subject that holds interrogations. That
    is a statement about the document rather than about this value, so it is the
    write that refuses it, with a `TeachersError`, and not this class.
    """

    firstname: str
    surname: str
    tel: str | None = None
    email: str | None = None
    subjects: set[Subject | SubjectId] = field(default_factory=set)


@dataclass
class StudentData:
    """A student, detached from the document.

    The same card as `TeacherData`, with a different set at the end.
    `doc.students[...].to_data()` hands one back, and `doc.students.add` and
    `doc.students.update` take one:

        clm.StudentData("Harry", "Potter", tel="0601020304",
                        excluded_periods={first_period})

    `firstname`, `surname`, `tel` and `email` behave exactly as they do on a
    `TeacherData`, and for the same reasons: the model keeps one card for both
    kinds of person.

    `excluded_periods` is the set of periods this student takes no part in. It
    takes `Period` handles and `PeriodId`s interchangeably, and `to_data()`
    fills it with ids. Two values naming the same periods, one by handles and
    one by ids, do not compare equal — a handle and an id hash differently.

    Which subjects a student takes is not here. The model keeps that in a
    junction table of its own, keyed by period and subject, which python reads
    and writes as `doc.assignments`. Excluding a period through this class is
    therefore a write that reaches those rows: nobody can be assigned in a
    period they take no part in, so `doc.students.update` unassigns them there
    and reports it.
    """

    firstname: str
    surname: str
    tel: str | None = None
    email: str | None = None
    excluded_periods: set[Period | PeriodId] = field(default_factory=set)


@dataclass
class InterrogationData:
    """How one subject's interrogations are laid out, detached from the document.

    `subject.interrogation.to_data()` hands one back, and a `SubjectData` holds
    one:

        clm.SubjectData("Maths", interrogation=clm.InterrogationData(
            duration=30, periodicity=clm.EveryNWeeks(1)))

    Every field has the model's own default, so `clm.InterrogationData()` is
    exactly what the application creates when a user switches a subject's colles
    on. It is written before `SubjectData` here for the same reason: that class
    holds one of these as its own default.

    `students_per_group` and `groups_per_interrogation` are `(min, max)` pairs,
    inclusive at both ends and counting from one — a group with no student in it
    and an interrogation with no group in it are not things the model can hold.

    `duration` is a whole number of minutes, at least one. `periodicity` is one
    of the four `Periodicity` values — `EveryNWeeks`, `OncePerBlock`,
    `CountInYear`, `CustomBlocks` — which check themselves when they are built,
    so nothing is left for this class to refuse.
    """

    students_per_group: tuple[int, int] = (2, 3)
    groups_per_interrogation: tuple[int, int] = (1, 1)
    duration: int = 60
    take_duration_into_account: bool = True
    periodicity: Periodicity = field(default_factory=_every_other_week)


@dataclass
class SubjectData:
    """A subject, detached from the document.

    `doc.subjects[...].to_data()` hands one back, and `doc.subjects.add` and
    `doc.subjects.update` take one:

        clm.SubjectData("Spé maths")

    That line creates a subject that **holds colles**, with the application's
    own default parameters, because `interrogation` defaults to a fresh
    `InterrogationData`. The subject that holds none is the exception, and it is
    spelled out:

        clm.SubjectData("Quidditch", interrogation=None)

    `name` is a plain string, the empty one included: the model types it that
    way.

    `excluded_periods` is the set of periods this subject does not run in. It
    takes `Period` handles and `PeriodId`s interchangeably, and `to_data()`
    fills it with ids. Two values naming the same periods, one by handles and
    one by ids, do not compare equal — a handle and an id hash differently.

    That field is here because a subject really holds it — `doc.snapshot()`
    would lose which subjects skip which periods otherwise. No subject op
    carries it, though, so rather than throwing it away quietly the two subject
    mutators refuse: `add` when it is not empty, `update` when it differs from
    what the document holds. What moves it is
    `doc.subjects.set_period_status(subject, period, active)`, and adding a
    subject that skips a period is therefore two calls, which a transaction
    makes one undo step.

    `week_pattern` says which weeks this subject really runs colles on. `None`
    means every week: the subject carries no pattern of its own, so only the
    weeks' own flags switch it off. A pattern can only take weeks away, never
    give one back — a week its period runs no colles on stays off whatever the
    pattern says. It takes a `WeekPattern` handle or a `WeekPatternId`, and
    `to_data()` fills it with an id.

    Unlike `excluded_periods`, that field the subject ops carry, so
    `doc.subjects.update` writes what the value names: a value naming no pattern
    clears the one the subject had. A read-modify-write leaves it where it was,
    since `to_data()` fills the field with the subject's own pattern.

    Rewriting the rest of the value through `doc.subjects.update` is a write
    that reaches most of the document. Setting `interrogation` to `None`
    dismantles everything that needed those colles — the teachers who held them,
    their slots in the subject, its group-list associations, its balancing
    options and the pairing rules naming it — and lengthening `duration` far
    enough to push a late slot past midnight takes that slot. Giving the subject
    a pattern that switches a week off takes the colles already written there.
    The result reports it either way.
    """

    name: str
    interrogation: InterrogationData | None = field(default_factory=InterrogationData)
    excluded_periods: set[Period | PeriodId] = field(default_factory=set)
    week_pattern: WeekPattern | WeekPatternId | None = None


@dataclass
class WeekPatternData:
    """A week pattern, detached from the document.

    `doc.week_patterns[...].to_data()` hands one back, and
    `doc.week_patterns.add` and `doc.week_patterns.update` take one:

        clm.WeekPatternData("Semaines paires", excluded_weeks={w1, w3})

    A pattern is stored as the weeks it *switches off*, and that is the whole of
    it: every week not in `excluded_weeks` is one the pattern leaves alone. So a
    pattern whose set is empty excludes nothing, which is an ordinary pattern
    and not an unfinished one.

    `name` is a plain string, the empty one included: the model types it that
    way, so a pattern the user never named reads as `""` rather than as `None`.

    `excluded_weeks` takes `Week` handles and `WeekId`s interchangeably, and
    `to_data()` fills it with ids. Two values naming the same weeks, one by
    handles and one by ids, do not compare equal — a handle and an id hash
    differently.

    A week that runs no interrogations of its own may perfectly well be in the
    set. The model keeps the two apart, so that switching such a week back on
    brings back the pattern it had.

    Adding a week to the set through `doc.week_patterns.update` is a write that
    reaches the colloscope: a slot following the pattern holds no interrogation
    on a week the pattern switches off, so the colles already written in those
    cells go and the result reports it.
    """

    name: str
    excluded_weeks: set[Week | WeekId] = field(default_factory=set)


@dataclass
class SlotData:
    """A slot, detached from the document.

    `doc.slots[...].to_data()` hands one back, and `doc.slots.add` and
    `doc.slots.update` take one:

        clm.SlotData(maths, snape, clm.Weekday.THURSDAY, datetime.time(14, 0))

    `subject` is the subject whose colles this slot carries. It is a field of
    the value rather than a separate argument, so `doc.slots.add(d)` reads it
    off the value like every other collection's add. It cannot be *changed*
    afterwards: the model files a slot under its subject in the list that gives
    it its position, so `doc.slots.update` refuses a value naming a different
    one instead of dropping the field. A read-modify-write never meets that,
    since `to_data()` fills the field with the slot's own subject.

    `subject`, `teacher` and `week_pattern` take handles and ids
    interchangeably, and `to_data()` fills them with ids. Two values naming
    the same entities, one by handles and one by ids, do not compare equal — a
    handle and an id hash differently.

    `weekday` is one of the seven `Weekday` values, and `start_time` a
    `datetime.time` falling on a whole minute — the model stores the time with
    minute precision, so one carrying seconds is refused when the value is used.
    Neither has an honest default: a slot that does not say when it happens is
    not a slot.

    A slot has no duration of its own. The subject fixes it, so the length of a
    colle is that subject's `interrogation.duration`.

    `extra_info` is what the export prints beside the slot — a room number, and
    the like. A plain string, the empty one included.

    `week_pattern` says which weeks this slot really runs on. `None` means every
    week: the slot carries no pattern of its own, so only the weeks' own flags
    switch it off.

    `cost` is what using this slot costs the solver. Zero leaves it alone, a
    positive cost tells the solver to avoid the slot, a negative one to favour
    it.
    """

    subject: Subject | SubjectId
    teacher: Teacher | TeacherId
    weekday: Weekday
    start_time: datetime.time
    extra_info: str = ""
    week_pattern: WeekPattern | WeekPatternId | None = None
    cost: int = 0


@dataclass
class IncompatData:
    """An incompatibility, detached from the document.

    `doc.incompats[...].to_data()` hands one back, and `doc.incompats.add` and
    `doc.incompats.update` take one:

        clm.IncompatData("Lundi Midi", maths, slots=[clm.TimeSlot(
            clm.Weekday.MONDAY, datetime.time(12, 0), 60)])

    An incompatibility says when the students of a subject may be unavailable:
    the busy windows of `slots`, at least `minimum_free_slots` of which must
    stay free.

    `subject` is the subject whose students this constrains. It is deliberately
    *not* required to hold interrogations of its own — a student can be
    declared in a subject purely so that an incompatibility can block slots for
    them, without the subject having colles. It takes a `Subject` handle or a
    `SubjectId`, like every other place in this API that names an entity;
    `to_data()` fills it with an id, so that a value carries no document around
    with it. Two values naming the same subject, one by handle and one by id,
    do not compare equal — a handle and an id hash differently.

    `slots` is the list of busy windows, as `TimeSlot` values — a day, a start
    time and a duration. The windows are data, not handles: nothing points at
    one by position, so a script takes them apart and compares them as values.
    The model's own window type refuses a window that crosses midnight, and
    `TimeSlot` refuses it when it is built, so a list here only ever holds
    windows the document could hold.

    `minimum_free_slots` is how many of the windows must stay free, at least
    one: an incompatibility that could spare every window would be no
    incompatibility at all. It has no model default, and 1 is the neutral one.

    `week_pattern` says which weeks this incompatibility applies on. `None`
    means every week: the incompatibility has no pattern of its own, so only
    the weeks' own flags switch it off. It takes a `WeekPattern` handle or a
    `WeekPatternId`, and `to_data()` fills it with an id.
    """

    name: str
    subject: Subject | SubjectId
    slots: list[TimeSlot] = field(default_factory=list)
    minimum_free_slots: int = 1
    week_pattern: WeekPattern | WeekPatternId | None = None


def _sixteen_unnamed() -> list[str | None]:
    """The group names a new `GroupListData` comes with.

    The model's own default: sixteen unnamed groups, which is room for a class
    of forty-eight students at three per group. A fresh list every call, since
    a dataclass must never share a mutable default between its instances.
    """

    return [None] * 16


def _automatic_filling() -> AutomaticGroups:
    """The filling a new `GroupListData` comes with.

    A function rather than a value, because there is no value to write here:
    `AutomaticGroups` is one of the rust classes, and this file is compiled
    while `collomatique` is still initializing, so nothing of it can be
    imported at that moment. A `default_factory` runs when a value is built
    instead, which is long after the module is whole.
    """

    import collomatique

    return collomatique.AutomaticGroups()


@dataclass
class GroupListData:
    """A group list, detached from the document.

    `doc.group_lists[...].to_data()` hands one back, and
    `doc.group_lists.add` and `doc.group_lists.update` take one:

        clm.GroupListData(
            "Maisons",
            group_names=["Gryffondor", "Serpentard"],
            filling=clm.PrefilledGroups(({harry, hermione}, {ron, neville})))

    Every field takes the model's own default, so `clm.GroupListData()` is
    exactly what the application creates when a user adds a group list — a
    list named « Liste », two to three students per group, sixteen unnamed
    groups, and the solver filling them.

    `name` is a plain string, the empty one included: the model types it that
    way.

    `students_per_group` is a `(min, max)` pair, inclusive at both ends and
    counting from one.

    `group_names` names the groups, entry `i` naming group `i`. Its length is
    the group count, and every group number in the colloscope is measured
    against it. `None` is a group that shows its number, and a stored name is
    a non-empty string — `""` is refused when the value is used.

    `filling` is how the groups are filled, and it keeps the sum the model
    keeps: `AutomaticGroups` lets the solver place the students, and
    `PrefilledGroups` fixes the set of students of each group. A flat encoding
    of the two would have two states that mean nothing — both shapes set, or
    neither — so they stay two classes under the `Filling` base:

        clm.AutomaticGroups(excluded_students={ron})
        clm.PrefilledGroups(({harry, hermione}, {ron, neville}))

    The students inside take `Student` handles and `StudentId`s
    interchangeably, and `to_data()` fills them with ids — two fillings naming
    the same students in the two spellings do not compare equal, since a
    handle and an id hash differently. A prefilled filling must have exactly
    `len(group_names)` groups, and no student may appear in two of them; both
    are checked when the value is used, by the model's own constructor, whose
    message is the one a script meets.

    The value carries the whole list, so `doc.group_lists.update` rewrites the
    filling with the parameters: the model seals the two together, and there is
    no writing one without the other. That write reaches the colloscope, which
    is measured against the list — fewer groups than a colle names, a student
    the list starts excluding, or a filling that stops being automatic all cost
    what no longer fits, and the result reports it.
    """

    name: str = "Liste"
    students_per_group: tuple[int, int] = (2, 3)
    group_names: list[str | None] = field(default_factory=_sixteen_unnamed)
    filling: Filling = field(default_factory=_automatic_filling)


@dataclass
class PairingRuleSideData:
    """One end of a pairing rule, detached from the document.

    `rule.antecedent.to_data()` and `rule.consequent.to_data()` hand one back,
    and a `PairingRuleData` holds one of each:

        clm.PairingRuleSideData(maths)
        clm.PairingRuleSideData(maths, should_have=False)

    `subject` is the subject this end of the rule is about. It takes a
    `Subject` handle or a `SubjectId`, like every other place in this API
    that names an entity; `to_data()` fills it with an id, so that a value
    carries no document around with it. Two sides naming the same subject,
    one by handle and one by id, do not compare equal — a handle and an id
    hash differently.

    `should_have` is whether a student marked for the rule is marked *for*
    this subject's interrogation, or marked off it. `True` is the neutral
    spelling, the one the application itself starts a new rule with.
    """

    subject: Subject | SubjectId
    should_have: bool = True


@dataclass
class PairingRuleData:
    """A pairing rule, detached from the document.

    `doc.pairings[...].to_data()` hands one back, and `doc.pairings.add` and
    `doc.pairings.update` take one:

        clm.PairingRuleData(clm.PairingRuleSideData(maths),
                            clm.PairingRuleSideData(physics))

    The rule is an implication between two subjects: a student who satisfies
    the `antecedent` in a week — who `should_have` its subject's
    interrogation, or not — must also satisfy the `consequent`.

    The two ends are plain values, not leaves, so they are written as well as
    read: `d.antecedent.should_have = False` is a real mutation of a detached
    builder, and the same line on a frozen leaf would write to a temporary
    that is thrown away.

    `excluded_periods` is the set of periods the rule does not apply to. It
    takes `Period` handles and `PeriodId`s interchangeably, and `to_data()`
    fills it with ids. Two values naming the same periods, one by handles and
    one by ids, do not compare equal — a handle and an id hash differently.

    `soft` says whether the rule is an objective for the solver to optimize
    rather than a constraint it must enforce. `False` is the strict spelling,
    the one the application itself starts a new rule with.

    A rule whose two ends name the same subject is meaningless — an
    implication from a subject to itself — and it is refused when the value
    is used, by the model's own constructor, whose message is the one a
    script meets.
    """

    antecedent: PairingRuleSideData
    consequent: PairingRuleSideData
    excluded_periods: set[Period | PeriodId] = field(default_factory=set)
    soft: bool = False


@dataclass
class SlotPairingRuleSideData:
    """One end of a slot pairing rule, detached from the document.

    `rule.antecedent.to_data()` and `rule.consequent.to_data()` hand one
    back, and a `SlotPairingRuleData` holds one of each:

        clm.SlotPairingRuleSideData(first_slot)
        clm.SlotPairingRuleSideData(first_slot, should_have=False)

    The same card as `PairingRuleSideData`, with a slot in place of a subject:
    the rule is an implication between two slots, and this end says which slot
    it is about and whether a week marked for the rule has that slot used.

    `slot` takes a `Slot` handle or a `SlotId`, and `to_data()` fills it with
    an id — two sides naming the same slot in the two spellings do not compare
    equal, since a handle and an id hash differently. `should_have` defaults
    to `True`, the spelling the application itself starts a new rule with.
    """

    slot: Slot | SlotId
    should_have: bool = True


@dataclass
class SlotPairingRuleData:
    """A slot pairing rule, detached from the document.

    `doc.slot_pairings[...].to_data()` hands one back, and
    `doc.slot_pairings.add` and `doc.slot_pairings.update` take one:

        clm.SlotPairingRuleData(clm.SlotPairingRuleSideData(first_slot),
                                clm.SlotPairingRuleSideData(second_slot))

    The rule is the slots' version of a pairing rule: if the `antecedent`
    slot is used in a week, the `consequent` slot must also be used — or not.

    `excluded_periods` and `soft` behave exactly as they do on a
    `PairingRuleData`, and for the same reasons.

    A rule whose two ends name the same slot is meaningless — an implication
    from a slot to itself — and it is refused when the value is used, by the
    model's own constructor, whose message is the one a script meets. That
    both slots belong to the same subject is a statement about the document,
    and it is the write that refuses it, not this class.
    """

    antecedent: SlotPairingRuleSideData
    consequent: SlotPairingRuleSideData
    excluded_periods: set[Period | PeriodId] = field(default_factory=set)
    soft: bool = False


def _objective_rotation() -> Enforcement:
    """The rotation goal a new `BalancingData` comes with.

    A function rather than a value, because there is no value to write here:
    `Enforcement` is one of the rust classes, and this file is compiled while
    `collomatique` is still initializing, so nothing of it can be imported at
    that moment. A `default_factory` runs when a value is built instead, which
    is long after the module is whole.
    """

    import collomatique

    return collomatique.Enforcement.OBJECTIVE


@dataclass
class LimitsData:
    """The limits a student's interrogation schedule is held to, detached.

    `doc.settings.global_limits.to_data()` and the `Limits` sub-views hand one
    back, and `doc.settings.set_global_limits` and
    `doc.settings.set_student_limits` take one:

        clm.LimitsData(
            interrogations_per_week_min=clm.Limit(2, clm.Enforcement.STRICT))

    An entry is a whole: a field left at `None` does not mean "inherit" — it
    **disables** the corresponding limit of the entry the student inherits
    from. That is the model's whole-entry rule, and it stays in the model:
    this value is dumb, and only the write that reads it back decides.

    Each field is a `Limit` or `None` — a count with the `Enforcement` that
    says whether it is an objective for the solver or a hard constraint:

        clm.Limit(3, clm.Enforcement.STRICT)

    `interrogations_per_week_min` and `interrogations_per_week_max` take any
    count, zero included — the model types them that way. A
    `max_interrogations_per_day` of zero is refused when the value is used,
    since a day in which no interrogation may happen at all is not a limit.

    Every field defaults to `None`, which is the model's own default: an empty
    entry disables every limit, and it is what a document with no limits set
    holds.
    """

    interrogations_per_week_min: Limit | None = None
    interrogations_per_week_max: Limit | None = None
    max_interrogations_per_day: Limit | None = None


@dataclass
class BalancingData:
    """The balancing goals one subject's colles are scheduled under, detached.

    `doc.balancing.global_options.to_data()` and the `BalancingOptions`
    sub-views hand one back, and `doc.balancing.set_global` and
    `doc.balancing.set_subject` take one:

        clm.BalancingData(
            teacher_rotation=clm.Enforcement.OBJECTIVE,
            year_teacher_rotation=True)

    Like a `LimitsData`, an entry is a whole: a rotation goal left at `None`
    is **not pursued** — it neither constrains the solver nor weighs in its
    objective — and it never inherits the goal of the entry the subject
    inherits from. That is the model's whole-entry rule, and it stays in the
    model.

    The three rotation goals are each an `Enforcement` or `None` — the
    `OBJECTIVE` spelling optimizes for the goal, `STRICT` makes it a hard
    constraint:

        clm.Enforcement.OBJECTIVE   # optimize for it
        clm.Enforcement.STRICT      # a constraint

    `year_teacher_rotation` and `period_teacher_rotation` are whether each
    teacher is asked to see the same number of interrogations, over the whole
    year and within each period.

    The defaults are the model's own: teacher rotation pursued as an objective
    and nothing else.
    """

    teacher_rotation: Enforcement | None = field(default_factory=_objective_rotation)
    slot_rotation: Enforcement | None = None
    avoid_twice_in_a_row: Enforcement | None = None
    year_teacher_rotation: bool = False
    period_teacher_rotation: bool = False


def _white() -> Color:
    """The background color a new `ExportGlobalConfigData` comes with.

    A function rather than a value, because there is no value to write here:
    `Color` is one of the rust classes, and this file is compiled while
    `collomatique` is still initializing, so nothing of it can be imported at
    that moment. A `default_factory` runs when a value is built instead, which
    is long after the module is whole.
    """

    import collomatique

    return collomatique.Color(255, 255, 255)


def _stripes_color() -> Color:
    """The stripes color a new `ExportGlobalConfigData` comes with.

    The same shape as `_white`, for the same reason: `Color` is one of the
    rust classes, and this file is compiled while `collomatique` is still
    initializing.
    """

    import collomatique

    return collomatique.Color(220, 220, 230)


@dataclass
class ExportGlobalConfigData:
    """The settings shared by every sheet of the export, detached.

    `doc.export_config.global_config.to_data()` hands one back, and
    `doc.export_config.set_global` takes one:

        clm.ExportGlobalConfigData(stripes_color=clm.Color(240, 240, 245))

    `background_color` is the color every sheet is painted over, and
    `stripes_color` the tint of the alternating stripes the rows are drawn
    with; `stripes_color_enabled` says whether the stripes are drawn at all.
    The three take the model's own defaults, so `clm.ExportGlobalConfigData()`
    is exactly what a document holds when nothing was ever changed.
    """

    background_color: Color = field(default_factory=_white)
    stripes_color_enabled: bool = True
    stripes_color: Color = field(default_factory=_stripes_color)


def _landscape() -> Orientation:
    """The orientation a new `ExportColloscopeConfigData` comes with.

    A function rather than a value, because there is no value to write here:
    `Orientation` is one of the rust classes, and this file is compiled while
    `collomatique` is still initializing, so nothing of it can be imported at
    that moment. A `default_factory` runs when a value is built instead, which
    is long after the module is whole.
    """

    import collomatique

    return collomatique.Orientation.LANDSCAPE


def _no_interrogation_color() -> Color:
    """The empty-cell color a new `ExportColloscopeConfigData` comes with.

    The same shape as `_white`, for the same reason: `Color` is one of the
    rust classes, and this file is compiled while `collomatique` is still
    initializing.
    """

    import collomatique

    return collomatique.Color(140, 140, 140)


def _annotation_color() -> Color:
    """The annotation tint a new `ExportColloscopeConfigData` comes with.

    The same shape as `_white`, for the same reason: `Color` is one of the
    rust classes, and this file is compiled while `collomatique` is still
    initializing.
    """

    import collomatique

    return collomatique.Color(255, 255, 0)


@dataclass
class ExportColloscopeConfigData:
    """The settings of the colloscope sheet, detached.

    `doc.export_config.colloscope_config.to_data()` hands one back, and
    `doc.export_config.set_colloscope_config` takes one:

        clm.ExportColloscopeConfigData(
            sheet_name="Colles", no_interrogation_color=clm.Color(200, 200, 200))

    Every field takes the model's own default, so
    `clm.ExportColloscopeConfigData()` is exactly what a document holds when
    nothing was ever changed.

    `sheet_name` names the sheet in the workbook, and `orientation` whether it
    is printed tall or wide. `extra_info_column_enabled`, `teacher_email_enabled`
    and `teacher_tel_enabled` say whether the three optional columns are
    written, each with the `*_name` (or `teacher_*`) field beside it holding
    the column's heading — the empty string is a heading a script chose, so it
    is allowed, the way the model stores it.

    `display_week_dates` and `display_annotations` say whether the week dates
    and the weeks' annotations are written. `no_interrogation_color` is the
    paint of a cell that holds no interrogation, `annotation_color_enabled`
    and `annotation_color` the tint of the annotation cells.

    `extra_colors` is the map of extra cell colors, keyed by the label that
    names them — a plain dict, because a value is written as well as read:
    `d.extra_colors["Vacances"] = clm.Color(255, 240, 200)` is a real
    mutation of a detached builder, where the read surface's `mappingproxy`
    refuses it.
    """

    sheet_name: str = "Colloscope"
    extra_info_column_enabled: bool = True
    extra_info_column_name: str = "Info"
    teacher_email_enabled: bool = True
    teacher_email: str = "Contact"
    teacher_tel_enabled: bool = False
    teacher_tel: str = ""
    orientation: Orientation = field(default_factory=_landscape)
    display_week_dates: bool = True
    display_annotations: bool = True
    no_interrogation_color: Color = field(default_factory=_no_interrogation_color)
    annotation_color_enabled: bool = True
    annotation_color: Color = field(default_factory=_annotation_color)
    extra_colors: dict[str, Color] = field(default_factory=dict)


@dataclass
class ExportStudentGroupsConfigData:
    """The settings of one per-student-groups sheet, detached.

    `doc.export_config.all_groups_config.to_data()` and the two sibling
    views hand one back, and `doc.export_config.set_all_groups_config` and
    its two siblings take one.

    The model has no one default for this shape: each of the three sheets is
    born through its own constructor, and the dataclass mirrors them as three
    classmethods:

        clm.ExportStudentGroupsConfigData.all_groups()
        clm.ExportStudentGroupsConfigData.automatic_groups()
        clm.ExportStudentGroupsConfigData.prefilled_groups()

    `sheet_name` names the sheet in the workbook, and it is required: it is
    the one field that says *which* sheet a value is for, and the classmethods
    above are how the application's own defaults are spelled. Which sheet a
    value is *written* to is the setter it is handed to and never this field:
    `set_automatic_groups_config` writes the automatic-groups sheet whatever
    name the value carries.

    `orientation` is `None` when the sheet's orientation is auto-detected from
    the group count when the export is written — the model's own rule, so
    `None` is a value here, never an absence. `show_emails` and `show_tel`
    say whether the two contact columns are written.
    """

    sheet_name: str
    orientation: Orientation | None = None
    show_emails: bool = True
    show_tel: bool = False

    @classmethod
    def all_groups(cls) -> ExportStudentGroupsConfigData:
        """The sheet for every group of the document, as the model defaults it"""

        return cls("Tous les groupes")

    @classmethod
    def automatic_groups(cls) -> ExportStudentGroupsConfigData:
        """The sheet for the automatic groups, as the model defaults it"""

        return cls("Groupes automatiques")

    @classmethod
    def prefilled_groups(cls) -> ExportStudentGroupsConfigData:
        """The sheet for the prefilled groups, as the model defaults it"""

        return cls("Groupes préremplis")


def _portrait() -> Orientation:
    """The orientation a new `ExportGroupListConfigData` comes with.

    A function rather than a value, because there is no value to write here:
    `Orientation` is one of the rust classes, and this file is compiled while
    `collomatique` is still initializing, so nothing of it can be imported at
    that moment. A `default_factory` runs when a value is built instead, which
    is long after the module is whole.
    """

    import collomatique

    return collomatique.Orientation.PORTRAIT


@dataclass
class ExportGroupListConfigData:
    """The settings of the per-group-list sheets, detached.

    `doc.export_config.per_group_list_config.to_data()` hands one back, and
    `doc.export_config.set_per_group_list_config` takes one:

        clm.ExportGroupListConfigData(center_vertically=True)

    Every field takes the model's own default, so
    `clm.ExportGroupListConfigData()` is exactly what a document holds when
    nothing was ever changed. `orientation` is whether the sheets are printed
    tall or wide, `show_emails` and `show_tel` whether the two contact columns
    are written, and `center_vertically` whether the sheet is centered on the
    page.
    """

    orientation: Orientation = field(default_factory=_portrait)
    show_emails: bool = True
    show_tel: bool = False
    center_vertically: bool = False


@dataclass
class ExportConfigData:
    """The whole export configuration, detached.

    `doc.export_config.to_data()` hands one back, and the coarse door takes
    one: a `DocumentData` holds one of these, so `doc.replace_all` writes a
    whole export configuration at once. No export op takes it — the eleven
    mutators each patch one field of the document's own configuration.
    `doc.export_xlsx(path, config)` takes one as well, and stores nothing: it
    says how to write that one workbook, and the document keeps its own.

    The tree mirrors the model's own shape: the settings shared by every sheet
    in `global_config`, then the five switches that say which sheets are part
    of the export at all — sitting *beside* the configs they gate, never
    inside them, which is the model's memory of what was chosen before a
    section was switched off — then the four per-sheet configs.

    Every field takes the model's own default, so `clm.ExportConfigData()` is
    exactly what `clm.new_document()` holds.
    """

    global_config: ExportGlobalConfigData = field(default_factory=ExportGlobalConfigData)
    colloscope_enabled: bool = True
    all_groups_enabled: bool = True
    automatic_groups_enabled: bool = False
    prefilled_groups_enabled: bool = False
    per_group_list_enabled: bool = True
    colloscope_config: ExportColloscopeConfigData = field(
        default_factory=ExportColloscopeConfigData)
    all_groups_config: ExportStudentGroupsConfigData = field(
        default_factory=ExportStudentGroupsConfigData.all_groups)
    automatic_groups_config: ExportStudentGroupsConfigData = field(
        default_factory=ExportStudentGroupsConfigData.automatic_groups)
    prefilled_groups_config: ExportStudentGroupsConfigData = field(
        default_factory=ExportStudentGroupsConfigData.prefilled_groups)
    per_group_list_config: ExportGroupListConfigData = field(
        default_factory=ExportGroupListConfigData)


@dataclass
class ColloscopeData:
    """The whole colloscope, detached.

    `doc.colloscope.to_data()` hands one back and `doc.colloscope.install`
    takes one whole; `doc.colloscope.set_interrogation` and
    `doc.colloscope.set_group_list` are the row-by-row doors, one row each:

        clm.ColloscopeData(
            interrogations={(first_slot, first_week): {0, 2}},
            group_lists={automatic: {harry: 0, hermione: 2}})

    The result of a resolution, in the two sparse tables the model stores:
    `interrogations` says which group numbers sit in which `(slot, week)`
    cell, and `group_lists` says how each automatic group list was filled,
    student by student. The group numbers in the first are indices into the
    group list the cell's subject uses on that week's period — the same
    numbers the read surface hands out, so a value and a handle agree. A
    prefilled list never appears in the second: it has groups of its own.

    The keys of both tables name entities, so they take handles and ids
    interchangeably, in any mix, like every other place in this API;
    `to_data()` fills them with ids, so that a value carries no document
    around with it. Two values naming the same cells, one by handles and one
    by ids, do not compare equal — a handle and an id hash differently. For
    the same reason a mapping can name one cell, one list or one student
    twice, once in each spelling; that is refused when the value is used,
    rather than quietly keeping the last entry.

    A hand-built value need not be canonical: an empty group set or an empty
    placement map just means "no row", which is what the payload promises
    its callers.
    """

    interrogations: dict[tuple[Slot | SlotId, Week | WeekId], set[int]] = field(
        default_factory=dict)
    group_lists: dict[GroupList | GroupListId, dict[Student | StudentId, int]] = field(
        default_factory=dict)


@dataclass
class WeekData:
    """One week of the document, detached.

    `doc.weeks[...].to_data()` hands one back:

        clm.WeekData(first_period)
        clm.WeekData(first_period, interrogations=False,
                     annotation="Rentrée")

    `period` is the period this week belongs to, and it is authoritative in
    the model: a week is filed under its period in the list that gives it its
    position. It takes a `Period` handle or a `PeriodId`, like every other
    place in this API that names an entity; `to_data()` fills it with an id,
    so that a value carries no document around with it. Two values naming the
    same period, one by handle and one by id, do not compare equal — a handle
    and an id hash differently.

    `interrogations` is whether colles happen on this week at all. It
    defaults to `True`, which is the model's own default — a week that is
    added is a week that runs colles.

    `annotation` is the week's label — « Rentrée », « Vacances » — or
    `None`. Absent is `None`, never `""`: the model types this field as an
    optional non-empty string, so the empty one is refused when the value
    is used.

    No week op takes a `WeekData`: the two week writes carry one field
    each, as `doc.weeks.set_status(week, active)` and
    `.set_annotation(week, text)`. This class exists so that `week.to_data()`
    has a detached shape to hand back and `doc.snapshot()` can hold a whole
    document.

    The handle's `.index` and `.monday` are not fields: both are derived —
    the index from the week's place in the walk, the monday from the index
    and the document's start date. A value that stored them could contradict
    itself.
    """

    period: Period | PeriodId
    interrogations: bool = True
    annotation: str | None = None


@dataclass
class DocumentData:
    """The whole document, detached.

    `doc.snapshot()` hands one back — the same conversion `to_data()` is, run
    over everything at once:

        tree = doc.snapshot()   # DocumentData

    The tree mirrors the document section by section: `params` in the first
    eighteen fields, then the colloscope and the export configuration. The
    entity sections are dicts keyed by id, so the order of a section is the
    order of its dict — python has preserved insertion order since 3.7, and
    that is what carries the document's user orders: `subjects` in the order
    `doc.subjects` shows them, `slots` in the `doc.slots` walk order, `weeks`
    in global week order. `periods` is a plain list, because a period has
    nothing but its identity and its place.

    Every field is defaulted to an empty document, so `clm.DocumentData()` is
    exactly what `clm.new_document()` holds.

    The keys and the entity references inside name entities, so they take
    handles and ids interchangeably, like every other place in this API;
    `snapshot()` always fills them with ids, so that a tree carries no
    document around with it. Two trees naming the same entities, one by
    handles and one by ids, do not compare equal — a handle and an id hash
    differently. For the same reason a section can name one entity twice,
    once in each spelling; that is refused when the tree is used, rather
    than quietly keeping the last entry.

    The two junction tables hold the stored rows only: an absent row is
    simply not there, exactly as the model stores it.

    The coarse door's `doc.replace_all(tree, label)` takes one of these back,
    as a single undo step. A tree can rename, delete and rewire, but it cannot
    add: every id in it has to name an entity the document already holds, and
    ids have no constructor. A script that wants one section still calls the
    handle's own `to_data()`.
    """

    first_week: datetime.date | None = None
    periods: list[PeriodId] = field(default_factory=list)
    weeks: dict[WeekId, WeekData] = field(default_factory=dict)
    subjects: dict[SubjectId, SubjectData] = field(default_factory=dict)
    teachers: dict[TeacherId, TeacherData] = field(default_factory=dict)
    students: dict[StudentId, StudentData] = field(default_factory=dict)
    assignments: dict[tuple[PeriodId, SubjectId], set[StudentId]] = field(
        default_factory=dict)
    week_patterns: dict[WeekPatternId, WeekPatternData] = field(default_factory=dict)
    slots: dict[SlotId, SlotData] = field(default_factory=dict)
    incompats: dict[IncompatId, IncompatData] = field(default_factory=dict)
    group_lists: dict[GroupListId, GroupListData] = field(default_factory=dict)
    group_list_associations: dict[tuple[PeriodId, SubjectId], GroupListId] = field(
        default_factory=dict)
    pairings: dict[PairingRuleId, PairingRuleData] = field(default_factory=dict)
    slot_pairings: dict[SlotPairingRuleId, SlotPairingRuleData] = field(
        default_factory=dict)
    global_limits: LimitsData = field(default_factory=LimitsData)
    student_limits: dict[StudentId, LimitsData] = field(default_factory=dict)
    global_balancing: BalancingData = field(default_factory=BalancingData)
    subject_balancing: dict[SubjectId, BalancingData] = field(default_factory=dict)
    colloscope: ColloscopeData = field(default_factory=ColloscopeData)
    export_config: ExportConfigData = field(default_factory=ExportConfigData)


@dataclass
class PeriodSolveConfig:
    """What one period is to a solve.

    A `ColloscopeSolveConfig` holds one of these per period it says something
    about. A period it does not name is recomputed from scratch, which is what
    both fields default to.

    `recompute` is whether the solver works out this period's interrogations
    again. A period that is not recomputed keeps exactly what the document
    holds there: those values are handed to the solver pinned.

    `use_current_values` says what the values already there are worth, and it
    means something on both sides of `recompute`. On a period that *is*
    recomputed, the solve is anchored softly to them, so the result stays as
    close to the current colloscope as the rest of the problem allows. On a
    period that is not, it says the pinned values may be relied upon: the
    constraints that reach across the pin survive — priced as soft penalties
    when `objectify_cross_fixed_period` names a weight, kept hard when it is
    `None` — instead of being dropped, which is what happens without it.

    All four combinations therefore mean something, which is why this is two
    independent booleans rather than one switch.
    """

    recompute: bool = True
    use_current_values: bool = False


@dataclass
class GroupListSolveConfig:
    """What one automatic group list is to a solve.

    A `ColloscopeSolveConfig` holds one of these per group list it says
    something about. A list it does not name is recomputed freely, which is
    what both fields default to.

    Only an automatic list belongs here. A prefilled list has no groups to
    work out — somebody wrote them — so naming one is refused when the config
    is used, rather than quietly ignored.

    `recompute` is whether the solver fills this list again. A list that is
    not recomputed keeps the groups the document holds: they are pinned.

    `previous_values_as_objective` anchors the recomputed groups softly to
    the ones already there, so a student stays where they were unless the
    rest of the problem pays for moving them. It only means something on a
    list that *is* recomputed: `recompute=False` together with
    `previous_values_as_objective=True` is refused when the config is used,
    since there would be nothing for the anchor to hold on to.
    """

    recompute: bool = True
    previous_values_as_objective: bool = False


@dataclass
class ColloscopeSolveConfig:
    """What a solve recomputes, and what it must leave alone.

    `doc.build_colloscope_model(config)` takes one of these, and the model it
    hands back is what the MPS export writes:

        config = clm.ColloscopeSolveConfig()
        model = doc.build_colloscope_model(config)
        model.export_mps("probleme.mps")

    `clm.ColloscopeSolveConfig()` recomputes everything from scratch, and
    that is what an empty `periods` and an empty `group_lists` mean: a period
    or a group list the config does not name gets the default of its own
    class. A config only ever spells out what departs from that.

    `periods` and `group_lists` are keyed by entity, and take handles and ids
    interchangeably, like every other place in this API that names one. A
    mapping that names the same period or the same list twice, once in each
    spelling, is refused when the config is used, rather than quietly keeping
    the last entry.

    `objectify_cross_fixed_period` is what a constraint reaching across a
    pinned period is worth: a weight prices it as a soft penalty, and `None`
    keeps it hard — which drops it wherever the pin makes it unsatisfiable.
    `l1_anchor_weight` is what one step away from an anchored current value
    costs, for the anchors `use_current_values` and
    `previous_values_as_objective` ask for.

    Both weights are the model's own defaults. A weight that is negative or
    not finite is refused when the config is used; zero is allowed, and means
    the term is written and prices nothing.

    A config is not stored on the document. It is an argument, and every
    build takes its own.
    """

    periods: dict[Period | PeriodId, PeriodSolveConfig] = field(default_factory=dict)
    group_lists: dict[GroupList | GroupListId, GroupListSolveConfig] = field(
        default_factory=dict)
    objectify_cross_fixed_period: float | None = 1000.0
    l1_anchor_weight: float = 1000.0


@dataclass
class DefaultConfig:
    """How the complete solve is run.

    The full branch-and-bound: it looks for the best colloscope it can find,
    and it is the only substrategy that can prove there is no better one.
    `ConductorStrategy(default_config=DefaultConfig())` is what enables it.

    Both limits are counted in whole seconds. `time_limit` bounds the solve as
    a whole; `incumbent_time_limit` bounds it from the moment it first holds a
    colloscope, so a solve that finds one quickly and then grinds gives up
    early. They are independent, and the solve ends at whichever deadline
    comes first.

    `None` is how "no limit" is said. `0` is not: it is refused when the
    strategy is used, so that a limit and the absence of one never look alike.
    """

    time_limit: int | None = None
    incumbent_time_limit: int | None = None


@dataclass
class WarmStartConfig:
    """How the warm-start solve is run.

    A first, cheap solve that looks only for *a* colloscope and not for a good
    one; what it finds is handed to the solves that do optimise, as a place to
    start. This is the one substrategy `ConductorStrategy()` enables on its
    own.

    `time_limit` bounds that search, in whole seconds, `None` for no limit and
    `0` refused.
    """

    time_limit: int | None = None


@dataclass
class IncrementalConfig:
    """How the incremental solve is run.

    The other way to a first colloscope: the problem is solved in epochs, each
    one anchored to what the epoch before it decided, which usually reaches a
    better starting point than a warm start does. The two fill the same role,
    so enabling both is redundant — `ConductorStrategy.optimize()` uses this
    one alone.

    `l1_weight` is what one step away from the previous epoch's decisions
    costs: the larger it is, the stickier those decisions.
    `distance_tolerance` is how close to the best an epoch has to get before
    it may stop; `0.0` asks every epoch to be solved to proven optimality.
    Both refuse a negative or non-finite number when the strategy is used.

    `epoch_time_limit` and `epoch_incumbent_time_limit` bound each epoch, in
    whole seconds, and the clock starts again at every epoch. `None` is no
    limit, `0` is refused.
    """

    l1_weight: float = 1000.0
    distance_tolerance: float = 10.0
    epoch_time_limit: int | None = None
    epoch_incumbent_time_limit: int | None = 60


@dataclass
class FuzzyConfig:
    """How the fuzzy exploration is run.

    What fills the worker slots the complete solve leaves idle: it perturbs
    the colloscope in hand at random, repairs what the perturbation broke, and
    keeps the result if it is better. It needs a colloscope to start from, so
    it only means something next to a substrategy that produces one.

    `fuzzy_sigma` is how hard each perturbation pushes — the default nudges
    about one binary variable in eighty. `find_closest_tolerance` is how close
    to the perturbed point the repair has to land before it may stop. Both
    refuse a negative or non-finite number when the strategy is used.

    `time_limit` and `incumbent_time_limit` bound each repair, in whole
    seconds, `None` for no limit and `0` refused.
    """

    fuzzy_sigma: float = 0.2
    find_closest_tolerance: float = 10.0
    time_limit: int | None = None
    incumbent_time_limit: int | None = None


@dataclass
class ConductorStrategy:
    """How a solve is run: which substrategies, on how many worker slots.

    `model.solve(strategy)` takes one of these:

        strategy = clm.ConductorStrategy.optimize()
        run = model.solve(strategy)
        outcome = run.wait()

    Each `*_config` field both enables its substrategy and tunes it: `None`
    disables it, an object enables it. `worker_count` is how many of them may
    run at once; it counts from 1, and `0` is refused when the strategy is
    used.

    `ConductorStrategy()` is the application's « Recherche simple » — one
    worker, warm-start only, which finds a colloscope and stops there. The two
    classmethods below are the application's own presets.

    `warm_start_incumbent` says what the run does with a ready-made solution
    handed to it as a warm start: `True` evaluates it against the model and
    adopts it as the initial incumbent if it holds up, `False` leaves it its
    weaker role of a hint each worker starts from. A solve run from here hands
    no warm start over, so the field changes nothing today; it is here because
    a strategy built in a script is the application's structure, whole.

    A strategy is not stored on the model. It is an argument, and every solve
    takes its own.
    """

    worker_count: int = 1
    default_config: DefaultConfig | None = None
    warm_start_config: WarmStartConfig | None = field(default_factory=WarmStartConfig)
    incremental_config: IncrementalConfig | None = None
    fuzzy_config: FuzzyConfig | None = None
    warm_start_incumbent: bool = True

    @classmethod
    def search(cls) -> ConductorStrategy:
        """The « Recherche simple » preset: find a colloscope, fast.

        Built on the rust side, out of the structure the application itself
        opens its dialog with, and handed back as a plain `ConductorStrategy`
        a script may then edit. So this cannot drift from the application: it
        *is* the application's preset.
        """
        import collomatique

        return collomatique._conductor_search()

    @classmethod
    def optimize(cls) -> ConductorStrategy:
        """The « Optimisation complète » preset, sized to this machine.

        The application's other preset, built the same way — everything on
        except the warm start, which the incremental solve replaces. Its
        worker count follows the number of cores this machine reports, so it
        is not the same number everywhere.
        """
        import collomatique

        return collomatique._conductor_optimize()

    def warnings(self) -> tuple[ConductorWarning, ...]:
        """What is wrong with this strategy, seen before anything runs.

        A tuple of `ConductorWarning` members, empty when there is nothing to
        say, in a fixed order. These are the very remarks the application's
        solve dialog shows, and `str()` of one is the sentence it shows:

            for w in strategy.warnings():
                print(w)

        None of them stops a solve. A strategy that warns still runs — a
        warning says the setup wastes work or never proves it is done, and
        what to do about that is the script's call.
        """
        import collomatique

        return collomatique._conductor_warnings(self)


@dataclass
class GroupListsGenerationRequest:
    """What to generate: which subjects get a new group list, and what the
    generator must respect while it builds them.

    `doc.generate_group_lists(request)` takes one, and
    `doc.default_generation_request()` hands back the very selection the
    application's own generation dialog opens with:

        req = doc.default_generation_request()
        req.rebuild = {(period, maths) for period in doc.periods}
        result = doc.generate_group_lists(req)

    `rebuild` is a set of `(period, subject)` pairs, each of which gets a
    freshly built list, associated to that pair. Two pairs whose students and
    group-size range are the same share *one* list rather than getting two
    alike — so the generation produces at most as many lists as there are
    pairs, and often fewer.

    `kept_lists` is a set of group lists the generator reads as fixed: the
    partners they already put together count in what it is trying to
    maximize, and nothing about them is rewritten. They must be prefilled
    lists, since an automatic one has no groups yet to respect.

    Every reference here is an entity of the document the request is handed
    to, a handle or an id indifferently, like everywhere else in this API.

    The value is dumb, like every other: what a request cannot ask for —
    a subject that runs no interrogations, a kept list that is not prefilled,
    a class the group sizes cannot split — is decided when it is used, and
    `doc.generate_group_lists` is where those refusals are written down.
    """

    rebuild: set[tuple[Period | PeriodId, Subject | SubjectId]] = field(
        default_factory=set
    )
    kept_lists: set[GroupList | GroupListId] = field(default_factory=set)
