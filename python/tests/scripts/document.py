import pathlib

import collomatique

# `source` and `target` are paths handed in by the rust side. `source` is a
# throwaway copy: `doc.save()` writes back to its origin, so this must never be
# a file that lives in the repository.
doc = collomatique.load(source)

assert isinstance(doc, collomatique.Document)
assert isinstance(doc.source_path, pathlib.Path)
assert doc.source_path == pathlib.Path(source)

# The example is a file this version reads whole, so nothing was dropped.
assert doc.caveats == frozenset()

doc.save()  # back to the origin
doc.save(target)  # save-as

# Saving elsewhere does not move the origin: a later `save()` still writes the
# file the document came from.
assert doc.source_path == pathlib.Path(source)

blank = collomatique.new_document()
assert isinstance(blank, collomatique.Document)
assert blank.source_path is None

try:
    blank.save()
except collomatique.NoOrigin:
    pass
else:
    raise AssertionError("save() with no origin must raise")

# Every exception the module raises descends from `collomatique.Error`, so a
# script that only cares that the call failed has one thing to catch.
assert issubclass(collomatique.NoOrigin, collomatique.Error)
assert issubclass(collomatique.IdCeilingExceeded, collomatique.SaveError)

# A failed write is a `SaveError` rather than an OSError escaping from the
# rust side.
missing = pathlib.Path(source).parent / "no-such-directory" / "doc.collomatique"
try:
    blank.save(missing)
except collomatique.SaveError:
    pass
else:
    raise AssertionError("saving into a missing directory must raise")
