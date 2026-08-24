import pathlib

import collomatique
from collomatique.dialogs import open_file

# The rust side stands in for the desktop: it records every request and answers
# each one from a list written in the test, in the order the calls below make
# them. `start_dir` is a directory, and `chosen_file`, `saved_file` and
# `chosen_folder` are the three paths it is going to hand back.

# `collomatique.dialogs` is a module in its own right, not merely an attribute:
# a script may import from it, and what it imports is the same function.
assert open_file is collomatique.dialogs.open_file
assert open_file.__module__ == "collomatique.dialogs"

# There is no order those arguments naturally come in, so there is no positional
# form of them either.
try:
    collomatique.dialogs.open_file("Ouvrir")
except TypeError:
    pass
else:
    raise AssertionError("the dialog arguments are keyword-only")

# A path comes back as a `pathlib.Path`, not as a string.
picked = collomatique.dialogs.open_file(
    title="Ouvrir la liste des élèves",
    filters=[
        ("Fichiers collomatique", ["*.collomatique"]),
        ("Tous les fichiers", ["*"]),
    ],
    directory=start_dir,
)
assert isinstance(picked, pathlib.Path)
assert picked == chosen_file

# The three ways of writing an extension all mean the same one, which rust
# checks on the request this made.
written = collomatique.dialogs.save_file(
    title="Exporter le colloscope",
    filters=[("Tableur", [".csv", "csv", "*.xlsx"])],
    directory=start_dir,
    file_name="sortie.csv",
)
assert written == saved_file

folder = collomatique.dialogs.pick_folder(
    title="Choisir un dossier",
    directory=start_dir,
)
assert folder == chosen_folder

# Cancelling is the ordinary way out of a dialog, so it is a value and not an
# exception.
assert collomatique.dialogs.open_file(title="Annulé") is None

# An extension that is nothing once the `*.` is off cannot filter anything. This
# one is refused here, before any dialog is opened — rust counts the requests it
# was asked for, and this is not one of them.
try:
    collomatique.dialogs.open_file(filters=[("Vide", ["."])])
except ValueError:
    pass
else:
    raise AssertionError("an empty extension should not build a filter")

# A machine with nothing to draw a dialog on says so, rather than waiting for a
# click that is never coming.
try:
    collomatique.dialogs.open_file()
except collomatique.DialogUnavailable as refused:
    assert str(refused) == refusal
else:
    raise AssertionError("a dialog that cannot be shown should raise")

assert issubclass(collomatique.DialogUnavailable, collomatique.Error)

# Every argument is optional: the bare call is the generic door, with the
# desktop's own title and no filter at all.
assert collomatique.dialogs.pick_folder() is None
