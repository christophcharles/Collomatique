//! The values a read hands back that are not entities
//!
//! Small immutable classes for data the model
//! stores inline — no id, no place in a collection, nothing to go stale. They
//! are constructible, so a script names the value it expects and compares it
//! rather than picking it apart field by field, and construction validates what
//! the model validates.
//!
//! These are rust classes rather than the `.py` dataclasses of
//! `docs/python/new_api_design.md` §2, for the reason `caveats.rs` already
//! gives: that section's argument is about nested *mutable* copies, and it does
//! not apply to a flat immutable value that only ever travels out of rust.
//!
//! This module opens with the periodicity family, which is what the subjects
//! need, [TimeSlot], which the incompatibilities hand out, the settings
//! vocabulary — [Enforcement] and [Limit] — which the settings and balancing
//! read surfaces hand out, and [Color] and [Orientation], which the export
//! configuration hands out. The rest of the vocabulary lands with the
//! collections that hand it out.

use std::num::NonZeroU32;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use collomatique_state_colloscopes::export_config::{Color as RawColor, PageOrientation};
use collomatique_state_colloscopes::settings::SoftParam;
use collomatique_state_colloscopes::{NonEmptyRangeInclusive, SubjectPeriodicity};

/// A day of the week
///
/// The seven days are class attributes — `clm.Weekday.MONDAY` — and a read hands
/// back the member itself: pyo3 keeps one object per day, so two thursdays read
/// out of a document are the same object and even `is` compares them correctly.
/// `eq` and `hash` are declared all the same, so that a script comparing two days
/// keeps its answer if that ever stops being true.
///
/// The days are in the model's own order, monday first, which is the order the
/// application draws its grid in.
// The days travel both ways already: a read hands the member out, and a
// [TimeSlot]'s construction takes one back in.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    #[pyo3(name = "MONDAY")]
    Monday,
    #[pyo3(name = "TUESDAY")]
    Tuesday,
    #[pyo3(name = "WEDNESDAY")]
    Wednesday,
    #[pyo3(name = "THURSDAY")]
    Thursday,
    #[pyo3(name = "FRIDAY")]
    Friday,
    #[pyo3(name = "SATURDAY")]
    Saturday,
    #[pyo3(name = "SUNDAY")]
    Sunday,
}

impl Weekday {
    /// The python day for one model day
    ///
    /// Written as a match rather than as arithmetic on the day number, so that
    /// nothing here depends on which end of the week the underlying calendar
    /// counts from.
    pub(crate) fn from_model(weekday: collomatique_time::Weekday) -> Weekday {
        match weekday.into_inner() {
            chrono::Weekday::Mon => Weekday::Monday,
            chrono::Weekday::Tue => Weekday::Tuesday,
            chrono::Weekday::Wed => Weekday::Wednesday,
            chrono::Weekday::Thu => Weekday::Thursday,
            chrono::Weekday::Fri => Weekday::Friday,
            chrono::Weekday::Sat => Weekday::Saturday,
            chrono::Weekday::Sun => Weekday::Sunday,
        }
    }

    /// The model day for one python day
    ///
    /// The reverse of [from_model], written as a match for the same reason: a
    /// new day over there is a compile error here. A leaf value's construction
    /// is the first thing that takes a day back in; the write surface will be
    /// the second.
    pub(crate) fn to_model(self) -> collomatique_time::Weekday {
        match self {
            Weekday::Monday => collomatique_time::Weekday(chrono::Weekday::Mon),
            Weekday::Tuesday => collomatique_time::Weekday(chrono::Weekday::Tue),
            Weekday::Wednesday => collomatique_time::Weekday(chrono::Weekday::Wed),
            Weekday::Thursday => collomatique_time::Weekday(chrono::Weekday::Thu),
            Weekday::Friday => collomatique_time::Weekday(chrono::Weekday::Fri),
            Weekday::Saturday => collomatique_time::Weekday(chrono::Weekday::Sat),
            Weekday::Sunday => collomatique_time::Weekday(chrono::Weekday::Sun),
        }
    }

    /// The day's french name, capitalized — « Lundi »
    ///
    /// The model's own `capitalize` (`collomatique_time`) is what the
    /// application displays, so the reprs that name a day use the same word
    /// the user reads there.
    pub(crate) fn french(self) -> &'static str {
        self.to_model().capitalize()
    }
}

/// A `(min, max)` pair, inclusive at both ends
///
/// The model's `NonEmptyRangeInclusive` never leaks as a class: a range reads as
/// the tuple `docs/python/new_api_design.md` §14 already writes,
/// `students_per_group=(2, 3)`.
pub(crate) type Range = (u32, u32);

/// The range a model field holds
pub(crate) fn range(bounds: &NonEmptyRangeInclusive<u32>) -> Range {
    (*bounds.start(), *bounds.end())
}

/// The same, for a field the model counts with a non-zero type
///
/// Python sees plain ints either way: "at least one" is a rust way of storing an
/// invariant, and a script reading `(2, 3)` needs no vocabulary for it.
pub(crate) fn nonzero_range(bounds: &NonEmptyRangeInclusive<NonZeroU32>) -> Range {
    (bounds.start().get(), bounds.end().get())
}

/// Checks a `(min, max)` a script wrote down
///
/// The boundary of §6 applied one milestone early: a leaf value refuses
/// nonsense when it is built, so step 3 can take these objects as they are.
fn checked_range(what: &str, bounds: Range) -> PyResult<Range> {
    let (min, max) = bounds;
    if min > max {
        return Err(PyValueError::new_err(format!(
            "{what} is a (min, max) range, and {min} is above {max}"
        )));
    }
    Ok(bounds)
}

/// Checks a count the model stores as "at least one"
fn at_least_one(what: &str, value: u32) -> PyResult<NonZeroU32> {
    NonZeroU32::new(value)
        .ok_or_else(|| PyValueError::new_err(format!("{what} is at least 1, and 0 was given")))
}

/// One block of a custom periodicity
///
/// The blocks of a [CustomBlocks] run one after another: `delay_in_weeks` counts
/// from the end of the previous block, or from the start of the schedule for the
/// first one, and `count` says how many interrogations should fall inside.
///
/// ```python
/// clm.WeekBlock(0, 2, (1, 1))
/// ```
///
/// It is the one leaf value python passes back *in*, as the blocks of a
/// [CustomBlocks], so it opts into the extraction its `Clone` would once have
/// given it for nothing.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WeekBlock {
    delay_in_weeks: u32,
    size_in_weeks: NonZeroU32,
    count: Range,
}

#[pymethods]
impl WeekBlock {
    #[new]
    fn new(delay_in_weeks: u32, size_in_weeks: u32, count: Range) -> PyResult<WeekBlock> {
        Ok(WeekBlock {
            delay_in_weeks,
            size_in_weeks: at_least_one("a block's size_in_weeks", size_in_weeks)?,
            count: checked_range("a block's count", count)?,
        })
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("delay_in_weeks", "size_in_weeks", "count");

    /// Weeks between the previous block and this one, or before the first
    #[getter]
    fn delay_in_weeks(&self) -> u32 {
        self.delay_in_weeks
    }

    /// How many weeks the block lasts, at least one
    #[getter]
    fn size_in_weeks(&self) -> u32 {
        self.size_in_weeks.get()
    }

    /// How many interrogations should fall in the block, as a `(min, max)` range
    #[getter]
    fn count(&self) -> Range {
        self.count
    }

    fn __repr__(&self) -> String {
        format!(
            "WeekBlock(delay_in_weeks={}, size_in_weeks={}, count={:?})",
            self.delay_in_weeks,
            self.size_in_weeks.get(),
            self.count,
        )
    }
}

impl WeekBlock {
    /// The python block for one model block
    fn from_model(block: &collomatique_state_colloscopes::subjects::WeekBlock) -> WeekBlock {
        WeekBlock {
            delay_in_weeks: block.delay_in_weeks,
            size_in_weeks: block.size_in_weeks,
            count: range(&block.interrogation_count_in_block),
        }
    }
}

/// How often a subject's interrogations come round
///
/// The base class of the four periodicities, so `isinstance(p, Periodicity)`
/// catches all of them. It has no constructor of its own: every periodicity is
/// one of the subclasses, and `collomatique.Periodicity()` raises `TypeError`.
#[pyclass(module = "collomatique", subclass, frozen)]
pub struct Periodicity;

/// An interrogation every `n` weeks, strictly
///
/// The regularity is exact: a student interrogated on week 1 with `n=2` is next
/// interrogated on week 3, not merely somewhere in weeks 3 and 4.
#[pyclass(module = "collomatique", extends = Periodicity, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct EveryNWeeks {
    n: NonZeroU32,
}

/// One interrogation per block of `weeks_per_block` weeks
///
/// The blocks are regular, the placement inside them is not: with blocks of two
/// weeks, an interrogation on week 2 and the next on week 3 is allowed —
/// `minimum_week_separation` is what puts a floor under that.
#[pyclass(module = "collomatique", extends = Periodicity, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct OncePerBlock {
    weeks_per_block: NonZeroU32,
    minimum_week_separation: NonZeroU32,
}

/// A total number of interrogations over the year, placed freely
///
/// The most flexible of the four, and the one that can produce the most unequal
/// colloscopes: nothing but `minimum_week_separation` says when they happen.
#[pyclass(module = "collomatique", extends = Periodicity, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct CountInYear {
    count: Range,
    minimum_week_separation: u32,
}

/// A number of interrogations per block, with the blocks written out
///
/// The general form of [OncePerBlock]: the blocks are arbitrary rather than
/// regular, which is what a year with a handful of interrogations on irregular
/// dates needs.
#[pyclass(module = "collomatique", extends = Periodicity, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct CustomBlocks {
    blocks: Vec<WeekBlock>,
    minimum_week_separation: u32,
}

#[pymethods]
impl EveryNWeeks {
    #[new]
    fn new(n: u32) -> PyResult<PyClassInitializer<Self>> {
        Ok(EveryNWeeks {
            n: at_least_one("a periodicity in weeks", n)?,
        }
        .init())
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("n",);

    /// How many weeks apart two interrogations of a student are
    #[getter]
    fn n(&self) -> u32 {
        self.n.get()
    }

    fn __repr__(&self) -> String {
        format!("EveryNWeeks(n={})", self.n.get())
    }
}

#[pymethods]
impl OncePerBlock {
    #[new]
    fn new(
        weeks_per_block: u32,
        minimum_week_separation: u32,
    ) -> PyResult<PyClassInitializer<Self>> {
        Ok(OncePerBlock {
            weeks_per_block: at_least_one("a block's weeks_per_block", weeks_per_block)?,
            // The model types this one non-zero as well, and says why: a block
            // holds at most one interrogation, so two of them can never fall in
            // the same week anyway.
            minimum_week_separation: at_least_one(
                "a block periodicity's minimum_week_separation",
                minimum_week_separation,
            )?,
        }
        .init())
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) =
        ("weeks_per_block", "minimum_week_separation");

    /// How many weeks one block lasts
    #[getter]
    fn weeks_per_block(&self) -> u32 {
        self.weeks_per_block.get()
    }

    /// The floor under the gap between two interrogations of a student
    #[getter]
    fn minimum_week_separation(&self) -> u32 {
        self.minimum_week_separation.get()
    }

    fn __repr__(&self) -> String {
        format!(
            "OncePerBlock(weeks_per_block={}, minimum_week_separation={})",
            self.weeks_per_block.get(),
            self.minimum_week_separation.get(),
        )
    }
}

#[pymethods]
impl CountInYear {
    #[new]
    fn new(count: Range, minimum_week_separation: u32) -> PyResult<PyClassInitializer<Self>> {
        Ok(CountInYear {
            count: checked_range("a yearly interrogation count", count)?,
            minimum_week_separation,
        }
        .init())
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("count", "minimum_week_separation");

    /// How many interrogations over the year, as a `(min, max)` range
    #[getter]
    fn count(&self) -> Range {
        self.count
    }

    /// The floor under the gap between two interrogations of a student
    ///
    /// Zero is allowed here: nothing stops two of them falling in one week.
    #[getter]
    fn minimum_week_separation(&self) -> u32 {
        self.minimum_week_separation
    }

    fn __repr__(&self) -> String {
        format!(
            "CountInYear(count={:?}, minimum_week_separation={})",
            self.count, self.minimum_week_separation,
        )
    }
}

#[pymethods]
impl CustomBlocks {
    #[new]
    fn new(blocks: Vec<WeekBlock>, minimum_week_separation: u32) -> PyClassInitializer<Self> {
        CustomBlocks {
            blocks,
            minimum_week_separation,
        }
        .init()
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("blocks", "minimum_week_separation");

    /// The blocks, in order, as a tuple of [WeekBlock]
    #[getter]
    fn blocks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, self.blocks.iter().cloned())
    }

    /// The floor under the gap between two interrogations of a student
    #[getter]
    fn minimum_week_separation(&self) -> u32 {
        self.minimum_week_separation
    }

    fn __repr__(&self) -> String {
        let blocks: Vec<_> = self.blocks.iter().map(WeekBlock::__repr__).collect();
        format!(
            "CustomBlocks(blocks=({}{}), minimum_week_separation={})",
            blocks.join(", "),
            // A one-element python tuple keeps its trailing comma, and this repr
            // is meant to be pasteable.
            if blocks.len() == 1 { "," } else { "" },
            self.minimum_week_separation,
        )
    }
}

/// Pairs a value with its base class, which is how a subclass instance is built
///
/// The tuple form is deprecated in pyo3, as `caveats.rs` notes too.
macro_rules! init_as_periodicity {
    ($($name:ident),* $(,)?) => { $(
        impl $name {
            fn init(self) -> PyClassInitializer<Self> {
                PyClassInitializer::from(Periodicity).add_subclass(self)
            }
        }
    )* };
}

init_as_periodicity!(EveryNWeeks, OncePerBlock, CountInYear, CustomBlocks);

/// Builds the python periodicity for one model periodicity
///
/// Written as a match on the model enum rather than as a `From` impl per
/// variant, so that a new variant over there is a compile error here.
pub(crate) fn periodicity(py: Python<'_>, periodicity: &SubjectPeriodicity) -> PyResult<Py<PyAny>> {
    Ok(match periodicity {
        SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks,
        } => Py::new(
            py,
            EveryNWeeks {
                n: *periodicity_in_weeks,
            }
            .init(),
        )?
        .into_any(),
        SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block,
            minimum_week_separation,
        } => Py::new(
            py,
            OncePerBlock {
                weeks_per_block: *weeks_per_block,
                minimum_week_separation: *minimum_week_separation,
            }
            .init(),
        )?
        .into_any(),
        SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year,
            minimum_week_separation,
        } => Py::new(
            py,
            CountInYear {
                count: range(interrogation_count_in_year),
                minimum_week_separation: *minimum_week_separation,
            }
            .init(),
        )?
        .into_any(),
        SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks,
            minimum_week_separation,
        } => Py::new(
            py,
            CustomBlocks {
                blocks: blocks.iter().map(WeekBlock::from_model).collect(),
                minimum_week_separation: *minimum_week_separation,
            }
            .init(),
        )?
        .into_any(),
    })
}

/// A busy window: a day, a start time and a duration
///
/// The slots of an incompatibility read as
/// these values — « monday 12:00, one hour » — and a script that wants to name
/// the same window back builds one:
///
/// ```python
/// clm.TimeSlot(clm.Weekday.MONDAY, datetime.time(12, 0), 60)
/// ```
///
/// Construction validates what `SlotWithDuration::new` validates
/// (`collomatique_time`, the model's own type for this shape): the start time
/// must be a whole minute, the duration at least one minute, and the window
/// must not cross midnight into the next day — a window ending exactly at
/// midnight is fine. A window that refuses to exist raises `ValueError`.
///
/// It opts into extraction like [WeekBlock]: step 3's dataclasses will hold
/// these values in their fields, and the write surface will pass them back in.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TimeSlot {
    weekday: Weekday,
    start_time: chrono::NaiveTime,
    duration: NonZeroU32,
}

#[pymethods]
impl TimeSlot {
    #[new]
    fn new(weekday: Weekday, start_time: chrono::NaiveTime, duration: u32) -> PyResult<TimeSlot> {
        // The model's own three checks, in the model's own order: a whole
        // minute to start, at least one minute of duration, and no crossing of
        // the midnight boundary.
        let start_time = collomatique_time::WholeMinuteTime::new(start_time).ok_or_else(|| {
            PyValueError::new_err(
                "a TimeSlot's start_time must be a whole minute, with no seconds \
                     or microseconds",
            )
        })?;
        let duration = at_least_one("a TimeSlot's duration", duration)?;
        let start = collomatique_time::SlotStart {
            weekday: weekday.to_model(),
            start_time,
        };
        let _ = collomatique_time::SlotWithDuration::new(
            start,
            collomatique_time::NonZeroMinutes::from(duration),
        )
        .ok_or_else(|| {
            PyValueError::new_err("a TimeSlot cannot cross midnight into the next day")
        })?;

        Ok(TimeSlot {
            weekday,
            start_time: *start_time.inner(),
            duration,
        })
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("weekday", "start_time", "duration");

    /// The day of the week this window falls on
    #[getter]
    fn weekday(&self) -> Weekday {
        self.weekday
    }

    /// The time of day the window starts, as a `datetime.time`
    ///
    /// Whole minutes: the model stores the time with minute precision, so the
    /// seconds and the microseconds are always zero.
    #[getter]
    fn start_time(&self) -> chrono::NaiveTime {
        self.start_time
    }

    /// How long the window lasts, in minutes, at least one
    #[getter]
    fn duration(&self) -> u32 {
        self.duration.get()
    }

    fn __repr__(&self) -> String {
        format!(
            "TimeSlot(weekday={}, start_time={}, duration={})",
            self.weekday.french(),
            self.start_time.format("%H:%M"),
            self.duration.get(),
        )
    }
}

impl TimeSlot {
    /// The python window for one model window
    pub(crate) fn from_model(slot: &collomatique_time::SlotWithDuration) -> TimeSlot {
        TimeSlot {
            weekday: Weekday::from_model(slot.start().weekday),
            start_time: *slot.start().start_time.inner(),
            duration: slot.duration().get(),
        }
    }
}

/// Whether a goal is an objective or a hard constraint
///
/// One vocabulary for every `SoftParam` in the model:
/// a limit or a balancing goal is either
/// `OBJECTIVE` — the solver optimizes for it — or `STRICT` — a hard constraint.
/// `None` where the goal is not pursued at all is spelled by the read itself,
/// as an absent optional, never by a third member.
///
/// The two members are class attributes, like [Weekday]'s days:
/// `clm.Enforcement.STRICT` — pyo3 keeps one object per member, so two reads
/// of the same enforcement are the same object.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Enforcement {
    #[pyo3(name = "OBJECTIVE")]
    Objective,
    #[pyo3(name = "STRICT")]
    Strict,
}

/// The enforcement as the repr spells it, the way the class attribute is named
fn enforcement_spelling(enforcement: Enforcement) -> &'static str {
    match enforcement {
        Enforcement::Objective => "Enforcement.OBJECTIVE",
        Enforcement::Strict => "Enforcement.STRICT",
    }
}

impl std::fmt::Display for Enforcement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(enforcement_spelling(*self))
    }
}

#[pymethods]
impl Enforcement {
    fn __repr__(&self) -> String {
        enforcement_spelling(*self).to_owned()
    }
}

impl Enforcement {
    /// The python enforcement for one model `soft` flag
    pub(crate) fn from_model(soft: bool) -> Enforcement {
        if soft {
            Enforcement::Objective
        } else {
            Enforcement::Strict
        }
    }
}

/// One limit on a student's interrogations
///
/// A field of the settings [Limits] view: a
/// count and whether the count is an objective for the solver or a hard
/// constraint.
///
/// ```python
/// clm.Limit(3, clm.Enforcement.STRICT)
/// ```
///
/// The value is a plain count, and nothing about the range is checked at
/// construction: the model's per-week fields take zero, and the at-least-one
/// rule on `max_interrogations_per_day` is the model's own, enforced where the
/// model stores that field — step 3 refuses a zero there, not here.
///
/// [Limits]: crate::collections::settings::Limits
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Limit {
    value: u32,
    enforcement: Enforcement,
}

#[pymethods]
impl Limit {
    #[new]
    fn new(value: u32, enforcement: Enforcement) -> Limit {
        Limit { value, enforcement }
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("value", "enforcement");

    /// The count the limit sets
    #[getter]
    fn value(&self) -> u32 {
        self.value
    }

    /// Whether the count is an objective or a hard constraint
    #[getter]
    fn enforcement(&self) -> Enforcement {
        self.enforcement
    }

    fn __repr__(&self) -> String {
        format!(
            "Limit(value={}, enforcement={})",
            self.value,
            enforcement_spelling(self.enforcement),
        )
    }
}

/// The python limit for one model limit whose count is a plain int
pub(crate) fn limit(soft: &SoftParam<u32>) -> Limit {
    Limit {
        value: soft.value,
        enforcement: Enforcement::from_model(soft.soft),
    }
}

/// The same, for a count the model stores as at least one
pub(crate) fn nonzero_limit(soft: &SoftParam<NonZeroU32>) -> Limit {
    Limit {
        value: soft.value.get(),
        enforcement: Enforcement::from_model(soft.soft),
    }
}

/// Whether an exported sheet is printed tall or wide
///
/// The two members are class attributes, like [Weekday]'s days:
/// `clm.Orientation.PORTRAIT` — pyo3 keeps one object per member, so two reads
/// of the same orientation are the same object.
///
/// `repr` echoes the member's identifier, like [Enforcement]'s does; `str` is
/// the french word the application's own dropdown shows, « Portrait » or
/// « Paysage » — the caveat convention, a human word for the human reader.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    #[pyo3(name = "PORTRAIT")]
    Portrait,
    #[pyo3(name = "LANDSCAPE")]
    Landscape,
}

/// The orientation as `repr` spells it, the way the class attribute is named
fn orientation_spelling(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Portrait => "Orientation.PORTRAIT",
        Orientation::Landscape => "Orientation.LANDSCAPE",
    }
}

#[pymethods]
impl Orientation {
    fn __repr__(&self) -> String {
        orientation_spelling(*self).to_owned()
    }

    /// The french word the application shows in its orientation dropdown
    fn __str__(&self) -> &'static str {
        self.french()
    }
}

impl Orientation {
    /// The python orientation for one model orientation
    ///
    /// Written as a match rather than as a `From` impl, so that a new variant
    /// over there is a compile error here — the same reason the periodicity
    /// conversion is a match.
    pub(crate) fn from_model(orientation: &PageOrientation) -> Orientation {
        match orientation {
            PageOrientation::Portrait => Orientation::Portrait,
            PageOrientation::Landscape => Orientation::Landscape,
        }
    }

    /// The orientation's french word — « Portrait » or « Paysage »
    ///
    /// The word the application's dropdown shows, which is what `str()` hands
    /// back — the same choice as a [TimeSlot] repr naming its day « Lundi ».
    pub(crate) fn french(self) -> &'static str {
        match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Paysage",
        }
    }
}

/// A color, as its red, green and blue channels
///
/// A plain value a script builds and compares — the background of a sheet, the
/// tint of an annotation cell:
///
/// ```python
/// clm.Color(255, 255, 255)
/// ```
///
/// Construction validates what the model's own channels hold: each of the
/// three is 0-255, and a channel outside that raises `ValueError`. It opts
/// into extraction like [Limit], so step 3's dataclasses can hold it in their
/// fields and pass it back in.
#[pyclass(module = "collomatique", frozen, eq, hash, from_py_object)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

/// Checks one channel of a [Color]
///
/// Takes an `i64` rather than a `u32`, so that a negative channel reaches this
/// check instead of failing the conversion first: "0-255, and -1 was given" is
/// the whole truth, where an `OverflowError` about a negative number would
/// only be half of it.
fn channel(what: &str, value: i64) -> PyResult<u8> {
    u8::try_from(value).map_err(|_| {
        PyValueError::new_err(format!("a Color's {what} is 0-255, and {value} was given"))
    })
}

#[pymethods]
impl Color {
    #[new]
    fn new(red: i64, green: i64, blue: i64) -> PyResult<Color> {
        Ok(Color {
            red: channel("red", red)?,
            green: channel("green", green)?,
            blue: channel("blue", blue)?,
        })
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) = ("red", "green", "blue");

    /// How much red the color holds, 0-255
    #[getter]
    fn red(&self) -> u8 {
        self.red
    }

    /// How much green the color holds, 0-255
    #[getter]
    fn green(&self) -> u8 {
        self.green
    }

    /// How much blue the color holds, 0-255
    #[getter]
    fn blue(&self) -> u8 {
        self.blue
    }

    fn __repr__(&self) -> String {
        format!(
            "Color(red={}, green={}, blue={})",
            self.red, self.green, self.blue,
        )
    }
}

impl Color {
    /// The python color for one model color
    pub(crate) fn from_model(color: &RawColor) -> Color {
        Color {
            red: color.red,
            green: color.green,
            blue: color.blue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Weekday;

    /// Every day converts to its own member, and to no other
    ///
    /// The documents the scripts read only ever run colles from monday to
    /// friday, so a saturday swapped with a sunday would slip through them. The
    /// seven pairs are written out here, where no fixture is needed. The french
    /// name pinned is the model's own capitalized one, which is what a repr
    /// shows.
    #[test]
    fn every_day_converts_to_its_own_member() {
        let days = [
            (chrono::Weekday::Mon, Weekday::Monday, "Lundi"),
            (chrono::Weekday::Tue, Weekday::Tuesday, "Mardi"),
            (chrono::Weekday::Wed, Weekday::Wednesday, "Mercredi"),
            (chrono::Weekday::Thu, Weekday::Thursday, "Jeudi"),
            (chrono::Weekday::Fri, Weekday::Friday, "Vendredi"),
            (chrono::Weekday::Sat, Weekday::Saturday, "Samedi"),
            (chrono::Weekday::Sun, Weekday::Sunday, "Dimanche"),
        ];

        for (model, expected, name) in days {
            let converted = Weekday::from_model(collomatique_time::Weekday(model));
            assert!(converted == expected, "{name} should convert to itself");
            assert_eq!(
                converted.french(),
                name,
                "{name} should be named after itself"
            );
        }
    }

    /// Every day converts back to its own model day, and to no other
    ///
    /// The reverse of the conversion above, pinned the same way: a [TimeSlot]'s
    /// construction is the first thing that asks for it, and a day swapped for
    /// its neighbour would make a window validate on the wrong day.
    #[test]
    fn every_day_converts_back_to_its_own_member() {
        let days = [
            (Weekday::Monday, chrono::Weekday::Mon, "Lundi"),
            (Weekday::Tuesday, chrono::Weekday::Tue, "Mardi"),
            (Weekday::Wednesday, chrono::Weekday::Wed, "Mercredi"),
            (Weekday::Thursday, chrono::Weekday::Thu, "Jeudi"),
            (Weekday::Friday, chrono::Weekday::Fri, "Vendredi"),
            (Weekday::Saturday, chrono::Weekday::Sat, "Samedi"),
            (Weekday::Sunday, chrono::Weekday::Sun, "Dimanche"),
        ];

        for (day, expected, name) in days {
            let converted = day.to_model();
            assert!(
                converted.into_inner() == expected,
                "{name} should convert to itself"
            );
        }
    }
}

/// Adds the leaf value classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Weekday>()?;
    m.add_class::<WeekBlock>()?;
    m.add_class::<Periodicity>()?;
    m.add_class::<EveryNWeeks>()?;
    m.add_class::<OncePerBlock>()?;
    m.add_class::<CountInYear>()?;
    m.add_class::<CustomBlocks>()?;
    m.add_class::<TimeSlot>()?;
    m.add_class::<Enforcement>()?;
    m.add_class::<Limit>()?;
    m.add_class::<Color>()?;
    m.add_class::<Orientation>()?;
    Ok(())
}
