use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>>,
    Error,
> {
    todo!()
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<
    Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
    IdError<Error, Id>,
> {
    todo!()
}

pub async fn add(
    pool: &SqlitePool,
    colloscope: &Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
) -> std::result::Result<Id, Error> {
    todo!()
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    todo!()
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    colloscope: &Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
) -> std::result::Result<(), Error> {
    todo!()
}
