use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    _pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>>,
    Error,
> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _index: Id,
) -> std::result::Result<
    Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    IdError<Error, Id>,
> {
    todo!()
}

pub async fn add(
    _pool: &SqlitePool,
    _subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<
    Id,
    Cross3Error<Error, super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<
    (),
    Cross3IdError<
        Error,
        Id,
        super::subject_groups::Id,
        super::incompats::Id,
        super::group_lists::Id,
    >,
> {
    todo!()
}
