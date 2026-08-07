# FIXME: a student excluded from an automatic group list breaks the model build

`build_model` panics on a valid document when a student is assigned to a subject
but excluded from the automatic group list associated to that (period, subject):

```
ExtraError(StudentAtInterrogation { student: 7, slot: 19, week: 0 },
  UndeclaredVariable(Extra(StudentAtInterrogationInGroup {
    student: 7, slot: 19, week: 0, group_list: 46, group: 1 })))
```

Regression test: `builds_with_a_student_excluded_from_the_automatic_group_list`
in `constraints-colloscopes/tests/build_model_regression.rs`, fixture
`excluded_student_in_automatic_group_list.collomatique` (group list 46 is
`Automatic { excluded_students: [7, 11] }` and is associated to period 0 /
subject 13, where student 7 is assigned). Red today.

The mismatch is between two functions in `constraints-colloscopes/src/extras.rs`:

- `build_student_at_interrogation_in_group` declares the per-group variables only
  for `students_for_group_list`, which drops `excluded_students` on an automatic
  list;
- `build_student_at_interrogation`, in its `Automatic` branch, sums over those
  same variables for every student `is_student_enrolled` accepts — no exclusion
  check.

Same family as the `UndeclaredExtra` bugs audited in Aug 2026, new instance.

Established while investigating:

- Not a balancing problem. Same document, same `SolveConfig`, every balancing
  goal forced off: still panics. Forced soft everywhere: still panics.
- Not `SolveConfig`-dependent: the plain `build_model(&params)` panics too.
- The document is valid state: it comes from a `property_build` walk, which
  asserts `broken_invariants() == Ok({})` after every op.
- The fuzz net does not reach it on the current generator — 120 seeds instead of
  the usual 15 stayed green. It surfaced only after the balancing three-state
  change reshuffled the random stream.

Where to look next: which of the two sides is wrong. Either the declaration
should cover every enrolled student (and an excluded student's group variables
are then all infeasible), or the definition should treat an excluded student the
same way it treats a `Prefilled` list with no group for them — `infeasible()`,
which is the existing precedent a few lines above.
