import collomatique

# `source` is a file that was written by a newer Collomatique and holds one
# block this build cannot read. `newer_version` and `spec_version` are what the
# rust side put in it.
doc = collomatique.load(source)

# The caveats are values, so a script says what it expects by building it.
expected = frozenset(
    {
        collomatique.CreatedWithNewerVersion(newer_version),
        collomatique.UnknownEntry(block_name, spec_version),
    }
)
assert doc.caveats == expected
assert collomatique.UnknownEntry(block_name, spec_version) in doc.caveats

# A caveat that is not there is not there: equality looks at the payload, not
# just at the class.
assert collomatique.UnknownEntry("some-other-block", spec_version) not in doc.caveats

for caveat in doc.caveats:
    assert isinstance(caveat, collomatique.Caveat)
    assert str(caveat) != ""
    assert repr(caveat) != ""

# `__match_args__` is there, so a script can take the caveats apart with
# `match` instead of testing classes by hand.
seen = set()
for caveat in doc.caveats:
    match caveat:
        case collomatique.CreatedWithNewerVersion(version):
            assert version == newer_version
            seen.add("version")
        case collomatique.UnknownEntry(name, spec):
            assert name == block_name
            assert spec == spec_version
            # The sentence names the block, which is the point of carrying it.
            assert name in str(caveat)
            assert name in repr(caveat)
            seen.add("entry")
        case _:
            raise AssertionError(f"unexpected caveat {caveat!r}")
assert seen == {"version", "entry"}

# The base class is a base class: it is what `isinstance` is for, not something
# a script builds.
try:
    collomatique.Caveat()
except TypeError:
    pass
else:
    raise AssertionError("Caveat() must not be constructible")

# A document that was never on disk read nothing, so it lost nothing.
assert collomatique.new_document().caveats == frozenset()

# Writing back over the file would drop the block that could not be read, and
# the script never named that file, so it is refused.
try:
    doc.save()
except collomatique.CaveatedOverwrite as error:
    # The message says what was lost and how to write anyway, rather than
    # leaving the script author to guess.
    assert block_name in str(error)
    assert "ignore_caveats" in str(error)
else:
    raise AssertionError("save() over a caveated origin must raise")

assert issubclass(collomatique.CaveatedOverwrite, collomatique.SaveError)
assert issubclass(collomatique.CaveatedOverwrite, collomatique.Error)

# Every form that names a destination writes, because naming one is a choice.
doc.save(target)  # somewhere else: the suspect original survives
doc.save(doc.source_path)  # the origin, but said out loud
doc.save(ignore_caveats=True)  # the origin, deliberately
doc.save(target, ignore_caveats=True)  # a no-op flag with a path, but accepted

# None of that made the original file any more readable.
assert doc.caveats == expected
