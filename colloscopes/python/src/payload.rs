//! The structural walk from one model value to python data
//!
//! The model describes what it refused (`collomatique_ops::UpdateError`) and
//! what it repaired (`collomatique_state_colloscopes::Fix`) with trees of
//! externally-tagged enums, and both derive `Serialize`. So the walk over them
//! is a serde [Serializer] whose output is python objects rather than text:
//! serde already knows the shape, and asking it means nothing here names a
//! variant. That is §6's constraint (`docs/python/new_api_design.md`): a case
//! added in `colloscopes/ops/`, or a repair added in `colloscopes/state-colloscopes/`, reaches a script
//! on its own.
//!
//! What comes out:
//!
//! - a unit variant is its own name, a python `str`;
//! - any other variant is a one-key dict, `{name: payload}`, and the payload of
//!   a tuple or newtype variant is always a tuple, so a case that carries one
//!   thing and a case that carries two look alike to the caller;
//! - a struct variant is a one-key dict of a dict. No error in `colloscopes/ops/` has one
//!   today, and every repair in `colloscopes/state-colloscopes/` is one;
//! - a *named* newtype struct is the id class when its name is one of the
//!   eleven (`crate::ids::from_serde`), and its inner value otherwise;
//! - everything else is the obvious python equivalent — numbers, strings,
//!   `None`, lists, dicts.
//!
//! Two readers take that shape apart, and both start with [peel]: a refusal has
//! three levels of it (`crate::errors::from_data`), and a repair has one
//! ([repair], for `crate::results::Warning`).

use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
use serde::{Serialize, Serializer, ser};

#[cfg(test)]
mod tests;

/// The python data one serde-able model value becomes
pub(crate) fn to_py<'py, T: ?Sized + Serialize>(
    py: Python<'py>,
    value: &T,
) -> Result<Bound<'py, PyAny>, Failed> {
    value.serialize(PyData { py })
}

/// One level of an externally-tagged enum, as the walk above wrote it
///
/// A variant is `{"CutPeriod": (…,)}`, and a unit variant is the bare name,
/// which carries the empty tuple. Anything else is a shape the rust side has
/// grown since, and a reader stops there rather than guessing.
pub(crate) fn peel<'py>(data: &Bound<'py, PyAny>) -> Option<(String, Bound<'py, PyAny>)> {
    if let Ok(name) = data.extract::<String>() {
        return Some((name, PyTuple::empty(data.py()).into_any()));
    }

    let dict = data.cast::<PyDict>().ok()?;
    if dict.len() != 1 {
        return None;
    }

    let (name, payload) = dict.iter().next()?;
    Some((name.extract::<String>().ok()?, payload))
}

/// The field a repair carries for the op, not for the reader
const REBUILT: &str = "rebuilt";

/// The name and the coordinates of one repair
///
/// The `rebuilt` field is dropped. It is the whole-value argument the
/// elementary op needs — `Fix`'s own docs say the semantic fields beside it
/// "are what a consumer describes the repair with" and that the two are
/// redundant on purpose, and `ops::warning_text` renders all its sentences
/// without reading one. Showing it would also put the model's storage shape in
/// front of a script: a rebuilt `GroupList` is its private serde mirror, a
/// rebuilt `WeekPattern` is the exclusion set it is stored as, and §2 built the
/// `*Data` classes to keep those out of sight. A script that wants what the
/// entity holds now reads it off the document the write just left.
///
/// Dropped here by name rather than skipped in `state-colloscopes`, so that a
/// variant grown later carries no payload here either — and so that `Fix`'s own
/// `Serialize` stays whole for every other reader.
pub(crate) fn repair<'py>(
    py: Python<'py>,
    fix: &collomatique_state_colloscopes::Fix,
) -> Result<(Option<String>, Bound<'py, PyAny>), Failed> {
    let data = to_py(py, fix)?;

    let Some((kind, fields)) = peel(&data) else {
        // A shape the walk cannot follow: keep what it reached, the way
        // `crate::errors::from_data` does, rather than guessing.
        return Ok((None, data));
    };

    if let Ok(dict) = fields.cast::<PyDict>()
        && dict.contains(REBUILT)?
    {
        dict.del_item(REBUILT)?;
    }

    Ok((Some(kind), fields))
}

/// A walk that could not be made
///
/// Serde's contract asks for an error type, and python's own failures — a dict
/// that will not take a key, a class that will not build — arrive as one too.
/// Nothing in `colloscopes/ops/` can actually produce either.
#[derive(Debug)]
pub(crate) struct Failed(String);

impl std::fmt::Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Failed {}

impl ser::Error for Failed {
    fn custom<T: std::fmt::Display>(message: T) -> Failed {
        Failed(message.to_string())
    }
}

impl From<PyErr> for Failed {
    fn from(error: PyErr) -> Failed {
        Failed(error.to_string())
    }
}

/// The serializer itself: it holds nothing but the interpreter it builds in
#[derive(Clone, Copy)]
struct PyData<'py> {
    py: Python<'py>,
}

/// One python object, from anything pyo3 already converts
fn object<'py, T: IntoPyObjectExt<'py>>(
    py: Python<'py>,
    value: T,
) -> Result<Bound<'py, PyAny>, Failed> {
    Ok(value.into_bound_py_any(py)?)
}

/// `{name: payload}` — one variant, the way serde tags it
fn variant<'py>(
    py: Python<'py>,
    name: &str,
    payload: Bound<'py, PyAny>,
) -> Result<Bound<'py, PyAny>, Failed> {
    let dict = PyDict::new(py);
    dict.set_item(name, payload)?;
    Ok(dict.into_any())
}

impl<'py> Serializer for PyData<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    type SerializeSeq = Items<'py>;
    type SerializeTuple = Items<'py>;
    type SerializeTupleStruct = Items<'py>;
    type SerializeTupleVariant = Items<'py>;
    type SerializeMap = Entries<'py>;
    type SerializeStruct = Fields<'py>;
    type SerializeStructVariant = Fields<'py>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        object(self.py, value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(PyBytes::new(self.py, value).into_any())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.py.None().into_bound(self.py))
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        // An optional payload is its value, the way an absent one is `None`:
        // python has no wrapper to put around it.
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.py.None().into_bound(self.py))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(self.py.None().into_bound(self.py))
    }

    /// A case that carries nothing is its own name
    fn serialize_unit_variant(
        self,
        _enum_name: &'static str,
        _index: u32,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        object(self.py, name)
    }

    /// An id, and nothing else, keeps its class
    ///
    /// `PeriodId(u64)` and its ten siblings reach serde as *named* newtype
    /// structs, and the name is the one place a structural walk can still tell
    /// an id from a count. A newtype struct whose name is not an id class
    /// passes its inner value through untouched.
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let inner = value.serialize(self)?;
        Ok(crate::ids::from_serde(self.py, name, inner)?)
    }

    /// A one-field variant, written as the one-element tuple it is
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _enum_name: &'static str,
        _index: u32,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let inner = value.serialize(self)?;
        let payload = PyTuple::new(self.py, [inner])?;
        variant(self.py, name, payload.into_any())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(Items::new(self.py, Shape::List))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(Items::new(self.py, Shape::Tuple))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(Items::new(self.py, Shape::Tuple))
    }

    fn serialize_tuple_variant(
        self,
        _enum_name: &'static str,
        _index: u32,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(Items::new(self.py, Shape::Variant(name)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(Entries {
            py: self.py,
            dict: PyDict::new(self.py),
            key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Fields::new(self.py, None))
    }

    fn serialize_struct_variant(
        self,
        _enum_name: &'static str,
        _index: u32,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(Fields::new(self.py, Some(name)))
    }
}

/// What a run of items ends up as
enum Shape {
    /// A sequence, which python takes as a list
    List,
    /// A tuple, or a tuple struct
    Tuple,
    /// A tuple variant: the tuple, under the variant's name
    Variant(&'static str),
}

/// Items collected one by one, for every sequence-shaped thing serde has
struct Items<'py> {
    py: Python<'py>,
    shape: Shape,
    items: Vec<Bound<'py, PyAny>>,
}

impl<'py> Items<'py> {
    fn new(py: Python<'py>, shape: Shape) -> Items<'py> {
        Items {
            py,
            shape,
            items: Vec::new(),
        }
    }

    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        self.items.push(value.serialize(PyData { py: self.py })?);
        Ok(())
    }

    fn finish(self) -> Result<Bound<'py, PyAny>, Failed> {
        match self.shape {
            Shape::List => Ok(PyList::new(self.py, self.items)?.into_any()),
            Shape::Tuple => Ok(PyTuple::new(self.py, self.items)?.into_any()),
            Shape::Variant(name) => {
                let payload = PyTuple::new(self.py, self.items)?;
                variant(self.py, name, payload.into_any())
            }
        }
    }
}

impl<'py> ser::SerializeSeq for Items<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}

impl<'py> ser::SerializeTuple for Items<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}

impl<'py> ser::SerializeTupleStruct for Items<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}

impl<'py> ser::SerializeTupleVariant for Items<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}

/// A map, built from the key and the value serde hands over separately
struct Entries<'py> {
    py: Python<'py>,
    dict: Bound<'py, PyDict>,
    key: Option<Bound<'py, PyAny>>,
}

impl<'py> ser::SerializeMap for Entries<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Failed> {
        self.key = Some(key.serialize(PyData { py: self.py })?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Failed> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Failed("a map value arrived before its key".to_string()))?;
        self.dict
            .set_item(key, value.serialize(PyData { py: self.py })?)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        Ok(self.dict.into_any())
    }
}

/// A struct's named fields, and the variant name when it is a struct variant
struct Fields<'py> {
    py: Python<'py>,
    dict: Bound<'py, PyDict>,
    name: Option<&'static str>,
}

impl<'py> Fields<'py> {
    fn new(py: Python<'py>, name: Option<&'static str>) -> Fields<'py> {
        Fields {
            py,
            dict: PyDict::new(py),
            name,
        }
    }

    fn set<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Failed> {
        self.dict
            .set_item(key, value.serialize(PyData { py: self.py })?)?;
        Ok(())
    }

    fn finish(self) -> Result<Bound<'py, PyAny>, Failed> {
        match self.name {
            Some(name) => variant(self.py, name, self.dict.into_any()),
            None => Ok(self.dict.into_any()),
        }
    }
}

impl<'py> ser::SerializeStruct for Fields<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Failed> {
        self.set(key, value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}

impl<'py> ser::SerializeStructVariant for Fields<'py> {
    type Ok = Bound<'py, PyAny>;
    type Error = Failed;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Failed> {
        self.set(key, value)
    }

    fn end(self) -> Result<Self::Ok, Failed> {
        self.finish()
    }
}
