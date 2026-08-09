import collomatique

# The one script here that talks to the real `rfd`, so the one script here that
# needs a human. Answer it either way: cancelling says as much about the plumbing
# as picking does.
chosen = collomatique.dialogs.open_file(
    title="Un vrai sélecteur de fichier",
    filters=[
        ("Fichiers collomatique", ["*.collomatique"]),
        ("Tous les fichiers", ["*"]),
    ],
)
