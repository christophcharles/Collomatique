import dataclasses

import collomatique

# `source` is the generation fixture: two periods, five students, two subjects
# that run colles on groups of two to three, one that runs none, and one whose
# groups of exactly four nobody can fill. What each part of it is for is
# written out beside the fixture on the rust side; this script leans on the
# shapes and names its assertions after them.
doc = collomatique.load(source)

periods = list(doc.periods)
p1, p2 = periods[0], periods[1]

subjects = {subject.name: subject for subject in doc.subjects}
sortileges = subjects["Sortilèges"]
metamorphose = subjects["Métamorphose"]
botanique = subjects["Botanique"]
potions = subjects["Potions"]

maisons = next(gl for gl in doc.group_lists if gl.name == "Maisons")
automatique = next(gl for gl in doc.group_lists if gl.name == "Automatique")

# ------------------------------------------------------------- the default

default = doc.default_generation_request()
assert isinstance(default, collomatique.GroupListsGenerationRequest)

# Every pair that could take a list and has none. Botanique is absent
# throughout: a subject that runs no interrogations needs no group list.
assert default.rebuild == {
    (p1.id, metamorphose.id),
    (p1.id, potions.id),
    (p2.id, sortileges.id),
    (p2.id, metamorphose.id),
    (p2.id, potions.id),
}

# (period 1, Sortilèges) is missing because « Maisons » already serves it, and
# the two Potions pairs are *present* although no list can ever be built for
# them: the application's dialog offers them too and makes the user clear them,
# and this call is that dialog's own default. Feasibility is
# `generate_group_lists`'s gate, not this one's.
assert (p1.id, sortileges.id) not in default.rebuild

# Every prefilled list, and only those: an automatic one has no groups yet to
# respect.
assert default.kept_lists == {maisons.id}

# It holds ids, never handles — a detached value carries no document around.
assert all(
    isinstance(period, collomatique.PeriodId)
    and isinstance(subject, collomatique.SubjectId)
    for period, subject in default.rebuild
)
assert all(isinstance(gl, collomatique.GroupListId) for gl in default.kept_lists)

# An ordinary dataclass, and a fresh one every call: editing what one answered
# does not edit what the next one will.
assert [f.name for f in dataclasses.fields(default)] == ["rebuild", "kept_lists"]
assert collomatique.GroupListsGenerationRequest() == (
    collomatique.GroupListsGenerationRequest(rebuild=set(), kept_lists=set())
)
default.rebuild.clear()
assert doc.default_generation_request().rebuild

# --------------------------------------------------------------- generating

# Built by hand, and by handle as much as by id: a reference is a reference
# wherever this API takes one. Potions stays out — it is the error script's
# business.
request = collomatique.GroupListsGenerationRequest(
    rebuild={
        (p1, sortileges),
        (p1.id, metamorphose.id),
        (p2, sortileges.id),
        (p2.id, metamorphose),
    },
    kept_lists={maisons},
)

lines = []
result = doc.generate_group_lists(request, on_log=lines.append)

assert lines
assert all(isinstance(line, str) for line in lines)
assert all(line.startswith("[greedy]") for line in lines)

# Nobody takes Métamorphose on the second period, so no list is built for that
# pair. Not an error, and said structurally rather than in a sentence.
assert result.skipped == frozenset({(p2.id, metamorphose.id)})
assert isinstance(result.skipped, frozenset)

# Two lists for three buildable pairs: on period 1 the same five students take
# Sortilèges and Métamorphose with the same group sizes, so one list serves
# both. On period 2 Sortilèges is four students, which is a different list.
entries = result.entries
assert isinstance(entries, list)
assert len(entries) == 2

coverage = {
    entry[0].name: entry[1] for entry in entries
}
assert coverage == {
    "Sortilèges et Métamorphose (période 1)": frozenset(
        {(p1.id, sortileges.id), (p1.id, metamorphose.id)}
    ),
    "Sortilèges (période 2)": frozenset({(p2.id, sortileges.id)}),
}

# The lists themselves: prefilled, unnamed groups, and a real partition of the
# students the pair assigns, at sizes the subject allows.
# The pairs a request names, as the ids an entry reports them by.
asked = {
    (p1.id, sortileges.id),
    (p1.id, metamorphose.id),
    (p2.id, sortileges.id),
    (p2.id, metamorphose.id),
}

sizes = {}
for value, covered in entries:
    assert isinstance(value, collomatique.GroupListData)
    assert isinstance(value.filling, collomatique.PrefilledGroups)
    assert value.students_per_group == (2, 3)
    assert value.group_names == [None] * len(value.filling.groups)

    placed = [student for group in value.filling.groups for student in group]
    assert len(placed) == len(set(placed)), "no student sits in two groups"
    assert all(2 <= len(group) <= 3 for group in value.filling.groups)
    sizes[value.name] = sorted(len(group) for group in value.filling.groups)

    # An entry covers pairs the request asked for, and nothing else.
    assert covered <= asked

assert sizes == {
    "Sortilèges et Métamorphose (période 1)": [2, 3],
    "Sortilèges (période 2)": [2, 2],
}

# ------------------------------------------------- what rust compares against

# Ids are opaque, so what crosses back is what an id stands for: a period by
# its place in the document, a subject by its name. Rust runs the very same
# generator on the very same file and compares — which is what says this door
# is the application's generation and not a second one that looks like it.
index = {period.id: n for n, period in enumerate(periods)}
subject_name = {subject.id: subject.name for subject in doc.subjects}


def label(pair):
    period, subject = pair
    return f"{index[period]}:{subject_name[subject]}"


default_pair_labels = sorted(label(pair) for pair in doc.default_generation_request().rebuild)
default_kept_names = sorted(
    doc.group_lists[gl].name for gl in doc.default_generation_request().kept_lists
)
skipped_labels = sorted(label(pair) for pair in result.skipped)

# Read before the rename below, which is a script's edit and not the
# generator's answer.
entry_names = [value.name for value, _covered in entries]
entry_sizes = [
    sorted(len(group) for group in value.filling.groups) for value, _covered in entries
]

# The same document and the same request, so the same lists: the generator is
# not a random search that happens to have converged.
again = doc.generate_group_lists(request)
assert [(v.name, v.filling.groups) for v, _ in again.entries] == [
    (v.name, v.filling.groups) for v, _ in entries
]
assert again.skipped == result.skipped

# The repr says what it holds and names nothing inside it.
assert repr(result) == "<collomatique.GroupListsGenerationResult lists=2 skipped=1>"

# The entries are the result's own objects, handed back rather than rebuilt —
# which is what makes renaming in place work at all.
assert result.entries[0][0] is entries[0][0]

# A generation writes nothing. There is no undo slot because there was no
# operation, and the document still holds the two lists it was loaded with.
assert doc.can_undo is False
assert len(doc.group_lists) == 2
assert doc.group_lists.association_for(p1, sortileges) == maisons
assert doc.group_lists.association_for(p2, sortileges) is None

# ------------------------------------------------------------ landing them

# Renaming is editing the value, before it is landed. The other list keeps the
# name the generator gave it.
for value, _covered in entries:
    if value.name == "Sortilèges (période 2)":
        value.name = "Second semestre"

landed = doc.group_lists.add_generated(entries)

# A plain result: the op mints a list per entry and reports no id back, so
# there is no one created thing to hand over.
assert isinstance(landed, collomatique.OpResult)
assert not isinstance(landed, collomatique.AddResult)

# Nothing to repair: the fixture holds no colles for a new group bound to cut.
assert landed.warnings == []

names = {gl.name for gl in doc.group_lists}
assert names == {"Maisons", "Automatique", "Sortilèges et Métamorphose (période 1)", "Second semestre"}

first = next(gl for gl in doc.group_lists if gl.name == "Sortilèges et Métamorphose (période 1)")
second = next(gl for gl in doc.group_lists if gl.name == "Second semestre")

# Every covered pair points at the list built for it — including the pair
# « Maisons » used to serve, whose association is overwritten.
assert doc.group_lists.association_for(p1, sortileges) == first
assert doc.group_lists.association_for(p1, metamorphose) == first
assert doc.group_lists.association_for(p2, sortileges) == second

# Overwritten, not deleted: an orphaned list is an ordinary document.
assert maisons in doc.group_lists
assert maisons.name == "Maisons"

# One operation, and so one undo slot, however many lists it added.
doc.undo()
assert len(doc.group_lists) == 2
assert doc.group_lists.association_for(p1, sortileges) == maisons
assert doc.group_lists.association_for(p1, metamorphose) is None
assert doc.can_undo is False

# --------------------------------------------------- the door on its own

# The shape is what the door takes, not the class that produced it: entries a
# script builds by hand land the same way, with no generation behind them.
by_hand = doc.group_lists.add_generated(
    [
        (
            collomatique.GroupListData(
                "À la main",
                students_per_group=(2, 3),
                group_names=[None, None],
                filling=collomatique.PrefilledGroups(
                    (set(list(doc.students)[:2]), set(list(doc.students)[2:4]))
                ),
            ),
            {(p2, metamorphose)},
        )
    ]
)
assert isinstance(by_hand, collomatique.OpResult)
assert doc.group_lists.association_for(p2, metamorphose).name == "À la main"

doc.undo()
assert len(doc.group_lists) == 2

# The automatic list was never touched by any of this.
assert automatique.groups is None
