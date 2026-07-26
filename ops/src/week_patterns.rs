use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WeekPatternsUpdateWarning {
    LooseInterrogationSlot(collomatique_state_colloscopes::SlotId),
    LooseScheduleIncompat(collomatique_state_colloscopes::IncompatId),
    LooseColloscopeDataForSlot(collomatique_state_colloscopes::SlotId),
}

impl WeekPatternsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            WeekPatternsUpdateWarning::LooseInterrogationSlot(slot_id) => {
                let Some((subject_id, slot)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_with_subject(*slot_id)
                else {
                    return None;
                };
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .get(&slot.teacher_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(subject_id)
                else {
                    return None;
                };
                Some(format!(
                    "Pertes du créneaux de colle du colleur {} {} pour la matière \"{}\" le {} à {}",
                    teacher.desc.firstname,
                    teacher.desc.surname,
                    subject.parameters.name,
                    slot.start_time.weekday,
                    slot.start_time.start_time.into_inner(),
                ))
            }
            Self::LooseScheduleIncompat(incompat_id) => {
                let Some(incompat) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(incompat.subject_id)
                else {
                    return None;
                };

                let slot_desc: Vec<_> = incompat
                    .slots
                    .iter()
                    .map(|slot| {
                        format!(
                            "le {} à {}",
                            slot.start().weekday,
                            slot.start().start_time.into_inner()
                        )
                    })
                    .collect();

                Some(format!(
                    "Perte d'une incompatibilité horaire pour la matière \"{}\" ({})",
                    subject.parameters.name,
                    slot_desc.join(", "),
                ))
            }
            Self::LooseColloscopeDataForSlot(slot_id) => {
                let Some((subject_id, slot)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_with_subject(*slot_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(subject_id)
                else {
                    return None;
                };
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .get(&slot.teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de remplissage du créneaux de colle du colleur {} {} pour la matière \"{}\" le {} à {} dans le colloscope",
                    teacher.desc.firstname,
                    teacher.desc.surname,
                    subject.parameters.name,
                    slot.start_time.weekday,
                    slot.start_time.start_time.into_inner(),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeekPatternsUpdateOp {
    AddNewWeekPattern(collomatique_state_colloscopes::week_patterns::WeekPattern),
    UpdateWeekPattern(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::week_patterns::WeekPattern,
    ),
    DeleteWeekPattern(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeekPatternsUpdateError {
    #[error(transparent)]
    AddNewWeekPattern(#[from] AddNewWeekPatternError),
    #[error(transparent)]
    UpdateWeekPattern(#[from] UpdateWeekPatternError),
    #[error(transparent)]
    DeleteWeekPattern(#[from] DeleteWeekPatternError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewWeekPatternError {
    #[error("Week pattern excludes an invalid week {0:?}")]
    WeekPatternExcludesInvalidWeek(collomatique_state_colloscopes::WeekId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
    #[error("Week pattern excludes an invalid week {0:?}")]
    WeekPatternExcludesInvalidWeek(collomatique_state_colloscopes::WeekId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl WeekPatternsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            Self::AddNewWeekPattern(_) => None,
            Self::UpdateWeekPattern(week_pattern_id, new_week_pattern) => {
                let inner = data.get_data().get_inner_data();
                if inner
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                    .is_none()
                {
                    return None;
                }
                let new_excluded = &new_week_pattern.excluded_weeks;

                // For every slot on this pattern, a non-empty colloscope row on a
                // week the new exclusion set newly disables must be cleared. A row
                // only exists on a week the slot is currently active on, so
                // "active before" is implicit; only "excluded after" needs
                // checking.
                for (slot_id, slot) in inner.params.slots.all_slots() {
                    if slot.week_pattern != Some(*week_pattern_id) {
                        continue;
                    }
                    for (week_id, _groups) in inner.colloscope.interrogations_for_slot(*slot_id) {
                        if !new_excluded.contains(&week_id) {
                            continue;
                        }
                        return Some(CleaningOp {
                            warning: WeekPatternsUpdateWarning::LooseColloscopeDataForSlot(
                                *slot_id,
                            ),
                            op: UpdateOp::Colloscope(
                                ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                    *slot_id,
                                    week_id,
                                    std::collections::BTreeSet::new(),
                                ),
                            ),
                        });
                    }
                }

                None
            }
            Self::DeleteWeekPattern(week_pattern_id) => {
                for (slot_id, slot) in data.get_data().get_inner_data().params.slots.all_slots() {
                    if slot.week_pattern == Some(*week_pattern_id) {
                        return Some(CleaningOp {
                            warning: WeekPatternsUpdateWarning::LooseInterrogationSlot(*slot_id),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                        });
                    }
                }

                for (incompat_id, incompat) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                    .iter()
                {
                    let incompat_id = &incompat_id;
                    if incompat.week_pattern_id == Some(*week_pattern_id) {
                        return Some(CleaningOp {
                            warning: WeekPatternsUpdateWarning::LooseScheduleIncompat(*incompat_id),
                            op: UpdateOp::Incompatibilities(
                                IncompatibilitiesUpdateOp::DeleteIncompat(*incompat_id),
                            ),
                        });
                    }
                }

                None
            }
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::WeekPatternId>, WeekPatternsUpdateError>
    {
        match self {
            Self::AddNewWeekPattern(week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Add(
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, Reference, WeekRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any pattern->week dangle
                            // in the set was introduced by this Add; the dangling
                            // target is the bad excluded week id.
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Week {
                                        target,
                                        site: WeekRefSite::WeekPatternExcludedWeek(_),
                                    }) = inv
                                    {
                                        return AddNewWeekPatternError::WeekPatternExcludesInvalidWeek(*target);
                                    }
                                }
                                panic!("Unexpected invariant breaks during AddNewWeekPattern: {set:?}");
                            }
                            _ => panic!("Unexpected error during AddNewWeekPattern: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::WeekPatternId(new_id)) = result
                else {
                    panic!("Unexpected result from WeekPatternOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateWeekPattern(week_pattern_id, week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Update(
                                *week_pattern_id,
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, PrecheckError, Reference,
                            WeekPatternPrecheckError, WeekRefSite,
                        };
                        match e {
                            Error::Precheck(PrecheckError::WeekPattern(
                                WeekPatternPrecheckError::InvalidWeekPatternId(id),
                            )) => UpdateWeekPatternError::InvalidWeekPatternId(id),
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Week {
                                        target,
                                        site: WeekRefSite::WeekPatternExcludedWeek(_),
                                    }) = inv
                                    {
                                        return UpdateWeekPatternError::WeekPatternExcludesInvalidWeek(*target);
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateWeekPattern: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateWeekPattern: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteWeekPattern(week_pattern_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Remove(*week_pattern_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, PrecheckError, Reference,
                            WeekPatternPrecheckError, WeekPatternRefSite,
                        };
                        match e {
                            Error::Precheck(PrecheckError::WeekPattern(
                                WeekPatternPrecheckError::InvalidWeekPatternId(id),
                            )) => DeleteWeekPatternError::InvalidWeekPatternId(id),
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        site,
                                        ..
                                    }) = inv
                                    {
                                        match site {
                                            WeekPatternRefSite::IncompatWeekPattern(_) => panic!(
                                                "Incompats should be cleaned before removing week patterns"
                                            ),
                                            WeekPatternRefSite::SlotWeekPattern(_) => panic!(
                                                "Slots should be cleaned before removing week patterns"
                                            ),
                                        }
                                    }
                                }
                                panic!("Unexpected invariant breaks during DeleteWeekPattern: {set:?}");
                            }
                            _ => panic!("Unexpected error during DeleteWeekPattern: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::WeekPatterns,
            match self {
                WeekPatternsUpdateOp::AddNewWeekPattern(_desc) => {
                    "Ajouter un modèle de périodicité".into()
                }
                WeekPatternsUpdateOp::UpdateWeekPattern(_id, _desc) => {
                    "Modifier un modèle de périodicité".into()
                }
                WeekPatternsUpdateOp::DeleteWeekPattern(_id) => {
                    "Supprimer un modèle de périodicité".into()
                }
            },
        )
    }
}
