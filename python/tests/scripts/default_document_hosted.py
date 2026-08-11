import collomatique

# This script runs inside an application, the way one started from the script
# editor does. `other_source` is a real colloscope sitting on disk — the stale
# argument the order of the chain is there to ignore. The desktop the rust side
# installed has no answer for anything, so a chooser opened here fails the test
# rather than passing it quietly.

doc = collomatique.default_document(other_source)

# The host comes first, and not merely first among equals: a script run inside
# collomatique must never start editing a file on disk because an argument was
# lying around.
assert doc.is_hosted
assert doc.source_path is None

# The chain goes through `current_document`, so what it hands back is the
# application's document itself and not a second copy of it.
assert doc is collomatique.current_document()

# The other two ways of calling it reach the same document. The one with no path
# at all has nothing left to fall through to, and `dialog=False` — the cron job's
# form — cannot even ask: neither of them opens a chooser, which rust checks by
# counting what the desktop was asked for.
assert collomatique.default_document() is doc
assert collomatique.default_document(dialog=False) is doc
