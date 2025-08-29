use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    _pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, SlotSelection<super::subjects::Id, super::time_slots::Id>>,
    Error,
> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _index: Id,
) -> std::result::Result<
    SlotSelection<super::subjects::Id, super::time_slots::Id>,
    IdError<Error, Id>,
> {
    todo!()
}

pub async fn add(
    _pool: &SqlitePool,
    _slot_selection: &SlotSelection<super::subjects::Id, super::time_slots::Id>,
) -> std::result::Result<Id, Error> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), Error> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _slot_selection: &SlotSelection<super::subjects::Id, super::time_slots::Id>,
) -> std::result::Result<(), Error> {
    todo!()
}
