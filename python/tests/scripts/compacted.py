import collomatique

# `source` is a throwaway copy of a real colloscope, and `target` is where the
# compacted copy goes. `caveated_source` is a file this build cannot read whole.
doc = collomatique.load(source)
compacted = doc.compacted()

# A copy, not a renumbering in place: the script keeps both, and the document it
# started from is still the document it started from.
assert isinstance(compacted, collomatique.Document)
assert compacted is not doc

# The copy came from the same file, so it saves back to it — which is the whole
# point of the id-ceiling rescue, `clm.load(f).compacted().save()`.
assert compacted.source_path == doc.source_path
assert compacted.caveats == doc.caveats == frozenset()

compacted.save(target)
doc.save(original)  # the original is untouched and still writes

# Nothing to renumber and nowhere to write, but neither is an error.
blank = collomatique.new_document().compacted()
assert blank.source_path is None
assert blank.caveats == frozenset()

# Compaction is not a way round the caveat guard: the copy carries the caveats
# of the file it came from, so writing back to that file is still refused.
suspect = collomatique.load(caveated_source)
caveated = suspect.compacted()
assert caveated.source_path == suspect.source_path
assert caveated.caveats == suspect.caveats != frozenset()

try:
    caveated.save()
except collomatique.CaveatedOverwrite:
    pass
else:
    raise AssertionError("save() over a caveated origin must raise after compaction")
