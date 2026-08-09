import collomatique

# The second half: « Semaines paires » is gone, and so is the second period with
# every week in it. The handles stage one kept now name things the document no
# longer holds.
assert len(doc.week_patterns) == pattern_count_before - 1
assert len(doc.weeks) == week_count_before - 3

# What still works: the id, `==` and `hash`. None of them reads the state, which
# is exactly why a dict keyed on handles survives the removal.
assert doomed.id == doomed_id
assert doomed == doomed
assert hash(doomed) == hash(doomed)
assert by_handle[doomed] == "pattern"
assert by_handle[doomed_week] == "week"

# What does not: everything that reads. The message names the kind and the id.
for attribute in ("name", "excluded_weeks"):
    try:
        getattr(doomed, attribute)
    except collomatique.StaleHandleError as error:
        assert "WeekPattern" in str(error)
        assert repr(doomed_id) in str(error)
    else:
        raise AssertionError(f"a stale WeekPattern must raise on .{attribute}")

# A repr never raises — logging a dead handle is exactly when it matters — and it
# says the handle is dead.
assert repr(doomed).startswith("<WeekPattern #")
assert repr(doomed).endswith("(stale)>")

# The mapping conventions, for both an id and a handle that name nothing.
assert doc.week_patterns.get(doomed_id) is None
assert doc.week_patterns.get(doomed) is None
assert doomed_id not in doc.week_patterns
assert doomed not in doc.week_patterns
try:
    doc.week_patterns[doomed]
except KeyError:
    pass
else:
    raise AssertionError("a dead week pattern handle must not resolve")

# And the other convention, on the same dead references. `is_week_active` takes
# its week and its pattern as arguments, so a dead one raises instead of being
# answered — where the model itself would have shrugged: it says `false` for a
# week it does not hold and treats a pattern it does not hold as excluding
# nothing (rust pins both). A script reading those answers would conclude the
# colles had stopped that week, when what really happened is that it lost track
# of its own document.
for pattern in (doomed, doomed_id):
    try:
        doc.is_week_active(live_week, pattern)
    except collomatique.StaleHandleError as error:
        assert "WeekPattern" in str(error)
        assert repr(doomed_id) in str(error)
        # The removal really is what the message says it is: this document held
        # that pattern a moment ago and holds it no longer.
        assert "is not in this document" in str(error)
    else:
        raise AssertionError("a dead pattern argument must raise")

for week in (doomed_week, doomed_week_id):
    for pattern in (None, every):
        try:
            doc.is_week_active(week, pattern)
        except collomatique.StaleHandleError as error:
            assert "Week" in str(error)
            assert repr(doomed_week_id) in str(error)
            assert "is not in this document" in str(error)
        else:
            raise AssertionError("a dead week argument must raise")

# What survived answers as it did. The pattern that switched every week off has
# had the dead ones taken out of its set by the removal's own repairs, so what is
# left in it is still this document's weeks — and it still switches them off.
assert doc.is_week_active(live_week, every)
assert doc.is_week_active(live_week)
assert all(week in doc.weeks for week in all_off.excluded_weeks)
assert len(all_off.excluded_weeks) == len(doc.weeks)
assert not doc.is_week_active(live_week, all_off)
assert unnamed.excluded_weeks == frozenset({live_week})
