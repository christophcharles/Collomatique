import collomatique

# `source` is a throwaway copy of the two-filling document: the automatic group
# list in it is what gives the problem variables to talk about, and a student
# added to it later is an edit the model cannot fail to notice.
doc = collomatique.load(source)

assert issubclass(collomatique.ModelBuildError, collomatique.Error)

automatic = next(gl for gl in doc.group_lists if gl.groups is None)

config = collomatique.ColloscopeSolveConfig()

# ------------------------------------------------------------- the plain build

model = doc.build_colloscope_model(config)
assert isinstance(model, collomatique.ColloscopeModel)

# The repr is the whole read surface: two counts, and no name of anything
# inside. Rust reads the numbers out of it and compares them with its own
# build of the same document.
shown = repr(model)
assert shown == str(model)

# Opaque: a model is a token for a built problem, not a view of one.
assert not hasattr(model, "problem")
assert not hasattr(model, "variables")
assert not hasattr(model, "constraints")

# A build reads the document and writes nothing to it, so it takes no undo
# slot — there is nothing to undo on a document nothing was applied to.
assert doc.can_undo is False
assert doc.can_redo is False

# --------------------------------------------------------------------- the log

lines = []
logged = doc.build_colloscope_model(config, on_log=lines.append)

# The same document and the same config, so the same problem: the log is
# something the build says on the way, not something it does.
assert repr(logged) == shown

assert lines
assert all(isinstance(line, str) for line in lines)

# The three phases the builder announces — the initial model, the reduced one,
# and the final build — arrive in order and whole.
markers = [line for line in lines if line.startswith("---")]
assert len(markers) == 3
assert "(1/3)" in markers[0]
assert "(2/3)" in markers[1]
assert "(3/3)" in markers[2]


class Boom(Exception):
    pass


seen = []


def angry(line):
    seen.append(line)
    raise Boom(line)


try:
    doc.build_colloscope_model(config, on_log=angry)
except Boom as e:
    raised = str(e)
else:
    raise AssertionError("a log callback that raises must be heard")

# Once, and never again: the build was not torn in half, it ran to its end with
# nobody listening, and the exception came out afterwards.
assert len(seen) == 1
assert seen[0] == lines[0]
assert raised == seen[0]

# ------------------------------------------------------ what the config is for

# A group list that is not recomputed keeps the groups it has, so the problem
# has less to work out — a different problem, from the same document.
pinned = collomatique.ColloscopeSolveConfig(
    group_lists={automatic: collomatique.GroupListSolveConfig(recompute=False)},
)
reduced = doc.build_colloscope_model(pinned)
assert repr(reduced) != shown

# And a list recomputed but held close to the groups it has is a problem with
# something to weigh: the anchor is an objective term, which nothing in the
# plain build has.
anchored = doc.build_colloscope_model(
    collomatique.ColloscopeSolveConfig(
        group_lists={
            automatic: collomatique.GroupListSolveConfig(
                previous_values_as_objective=True
            )
        },
    )
)
anchored_shown = repr(anchored)
assert anchored_shown != shown

# ------------------------------------------------------------- what is refused

# The config is required: there is no second meaning for its absence, so there
# is no default for it either.
try:
    doc.build_colloscope_model()
except TypeError:
    pass
else:
    raise AssertionError("build_colloscope_model takes a config")

# `on_log` is keyword-only, so a script that hands it over positionally is
# handing over a config.
try:
    doc.build_colloscope_model(config, print)
except TypeError:
    pass
else:
    raise AssertionError("on_log is keyword-only")

# A config that is not one, refused where every other value is — at the
# boundary, before anything is built.
try:
    doc.build_colloscope_model(3)
except TypeError as e:
    not_a_config = str(e)
else:
    raise AssertionError("build_colloscope_model takes a ColloscopeSolveConfig")

assert "ColloscopeSolveConfig" in not_a_config

# ------------------------------------------------------------- and detachment

with doc.transaction("Add a student"):
    doc.students.add(collomatique.StudentData("Rogue", "Severus"))

assert doc.undo_name == "Add a student"

# The model was a snapshot of the document as it stood: the document has a
# student more now, and the model still says exactly what it said.
assert repr(model) == shown

# Which the build itself proves is a real difference — the new student joins
# the automatic list, so the problem built now is not the one built before.
after = doc.build_colloscope_model(config)
assert repr(after) != shown

# And that build wrote nothing either: the block above is still the last thing
# that happened to this document.
assert doc.undo_name == "Add a student"
assert doc.can_redo is False
