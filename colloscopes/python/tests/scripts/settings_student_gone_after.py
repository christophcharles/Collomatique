import collomatique

# The second half: Hermione is gone, so both views go with her — the resolved
# one because it is bound to the student, the raw one because its student and
# its entry are both gone. The message says which death each is.
for view in (resolved, override_view):
    for attribute in ("interrogations_per_week_min", "interrogations_per_week_max",
                      "max_interrogations_per_day"):
        try:
            getattr(view, attribute)
        except collomatique.StaleHandleError as error:
            assert "Limits" in str(error)
            assert repr(hermione.id) in str(error)
            assert "is no longer in the document" in str(error)
        else:
            raise AssertionError(f"a stale Limits view must raise on .{attribute}")

# `==` and `hash` never read the state, so they outlive the student.
assert resolved == resolved
assert override_view == override_view
assert resolved != override_view  # the two views stay distinct, dead as they are
assert hash(resolved) == hash(resolved)

# Neither repr raises, and both say so.
assert repr(resolved).startswith("<Limits #")
assert repr(resolved).endswith("(périmé)>")
assert repr(override_view).startswith("<Limits #")
assert repr(override_view).endswith("(périmé)>")

# Her override row is gone with her, and asking for a dead student's limits is
# a stale argument — the model's forgiving answer is not mirrored.
assert doc.settings.overrides() == ()
for ask in (lambda: doc.settings.limits_for(hermione),
            lambda: doc.settings.limits_for(hermione.id),
            lambda: doc.settings.override_for(hermione)):
    try:
        ask()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead student argument must raise StaleHandleError")
