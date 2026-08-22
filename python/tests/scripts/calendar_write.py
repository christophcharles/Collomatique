import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the labels are the
# french names `ops` gives the operations this script makes — handed in from
# rust so that this pins the operations' own labels and not merely some
# strings. `colle_label` and `exclude_label` are the two writes of other
# families the script makes, and they carry those families' names rather than
# this one's.
doc = collomatique.load(source)
periods = doc.periods
weeks = doc.weeks

period_count = len(periods)
week_count = len(weeks)
first, second, third = list(periods)
assert period_count == 3

second_weeks = list(second.weeks)
third_weeks = list(third.weeks)

# ------------------------------------------------------------------- the setup


def writable_cell(candidates):
    """A `(slot, week)` a colle can really be written on, over `candidates`.

    A cell needs three things at once: the model must allow an interrogation
    there, the slot's subject must use a group list on that week's period, and
    that list must hold a group to name.
    """

    for week in candidates:
        for slot in doc.slots:
            if not doc.is_interrogation_possible(slot, week):
                continue
            group_list = doc.group_lists.association_for(week.period, slot.subject)
            if group_list is not None and group_list.group_count > 0:
                return slot, week
    raise AssertionError("the example holds a cell a colle can be written on")


def pattern_skipping(week):
    """The one week pattern that leaves `week` out.

    The example's two patterns share every week between them, so there is
    exactly one — which is what makes every week removed below free exactly one
    exclusion.
    """

    skipping = [p for p in doc.week_patterns if week in p.excluded_weeks]
    assert len(skipping) == 1
    return skipping[0]


# The subject the removal cascade finds excluding a period: one the example
# associates no group list with, so taking it off a period drops its enrolment
# row and nothing else.
spare = next(
    s for s in doc.subjects if doc.group_lists.association_for(third, s) is None
)
spare_row = doc.assignments[third, spare]
assert spare_row

excluded = doc.subjects.set_period_status(spare, third, False)
assert [w.kind for w in excluded.warnings] == ["ClearAssignmentRow"]
assert spare.excluded_periods == frozenset({third})

# The example holds no colle at all, so the cascades below need three, written
# through the surface piece 13 published: one on a week whose colles this script
# switches off, one on a week the cut hands over to another period, and one on a
# week a shrink drops.
cell_slot, cell_week = writable_cell(second.weeks)
tail_slot, tail_week = writable_cell(w for w in second_weeks[6:] if w != cell_week)
doomed_slot, doomed_week = writable_cell(reversed(third.weeks))
assert len({cell_week, tail_week, doomed_week}) == 3

for slot, week in (
    (cell_slot, cell_week),
    (tail_slot, tail_week),
    (doomed_slot, doomed_week),
):
    assert doc.colloscope.set_interrogation(slot, week, {0}).warnings == []
    assert doc.colloscope.interrogation(slot, week) == frozenset({0})

# ------------------------------------------------------- the periods that grow

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = periods.add(3)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)
# Nothing can name a period that does not exist yet, and its weeks are as new as
# it is, so there is nothing for the cascade to repair.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
fresh = added.created
assert isinstance(fresh, collomatique.Period)
assert isinstance(fresh.id, collomatique.PeriodId)
assert fresh in periods
assert periods[fresh.id] == fresh
assert len(periods) == period_count + 1
# A period is added last, which is the only place one can be added: the list is
# the year in order.
assert fresh.index == period_count
assert repr(added).startswith("AddResult(created=<Period #")
assert "warnings=[]" in repr(added)

# The weeks are created with it, all of them holding colles and none annotated.
fresh_weeks = list(fresh.weeks)
assert len(fresh_weeks) == 3
assert all(week.interrogations is True for week in fresh_weeks)
assert all(week.annotation is None for week in fresh_weeks)
assert len(weeks) == week_count + 3
# They land at the end of the global week order, where their period is.
assert [week.index for week in fresh_weeks] == [
    week_count,
    week_count + 1,
    week_count + 2,
]

# A period with no week is the model's canonical empty period, not a refusal.
empty = periods.add(0).created
assert empty.weeks == ()
assert empty.index == period_count + 1
assert len(weeks) == week_count + 3

# ------------------------------------------------------ what a week says of it

last = fresh_weeks[2]
annotated = weeks.set_annotation(last, "Vacances")
assert isinstance(annotated, collomatique.OpResult)
assert not isinstance(annotated, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(annotated, "created")
# Nothing in the document reads an annotation, so this repairs nothing.
assert annotated.warnings == []
assert last.annotation == "Vacances"

# `None` clears it, and the week says nothing again — the `""` a script might
# have reached for is not a spelling of that, and is refused below.
assert weeks.set_annotation(last, None).warnings == []
assert last.annotation is None
weeks.set_annotation(last, "Vacances")

# Switching the colles off leaves the week where it is: it still exists, still
# counts in the order, and still carries what it says.
off = weeks.set_status(last, False)
assert off.warnings == []
assert last.interrogations is False
assert last.annotation == "Vacances"
assert last.index == week_count + 2

# ------------------------------------------------------- the periods that move

# Growing repeats the last week, annotation and all: that is what the
# application's own week-count spinner produces.
grown = periods.set_week_count(fresh, 5)
assert grown.warnings == []
fresh_weeks = list(fresh.weeks)
assert len(fresh_weeks) == 5
assert [week.annotation for week in fresh_weeks[3:]] == ["Vacances", "Vacances"]
assert all(week.interrogations is False for week in fresh_weeks[3:])
# The front of the period never moved.
assert [week.annotation for week in fresh_weeks[:3]] == [None, None, "Vacances"]
assert len(weeks) == week_count + 5

# Shrinking drops the weeks off the end, and nothing else. Nothing named these
# two — they were made a moment ago — so there is nothing to repair either.
shrunk = periods.set_week_count(fresh, 3)
assert shrunk.warnings == []
assert len(fresh.weeks) == 3
assert list(fresh.weeks) == fresh_weeks[:3]
assert len(weeks) == week_count + 3

for gone in fresh_weeks[3:]:
    try:
        gone.annotation
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("reading a dropped week must raise")

# Asking for the count the period already has writes nothing new — and is still
# a write, and still its own undo slot.
assert periods.set_week_count(fresh, 3).warnings == []
assert list(fresh.weeks) == fresh_weeks[:3]

# -------------------------------------------------------- the colles of a week

# Switching the colles off on a week takes the ones already written there: they
# cannot stand on a week that holds none.
cleared = weeks.set_status(cell_week, False)
assert [w.kind for w in cleared.warnings] == ["ClearInterrogationCell"]
assert cleared.warnings[0].details == {"slot": cell_slot.id, "week": cell_week.id}
# The write asked for the week itself, so no repair hangs off another.
assert cleared.warnings[0].parent is None
assert str(cleared.warnings[0])
assert cell_week.interrogations is False
assert doc.colloscope.interrogation(cell_slot, cell_week) is None

# Switching them back on only ever widens what the document allows, so there is
# nothing to repair — and nothing comes back: the colle is gone, and the week
# holding colles again does not remember it. Undo does.
back = weeks.set_status(cell_week, True)
assert back.warnings == []
assert cell_week.interrogations is True
assert doc.colloscope.interrogation(cell_slot, cell_week) is None

doc.undo()
doc.undo()
assert cell_week.interrogations is True
assert doc.colloscope.interrogation(cell_slot, cell_week) == frozenset({0})

# ------------------------------------------------------- the weeks that are lost

# A shrink of a period that has been lived in: the weeks off its end go, and
# they take what stood on them. The colle sits on the first of them, which the
# removals — running from the end backwards — reach last.
kept = third_weeks.index(doomed_week)
dropped = third_weeks[kept:]
assert len(dropped) > 1

expected = []
for week in reversed(dropped):
    expected.append(
        (
            "RemoveWeekPatternExclusion",
            {"pattern": pattern_skipping(week).id, "week": week.id},
        )
    )
    if week == doomed_week:
        expected.append(
            ("ClearInterrogationCell", {"slot": doomed_slot.id, "week": week.id})
        )

lost = periods.set_week_count(third, kept)
assert [(w.kind, w.details) for w in lost.warnings] == expected
assert all(w.parent is None for w in lost.warnings)
assert all(str(w) for w in lost.warnings)

assert list(third.weeks) == third_weeks[:kept]
assert len(weeks) == week_count + 3 - len(dropped)
# The colle went with its week, and asking after it is asking after the week:
# a dead coordinate is refused rather than answered, so what says the cell is
# gone is that the week is.
for gone in dropped:
    assert gone not in weeks

# ------------------------------------------------------------- cut, then merge

order = [week.id for week in weeks]

# Cutting hands the tail weeks to a brand new period, in order and with their
# colles: the tail is given what the cut period held before the first week
# moves, so a colle is as legal at its new coordinate as it was at the old one.
cut = periods.cut(second, 6)
assert isinstance(cut, collomatique.AddResult)
assert cut.warnings == []

tail = cut.created
assert isinstance(tail, collomatique.Period)
# The tail lands right after the period it came out of, so the year keeps its
# order.
assert tail.index == second.index + 1
assert third.index == tail.index + 1
assert len(periods) == period_count + 3
assert list(second.weeks) == second_weeks[:6]
assert list(tail.weeks) == second_weeks[6:]
assert tail_week.period == tail
assert doc.colloscope.interrogation(tail_slot, tail_week) == frozenset({0})
# The weeks changed period and the global order did not: the tail lands right
# where its weeks already were, so nothing a week pattern says changes meaning.
assert [week.id for week in weeks] == order
# What the cut period held, the tail holds too — which is why the moved colles
# are still legal.
assert doc.group_lists.association_for(
    tail, tail_slot.subject
) == doc.group_lists.association_for(second, tail_slot.subject)
assert (
    doc.assignments[tail, tail_slot.subject]
    == doc.assignments[second, tail_slot.subject]
)

# And merging it back into the period it was cut from is the undoing of that:
# the weeks are appended in order, the emptied period goes, and what it was
# holding is dropped in silence, since the surviving period says exactly the
# same thing about every one of the moved weeks.
merged = periods.merge_with_previous(tail)
assert merged.warnings == []
assert tail not in periods
assert len(periods) == period_count + 2
assert list(second.weeks) == second_weeks
assert tail_week.period == second
assert doc.colloscope.interrogation(tail_slot, tail_week) == frozenset({0})

# The first period has nothing before it, and asking anyway is told so rather
# than quietly doing nothing. The case names nothing — there is no other period
# for it to name — and carries the empty tuple rather than nothing at all, so
# that every case reads the same way.
try:
    periods.merge_with_previous(first)
except collomatique.GeneralPlanningError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert str(error)
    assert error.op == "MergeWithPreviousPeriod"
    assert error.case == "NoPreviousPeriodToMergeWith"
    assert error.details == ()
else:
    raise AssertionError("the first period has no previous one to merge with")

# Keeping every week a period has is a legal cut — the tail is then an empty
# period, which is a period like any other. Keeping more is not, and the model
# names both counts.
try:
    periods.cut(first, len(first.weeks) + 1)
except collomatique.GeneralPlanningError as error:
    assert error.op == "CutPeriod"
    assert error.case == "RemainingWeekCountTooBig"
    assert error.details == (len(first.weeks) + 1, len(first.weeks))
else:
    raise AssertionError("a cut cannot keep more weeks than the period holds")
assert len(periods) == period_count + 2

# This is what rust reads back off the disk.
doc.save(target)

# ----------------------------------------------------------------- the removal

# A period is its weeks, so there is no removal that keeps them — and the weeks
# go because the call asked for them, so no warning names one. What the result
# carries is what they broke, and what the period itself was holding: one freed
# pattern exclusion per week, then the sites keyed on the period, in the
# registry's own order — the subject that excluded it, its enrolment rows, and
# the group lists its subjects used there.
kept_weeks = list(third.weeks)
rows = [s for s in doc.subjects if doc.assignments[third, s]]
associations = [
    s for s in doc.subjects if doc.group_lists.association_for(third, s) is not None
]
assert spare not in rows
assert len(rows) == len(list(doc.subjects)) - 1
assert associations

# Read before the write: a warning names the document as it was, and the
# patterns stop skipping these weeks the moment they go.
freed = [(week.id, pattern_skipping(week).id) for week in kept_weeks]

removed = periods.remove_with_weeks(third)
warnings = removed.warnings

assert [w.kind for w in warnings] == (
    ["RemoveWeekPatternExclusion"] * len(kept_weeks)
    + ["RemoveSubjectPeriodExclusion"]
    + ["ClearAssignmentRow"] * len(rows)
    + ["UnassignGroupList"] * len(associations)
)
# The weeks are freed from the end backwards, one pattern exclusion each.
assert [w.details for w in warnings[: len(kept_weeks)]] == [
    {"pattern": pattern, "week": week} for week, pattern in reversed(freed)
]
assert warnings[len(kept_weeks)].details == {"subject": spare.id, "period": third.id}
# The subjects inside each of the last two blocks come in the model's own id
# order, which a script cannot sort by — so what is pinned is the block and what
# it names, not the order within it.
assert {w.details["subject"] for w in warnings if w.kind == "ClearAssignmentRow"} == {
    s.id for s in rows
}
assert {w.details["subject"] for w in warnings if w.kind == "UnassignGroupList"} == {
    s.id for s in associations
}
assert all(w.details["period"] == third.id for w in warnings if "period" in w.details)
assert all(w.parent is None for w in warnings)
assert all(str(w) for w in warnings)

assert third not in periods
assert len(periods) == period_count + 1
assert len(weeks) == week_count + 3 - len(dropped) - len(kept_weeks)
assert spare.excluded_periods == frozenset()

# ------------------------------------------------- what the arguments refuse

# The period is gone and so are its weeks, and a dead one is refused by the
# argument convention rather than by the model: the ops that name a period can
# only object to an id the document does not hold, and that is caught here,
# where the message can say which argument was wrong.
dead_week = kept_weeks[0]
for call in (
    lambda: periods.set_week_count(third, 3),
    lambda: periods.set_week_count(third.id, 3),
    lambda: periods.remove_with_weeks(third),
    lambda: periods.cut(third, 1),
    lambda: periods.merge_with_previous(third),
    lambda: weeks.set_status(dead_week, True),
    lambda: weeks.set_annotation(dead_week, "Vacances"),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead period or week must not be written through")

# A call that is wrong about both names the *week*: a value meant for nothing is
# moot, so the addressee is resolved first.
try:
    weeks.set_annotation(dead_week, "")
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

doc.undo()
assert third in periods
assert list(third.weeks) == kept_weeks
assert doc.assignments[third, rows[0]]
assert doc.group_lists.association_for(third, associations[0]) is not None
assert spare.excluded_periods == frozenset({third})

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything. A week count is an int, not a string, and a period
# cannot be named by one either.
for call in (
    lambda: periods.set_week_count(3, 1),
    lambda: periods.set_week_count(first, "3"),
    lambda: periods.cut("first", 1),
    lambda: periods.add("3"),
    lambda: weeks.set_status(3, True),
    lambda: weeks.set_status(first.weeks[0], 1),
    lambda: weeks.set_annotation(3, None),
    lambda: weeks.set_annotation(first.weeks[0], 3),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("an argument that was never a reference must not resolve")

# A count is a number of weeks, and there is no negative number of them.
try:
    periods.add(-1)
except OverflowError:
    pass
else:
    raise AssertionError("a negative week count must be refused")

# The empty string is not a spelling of « no annotation »: `None` is, and the
# refusal writes nothing.
kept_annotation = first.weeks[0].annotation
try:
    weeks.set_annotation(first.weeks[0], "")
except ValueError:
    pass
else:
    raise AssertionError("an empty annotation must be refused")
assert first.weeks[0].annotation == kept_annotation

# `other` is this same file loaded twice, so its periods and weeks carry the
# very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_period = list(other.periods)[0]
foreign_week = list(other.weeks)[0]

for call in (
    lambda: periods.remove_with_weeks(foreign_period),
    lambda: periods.set_week_count(foreign_period, 2),
    lambda: weeks.set_status(foreign_week, False),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("an entity of another document must not resolve")

assert len(periods) == period_count + 2

# ---------------------------------------------------------------- the undo walk

# Each accepted call was its own undo slot, named by the operation itself, and
# the stack walks all the way back to the document that was opened.
assert doc.undo_name == merge_label
doc.undo()
assert doc.redo_name == merge_label
assert list(second.weeks) == second_weeks[:6]
assert len(periods) == period_count + 3

assert doc.undo_name == cut_label
doc.undo()
assert list(second.weeks) == second_weeks
assert len(periods) == period_count + 2

assert doc.undo_name == week_count_label
doc.undo()
assert list(third.weeks) == third_weeks
assert doc.colloscope.interrogation(doomed_slot, doomed_week) == frozenset({0})

for _ in range(2):
    assert doc.undo_name == week_count_label
    doc.undo()
assert list(fresh.weeks) == fresh_weeks

assert doc.undo_name == week_count_label
doc.undo()
assert list(fresh.weeks) == fresh_weeks[:3]

assert doc.undo_name == status_off_label
doc.undo()
assert last.interrogations is True

assert doc.undo_name == annotate_label
doc.undo()
assert last.annotation is None

assert doc.undo_name == clear_annotation_label
doc.undo()
assert last.annotation == "Vacances"

assert doc.undo_name == annotate_label
doc.undo()
assert last.annotation is None

assert doc.undo_name == add_label
doc.undo()
assert empty not in periods
assert doc.undo_name == add_label
doc.undo()
assert fresh not in periods
assert len(periods) == period_count
assert len(weeks) == week_count

for _ in range(3):
    assert doc.undo_name == colle_label
    doc.undo()
assert doc.colloscope.interrogation(cell_slot, cell_week) is None

# The last slot is the one write of another family left, and it carries that
# family's name: an undo slot is named by the operation that filled it.
assert doc.undo_name == exclude_label
doc.undo()
assert spare.excluded_periods == frozenset()
assert doc.assignments[third, spare] == spare_row
assert doc.can_undo is False
