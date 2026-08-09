import collomatique

# The second half: the last period and its weeks are gone, and every handle
# stage one kept is now naming something the document no longer holds.
assert len(doc.periods) == period_count_before - 1
assert len(doc.weeks) == week_count_before - len(doomed_weeks)

# `StaleHandleError` is a `collomatique.Error`, so a script that only wants « the
# collomatique call failed » still catches it.
assert issubclass(collomatique.StaleHandleError, collomatique.Error)

# What still works: the id, `==` and `hash`. None of them reads the state, which
# is exactly why a dict keyed on handles survives the removal.
assert doomed.id == doomed_id
assert doomed == doomed
assert hash(doomed) == hash(doomed)
assert by_handle[doomed] == "period"
assert by_handle[doomed_weeks[0]] == "week"

# What does not: everything that reads. The message names the kind and the id.
for attribute in ("index", "weeks"):
    try:
        getattr(doomed, attribute)
    except collomatique.StaleHandleError as error:
        assert "Period" in str(error)
        assert repr(doomed_id) in str(error)
    else:
        raise AssertionError(f"a stale Period must raise on .{attribute}")

for attribute in ("period", "index", "interrogations", "annotation", "monday"):
    try:
        getattr(doomed_weeks[0], attribute)
    except collomatique.StaleHandleError as error:
        assert "Week" in str(error)
        assert repr(doomed_week_id) in str(error)
    else:
        raise AssertionError(f"a stale Week must raise on .{attribute}")

# A repr never raises — logging a dead handle is exactly when it matters — and
# it says the handle is dead.
assert repr(doomed).startswith("<Period #")
assert repr(doomed).endswith("(stale)>")
assert repr(doomed_weeks[0]).startswith("<Week #")
assert repr(doomed_weeks[0]).endswith("(stale)>")

# The mapping conventions, for both an id and a handle that name nothing.
assert doc.periods.get(doomed_id) is None
assert doc.periods.get(doomed) is None
assert doomed_id not in doc.periods
assert doomed not in doc.periods
try:
    doc.periods[doomed_id]
except KeyError:
    pass
else:
    raise AssertionError("a dead period id must not resolve")

assert doc.weeks.get(doomed_week_id) is None
assert doomed_week_id not in doc.weeks
try:
    doc.weeks[doomed_weeks[0]]
except KeyError:
    pass
else:
    raise AssertionError("a dead week handle must not resolve")

# The walk that was under way still meets the weeks that were there when it
# started, and the handles it mints for the dead ones are loud rather than
# missing.
rest = list(walk)
assert len(rest) == week_count_before - 1
dead = [week for week in rest if week.id == doomed_week_id]
assert len(dead) == 1
try:
    dead[0].index
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a handle minted for a removed week must be loud")

# The handles of what survived are untouched — the period that went was the last
# one, so nothing renumbered either.
assert survivor.index == 0
assert [week.id for week in survivor.weeks] == [week.id for week in survivor_weeks]
assert survivor_weeks[0].index == 0
assert first_seen.index == 0
