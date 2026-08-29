import dataclasses

import collomatique

# No document is opened here. The strategy family names no entity — it says
# how a solve is run, not what it recomputes — so it is the one family that
# crosses the boundary with nothing to resolve against.

# The field order of each class, which is what a positional call depends on.
assert [f.name for f in dataclasses.fields(collomatique.DefaultConfig)] == [
    "time_limit",
    "incumbent_time_limit",
]
assert [f.name for f in dataclasses.fields(collomatique.WarmStartConfig)] == [
    "time_limit",
]
assert [f.name for f in dataclasses.fields(collomatique.IncrementalConfig)] == [
    "l1_weight",
    "distance_tolerance",
    "epoch_time_limit",
    "epoch_incumbent_time_limit",
]
assert [f.name for f in dataclasses.fields(collomatique.FuzzyConfig)] == [
    "fuzzy_sigma",
    "find_closest_tolerance",
    "time_limit",
    "incumbent_time_limit",
]
assert [f.name for f in dataclasses.fields(collomatique.ConductorStrategy)] == [
    "worker_count",
    "default_config",
    "warm_start_config",
    "incremental_config",
    "fuzzy_config",
    "warm_start_incumbent",
]

# The classes are the module's, not a private submodule's, like every other
# value class.
for cls in (
    collomatique.DefaultConfig,
    collomatique.WarmStartConfig,
    collomatique.IncrementalConfig,
    collomatique.FuzzyConfig,
    collomatique.ConductorStrategy,
):
    assert cls.__module__ == "collomatique"

# The application's « Recherche simple »: one worker, warm-start only, and so
# a solve that finds a colloscope and stops there. Rust pins this against
# `ConductorStrategy::default()`, so the two sides cannot drift apart.
bare = collomatique.ConductorStrategy()
assert bare.worker_count == 1
assert bare.default_config is None
assert bare.warm_start_config == collomatique.WarmStartConfig()
assert bare.incremental_config is None
assert bare.fuzzy_config is None
# A run is handed no warm start from here, so this one changes nothing today.
# It is still the application's own default, on.
assert bare.warm_start_incumbent is True

# The enabled sub-config is a fresh object every time, and not one shared
# between every strategy ever built: it is mutable, so editing one strategy's
# must not edit another's.
assert collomatique.ConductorStrategy().warm_start_config is not bare.warm_start_config

# Every substrategy on, each with the defaults its own class comes with. Rust
# reads this one apart and pins each config against its own rust `Default`.
all_bare = collomatique.ConductorStrategy(
    default_config=collomatique.DefaultConfig(),
    warm_start_config=collomatique.WarmStartConfig(),
    incremental_config=collomatique.IncrementalConfig(),
    fuzzy_config=collomatique.FuzzyConfig(),
)

# The presets are built on the rust side and handed back as plain values, so
# what a classmethod answers is an ordinary strategy a script may edit.
search = collomatique.ConductorStrategy.search()
assert isinstance(search, collomatique.ConductorStrategy)
assert search == bare

optimize = collomatique.ConductorStrategy.optimize()
assert isinstance(optimize, collomatique.ConductorStrategy)
assert optimize.worker_count >= 1
assert optimize.default_config == collomatique.DefaultConfig()
# The warm start is off there: the incremental solve fills the same role, and
# better, so running both would be redundant work.
assert optimize.warm_start_config is None
assert optimize.incremental_config == collomatique.IncrementalConfig()
assert optimize.fuzzy_config == collomatique.FuzzyConfig()

# A strategy that says something about everything it can: every substrategy
# on, every limit written both ways round, and the two measurements at zero,
# which is a value they hold.
spelled_out = collomatique.ConductorStrategy(
    worker_count=3,
    default_config=collomatique.DefaultConfig(
        time_limit=600, incumbent_time_limit=120
    ),
    warm_start_config=collomatique.WarmStartConfig(time_limit=30),
    incremental_config=collomatique.IncrementalConfig(
        l1_weight=0.0,
        distance_tolerance=0.0,
        epoch_time_limit=45,
        epoch_incumbent_time_limit=None,
    ),
    fuzzy_config=collomatique.FuzzyConfig(
        fuzzy_sigma=0.0,
        find_closest_tolerance=2.5,
        time_limit=None,
        incumbent_time_limit=7,
    ),
    warm_start_incumbent=False,
)

# The strategies the boundary must refuse. They are built without complaint —
# that is the point of a dumb value — and rust extracts each one and reads the
# message.

# A solve runs on at least one worker.
no_worker = collomatique.ConductorStrategy(worker_count=0)
not_a_count = collomatique.ConductorStrategy(worker_count="x")

# A limit of zero seconds is refused rather than read as no limit at all:
# `None` is how that is said here.
zero_limit = collomatique.ConductorStrategy(
    warm_start_config=collomatique.WarmStartConfig(time_limit=0)
)
negative_limit = collomatique.ConductorStrategy(
    warm_start_config=collomatique.WarmStartConfig(time_limit=-5)
)
not_a_limit = collomatique.ConductorStrategy(
    warm_start_config=collomatique.WarmStartConfig(time_limit="x")
)

# A price the solver pays cannot be negative, and a measurement cannot be
# infinite.
negative_weight = collomatique.ConductorStrategy(
    incremental_config=collomatique.IncrementalConfig(l1_weight=-1.0)
)
infinite_sigma = collomatique.ConductorStrategy(
    fuzzy_config=collomatique.FuzzyConfig(fuzzy_sigma=float("inf"))
)

# And the ordinary shapes of wrong. A sub-config is read by its fields, not by
# its class, so what is refused is an object without them — and the refusal
# names the path from the class the script wrote down.
not_a_config = collomatique.ConductorStrategy(default_config=3)


class Half:
    """Half of a `DefaultConfig`, which duck typing lets through until it is
    read."""

    incumbent_time_limit = 60

    def __repr__(self):
        return "a half-written config"


half_a_config = collomatique.ConductorStrategy(default_config=Half())
