use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColloscopeUpdateWarning {}

impl ColloscopeUpdateWarning {
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
pub enum ColloscopeUpdateOp {
    UpdateColloscopeGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::colloscopes::ColloscopeGroupList,
    ),
    UpdateColloscopeInterrogation(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SlotId,
        usize,
        collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColloscopeUpdateError {
    #[error(transparent)]
    UpdateColloscopeGroupList(#[from] UpdateColloscopeGroupListError),
    #[error(transparent)]
    UpdateColloscopeInterrogation(#[from] UpdateColloscopeInterrogationError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateColloscopeGroupListError {
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateColloscopeInterrogationError {
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("invalid week number {1} in period {0:?}")]
    InvalidWeekNumberInPeriod(collomatique_state_colloscopes::PeriodId, usize),
    #[error("Interrogation on non-interrogation week")]
    InterrogationOnNonInterrogationWeek(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SlotId,
        usize,
    ),
    #[error("Invalid group number in interrogation")]
    InvalidGroupNumInInterrogation(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SlotId,
        usize,
    ),
}

impl ColloscopeUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<ColloscopeUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), ColloscopeUpdateError> {
        match self {
            Self::UpdateColloscopeGroupList(group_list_id, group_list) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::UpdateGroupList(
                                *group_list_id,
                                group_list.clone(),
                            )
                        ),
                        self.get_desc()
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupListId(group_list_id) =>
                                    UpdateColloscopeGroupListError::InvalidGroupListId(group_list_id),
                                collomatique_state_colloscopes::ColloscopeError::ExcludedStudentInGroupList(
                                    group_list_id,
                                    student_id
                                ) => UpdateColloscopeGroupListError::ExcludedStudentInGroupList(
                                    group_list_id,
                                    student_id,
                                ),
                                collomatique_state_colloscopes::ColloscopeError::InvalidStudentId(student_id) => {
                                    UpdateColloscopeGroupListError::InvalidStudentId(student_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupNumForStudentInGroupList(group_list_id, student_id) => {
                                    UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(group_list_id, student_id)
                                }
                                _ => panic!("Unexpected error on ColloscopeOp::UpdateGroupList: {:?}", ce),
                            }
                        } else {
                            panic!("Unexpected error during UpdateColloscopeGroupList: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateColloscopeInterrogation(
                period_id,
                slot_id,
                week_in_period,
                interrogation,
            ) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::UpdateInterrogation(
                                *period_id,
                                *slot_id,
                                *week_in_period,
                                interrogation.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidPeriodId(period_id) => {
                                    UpdateColloscopeInterrogationError::InvalidPeriodId(period_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidSlotId(slot_id) => {
                                    UpdateColloscopeInterrogationError::InvalidSlotId(slot_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidWeekNumberInPeriod(period_id, week_num) => {
                                    UpdateColloscopeInterrogationError::InvalidWeekNumberInPeriod(period_id, week_num)
                                }
                                collomatique_state_colloscopes::ColloscopeError::NoInterrogationOnWeek(period_id, slot_id, week_num) => {
                                    UpdateColloscopeInterrogationError::InterrogationOnNonInterrogationWeek(period_id, slot_id, week_num)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupNumInInterrogation(period_id, slot_id, week_num) => {
                                    UpdateColloscopeInterrogationError::InvalidGroupNumInInterrogation(period_id, slot_id, week_num)
                                }
                                _ => panic!("Unexpected error on ColloscopeOp::UpdateInterrogation: {:?}", ce),
                            }
                        } else {
                            panic!("Unexpected error during UpdateColloscopeInterrogation: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Colloscope,
            match self {
                ColloscopeUpdateOp::UpdateColloscopeGroupList(_id, _list) => {
                    "Mettre à jour une liste de groupe du colloscope".into()
                }
                ColloscopeUpdateOp::UpdateColloscopeInterrogation(_id, _slot, _week, _int) => {
                    "Mettre à jour une interrogation du colloscope".into()
                }
            },
        )
    }
}
