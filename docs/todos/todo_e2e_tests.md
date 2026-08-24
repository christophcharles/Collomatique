# TODO: End-to-end tests

We are lacking end-to-end tests, particularly around the Python API. Once
[the CLI revamp](todo_cli.md) lands and gives us a simple way to run Python
scripts against files, we should add tests that drive the whole software through
Python scripts.

There are at least four categories:

- **Solver on known fixtures** — run the solver on fixed inputs and check the
  results.
- **Import scripts with fictitious data** — exercise the import scripts (e.g.
  Pronote) against made-up data.
- **Complex ops on fixtures** — check involved operations on fixtures, such as
  the period-merge case (once a data-losing bug, now pinned green by
  `colloscopes/ops/tests/general_planning_content.rs`).
- **Data readable from Python is right** — with known fixtures, check that the
  data exposed to Python matches what is expected.

Note: some of these might not require Python at all.
