import collomatique

# `source` is a throwaway copy of a real colloscope, `other_source` a second
# document for the foreign-entity questions, and `target` is where the script
# leaves the document for rust to read back. `fold_label` is the name of the
# transaction the replace folds into, handed in from rust the way every other
# write test hands its labels.
doc = collomatique.load(source)
other = collomatique.load(other_source)

pristine = doc.snapshot()

# ------------------------------------------------------ the identity round trip

# The tree comes straight back out of the document, so nothing changes — but a
# global update is still a write, and it takes its own undo slot.
result = doc.replace_all(doc.snapshot())

assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)

# Always empty, and not merely empty this once: an incremental op repairs what
# it broke and says so, while a global update lands as given or is refused
# whole. There is nothing for it to repair.
assert result.warnings == []

assert doc.snapshot() == pristine
assert doc.undo_name == "Mise à jour globale"

doc.undo()
assert doc.snapshot() == pristine
assert doc.can_undo is False
doc.redo()
assert doc.undo_name == "Mise à jour globale"
doc.undo()

# --------------------------------------------------------- a tree that is wrong

# A teacher the slots still name. The tree drops the teacher and keeps the
# slots, so every one of those slots is left pointing at nothing — and the
# refusal names every one of them, not just the first, which is rust's half of
# this.
by_teacher = {}
for slot in doc.slots:
    by_teacher.setdefault(slot.teacher, []).append(slot)

doomed_teacher = max(by_teacher, key=lambda t: (len(by_teacher[t]), t.id))
orphan_count = len(by_teacher[doomed_teacher])
assert orphan_count >= 2, "the refusal has to have more than one thing to say"

broken = doc.snapshot()
del broken.teachers[doomed_teacher.id]

try:
    doc.replace_all(broken)
except collomatique.UpdateError as e:
    refusal = str(e)
else:
    raise AssertionError("a tree that leaves references dangling must be refused")

# Refused whole: not one field of it landed, and the refusal took no undo slot
# either.
assert doc.snapshot() == pristine
assert doc.can_undo is False

# --------------------------------------------------- trees that never get there

# A tree naming an entity of another document. It is refused where every other
# foreign reference is — at the argument boundary, before anything is applied.
foreign_teacher = list(other.teachers)[0]
with_a_foreign_teacher = collomatique.DocumentData(
    teachers={foreign_teacher: collomatique.TeacherData("Emmy", "Noether")}
)
try:
    doc.replace_all(with_a_foreign_teacher)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a tree naming another document's teacher must not resolve")

# And something that is not a tree at all.
try:
    doc.replace_all(3)
except TypeError:
    pass
else:
    raise AssertionError("replace_all takes a DocumentData")

assert doc.snapshot() == pristine
assert doc.can_undo is False

# ------------------------------------------------------- inside a transaction

# A global update is a write like any other, so it folds into the block it runs
# in: one undo slot for the block, carrying the block's name and not this call's.
with doc.transaction(fold_label):
    renamed = doc.snapshot()
    first_subject = list(renamed.subjects)[0]
    renamed.subjects[first_subject].name = "Vol et balai"
    doc.replace_all(renamed, "Never seen")
    doc.replace_all(doc.snapshot())

assert doc.undo_name == fold_label
doc.undo()
assert doc.snapshot() == pristine
assert doc.can_undo is False

# ------------------------------------------------------------- a real transform

subject = list(doc.subjects)[0]
incompat = list(doc.incompats)[0]
survivor = list(doc.subjects)[-1]
survivor_name = survivor.name
new_name = "Défense contre les forces du Mal"

tree = doc.snapshot()
tree.subjects[subject.id].name = new_name
del tree.incompats[incompat.id]

label = "Rebuilt from scratch"
applied = doc.replace_all(tree, label)
assert applied.warnings == []
assert doc.undo_name == label

# The ids are the document's own, so the handles held across the replace still
# name what they named — except the one whose entity the tree dropped.
assert doc.subjects[subject].name == new_name
assert survivor.name == survivor_name
assert incompat not in doc.incompats
try:
    incompat.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a handle to what the tree dropped must go stale")

# The document is the tree, exactly.
assert doc.snapshot() == tree

# -------------------------------------------------- and what a tree cannot do

# The snapshot taken before the transform still names the incompatibility the
# transform dropped. It cannot bring it back: a tree names its entities by id,
# and an id that names nothing in this document is refused like any other dead
# reference. Adding is the incremental ops' business.
try:
    doc.replace_all(pristine)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a tree cannot resurrect what a previous one removed")

assert doc.snapshot() == tree

doc.save(target)
