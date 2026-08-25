"""A read-only look at the hosted document: nothing sent, so nothing warned.

Started as `collomatique --python-file <this> <fixture>`, the same way as
`script_document_warn_e2e.py` and differing only in not sending -- which is what
makes the pair say that the warning follows the send, and not the missing
`--out`.
"""

import sys

import collomatique as clm

sys.stdout.reconfigure(line_buffering=True)

doc = clm.current_document()
assert doc is not None, "the file on the command line is the hosted document"
assert len(doc.students) == 24, f"the fixture has 24 students, not {len(doc.students)}"
print("read only, nothing sent")
