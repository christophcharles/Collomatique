import collomatique
import random

# `source` is the contacts fixture rust built: four teachers and four students,
# covering the four shapes a contact card can have — both fields, one, the
# other, neither. `second_source` is a byte copy of it, so the two documents
# hold the same people under the same ids, which is what makes the same-seed
# comparison below say something. `seed` and `label` come from rust too: the
# label is the french name `ops` gives the operation, so the undo assertions pin
# the operation's own name and not merely a string typed twice.
doc = collomatique.load(source)

teachers = list(doc.teachers)
students = list(doc.students)
people = teachers + students



def card(person):
    return (person.firstname, person.surname, person.tel, person.email)


def full_name(person):
    return (person.firstname, person.surname)



# Handles held from before the write, and what they read while the real names
# are still there.
before = [card(person) for person in people]
assert ("Minerva", "McGonagall", "0700000021", "mcgonagall@poudlard.fr") in before
assert ("Harry", "Potter", "0601020304", "harry.potter@poudlard.fr") in before

result = doc.anonymize_names(seed)

assert isinstance(result, collomatique.OpResult)
# The family creates nothing, so it answers a plain `OpResult` and never the
# `AddResult` subclass: there is no `created` at all, rather than one holding
# `None`.
assert not isinstance(result, collomatique.AddResult)
assert not hasattr(result, "created")
# A name is read by no foreign key and by no invariant, so there is never
# anything here for the cascade to repair — but the result says so rather than
# the call saying nothing at all.
assert result.warnings == []

# The handles did not go stale: a rename is an update and not a removal, so the
# eight held from before the write read what it left.
after = [card(person) for person in people]
assert len(after) == len(before) == 8

for old, new in zip(before, after):
    assert new[:2] != old[:2], f"{old} kept their name"
    # The contacts go with the names: what makes a document shareable is both.
    assert new[2] is None
    assert new[3] is None

# Nobody is handed a name somebody else already has.
assert len({new[:2] for new in after}) == 8

# The seed is the whole of what decides the names. The other document holds the
# same people under the same ids, so the same seed has to name them the same way
# — that is what lets a script anonymize two copies of a colloscope alike.
other = collomatique.load(second_source)
other_people = list(other.teachers) + list(other.students)
assert [card(person) for person in other_people] == before

other.anonymize_names(seed)
assert [card(person) for person in other_people] == after

# And another seed on that same document names everybody else, which is what
# says the payload is really read: a seed nobody looked at would name them the
# same way twice.
elsewhere = collomatique.load(source)
elsewhere_people = list(elsewhere.teachers) + list(elsewhere.students)
elsewhere.anonymize_names(seed + 1)
assert [full_name(person) for person in elsewhere_people] != [new[:2] for new in after]


def anonymized_with_python_seed(python_seed):
    random.seed(python_seed)
    run = collomatique.load(source)
    run.anonymize_names()
    return [full_name(person) for person in list(run.teachers) + list(run.students)]


# Left out, the seed is drawn from python's own `random`, which is a promise and
# not an implementation detail: seeding python makes the run reproducible
# without a script ever naming a seed, and seeding it differently names
# everybody else.
assert anonymized_with_python_seed(20260905) == anonymized_with_python_seed(20260905)
assert anonymized_with_python_seed(20260905) != anonymized_with_python_seed(20260906)
# What it drew is a real anonymization, and not the document left alone.
assert anonymized_with_python_seed(20260905) != [old[:2] for old in before]

# The whole thing is one history slot, named by the operation itself: a single
# undo hands every real name and every contact back.
assert doc.undo_name == label
doc.undo()
assert [card(person) for person in people] == before
assert doc.redo_name == label
doc.redo()
assert [card(person) for person in people] == after

# The seed is a 64-bit number, and that is the boundary's promise rather than
# the operation's: what falls outside is refused before anything is applied.
for bad in (-1, 2**64):
    try:
        doc.anonymize_names(bad)
    except OverflowError:
        pass
    else:
        raise AssertionError("a seed outside range(2**64) must be refused")

# Neither refusal left a slot behind: undoing once empties the history, so the
# accepted call is the only thing in it.
assert [card(person) for person in people] == after
doc.undo()
assert doc.can_undo is False
doc.redo()

# This is what rust reads back off the disk.
doc.save(target)
