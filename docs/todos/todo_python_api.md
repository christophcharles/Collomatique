# TODO: Revamp the Python API

Rework the entire Python API. The exposed structures should more closely mirror
the database data structures, and the Python copy semantics should be *way*
improved (clear, predictable value vs. reference behaviour).

The API should be **complete**: any modification of a file that is doable in the
GUI must also be doable from Python — including launching the solver.
