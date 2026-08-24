import collomatique

# The rust side stands in for the application: it holds a colloscope and records
# what the script sends back. `monday` and `other_monday` are two mondays, and
# `other_source` is a second colloscope file on disk.
doc = collomatique.current_document()

assert isinstance(doc, collomatique.Document)
assert doc.is_hosted is True

# A hosted document was never on disk, and what crosses the handoff is the
# document itself — not what the application could not read of the file behind
# it.
assert doc.source_path is None
assert doc.caveats == frozenset()

# Asking twice gives the same document, so a helper that asks for it again
# rather than being handed it still edits the one document.
assert collomatique.current_document() is doc

start = doc.periods.first_week
doc.periods.set_first_week(monday)
assert collomatique.current_document().periods.first_week == monday

# `save()` with no path goes where the document came from, and for this one that
# is back to the application.
doc.save()

# The slot belongs to the application, not to the hosted document: any document
# can go into it. This one comes from a file the application never saw.
other = collomatique.load(other_source)
other.periods.set_first_week(other_monday)
collomatique.send_to_host(other)

# Sending twice is allowed, and the last one wins.
collomatique.send_to_host(doc)

# A copy is not hosted, whatever it holds: compaction cannot travel to the
# application, so the compacted copy has nowhere to go on its own.
copy = doc.compacted()
assert copy.is_hosted is False
assert copy.source_path is None
try:
    copy.save()
except collomatique.NoOrigin:
    pass
else:
    raise AssertionError("a compacted copy of the hosted document has nowhere to save")

# An undo stays inside the script: it takes the date back here, while the
# application goes on holding what was sent, and rust checks that nothing
# crossed to say otherwise.
doc.undo()
assert doc.periods.first_week == start

assert issubclass(collomatique.NotHosted, collomatique.Error)
