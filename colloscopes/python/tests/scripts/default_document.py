import collomatique

# No application to talk to here, so the chain starts at its second link.
# `source` and `chosen` are real colloscopes — the second is what the fake
# desktop hands back from the chooser — and `missing` is a path with nothing
# behind it.

doc = collomatique.default_document(source)
assert not doc.is_hosted
assert doc.source_path == source

# A path that will not load is an error and not an invitation to the chooser:
# the chain is a list of sources, not a retry loop, and a script that named a
# file and got the name wrong wants to hear so.
try:
    collomatique.default_document(missing)
except collomatique.LoadError:
    pass
else:
    raise AssertionError("a path that does not load should raise")

# With no host and no path, the chooser is the last link. Rust checks what
# reached the desktop, which is the half no script can see.
chosen_doc = collomatique.default_document()
assert chosen_doc.source_path == chosen

# Dismissing the chooser is not a document — and not a `None` either, which
# would have every script write an `if doc is None` and give an obscure
# `AttributeError` twenty lines down to the one that forgot.
try:
    collomatique.default_document()
except collomatique.Cancelled:
    pass
else:
    raise AssertionError("a dismissed chooser should raise")

# A machine that cannot show a chooser at all says so, rather than waiting for a
# click that is never coming. It is a different answer from `NoDocument`, and
# the more precise one: there was somewhere left to look.
try:
    collomatique.default_document()
except collomatique.DialogUnavailable as refused:
    assert str(refused) == refusal
else:
    raise AssertionError("a chooser that cannot be shown should raise")

# `dialog=False` is what a cron job passes, where a chooser nobody is watching
# would wait forever. There is nothing left to try, so it raises straight away
# without asking the desktop — rust counts the requests, and this is not one.
try:
    collomatique.default_document(dialog=False)
except collomatique.NoDocument:
    pass
else:
    raise AssertionError("dialog=False with no other source should raise")

# Both are ordinary collomatique errors, so a script that only wants to know the
# call failed has one thing to catch.
assert issubclass(collomatique.NoDocument, collomatique.Error)
assert issubclass(collomatique.Cancelled, collomatique.Error)
