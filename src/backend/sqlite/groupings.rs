use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    _pool: &SqlitePool,
) -> std::result::Result<BTreeMap<Id, Grouping<super::time_slots::Id>>, Error> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _index: Id,
) -> std::result::Result<Grouping<super::time_slots::Id>, IdError<Error, Id>> {
    todo!()
}

pub async fn add(
    _pool: &SqlitePool,
    _grouping: &Grouping<super::time_slots::Id>,
) -> std::result::Result<Id, CrossError<Error, super::time_slots::Id>> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _grouping: &Grouping<super::time_slots::Id>,
) -> std::result::Result<(), CrossIdError<Error, Id, super::time_slots::Id>> {
    todo!()
}
