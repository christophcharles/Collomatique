# TODO: Preflight diagnostics (maybe 0.2.0)

Add **preflight diagnostics**: a way to check for obvious errors in the
constraints. Quick functions that run on the colloscope data and, using
heuristics, produce errors or warnings in a fraction of a second — so we do not
run the solver on an obviously inconsistent state.

These are distinct from the hard invariant/checker layer: that layer says a
state is *broken*, whereas diagnostics say a state is *probably wrong*. Severity
is error or warning.

Ideally this means:

- A new crate defining the `Diagnostic` trait plus a *lot* of implementations.
- The corresponding GUI — a new panel, or maybe a warning/error icon in the
  header bar that opens a window.
- The ability to add personalized diagnostics as **Python scripts that
  implement the `Diagnostic` trait** — added either manually or from a repo.
  Plan for debouncing and concurrent execution: these might run in a separate
  process that receives the new data with a debounce.
- As a smaller improvement, also expose a way to run the diagnostics *from*
  Python.
