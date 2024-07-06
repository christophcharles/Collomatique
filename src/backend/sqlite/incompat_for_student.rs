use super::*;

pub async fn set(
    _pool: &SqlitePool,
    _student_id: students::Id,
    _incompat_id: incompats::Id,
    _enabled: bool,
) -> std::result::Result<(), Id2Error<Error, students::Id, incompats::Id>> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _student_id: students::Id,
    _incompat_id: incompats::Id,
) -> std::result::Result<bool, Id2Error<Error, students::Id, incompats::Id>> {
    todo!()
}
