import collomatique

# The same generation fixture the happy path runs on. Here every way a request
# can be wrong is written down, and each is asked which door refuses it: the
# boundary refuses what the request *names*, the plan refuses what it *asks
# for*, and the write refuses what the document cannot hold.
doc = collomatique.load(source)

assert issubclass(collomatique.GroupListsGenerationError, collomatique.Error)

periods = list(doc.periods)
p1, p2 = periods[0], periods[1]

subjects = {subject.name: subject for subject in doc.subjects}
sortileges = subjects["Sortilèges"]
metamorphose = subjects["Métamorphose"]
botanique = subjects["Botanique"]
potions = subjects["Potions"]

maisons = next(gl for gl in doc.group_lists if gl.name == "Maisons")
automatique = next(gl for gl in doc.group_lists if gl.name == "Automatique")

messages = {}


def refused(name, request, exception=collomatique.GroupListsGenerationError):
    """Runs a request that must be refused, and keeps the sentence."""
    try:
        doc.generate_group_lists(request)
    except exception as error:
        messages[name] = str(error)
    else:
        raise AssertionError(f"`{name}` must be refused")


# ---------------------------------------------------- what a plan refuses

# Botanique runs no interrogations, so there is no list to build for it — and
# nothing about the document to fix, which is why this is the request's fault
# and not a write's.
refused(
    "no_interrogations",
    collomatique.GroupListsGenerationRequest(rebuild={(p1, botanique)}),
)
assert "interrogation" in messages["no_interrogations"]

# An automatic list has no groups yet, so there is nothing in it for the
# generator to respect.
refused(
    "kept_not_prefilled",
    collomatique.GroupListsGenerationRequest(
        rebuild={(p1, sortileges)}, kept_lists={automatique}
    ),
)
assert "prefilled" in messages["kept_not_prefilled"]

# Potions wants groups of exactly four and five students take it: two groups
# cannot both be filled, one cannot hold them all. The default request offers
# this pair, so a script meets this refusal on the shortest path there is.
refused(
    "unsatisfiable_size",
    collomatique.GroupListsGenerationRequest(rebuild={(p1, potions)}),
)
assert "5 students" in messages["unsatisfiable_size"]
assert (p1.id, potions.id) in doc.default_generation_request().rebuild

# Nothing above wrote anything: a refused request is a request, not a
# half-applied operation.
assert doc.can_undo is False

# ------------------------------------------------- what the boundary refuses

# A reference the document does not hold is refused before the plan is built,
# so a plan only ever fails on what a request asks for.
doomed = doc.subjects.add(collomatique.SubjectData("Divination")).created
dead_subject = doomed.id
doc.subjects.remove(doomed)

refused(
    "dead_subject",
    collomatique.GroupListsGenerationRequest(rebuild={(p1.id, dead_subject)}),
    collomatique.StaleHandleError,
)

doc.undo()
doc.undo()
assert doc.can_undo is False

# The shapes that are not a request at all. A value is dumb, so each of these
# is built without complaint and refused when it is read.
for name, request in [
    (
        "rebuild_not_iterable",
        collomatique.GroupListsGenerationRequest(rebuild=3),
    ),
    (
        "rebuild_not_pairs",
        collomatique.GroupListsGenerationRequest(rebuild={sortileges}),
    ),
    (
        "kept_not_iterable",
        collomatique.GroupListsGenerationRequest(kept_lists=3),
    ),
]:
    refused(name, request, TypeError)

assert messages["rebuild_not_iterable"] == (
    "a GroupListsGenerationRequest's rebuild is a set of (period, subject) pairs, "
    "and 3 cannot be iterated over"
)
assert messages["rebuild_not_pairs"].startswith(
    "a GroupListsGenerationRequest's rebuild holds (period, subject) pairs, and "
)
assert messages["kept_not_iterable"] == (
    "a GroupListsGenerationRequest's kept_lists is a set of entities, "
    "and 3 cannot be iterated over"
)

# ----------------------------------------------------- what the door refuses

result = doc.generate_group_lists(
    collomatique.GroupListsGenerationRequest(rebuild={(p1, sortileges)})
)
generated = result.entries[0][0]


def landing(name, entries, exception):
    try:
        doc.group_lists.add_generated(entries)
    except exception as error:
        messages[name] = str(error)
    else:
        raise AssertionError(f"`{name}` must be refused")


landing("entries_not_iterable", 3, TypeError)
assert messages["entries_not_iterable"] == (
    "add_generated takes a list of (GroupListData, coverage) pairs, "
    "and 3 cannot be iterated over"
)

landing("entries_not_pairs", [generated], TypeError)
assert messages["entries_not_pairs"].startswith(
    "add_generated holds pairs of a GroupListData and its (period, subject) coverage, and "
)

landing("coverage_not_iterable", [(generated, 3)], TypeError)
assert messages["coverage_not_iterable"] == (
    "an entry's coverage is a set of (period, subject) pairs, and 3 cannot be iterated over"
)

# A coverage the model refuses: Botanique needs no group list, so it takes no
# association either. This one is the write's own refusal, and it reaches a
# script as the family's exception.
landing(
    "coverage_without_interrogations",
    [(generated, {(p1, botanique)})],
    collomatique.GroupListsError,
)

# None of the refusals left a list behind.
assert len(doc.group_lists) == 2
assert doc.can_undo is False

# ------------------------------------------------------- a callback that raises


class Boom(Exception):
    pass


seen = []


def angry(line):
    seen.append(line)
    raise Boom(line)


try:
    doc.generate_group_lists(
        collomatique.GroupListsGenerationRequest(rebuild={(p1, sortileges)}),
        on_log=angry,
    )
except Boom as error:
    raised = str(error)
else:
    raise AssertionError("a log callback that raises must be heard")

# Once, and never again: the generation was not torn in half, it ran to its end
# with nobody listening, and the exception came out afterwards with no result.
assert len(seen) == 1
assert raised == seen[0]
assert doc.can_undo is False
assert len(doc.group_lists) == 2
