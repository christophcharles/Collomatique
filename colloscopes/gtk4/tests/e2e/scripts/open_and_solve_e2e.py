"""The road from a colloscope file to a solved colloscope.

Started as `collomatique --python-file <this>`, in a child process of its own.
The test hands it the document to open through `E2E_FIXTURE` and the path to
leave the answer at through `E2E_SAVE`; a script run that way has no argv and no
`__file__`, so the environment is how the two meet.

Everything this asserts, it asserts itself: the rust side reads the exit status
and the file that was left behind. So an assertion message here is what a
failing test will show, and they are written to be read.

The document is the frozen hogwarts fixture, and it is opened rather than built:
what this test is about is the whole road, and a document that came from a file
is the first stretch of it. The strategy is `ConductorStrategy.search()` -- the
application's own « Recherche simple », the preset a user meets first -- so what
runs here is what the solve dialog runs.
"""

import os
import sys

import collomatique as clm

# The test reads this script's output through a pipe, and a pipe is what makes
# python buffer a whole block at a time. An assertion below ends the run before
# such a block is ever flushed, so the lines that would say what happened just
# before it would be the ones lost.
sys.stdout.reconfigure(line_buffering=True)

fixture = os.environ["E2E_FIXTURE"]
target = os.environ["E2E_SAVE"]

# ------------------------------------------------------------------- the file

doc = clm.load(fixture)

# The fixture is frozen, so a caveat means this build no longer reads it whole
# -- which is a thing to know loudly, and the day to take a fresh copy of the
# example.
assert not doc.caveats, f"the frozen fixture should read whole, got {doc.caveats}"

# Not the document's whole shape, just enough that swapping the fixture for
# another file is a failure here rather than a surprise further down.
assert len(doc.students) == 24, f"the fixture has 24 students, not {len(doc.students)}"
assert len(doc.subjects) == 8, f"the fixture has 8 subjects, not {len(doc.subjects)}"
assert len(doc.slots) == 22, f"the fixture has 22 slots, not {len(doc.slots)}"

print(f"loaded {len(doc.students)} students, {len(doc.slots)} slots")

# ------------------------------------------------------------------ the solve

lines = []
model = doc.build_colloscope_model(clm.ColloscopeSolveConfig(), on_log=lines.append)
assert lines, "the build says something on the way"
print(f"model: {model!r}")

# No `engine=`: the child running this script *is* collomatique, so the engine
# is the rung the runner injected -- the ordinary case, and the one the
# application itself is on.
run = model.solve(clm.ConductorStrategy.search(), on_log=lines.append)
outcome = run.wait()
print(f"outcome: {outcome!r}, {len(lines)} log line(s)")

assert outcome.status in (clm.SolveStatus.OPTIMAL, clm.SolveStatus.FEASIBLE), (
    f"the fixture has a colloscope to find, so it is found, not {outcome.status!r}"
)
assert outcome.colloscope is not None, "a solved problem hands back a colloscope"

# ---------------------------------------------------------------- the install

doc.colloscope.install(outcome.colloscope)

placed = list(doc.colloscope.interrogations())
assert placed, "the installed colloscope holds interrogations"

# Every cell the model stores is a cell with somebody in it: `interrogations`
# yields only the non-empty ones, so an empty frozenset here would be a cell
# that should never have been written.
for slot, week, groups in placed:
    assert groups, f"a stored cell names no group: {slot!r}, {week!r}"

print(f"installed {len(placed)} interrogation cell(s)")

# ------------------------------------------------------------------- the file

# Somewhere the test named, never back over the fixture: `save()` with no
# argument writes to the file the document came from, and the fixture is in the
# repository.
doc.save(target)
print(f"saved to {target}")
