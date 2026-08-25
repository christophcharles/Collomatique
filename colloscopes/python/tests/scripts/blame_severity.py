import collomatique

severity = collomatique.SeverityLevel

# The six levels a violation can carry: the model's own five, and the pin of the
# solve configuration on top of them.
levels = [
    severity.FIXED,
    severity.INFEASIBILITY,
    severity.STRUCTURAL,
    severity.QUALITY,
    severity.PROGRESSIVE,
    severity.PREFERENCE,
]

assert len(set(levels)) == 6

# The names a script writes are the names it reads back.
names = [repr(level) for level in levels]

# Worst first, in the order they are written above: comparing is what a script
# does with a severity, and sorting a blame by it is what it is for.
assert levels == sorted(levels)
assert severity.FIXED < severity.INFEASIBILITY < severity.STRUCTURAL
assert severity.STRUCTURAL < severity.QUALITY < severity.PROGRESSIVE
assert severity.PROGRESSIVE < severity.PREFERENCE

shuffled = [
    severity.QUALITY,
    severity.PREFERENCE,
    severity.FIXED,
    severity.STRUCTURAL,
]
assert sorted(shuffled)[0] == severity.FIXED
assert min(shuffled) == severity.FIXED
assert max(shuffled) == severity.PREFERENCE

# A level compares equal to itself and to nothing else.
assert severity.FIXED == severity.FIXED
assert severity.FIXED != severity.PREFERENCE

# And it hashes, so a script can count a blame by severity.
assert len({level: index for index, level in enumerate(levels)}) == 6
