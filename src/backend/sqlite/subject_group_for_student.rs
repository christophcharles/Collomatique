use super::*;

pub async fn set(
    _pool: &SqlitePool,
    _student_id: students::Id,
    _subject_group_id: subject_groups::Id,
    _subject_id: Option<subjects::Id>,
) -> std::result::Result<(), Id3Error<Error, students::Id, subject_groups::Id, subjects::Id>> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _student_id: students::Id,
    _subject_group_id: subject_groups::Id,
) -> std::result::Result<Option<subjects::Id>, Id2Error<Error, students::Id, subject_groups::Id>> {
    todo!()
}
