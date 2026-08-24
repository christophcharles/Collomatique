import collomatique
from collomatique import ConductorStrategy, ConductorWarning

# No document here either: what a strategy is looked over for is decided by the
# strategy alone, before anything is built.

# The eight warnings, in the order the model declares them — which is the order
# `warnings()` answers in, and the order the application lists them in.
NAMES = [
    "NO_STRATEGY_ENABLED",
    "NO_OPTIMIZING",
    "NO_SEED",
    "STARVED_FUZZY",
    "WONT_FINISH",
    "COLD_FUZZY",
    "REDUNDANT_WARM_START",
    "OVERWHELMED_CPU",
]

# They are class attributes of one class, like the days of `Weekday`, and each
# is its own member.
members = [getattr(ConductorWarning, name) for name in NAMES]
for member in members:
    assert isinstance(member, ConductorWarning)
assert len(set(members)) == len(NAMES)
assert ConductorWarning.__module__ == "collomatique"

# A member compares with itself, and hashes — a script may keep warnings in a
# set, and ask whether a particular one is in what it was handed.
assert ConductorWarning.NO_SEED == ConductorWarning.NO_SEED
assert ConductorWarning.NO_SEED != ConductorWarning.COLD_FUZZY
assert len({ConductorWarning.NO_SEED, ConductorWarning.NO_SEED}) == 1

# The name of each member, which is what rust reads a warning tuple back as.
LOOKUP = {member: name for member, name in zip(members, NAMES)}


def names(warnings):
    """The names of a warning tuple, in the order it came in."""
    assert isinstance(warnings, tuple)
    return tuple(LOOKUP[warning] for warning in warnings)


# Nothing enabled at all: the one warning that says the solve would do
# literally nothing.
nothing = ConductorStrategy(warm_start_config=None)
assert nothing.warnings() == (ConductorWarning.NO_STRATEGY_ENABLED,)

# The application's own « Recherche simple » warns, and is reported as it is:
# it finds a colloscope and never tries to improve it, which is what it is for
# and still worth saying.
search = ConductorStrategy.search()
assert search.warnings() == (ConductorWarning.NO_OPTIMIZING,)

# Fuzzy alone has nothing to start from.
fuzzy_only = ConductorStrategy(
    warm_start_config=None, fuzzy_config=collomatique.FuzzyConfig()
)
assert fuzzy_only.warnings() == (ConductorWarning.NO_SEED,)

# Two at once, and in declaration order: the single slot is taken by the
# default worker, and there is no incumbent to hand the fuzzers either.
cold = ConductorStrategy(
    worker_count=1,
    default_config=collomatique.DefaultConfig(),
    warm_start_config=None,
    fuzzy_config=collomatique.FuzzyConfig(),
)
assert names(cold.warnings()) == ("STARVED_FUZZY", "COLD_FUZZY")

# Three at once, still in declaration order, and one of them about the machine
# rather than about the shape: no solve gets 4096 slots here.
crowded = ConductorStrategy(
    worker_count=4096,
    warm_start_config=collomatique.WarmStartConfig(),
    incremental_config=collomatique.IncrementalConfig(),
    fuzzy_config=collomatique.FuzzyConfig(),
)
assert names(crowded.warnings()) == (
    "WONT_FINISH",
    "REDUNDANT_WARM_START",
    "OVERWHELMED_CPU",
)
assert ConductorWarning.WONT_FINISH in crowded.warnings()
assert ConductorWarning.NO_SEED not in crowded.warnings()

# A strategy with nothing wrong with it warns about nothing.
assert (
    ConductorStrategy(
        worker_count=1,
        default_config=collomatique.DefaultConfig(),
        warm_start_config=collomatique.WarmStartConfig(),
    ).warnings()
    == ()
)

# The « Optimisation complète » preset says nothing about its shape. Whether it
# says anything about this machine's cores depends on the machine, so rust
# reads the names and compares them with what the application's own structure
# warns about.
optimize_names = names(ConductorStrategy.optimize().warnings())

# The sentence of each warning, which rust pins against the one the
# application's dialog shows.
sentences = {name: str(member) for member, name in LOOKUP.items()}

# Asking a strategy that cannot be read at all what is wrong with it answers
# about the reading, not about the shape: `warnings()` extracts exactly as
# `solve` does.
try:
    ConductorStrategy(worker_count=0).warnings()
except ValueError as error:
    malformed = str(error)
else:
    raise AssertionError("a strategy with no worker is one the boundary refuses")
