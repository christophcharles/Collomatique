# TODO: Expose the violated constraints (blame) to Python

The list of constraints a colloscope violates is not reachable from Python.
`--debug {checker,full}-blame[-max]` was the only door and it is gone.

The primitives exist: `Solution::blame` (unfiltered) and
`Solution::minimal_blame` (redundant violations removed via violation
implication) in `generic/ilp-modeler/src/lib.rs`, on the `Solution` returned by
`Model::checker_solution_from_data` or `Model::solution_from_data`.
`ConstraintDesc::user_readable` renders one in French, and
`ConstraintDesc::severity_level` sorts them.

The GUI already does exactly this: `launch_ilp_repr` in
`colloscopes/gtk4/src/editor/colloscope.rs` runs the checker reconstruction and
turns `minimal_blame` into the warnings list. That is the shape to follow — but
it runs the solver *in the process*, which is what
[running solvers in a separate process](todo_solver_separate_process.md) aims to
remove. So this waits on that todo: a Python call that cannot be interrupted or
discarded is not worth having.

Open questions:

- unfiltered blame, minimal blame, or both;
- whether it hangs off the document or off a built model (the checker
  reconstruction needs a model, so probably the latter);
- what a violated constraint looks like on the Python side: the rendered French
  string, a structured dataclass, or both.
