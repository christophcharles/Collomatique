import collomatique

# `source` is a document rust built for this script: the example has nobody who
# shared neither a number nor an email, no teacher who interrogates in nothing,
# and no student sitting a period out.
doc = collomatique.load(source)

teachers = list(doc.teachers)
students = list(doc.students)

assert [(teacher.firstname, teacher.surname) for teacher in teachers] == [
    ("Minerva", "McGonagall"),
    ("Severus", "Rogue"),
    ("Pomona", "Chourave"),
    ("Cuthbert", "Binns"),
]
assert [(student.firstname, student.surname) for student in students] == [
    ("Harry", "Potter"),
    ("Hermione", "Granger"),
    ("Ron", "Weasley"),
    ("Neville", "Londubat"),
]

mcgonagall, rogue, chourave, binns = teachers
potter, granger, weasley, londubat = students

# The four shapes a card can have, for a teacher and for a student alike. What
# was not shared is `None` — never `""`, and never a number that came from the
# neighbouring field.
assert mcgonagall.tel == "0700000021"
assert mcgonagall.email == "mcgonagall@poudlard.fr"
assert rogue.tel == "0700000022"
assert rogue.email is None
assert chourave.tel is None
assert chourave.email == "chourave@poudlard.fr"
assert binns.tel is None
assert binns.email is None

assert potter.tel == "0601020304"
assert potter.email == "harry.potter@poudlard.fr"
assert granger.tel == "0605060708"
assert granger.email is None
assert weasley.tel is None
assert weasley.email == "ron.weasley@poudlard.fr"
assert londubat.tel is None
assert londubat.email is None

for person in teachers + students:
    for value in (person.tel, person.email):
        assert value is None or (isinstance(value, str) and value != "")

# What a teacher interrogates in: a frozenset of live handles, empty for the one
# who interrogates in nothing.
sortileges, metamorphose = list(doc.subjects)
assert mcgonagall.subjects == frozenset({sortileges, metamorphose})
assert rogue.subjects == frozenset({sortileges})
assert chourave.subjects == frozenset({metamorphose})
assert binns.subjects == frozenset()

# The elements are handles and not names: they read the document through, so
# they answer the same as the subject the collection hands out.
assert sorted(subject.name for subject in mcgonagall.subjects) == [
    "Métamorphose",
    "Sortilèges",
]
assert all(subject in doc.subjects for subject in mcgonagall.subjects)

# The periods a student sits out, likewise — including the two extremes, the
# student who misses none and the one who misses every one of them.
first, second = list(doc.periods)
assert potter.excluded_periods == frozenset()
assert granger.excluded_periods == frozenset({first})
assert weasley.excluded_periods == frozenset({second})
assert londubat.excluded_periods == frozenset({first, second})
assert sorted(period.index for period in londubat.excluded_periods) == [0, 1]

# A frozenset is a snapshot of that moment, and a read never hands back anything
# mutable: there is nothing to add to and nothing to take away from.
try:
    mcgonagall.subjects.add(metamorphose)
except AttributeError:
    pass
else:
    raise AssertionError("a frozenset of handles must have nothing to add to")

# `other_source` is a copy of the example, whose people are numbered nowhere
# near this document's. An id knows no document, so an id from over there is a
# perfectly good `TeacherId` that names nothing here — and the mapping
# conventions are how a collection says so.
other = collomatique.load(other_source)
other_teacher = list(other.teachers)[0]
other_student = list(other.students)[0]

for collection, foreign in (
    (doc.teachers, other_teacher.id),
    (doc.students, other_student.id),
):
    assert foreign not in collection
    assert collection.get(foreign) is None
    try:
        collection[foreign]
    except KeyError:
        pass
    else:
        raise AssertionError("an id of another document must not resolve")

# And the same the other way round, since neither document holds the other's
# numbers. The handles themselves resolve where they belong, which is what says
# the two lookups above failed on the document and not on the ids.
assert mcgonagall.id not in other.teachers
assert other.students.get(potter.id) is None
assert other.teachers[other_teacher.id] == other_teacher
assert doc.students[potter.id] == potter

# The reprs name the person the way the application does — first name, then
# surname — since a repr exists to be read in a log.
assert repr(mcgonagall).startswith("<Teacher #")
assert repr(mcgonagall).endswith(" 'Minerva McGonagall'>")
assert repr(potter).endswith(" 'Harry Potter'>")
assert repr(doc.teachers) == "<collomatique.Teachers count=4>"
assert repr(doc.students) == "<collomatique.Students count=4>"
