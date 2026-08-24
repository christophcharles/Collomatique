import pathlib

import collomatique

# `source` is a throwaway copy of the two-filling document; the targets are
# where the script writes, handed in by the rust side so rust can read the
# files back and ask them how big a problem they hold.
doc = collomatique.load(source)

assert issubclass(collomatique.ExportError, collomatique.Error)

automatic = next(gl for gl in doc.group_lists if gl.groups is None)

config = collomatique.ColloscopeSolveConfig()
model = doc.build_colloscope_model(config)

# ------------------------------------------------------------ the whole problem

model.export_mps(full_target)

assert pathlib.Path(full_target).exists()

full = pathlib.Path(full_target).read_text()

# A well-formed MPS file: the sections a solver looks for, and the marker that
# says the file is finished.
assert full.startswith("NAME")
for section in ("OBJSENSE", "ROWS", "COLUMNS", "RHS"):
    assert f"\n{section}\n" in full
assert full.rstrip().endswith("ENDATA")

# An export writes a file, not the document: nothing was applied, so there is
# nothing to undo.
assert doc.can_undo is False
assert doc.can_redo is False

# And it does not spend the model either — the same model, asked twice, writes
# the same file. A path is a path whichever way it is spelled.
model.export_mps(pathlib.Path(again_target))
assert pathlib.Path(again_target).read_text() == full

# ----------------------------------------------------------- the checker problem

# The anchor is an objective term, so this model has an objective to write down
# — which is exactly what the checker file leaves out. Without it the two files
# would have nothing to differ by.
anchored = doc.build_colloscope_model(
    collomatique.ColloscopeSolveConfig(
        group_lists={
            automatic: collomatique.GroupListSolveConfig(
                previous_values_as_objective=True
            )
        },
    )
)

anchored.export_mps(anchored_target)
anchored.export_mps(checker_target, checker=True)

anchored_text = pathlib.Path(anchored_target).read_text()
checker_text = pathlib.Path(checker_target).read_text()

assert checker_text.startswith("NAME")
assert checker_text.rstrip().endswith("ENDATA")

# The constraints alone, without what only the objective needed: a smaller
# problem out of the same build, so a shorter file.
assert checker_text != anchored_text
assert len(checker_text) < len(anchored_text)

# ------------------------------------------------------ what the config is for

# A group list that keeps the groups it has leaves the solver less to work out,
# and the file says so: the config reaches the builder, and the build reaches
# the file.
pinned = doc.build_colloscope_model(
    collomatique.ColloscopeSolveConfig(
        group_lists={automatic: collomatique.GroupListSolveConfig(recompute=False)},
    )
)
pinned.export_mps(pinned_target)
assert pathlib.Path(pinned_target).read_text() != full

# ------------------------------------------------------------- what is refused

# `checker` is keyword-only, so a script that hands it over positionally is
# handing over a second path.
try:
    model.export_mps(bad_target, True)
except TypeError:
    pass
else:
    raise AssertionError("checker is keyword-only")

assert not pathlib.Path(bad_target).exists()

# A path with no directory to hold it. The writer's failures and the file
# system's arrive the same way as the spreadsheet export's, and the message
# names the path that failed.
missing = pathlib.Path(bad_target).parent / "no-such-directory" / "problem.mps"
try:
    model.export_mps(missing)
except collomatique.ExportError as e:
    failure = str(e)
else:
    raise AssertionError("a path with no directory to hold it must be refused")

assert failure.startswith(str(missing))
