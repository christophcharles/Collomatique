import collomatique

# `source` is a document rust built for this script: the example has no pattern
# that excludes nothing, none without a name, and none that switches every week
# off — and those are the three ends of what a pattern can be.
doc = collomatique.load(source)

patterns = list(doc.week_patterns)
weeks = list(doc.weeks)
every, even, unnamed, never = patterns

assert [pattern.name for pattern in patterns] == [
    "Toutes les semaines",
    "Semaines paires",
    "",
    "Aucune semaine",
]

# An unnamed pattern reads as `""` and not as `None`: the model types this field
# as a plain string, and python mirrors it rather than editorializing.
assert unnamed.name == ""
assert isinstance(unnamed.name, str)

# A pattern *is* its exception set, and the set runs from empty to whole.
positions = {week: index for index, week in enumerate(weeks)}
assert every.excluded_weeks == frozenset()
assert sorted(positions[week] for week in even.excluded_weeks) == [1, 3, 5]
assert sorted(positions[week] for week in unnamed.excluded_weeks) == [0]
assert never.excluded_weeks == frozenset(weeks)

# The weeks' own flags, which is the other half of the merged answer.
assert [week.interrogations for week in weeks] == [True, False, True, True, True, False]

# A pattern may switch off a week that holds no colles anyway: the model keeps
# the two apart, so a week switched back on finds its patterns as they were.
assert not weeks[1].interrogations
assert weeks[1] in even.excluded_weeks

# The whole grid, week by week and pattern by pattern. A `False` has two reasons
# here — the week holds no colles at all, or this pattern excludes it — and the
# point of the fixture is that both reasons appear, and that the two are
# independent.
expected = [
    # None   every   even    unnamed never
    [True, True, True, False, False],
    [False, False, False, False, False],
    [True, True, True, True, False],
    [True, True, False, True, False],
    [True, True, True, True, False],
    [False, False, False, False, False],
]
columns = [None] + patterns
assert [
    [doc.is_week_active(week, pattern) for pattern in columns] for week in weeks
] == expected

# `pattern=None` asks about no pattern at all, which is what a slot carrying none
# means, and it is the default.
assert [doc.is_week_active(week) for week in weeks] == [row[0] for row in expected]

# A pattern that excludes nothing answers exactly what the week does on its own,
# and one that excludes every week answers no whatever the week says.
assert [doc.is_week_active(week, every) for week in weeks] == [
    doc.is_week_active(week) for week in weeks
]
assert not any(doc.is_week_active(week, never) for week in weeks)

# `other_source` is a copy of the example, whose patterns are numbered nowhere
# near this document's. An id knows no document, so an id from over there is a
# perfectly good `WeekPatternId` that names nothing here — and this is where the
# two lookup conventions part company. A lookup is a
# legitimate question and answers in python's mapping vocabulary:
other = collomatique.load(other_source)
other_pattern = list(other.week_patterns)[0]

for foreign in (other_pattern, other_pattern.id):
    assert foreign not in doc.week_patterns
    assert doc.week_patterns.get(foreign) is None
    try:
        doc.week_patterns[foreign]
    except KeyError:
        pass
    else:
        raise AssertionError("a pattern of another document must not resolve")

# ... while an *argument* naming nothing was malformed before it had an answer,
# so it raises. Same two references, the other side of the line — and the two
# refusals do not have the same reason. The id is simply not here:
try:
    doc.is_week_active(weeks[0], other_pattern.id)
except collomatique.StaleHandleError as error:
    assert "WeekPattern" in str(error)
    assert repr(other_pattern.id) in str(error)
    assert "is not in this document" in str(error)
else:
    raise AssertionError("a pattern id of another document must raise")

# ... whereas the handle is refused for carrying its own document, whatever its
# id says — the reason that stays true when the two documents number their
# patterns alike, as two loads of one file do.
try:
    doc.is_week_active(weeks[0], other_pattern)
except collomatique.StaleHandleError as error:
    assert "WeekPattern" in str(error)
    assert "another document" in str(error)
else:
    raise AssertionError("a pattern handle of another document must raise")

# And the same the other way round, since neither document holds the other's
# numbers. The handle resolves where it belongs, which is what says the refusals
# above are about the document and not about the id.
assert every.id not in other.week_patterns
assert other.week_patterns[other_pattern.id] == other_pattern
assert doc.week_patterns[every.id] == every

# The reprs, including the one for a pattern with no name to show.
assert repr(even).endswith(" 'Semaines paires'>")
assert repr(unnamed).endswith(" ''>")
assert repr(doc.week_patterns) == "<collomatique.WeekPatterns count=4>"
