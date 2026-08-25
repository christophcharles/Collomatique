"""A document built from nothing: `--new` hosts an empty one, `--out` keeps it.

Started as `collomatique --python-file <this> --new --out <target>`, with no file
to open. `send_to_host` rather than `doc.save()` here -- the two are the same
door for a hosted document, and between them the family exercises both.
"""

import sys

import collomatique as clm

sys.stdout.reconfigure(line_buffering=True)

doc = clm.current_document()
assert doc is not None, "--new hosts an empty document"
assert doc.is_hosted is True
assert len(doc.students) == 0, (
    f"an empty document has no students, not {len(doc.students)}"
)

doc.students.add(clm.StudentData("Harry", "Potter"))
clm.send_to_host(doc)
print("sent 1 student")
