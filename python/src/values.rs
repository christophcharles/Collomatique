//! The values a read hands back that are not entities
//!
//! `docs/python/handle_api.md` §2.6: small immutable classes for data the model
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
//! need. The rest of §2.6 lands with the collections that hand it out.

use std::num::NonZeroU32;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use collomatique_state_colloscopes::{NonEmptyRangeInclusive, SubjectPeriodicity};

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

/// Adds the leaf value classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<WeekBlock>()?;
    m.add_class::<Periodicity>()?;
    m.add_class::<EveryNWeeks>()?;
    m.add_class::<OncePerBlock>()?;
    m.add_class::<CountInYear>()?;
    m.add_class::<CustomBlocks>()?;
    Ok(())
}
