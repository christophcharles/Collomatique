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
