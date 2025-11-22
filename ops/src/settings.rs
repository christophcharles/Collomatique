use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingsUpdateWarning {}

impl SettingsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettingsUpdateOp {
    UpdateGlobalLimits(collomatique_state_colloscopes::settings::Limits),
    UpdateStudentLimits(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::settings::Limits,
    ),
    RemoveStudentLimits(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettingsUpdateError {
    #[error(transparent)]
    UpdateStudentLimits(#[from] UpdateStudentLimitsError),
    #[error(transparent)]
    RemoveStudentLimits(#[from] RemoveStudentLimitsError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStudentLimitsError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoveStudentLimitsError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("No limits definied for student {0:?}")]
    NoLimitsForStudent(collomatique_state_colloscopes::StudentId),
}

impl SettingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            SettingsUpdateOp::UpdateGlobalLimits(_) => None,
            SettingsUpdateOp::UpdateStudentLimits(_, _) => None,
            SettingsUpdateOp::RemoveStudentLimits(_) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), SettingsUpdateError> {
        match self {
            Self::UpdateGlobalLimits(limits) => {
                let mut new_settings = data.get_data().get_inner_data().params.settings.clone();
                new_settings.global = limits.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::Update(new_settings),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::Update should never fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateStudentLimits(student_id, limits) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains_key(student_id)
                {
                    return Err(UpdateStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                let mut new_settings = data.get_data().get_inner_data().params.settings.clone();
                new_settings.students.insert(*student_id, limits.clone());

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::Update(new_settings),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::Update should not fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::RemoveStudentLimits(student_id) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains_key(student_id)
                {
                    return Err(RemoveStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                let mut new_settings = data.get_data().get_inner_data().params.settings.clone();
                if new_settings.students.remove(student_id).is_none() {
                    return Err(RemoveStudentLimitsError::NoLimitsForStudent(*student_id).into());
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::Update(new_settings),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::Update should not fail");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Settings,
            match self {
                SettingsUpdateOp::UpdateGlobalLimits(_) => {
                    "Mettre à jour les paramètres généraux de limites".into()
                }
                SettingsUpdateOp::UpdateStudentLimits(_, _) => {
                    "Mettre à jour les paramètres de limites d'un élève".into()
                }
                SettingsUpdateOp::RemoveStudentLimits(_) => {
                    "Supprimer les paramètres de limites d'un élève".into()
                }
            },
        )
    }
}
