"""The values a script builds, and what a read hands back detached.

`docs/python/values.md` is the design. §2 of `docs/python/new_api_design.md`
says why these are python dataclasses rather than rust classes: a value nests
and holds real mutable containers, and a pyo3 getter hands back a *clone* of
the struct it holds — so `value.nested.field = x` would quietly write to a
temporary that is thrown away. A dataclass has no such trap.

A value is dumb. It stores what it is given and checks nothing, so
`TeacherData("", "")` and `d.tel = 42` both simply work here. The checking
happens when the value is used — when a mutator extracts it — because that is
the last moment at which a message can still name the field that was wrong.

This file is never imported from disk. It is compiled from a string while
`collomatique` initializes, registered in `sys.modules` as `collomatique._data`,
and every name in `__all__` is re-exported into `collomatique` itself. So a
script writes `clm.TeacherData` and never names this module.

`from __future__ import annotations` is what makes the hints below legal. They
are strings that are never evaluated (PEP 563), so they may name `Subject`,
`SubjectId` and the other rust classes — which cannot be imported at runtime
here, since `collomatique` is still initializing while this file is compiled.
The `TYPE_CHECKING` block below is what makes those names resolvable to a type
checker and to a linter, and it runs on neither python's part nor ours.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collomatique import Period, PeriodId, Subject, SubjectId

__all__ = [
    "TeacherData",
    "StudentData",
]


@dataclass
class TeacherData:
    """A teacher, detached from the document.

    `doc.teachers[...].to_data()` hands one back, and the teacher mutators will
    take one:

        clm.TeacherData("Emmy", "Noether", email="noether@lycee.fr",
                        subjects={maths})

    `firstname` and `surname` are plain strings, the empty one included: the
    model types them that way, and python mirrors it rather than editorializing.

    `tel` and `email` are a non-empty string or `None`. Somebody who shared no
    number has none, rather than having an empty one, so `""` is refused when
    the value is used.

    `subjects` is the set of subjects this teacher interrogates in. It takes
    `Subject` handles and `SubjectId`s interchangeably, in any mix, like every
    other place in this API that names an entity; `to_data()` always fills it
    with ids, so that a value carries no document around with it. Two values
    naming the same subject, one by handle and one by id, therefore do *not*
    compare equal — a dataclass stores what it was given, and a handle and an id
    hash differently.

    A teacher may only be declared in a subject that holds interrogations. That
    is a statement about the document rather than about this value, so it is the
    write that refuses it, not this class.
    """

    firstname: str
    surname: str
    tel: str | None = None
    email: str | None = None
    subjects: set[Subject | SubjectId] = field(default_factory=set)


@dataclass
class StudentData:
    """A student, detached from the document.

    The same card as `TeacherData`, with a different set at the end:

        clm.StudentData("Harry", "Potter", tel="0601020304",
                        excluded_periods={first_period})

    `firstname`, `surname`, `tel` and `email` behave exactly as they do on a
    `TeacherData`, and for the same reasons: the model keeps one card for both
    kinds of person.

    `excluded_periods` is the set of periods this student takes no part in. It
    takes `Period` handles and `PeriodId`s interchangeably, and `to_data()`
    fills it with ids.

    Which subjects a student takes is not here. The model keeps that in a
    junction table of its own, keyed by period and subject, which python reads
    and writes as `doc.assignments`.
    """

    firstname: str
    surname: str
    tel: str | None = None
    email: str | None = None
    excluded_periods: set[Period | PeriodId] = field(default_factory=set)
