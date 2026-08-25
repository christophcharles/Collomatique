"""What a real colloscope is blamed for, and what a solved one is not.

Started as `collomatique --python-file <this>`, in a child process of its own.
The test hands it the two documents through `E2E_FIXTURE` and `E2E_SOLVED`; a
script run that way has no argv and no `__file__`, so the environment is how
they meet.

Everything this asserts, it asserts itself: the rust side reads only the exit
status. So an assertion message here is what a failing test will show, and they
are written to be read.

The two documents are the same hogwarts, one without a colloscope and one with
the colloscope a solve produced -- the second file is the first plus its
`Colloscope` block, and nothing else. So the two blames below are the same model
asked about two colloscopes, which is what makes them worth comparing.

A blame runs a real engine: the constraints are written against helper variables
a colloscope does not carry, and filling those in takes a solver. The child
running this script *is* collomatique, so the engine is the rung the runner
injected -- the ordinary case, and the one the application itself is on.
"""

import os
import sys

import collomatique as clm

# The test reads this script's output through a pipe, and a pipe is what makes
# python buffer a whole block at a time. An assertion below ends the run before
# such a block is ever flushed, so the lines that would say what happened just
# before it would be the ones lost.
sys.stdout.reconfigure(line_buffering=True)

fixture = os.environ["E2E_FIXTURE"]
solved_path = os.environ["E2E_SOLVED"]


def model_of(path):
    """The document at `path`, and its problem under the default config."""

    doc = clm.load(path)

    # The fixtures are frozen, so a caveat means this build no longer reads one
    # whole -- which is a thing to know loudly, and the day to take fresh
    # copies.
    assert not doc.caveats, f"{path} should read whole, got {doc.caveats}"

    return doc, doc.build_colloscope_model(clm.ColloscopeSolveConfig())


def summary(violations):
    """One line per severity present, for the failure that shows it."""

    counts = {}
    for violation in violations:
        counts[violation.severity] = counts.get(violation.severity, 0) + 1
    return ", ".join(f"{count} x {level!r}" for level, count in sorted(counts.items()))


# ------------------------------------------------- the document with no answer

doc, model = model_of(fixture)

assert not list(doc.colloscope.interrogations()), "the fixture was never solved"

violations = model.blame(doc.colloscope.to_data())
print(f"empty colloscope: {len(violations)} violation(s) -- {summary(violations)}")

# An empty colloscope breaks the rules about what a colloscope should hold: this
# document's group lists are all prefilled, so what is missing is the colles
# themselves -- nobody is interrogated in anything, and every rule about how
# often somebody should be has something to say about that.
assert violations, "an empty colloscope satisfies nothing, so it is blamable"

# But it is not *impossible*: nothing in this document contradicts itself, and
# no pin was asked for, so the two levels above STRUCTURAL must not appear. A
# FIXED here would mean a pin nobody set; an INFEASIBILITY would mean the
# document cannot be solved at all -- and the solved fixture below is the proof
# that it can.
worst = min(violation.severity for violation in violations)
assert worst >= clm.SeverityLevel.STRUCTURAL, (
    f"an unsolved but solvable document should not be blamed above STRUCTURAL, "
    f"got {worst!r}: "
    + "; ".join(str(v) for v in violations if v.severity < clm.SeverityLevel.STRUCTURAL)
)

# Worst first, on real data, and every violation says something.
severities = [violation.severity for violation in violations]
assert severities == sorted(severities), f"a blame comes back sorted: {severities}"
for violation in violations:
    assert str(violation) == violation.message
    assert violation.message.strip(), f"a violation with nothing to say: {violation!r}"

print(f"worst: {worst!r} -- {violations[0]}")

# ---------------------------------------------------- and the document with one

solved_doc, solved_model = model_of(solved_path)

assert list(solved_doc.colloscope.interrogations()), (
    "the solved fixture holds a colloscope"
)

solved_violations = solved_model.blame(solved_doc.colloscope.to_data())
print(f"solved colloscope: {len(solved_violations)} violation(s)")

assert solved_violations == [], (
    "a solved colloscope breaks nothing, got "
    + "; ".join(f"{v.severity!r}: {v}" for v in solved_violations)
)

# The same colloscope judged by the other document's model: the two files are
# one document apart from the `Colloscope` block, so the answer is the same one.
assert model.blame(solved_doc.colloscope.to_data()) == []
