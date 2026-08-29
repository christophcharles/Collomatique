"""A send that nothing will keep: no `--out`, so rust expects the warning.

Started as `collomatique --python-file <this> <fixture>`. The run still ends
well -- sending with nowhere to send to is not the script's mistake -- and what
the rust side reads is the sentence on stderr.
"""

import sys

import collomatique as clm

sys.stdout.reconfigure(line_buffering=True)

doc = clm.current_document()
assert doc is not None, "the file on the command line is the hosted document"

doc.students.add(clm.StudentData("Nymphadora", "Tonks"))
doc.save()
print("sent, with nowhere to go")
