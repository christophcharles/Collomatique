import collomatique

# The one script here that talks to the real `rfd`, so the one script here that
# needs a human. Answer them either way: cancelling says as much about the
# plumbing as picking does.
#
# All three in a row, and that sequence is the point rather than a convenience.
# The bus outlives a dialog — `ashpd` caches the session connection in a global
# — so the second call is the one that catches a runtime that did not.
opened = collomatique.dialogs.open_file(
    title="1/4 — ouvrir un fichier",
    filters=[
        ("Fichiers collomatique", ["*.collomatique"]),
        ("Tous les fichiers", ["*"]),
    ],
)

saved = collomatique.dialogs.save_file(
    title="2/4 — enregistrer sous",
    filters=[("Tableur", ["csv", "xlsx"])],
    file_name="sortie.csv",
)

folder = collomatique.dialogs.pick_folder(title="3/4 — choisir un dossier")

# The fourth is a chooser the module opens on the script's behalf rather than
# one the script asked for — the last link of `default_document`'s chain, and
# the only place a real portal ever sees it. Its title is the application's own
# « Ouvrir », so this one is not numbered. Cancel it, or pick a colloscope;
# picking anything else raises `LoadError`, which is the honest answer.
try:
    document = collomatique.default_document()
    document_path = document.source_path
except collomatique.Cancelled:
    document_path = None
