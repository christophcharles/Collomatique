import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

patterns = doc.week_patterns
assert isinstance(patterns, collomatique.WeekPatterns)

pattern_list = list(patterns)
assert len(pattern_list) == len(patterns)
assert all(isinstance(pattern, collomatique.WeekPattern) for pattern in pattern_list)

# Iteration is id order — the one order the document has for the patterns, since
# the model keeps no display order for them. So there is no `.index` either:
# there would be nothing for it to be the position in.
assert [pattern.id for pattern in pattern_list] == sorted(
    pattern.id for pattern in pattern_list
)
assert not hasattr(pattern_list[0], "index")

# The name is a plain string, empty allowed: the model types it that way.
pattern_names = [pattern.name for pattern in pattern_list]
assert all(isinstance(name, str) for name in pattern_names)

# What a pattern switches off comes back as a frozenset of live handles: they are
# this document's weeks, not positions copied out of it.
assert all(isinstance(pattern.excluded_weeks, frozenset) for pattern in pattern_list)
assert all(
    week in doc.weeks for pattern in pattern_list for week in pattern.excluded_weeks
)

weeks = list(doc.weeks)
positions = {week: index for index, week in enumerate(weeks)}
pattern_excluded_week_indices = [
    sorted(positions[week] for week in pattern.excluded_weeks)
    for pattern in pattern_list
]

# The merged answer, over every (week, pattern) pair the document has — the
# `None` column included, which is what a slot carrying no pattern asks: only the
# week's own flag counts there.
columns = [None] + pattern_list
activity = [
    [doc.is_week_active(week, pattern) for pattern in columns] for week in weeks
]
assert all(isinstance(answer, bool) for row in activity for answer in row)

# `None` is also the default, so a week on its own is asked with one argument.
assert [doc.is_week_active(week) for week in weeks] == [row[0] for row in activity]

# Both arguments take a handle or an id, and answer the same either way.
assert [
    [
        doc.is_week_active(week.id, None if pattern is None else pattern.id)
        for pattern in columns
    ]
    for week in weeks
] == activity

# Indexing takes an id or a handle, and hands back an equal handle either way.
for pattern in pattern_list:
    assert patterns[pattern.id] == pattern
    assert patterns[pattern] == pattern
    assert patterns.get(pattern.id) == pattern
    assert pattern.id in patterns
    assert pattern in patterns

assert 3 not in patterns
assert patterns.get(3) is None
try:
    patterns[3]
except KeyError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# A handle is a view, not the object the collection keeps: two of them for the
# same pattern are different objects that compare and hash the same. So a set or
# a dict can be keyed on patterns, which is what a script grouping slots by the
# weeks they run on does.
again = patterns[pattern_list[0].id]
assert again is not pattern_list[0]
assert hash(again) == hash(pattern_list[0])
assert len({again, pattern_list[0]}) == 1
assert len(set(pattern_list)) == len(pattern_list)

by_handle = {pattern: pattern.name for pattern in pattern_list}
assert len(by_handle) == len(pattern_list)
assert by_handle[again] == pattern_list[0].name

# A pattern is never equal to something that is not one — that is an answer and
# not an error, so `!=` against a stranger is simply true. Handles identify; they
# do not order, which is what ids are for.
assert pattern_list[0] != 3
assert pattern_list[0] != "Semaines paires"
assert not (pattern_list[0] == weeks[0])
assert pattern_list[0] != None  # noqa: E711 — `is not` would not call `__eq__`
try:
    pattern_list[0] < pattern_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.WeekPattern()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")
try:
    pattern_list[0].name = "Semaines impaires"
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# What is not a reference at all was never a question about this document, so it
# is a `TypeError` rather than a stale anything.
for rubbish in (3, "semaines paires", pattern_list[0]):
    try:
        doc.is_week_active(rubbish)
    except TypeError:
        pass
    else:
        raise AssertionError("a week argument takes a Week or a WeekId")
for rubbish in (3, "semaines paires", weeks[0]):
    try:
        doc.is_week_active(weeks[0], rubbish)
    except TypeError:
        pass
    else:
        raise AssertionError("a pattern argument takes a WeekPattern or an id")

# A handle from another document names nothing here, whatever its id says — and
# the two conventions of the api part company on it: a lookup answers, an
# argument raises.
other = collomatique.load(source)
other_pattern = list(other.week_patterns)[0]
assert other_pattern not in patterns
assert patterns.get(other_pattern) is None
assert other.week_patterns[other_pattern.id] == other_pattern
try:
    patterns[other_pattern]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")


# The refusal says *why*, and here the reason matters: `other` is this same file
# loaded twice, so its patterns carry the very ids this document uses. The
# handle is refused because it is somebody else's, and the message must not
# claim the id is missing here — it is not missing, it names another pattern
# altogether, and a script sent looking for a removal would find none.
assert other_pattern.id in doc.week_patterns
try:
    doc.is_week_active(weeks[0], other_pattern)
except collomatique.StaleHandleError as error:
    assert "WeekPattern" in str(error)
    assert "another document" in str(error)
    assert "is not in this document" not in str(error)
else:
    raise AssertionError("a pattern argument of another document must raise")
try:
    doc.is_week_active(list(other.weeks)[0])
except collomatique.StaleHandleError as error:
    assert "Week" in str(error)
    assert "another document" in str(error)
    assert "is not in this document" not in str(error)
else:
    raise AssertionError("a week argument of another document must raise")

# The reprs name the pattern the way a log wants to read it.
assert repr(pattern_list[0]).startswith("<WeekPattern #")
assert repr(pattern_list[0]).endswith(" %r>" % pattern_names[0])
assert repr(patterns) == "<collomatique.WeekPatterns count=%d>" % len(pattern_list)
