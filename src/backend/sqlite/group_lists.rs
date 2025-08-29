use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(_pool: &SqlitePool) -> Result<BTreeMap<Id, GroupList<super::students::Id>>> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _index: Id,
) -> std::result::Result<GroupList<super::students::Id>, IdError<Error, Id>> {
    todo!()
}

pub async fn add(
    _pool: &SqlitePool,
    _group_list: &GroupList<super::students::Id>,
) -> std::result::Result<
    Id,
    InvalidCrossError<Error, GroupList<super::students::Id>, super::students::Id>,
> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _group_list: &GroupList<super::students::Id>,
) -> std::result::Result<
    (),
    InvalidCrossIdError<Error, GroupList<super::students::Id>, Id, super::students::Id>,
> {
    todo!()
}
