import collomatique

doc = collomatique.current_document()

try:
    collomatique.send_to_host(doc)
except collomatique.DocumentChanged as error:
    message = str(error)
else:
    raise AssertionError("a refusal must reach the script")

# A script that only knows about writing failing keeps catching it.
assert issubclass(collomatique.DocumentChanged, collomatique.SaveError)

# The refusal changed nothing, so the document still speaks of what it read —
# and save() on the hosted document is the same crossing.
try:
    doc.save()
except collomatique.DocumentChanged:
    pass
else:
    raise AssertionError("save() on the hosted document crosses the same way")
