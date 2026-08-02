# TODO: Timeout after first incumbent for solvers

It is sometimes useful to *wait* for an incumbent we are fairly sure is coming,
*but* to then have a timeout that stops us from optimizing it needlessly.

The work:

- Add a trait for this "timeout after first incumbent" behaviour and implement
  it for `collo-cbc`.
- Include the corresponding parameter in the relevant strategy parameters — in
  particular `strategies/src/strategies/incremental.rs`, which can otherwise get
  stuck optimizing needlessly.
- Add the matching GUI options in
  `gtk4/src/editor/run_solver/conductor_config.rs`.
