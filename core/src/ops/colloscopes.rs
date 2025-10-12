use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColloscopesUpdateWarning {}

impl ColloscopesUpdateWarning {
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
pub enum ColloscopesUpdateOp {
    AddEmptyColloscope(String),
    CopyColloscope(collomatique_state_colloscopes::ColloscopeId, String),
    UpdateColloscope(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),
    DeleteColloscope(collomatique_state_colloscopes::ColloscopeId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColloscopesUpdateError {
    #[error(transparent)]
    AddEmptyColloscope(#[from] AddEmptyColloscopeError),
    #[error(transparent)]
    CopyColloscope(#[from] CopyColloscopeError),
    #[error(transparent)]
    UpdateColloscope(#[from] UpdateColloscopeError),
    #[error(transparent)]
    DeleteColloscope(#[from] DeleteColloscopeError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddEmptyColloscopeError {}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CopyColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("invalid rule id ({0:?})")]
    InvalidRuleId(collomatique_state_colloscopes::RuleId),
    #[error("invalid colloscope student id ({0:?})")]
    InvalidColloscopeStudentId(collomatique_state_colloscopes::ColloscopeStudentId),
    #[error("invalid colloscope period id ({0:?})")]
    InvalidColloscopePeriodId(collomatique_state_colloscopes::ColloscopePeriodId),
    #[error("invalid colloscope subject id ({0:?})")]
    InvalidColloscopeSubjectId(collomatique_state_colloscopes::ColloscopeSubjectId),
    #[error("invalid colloscope teacher id ({0:?})")]
    InvalidColloscopeTeacherId(collomatique_state_colloscopes::ColloscopeTeacherId),
    #[error("invalid colloscope week pattern id ({0:?})")]
    InvalidColloscopeWeekPatternId(collomatique_state_colloscopes::ColloscopeWeekPatternId),
    #[error("invalid colloscope slot id ({0:?})")]
    InvalidColloscopeSlotId(collomatique_state_colloscopes::ColloscopeSlotId),
    #[error("invalid colloscope incompat id ({0:?})")]
    InvalidColloscopeIncompatId(collomatique_state_colloscopes::ColloscopeIncompatId),
    #[error("invalid colloscope group list id ({0:?})")]
    InvalidColloscopeGroupListId(collomatique_state_colloscopes::ColloscopeGroupListId),
    #[error("invalid colloscope rule id ({0:?})")]
    InvalidColloscopeRuleId(collomatique_state_colloscopes::ColloscopeRuleId),
    #[error(transparent)]
    InvariantErrorInParameters(#[from] collomatique_state_colloscopes::InvariantError),
    #[error("Wrong period count")]
    WrongPeriodCountInColloscopeData,
    #[error("Wrong group list count")]
    WrongGroupListCountInColloscopeData,
    #[error("Wrong subject count in period")]
    WrongSubjectCountInPeriodInColloscopeData(collomatique_state_colloscopes::ColloscopePeriodId),
    #[error("Wrong slot count for subject in period")]
    WrongSlotCountForSubjectInPeriodInColloscopeData(
        collomatique_state_colloscopes::ColloscopePeriodId,
        collomatique_state_colloscopes::ColloscopeSubjectId,
    ),
    #[error("Wrong interrogation count for slot in period")]
    WrongInterrogationCountForSlotInPeriodInColloscopeData(
        collomatique_state_colloscopes::ColloscopePeriodId,
        collomatique_state_colloscopes::ColloscopeSlotId,
    ),
    #[error("Interrogation on non-interrogation week")]
    InterrogationOnNonInterrogationWeek(
        collomatique_state_colloscopes::ColloscopePeriodId,
        collomatique_state_colloscopes::ColloscopeSlotId,
        usize,
    ),
    #[error("Missing interrogation on interrogation week")]
    MissingInterrogationOnInterrogationWeek(
        collomatique_state_colloscopes::ColloscopePeriodId,
        collomatique_state_colloscopes::ColloscopeSlotId,
        usize,
    ),
    #[error("Invalid group number in interrogation")]
    InvalidGroupNumInInterrogation(
        collomatique_state_colloscopes::ColloscopePeriodId,
        collomatique_state_colloscopes::ColloscopeSlotId,
        usize,
    ),
    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(
        collomatique_state_colloscopes::ColloscopeGroupListId,
        collomatique_state_colloscopes::ColloscopeStudentId,
    ),
    #[error("wrong student count in group list")]
    WrongStudentCountInGroupList(collomatique_state_colloscopes::ColloscopeGroupListId),
    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(
        collomatique_state_colloscopes::ColloscopeGroupListId,
        collomatique_state_colloscopes::ColloscopeStudentId,
    ),
    #[error("duplicate internal id with respect to another colloscope")]
    DuplicateInternalId(u64),
    #[error("duplicate internal id with respect to global parameters")]
    InternalIdAlreadyInMainParams(u64),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
}

impl ColloscopesUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<RulesUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::ColloscopeId>, ColloscopesUpdateError> {
        match self {
            Self::AddEmptyColloscope(name) => {
                let (params, id_maps) = data.get_data().copy_main_params();
                let collo_data = collomatique_state_colloscopes::colloscopes::ColloscopeData::new_empty_from_params(&params);
                let new_colloscope = collomatique_state_colloscopes::colloscopes::Colloscope {
                    name: name.clone(),
                    params,
                    id_maps,
                    data: collo_data,
                };

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Add(new_colloscope),
                        ),
                        self.get_desc(),
                    )
                    .expect("An empty colloscope should always be valid");
                let Some(collomatique_state_colloscopes::NewId::ColloscopeId(new_id)) = result
                else {
                    panic!("Unexpected result from ColloscopeOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::CopyColloscope(colloscope_id, new_name) => {
                let Some(orig_colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return Err(CopyColloscopeError::InvalidColloscopeId(*colloscope_id).into());
                };

                let mut new_colloscope = orig_colloscope.clone();
                new_colloscope.name = new_name.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Add(new_colloscope),
                        ),
                        self.get_desc(),
                    )
                    .expect("A copied colloscope should always be valid");
                let Some(collomatique_state_colloscopes::NewId::ColloscopeId(new_id)) = result
                else {
                    panic!("Unexpected result from ColloscopeOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateColloscope(colloscope_id, colloscope) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Update(
                                *colloscope_id,
                                colloscope.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvariantErrorInParameters(invariant_error) => {
                                    UpdateColloscopeError::InvariantErrorInParameters(invariant_error)
                                }
                                collomatique_state_colloscopes::ColloscopeError::ColloscopeIdAlreadyExists(_) => {
                                    panic!("Unexpected error on ColloscopeOp::Update {:?}", ce);
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidStudentId(id) => {
                                    UpdateColloscopeError::InvalidStudentId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidPeriodId(id) => {
                                    UpdateColloscopeError::InvalidPeriodId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidSubjectId(id) => {
                                    UpdateColloscopeError::InvalidSubjectId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidTeacherId(id) => {
                                    UpdateColloscopeError::InvalidTeacherId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidWeekPatternId(id) => {
                                    UpdateColloscopeError::InvalidWeekPatternId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidSlotId(id) => {
                                    UpdateColloscopeError::InvalidSlotId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidIncompatId(id) => {
                                    UpdateColloscopeError::InvalidIncompatId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupListId(id) => {
                                    UpdateColloscopeError::InvalidGroupListId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidRuleId(id) => {
                                    UpdateColloscopeError::InvalidRuleId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeStudentId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeStudentId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopePeriodId(id) => {
                                    UpdateColloscopeError::InvalidColloscopePeriodId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeSubjectId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeSubjectId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeTeacherId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeTeacherId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeWeekPatternId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeWeekPatternId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeSlotId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeSlotId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeIncompatId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeIncompatId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeGroupListId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeGroupListId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeRuleId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeRuleId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongPeriodCountInColloscopeData => {
                                    UpdateColloscopeError::WrongPeriodCountInColloscopeData
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongGroupListCountInColloscopeData => {
                                    UpdateColloscopeError::WrongGroupListCountInColloscopeData
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongSubjectCountInPeriodInColloscopeData(period_id) => {
                                    UpdateColloscopeError::WrongSubjectCountInPeriodInColloscopeData(period_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(period_id, subject_id) => {
                                    UpdateColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(period_id, subject_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(period_id, slot_id) => {
                                    UpdateColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(period_id, slot_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InterrogationOnNonInterrogationWeek(period_id, slot_id, week) => {
                                    UpdateColloscopeError::InterrogationOnNonInterrogationWeek(period_id, slot_id, week)
                                }
                                collomatique_state_colloscopes::ColloscopeError::MissingInterrogationOnInterrogationWeek(period_id, slot_id, week) => {
                                    UpdateColloscopeError::MissingInterrogationOnInterrogationWeek(period_id, slot_id, week)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupNumInInterrogation(period_id, slot_id, group_num) => {
                                    UpdateColloscopeError::InvalidGroupNumInInterrogation(period_id, slot_id, group_num)
                                }
                                collomatique_state_colloscopes::ColloscopeError::ExcludedStudentInGroupList(group_list_id, student_id) => {
                                    UpdateColloscopeError::ExcludedStudentInGroupList(group_list_id, student_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::WrongStudentCountInGroupList(group_list_id) => {
                                    UpdateColloscopeError::WrongStudentCountInGroupList(group_list_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvalidGroupNumForStudentInGroupList(group_list_id, student_id) => {
                                    UpdateColloscopeError::InvalidGroupNumForStudentInGroupList(group_list_id, student_id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::DuplicateInternalId(id) => {
                                    UpdateColloscopeError::DuplicateInternalId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InternalIdAlreadyInMainParams(id) => {
                                    UpdateColloscopeError::InternalIdAlreadyInMainParams(id)
                                }
                            }
                        } else {
                            panic!("Unexpected error during UpdateColloscope: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteColloscope(colloscope_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Remove(*colloscope_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeId(id) => {
                                    DeleteColloscopeError::InvalidColloscopeId(id)
                                }
                                _ => panic!("Unexpected colloscope error during DeleteColloscope: {:?}", ce),
                            }
                        } else {
                            panic!("Unexpected error during DeleteColloscope: {:?}", e);
                        }
                    )?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Rules,
            match self {
                ColloscopesUpdateOp::AddEmptyColloscope(_name) => "Créer un colloscope vide".into(),
                ColloscopesUpdateOp::CopyColloscope(_id, _name) => "Dupliquer un colloscope".into(),
                ColloscopesUpdateOp::DeleteColloscope(_id) => "Supprimer un colloscope".into(),
                ColloscopesUpdateOp::UpdateColloscope(_id, _colloscope) => {
                    "Modifier un colloscope".into()
                }
            },
        )
    }
}
