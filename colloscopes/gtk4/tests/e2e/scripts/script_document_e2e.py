"""The document the command line opened, edited, and took back.

Started as `collomatique --python-file <this> --out <target> <fixture>`. Unlike
the other families here, nothing is named through the environment and nothing is
loaded: the feature under test is precisely that the command line carries the
document, so it arrives through `current_document()` and leaves through the file
`--out` writes -- which is what the rust side reads back.

Everything this asserts, it asserts itself; the rust side reads the exit status
and that file. So an assertion message here is what a failing test will show.
"""

import sys

import collomatique as clm

# The test reads this script's output through a pipe, and a pipe is what makes
# python buffer a whole block at a time. An assertion below ends the run before
# such a block is ever flushed.
sys.stdout.reconfigure(line_buffering=True)

doc = clm.current_document()
assert doc is not None, "the file on the command line is the hosted document"
assert doc.is_hosted is True

# A hosted document carries no caveats: whatever the file needed, the process
# that opened it already said on stderr.
assert doc.caveats == frozenset()

# The frozen hogwarts fixture, so a different count here is a different file.
assert len(doc.students) == 24, f"the fixture has 24 students, not {len(doc.students)}"

added = doc.students.add(clm.StudentData("Nymphadora", "Tonks"))
assert added.warnings == []
assert len(doc.students) == 25

# `save()` with no path goes where the document came from -- back to the process
# holding it, which is what `--out` then writes. The fixture is in the repository
# and is never written over: `--out` names somewhere else.
doc.save()
print("sent 25 students")
