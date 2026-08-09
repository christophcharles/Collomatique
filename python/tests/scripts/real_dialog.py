import collomatique

# The one script here that talks to the real `rfd`, so the one script here that
# needs a human. Answer them either way: cancelling says as much about the
# plumbing as picking does.
#
# All three in a row, and that sequence is the point rather than a convenience.
# The bus outlives a dialog — `ashpd` caches the session connection in a global
# — so the second call is the one that catches a runtime that did not.
opened = collomatique.dialogs.open_file(
    title="1/3 — ouvrir un fichier",
    filters=[
        ("Fichiers collomatique", ["*.collomatique"]),
        ("Tous les fichiers", ["*"]),
    ],
)

saved = collomatique.dialogs.save_file(
    title="2/3 — enregistrer sous",
    filters=[("Tableur", ["csv", "xlsx"])],
    file_name="sortie.csv",
)

folder = collomatique.dialogs.pick_folder(title="3/3 — choisir un dossier")
