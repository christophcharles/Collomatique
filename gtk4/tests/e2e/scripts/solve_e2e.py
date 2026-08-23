"""The one script the end-to-end solve tests run.

It is started as `collomatique --python-file <this>`, once per test, in a child
process of its own. A script run that way has no argv and no `__file__`, so the
test chooses what it should do through the `E2E_MODE` environment variable, and
hands over an engine path -- where the test is about one -- through `E2E_ENGINE`.

Everything this asserts, it asserts itself: the rust side reads only the exit
status, which is zero exactly when every assertion below held. So an assertion
message here is what a failing test will show, and they are written to be read.

The document is built from nothing rather than loaded from a fixture file, so
the problem stays small enough to solve in a moment: one period of two weeks,
one subject interrogated every week, one teacher, four students in two prefilled
groups, and two slots. Two groups needing one colle a week each, and two slots a
week to hold them, leaves exactly one shape of answer -- which is what lets the
`full` mode check the cells it installs.
"""

import datetime
import os
import sys

import collomatique as clm

# The tests read this script's output through a pipe, and a pipe is what makes
# python buffer a whole block at a time. An assertion below ends the run before
# such a block is ever flushed, so the lines that would say what happened just
# before it would be the ones lost.
sys.stdout.reconfigure(line_buffering=True)

# The four the api can hand back. A stopped run is `NO_SOLUTION` when it found
# nothing, so this really is the whole vocabulary and not a subset of it.
EVERY_STATUS = frozenset(
    {
        clm.SolveStatus.OPTIMAL,
        clm.SolveStatus.FEASIBLE,
        clm.SolveStatus.NO_SOLUTION,
        clm.SolveStatus.ERROR,
    }
)


def tiny_document():
    """A document holding one small, feasible colloscope problem."""

    doc = clm.new_document()

    with doc.transaction("Fixture"):
        period = doc.periods.add(2).created

        maths = doc.subjects.add(
            clm.SubjectData(
                "Maths",
                interrogation=clm.InterrogationData(
                    students_per_group=(2, 2),
                    duration=60,
                    periodicity=clm.EveryNWeeks(1),
                ),
            )
        ).created

        teacher = doc.teachers.add(
            clm.TeacherData("Emmy", "Noether", subjects={maths})
        ).created

        students = [
            doc.students.add(clm.StudentData(f"Prénom{i}", f"Nom{i}")).created
            for i in range(4)
        ]

        for hour in (8, 10):
            doc.slots.add(
                clm.SlotData(maths, teacher, clm.Weekday.MONDAY, datetime.time(hour, 0))
            )

        # Prefilled, so the solver has only the colles to place and not the
        # groups as well: fewer variables, and a colloscope this script can
        # predict.
        group_list = doc.group_lists.add(
            clm.GroupListData(
                "Groupes",
                students_per_group=(2, 2),
                group_names=["A", "B"],
                filling=clm.PrefilledGroups(
                    ({students[0], students[1]}, {students[2], students[3]})
                ),
            )
        ).created

        for student in students:
            doc.assignments.set(period, maths, student, True)

        doc.group_lists.set_association(period, maths, group_list)

    return doc


def built_model(doc, on_log=None):
    """The colloscope problem of `doc`, with the default config."""

    return doc.build_colloscope_model(clm.ColloscopeSolveConfig(), on_log=on_log)


def engine_from_environment():
    """The `engine=` the test wants passed, or `None` for « say nothing ».

    An unset variable and an empty one mean the same thing here, which is what
    lets one mode serve both the test that passes an engine and the tests that
    leave the choice to the rungs below.
    """

    return os.environ.get("E2E_ENGINE") or None


def full():
    """The whole road: warnings, log, solve, install, and a stop."""

    preset = clm.ConductorStrategy.search()

    # Printed rather than asserted on: the preset's warnings are pinned by the
    # python module's own tests, and what matters here is that asking for them
    # from a real command line works at all.
    for warning in preset.warnings():
        print(f"warning: {warning}")

    doc = tiny_document()

    lines = []
    model = built_model(doc, on_log=lines.append)
    assert lines, "the build says something on the way"
    print(f"model: {model!r}")

    # No `engine=`: the child running this script *is* collomatique, so the
    # engine is the rung the runner injected.
    run = model.solve(preset, on_log=lines.append)
    outcome = run.wait()
    print(f"outcome: {outcome!r}")

    assert outcome.status in (clm.SolveStatus.OPTIMAL, clm.SolveStatus.FEASIBLE), (
        f"a feasible problem is solved, not {outcome.status!r}"
    )
    assert outcome.colloscope is not None, "a solved problem hands back a colloscope"

    # Waiting twice answers the very same object.
    assert run.wait() is outcome

    doc.colloscope.install(outcome.colloscope)

    # Two groups, one colle each a week, two slots to hold them: every week has
    # both groups interrogated once, one per slot.
    for week in doc.weeks:
        placed = [doc.colloscope.interrogation(slot, week) for slot in doc.slots]
        assert all(cell is not None for cell in placed), (
            f"week {week.index} leaves a slot empty: {placed}"
        )
        assert sorted(g for cell in placed for g in cell) == [0, 1], (
            f"week {week.index} does not interrogate each group once: {placed}"
        )

    # -------------------------------------------------------------- and a stop

    # `stop()` is called straight after the start rather than from the first
    # progress event: a problem this small may well finish before it reports
    # anything at all, and a stop that never happens would test nothing. What
    # the two race over is which of them the engine hears first, so the only
    # thing asserted is that the run answers -- with any of the four -- and does
    # not wedge.
    events = []
    stopped_run = model.solve(preset, on_progress=events.append)
    stopped_run.stop()
    stopped_outcome = stopped_run.wait()
    print(f"stopped: {stopped_outcome!r}, {len(events)} progress event(s)")

    assert stopped_outcome.status in EVERY_STATUS
    assert repr(stopped_run) == "<SolveRun: finished>"

    # Stopping a finished run does nothing, and says so by not raising.
    stopped_run.stop()


def engine_rung():
    """A solve, on whichever engine the rungs settle on.

    The engine is never named in the assertions: a solve that produced a
    colloscope is the proof that one was found and that it ran. Which rung
    answered is the test's business, and it arranges that from outside.
    """

    doc = tiny_document()
    model = built_model(doc)

    run = model.solve(clm.ConductorStrategy.search(), engine=engine_from_environment())
    outcome = run.wait()
    print(f"outcome: {outcome!r}")

    assert outcome.status in (clm.SolveStatus.OPTIMAL, clm.SolveStatus.FEASIBLE), (
        f"a feasible problem is solved, not {outcome.status!r}"
    )
    assert outcome.colloscope is not None, "a solved problem hands back a colloscope"


def no_engine():
    """Nothing names an engine, and the refusal says where to name one."""

    assert issubclass(clm.NoEngine, clm.SolveError)
    assert issubclass(clm.SolveError, clm.Error)

    doc = tiny_document()
    model = built_model(doc)

    try:
        model.solve(clm.ConductorStrategy.search())
    except clm.NoEngine as e:
        # Kept in a name of this scope's own: `e` is unbound at the end of the
        # clause, and the assertions below outlive it.
        error = e
    else:
        raise AssertionError("a solve with no engine anywhere must refuse")

    assert isinstance(error, clm.SolveError)

    message = str(error)
    print(f"refusal: {message}")

    # The two doors a script can use are both named, because a message that
    # only says « no engine » leaves the reader nowhere to go.
    assert "engine=" in message, message
    assert "COLLOMATIQUE_ENGINE" in message, message


def dead_engine():
    """An engine that names a path with nothing at it.

    The failure has two possible doors -- the spawn itself, or the wait that
    finds the engine gone -- and which one it comes out of is the process
    machinery's business, not the api's. Either is a `SolveError`, and neither
    is a `NoEngine`: something *was* named.
    """

    doc = tiny_document()
    model = built_model(doc)

    try:
        run = model.solve(clm.ConductorStrategy.search())
    except clm.SolveError as e:
        error = e
    else:
        try:
            run.wait()
        except clm.SolveError as e:
            error = e
        else:
            raise AssertionError("an engine that does not exist cannot solve")

    print(f"failure: {error}")
    assert not isinstance(error, clm.NoEngine), "an engine was named; it was dead"


MODES = {
    "full": full,
    "engine_rung": engine_rung,
    "no_engine": no_engine,
    "dead_engine": dead_engine,
}

mode = os.environ["E2E_MODE"]
MODES[mode]()
print(f"{mode}: ok")
