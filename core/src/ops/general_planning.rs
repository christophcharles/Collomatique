use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeneralPlanningUpdateWarning {
    LooseStudentExclusionForPeriod(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseStudentAssignmentsForPeriod(collomatique_state_colloscopes::PeriodId),
    LooseSubjectDataForPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseSubjectAssociation(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseRuleDataForPeriod(
        collomatique_state_colloscopes::RuleId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl GeneralPlanningUpdateWarning {
    pub fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(student_id, period_id) => {
                let Some(student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des informations de présence de l'élève {} {} sur la période {}",
                    student.desc.firstname,
                    student.desc.surname,
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(period_id) => {
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des inscriptions des élèves sur la période {}",
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(subject_id, period_id) => {
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return None;
                };
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des informations de la matière \"{}\" sur la période {}",
                    subject.parameters.name,
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                group_list_id,
                subject_id,
                period_id,
            ) => {
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return None;
                };
                let Some(period_num) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de l'association de la matière \"{}\" à la liste de groupe \"{}\" pour la période {}",
                    subject.parameters.name, group_list.params.name, period_num+1
                ))
            }
            GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(rule_id, period_id) => {
                let Some(rule) = data.get_data().get_rules().rule_map.get(rule_id) else {
                    return None;
                };
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des informations de la règle \"{}\" sur la période {}",
                    rule.name,
                    period_index + 1
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::NaiveMondayDate),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(collomatique_state_colloscopes::PeriodId, usize),
    DeletePeriod(collomatique_state_colloscopes::PeriodId),
    CutPeriod(collomatique_state_colloscopes::PeriodId, usize),
    MergeWithPreviousPeriod(collomatique_state_colloscopes::PeriodId),
    UpdateWeekStatus(collomatique_state_colloscopes::PeriodId, usize, bool),
}

#[derive(Debug, Error)]
pub enum GeneralPlanningUpdateError {
    #[error(transparent)]
    UpdatePeriodWeekCount(#[from] UpdatePeriodWeekCountError),
    #[error(transparent)]
    DeletePeriod(#[from] DeletePeriodError),
    #[error(transparent)]
    CutPeriod(#[from] CutPeriodError),
    #[error(transparent)]
    MergeWithPreviousPeriod(#[from] MergeWithPreviousPeriodError),
    #[error(transparent)]
    UpdateWeekStatus(#[from] UpdateWeekStatusError),
}

#[derive(Debug, Error)]
pub enum UpdatePeriodWeekCountError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} implies a minimum total number of weeks of {1}")]
    SubjectImpliesMinimumWeekCount(collomatique_state_colloscopes::SubjectId, usize),
}

#[derive(Debug, Error)]
pub enum DeletePeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Debug, Error)]
pub enum CutPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Remaining week count ({0}) is larger than available week count ({1})")]
    RemainingWeekCountTooBig(usize, usize),
}

#[derive(Debug, Error)]
pub enum MergeWithPreviousPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("This is the first period and cannot be merged with the non-existent previous one")]
    NoPreviousPeriodToMergeWith,
}

#[derive(Debug, Error)]
pub enum UpdateWeekStatusError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

impl GeneralPlanningUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<PreCleaningOp<GeneralPlanningUpdateWarning>> {
        todo!()
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::PeriodId>, GeneralPlanningUpdateError> {
        todo!()
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::GeneralPlanning,
            match self {
                GeneralPlanningUpdateOp::DeleteFirstWeek => "Effacer le début des colles".into(),
                GeneralPlanningUpdateOp::UpdateFirstWeek(_date) => {
                    "Changer le début des colles".into()
                }
                GeneralPlanningUpdateOp::AddNewPeriod(_week_count) => "Ajouter une période".into(),
                GeneralPlanningUpdateOp::UpdatePeriodWeekCount(_period_id, _week_count) => {
                    "Modifier une période".into()
                }
                GeneralPlanningUpdateOp::DeletePeriod(_period_id) => "Supprimer une période".into(),
                GeneralPlanningUpdateOp::CutPeriod(_period_id, _new_week_count) => {
                    "Découper une période".into()
                }
                GeneralPlanningUpdateOp::MergeWithPreviousPeriod(_period_id) => {
                    "Fusionner deux périodes".into()
                }
                GeneralPlanningUpdateOp::UpdateWeekStatus(_period_id, _week_num, state) => {
                    if *state {
                        "Ajouter une semaine de colle".into()
                    } else {
                        "Supprimer une semaine de colle".into()
                    }
                }
            },
        )
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<GeneralPlanningUpdateWarning> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => vec![],
            GeneralPlanningUpdateOp::UpdateFirstWeek(_) => vec![],
            GeneralPlanningUpdateOp::AddNewPeriod(_) => vec![],
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(_, _) => vec![],
            GeneralPlanningUpdateOp::CutPeriod(_, _) => vec![],
            GeneralPlanningUpdateOp::UpdateWeekStatus(_, _, _) => vec![],
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                let mut output = vec![];

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        output.push(GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(
                            *subject_id,
                            *period_id,
                        ));
                    }
                }

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        output.push(GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(
                            *rule_id, *period_id,
                        ));
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id) {
                        output.push(
                            GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(
                                *student_id,
                                *period_id,
                            ),
                        );
                    }
                }

                let mut is_a_student_affected = false;

                let Some(period_assignments) = data
                    .get_data()
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .cloned()
                else {
                    return vec![];
                };

                for (_subject_id, assigned_students) in period_assignments.subject_map {
                    if !assigned_students.is_empty() {
                        is_a_student_affected = true;
                    }
                }

                if is_a_student_affected {
                    output.push(
                        GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                    );
                }

                if let Some(subject_map) = data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, group_list_id) in subject_map {
                        output.push(GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                            *group_list_id,
                            *subject_id,
                            *period_id,
                        ));
                    }
                }

                output
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let Some(pos) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return vec![];
                };
                if pos == 0 {
                    return vec![];
                }
                let previous_id = data.get_data().get_periods().ordered_period_list[pos - 1].0;

                let mut output = vec![];

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id)
                        != subject.excluded_periods.contains(&previous_id)
                    {
                        output.push(GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(
                            *subject_id,
                            *period_id,
                        ));
                    }
                }

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.excluded_periods.contains(period_id)
                        != rule.excluded_periods.contains(&previous_id)
                    {
                        output.push(GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(
                            *rule_id, *period_id,
                        ));
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id)
                        != student.excluded_periods.contains(&previous_id)
                    {
                        output.push(
                            GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(
                                *student_id,
                                *period_id,
                            ),
                        );
                    }
                }

                let Some(period_assignments) = data
                    .get_data()
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .cloned()
                else {
                    return vec![];
                };

                let Some(previous_assignments) =
                    data.get_data().get_assignments().period_map.get(period_id)
                else {
                    return vec![];
                };

                let mut is_a_student_affected = false;

                for (subject_id, assigned_students) in &period_assignments.subject_map {
                    let Some(previous_students) = previous_assignments.subject_map.get(subject_id)
                    else {
                        is_a_student_affected = true;
                        continue;
                    };

                    if *assigned_students != *previous_students {
                        is_a_student_affected = true;
                    }
                }

                if is_a_student_affected {
                    output.push(
                        GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                    );
                }

                if let Some(subject_map) = data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, group_list_id) in subject_map {
                        output.push(GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                            *group_list_id,
                            *subject_id,
                            *period_id,
                        ));
                    }
                }

                output
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::PeriodId>, GeneralPlanningUpdateError> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                        ),
                        self.get_desc(),
                    )
                    .expect("Deleting first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(
                                date.clone(),
                            )),
                        ),
                        self.get_desc(),
                    )
                    .expect("Updating first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                let new_desc = vec![true; *week_count];
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            match data.get_data().get_periods().ordered_period_list.last() {
                                Some((id, _)) => {
                                    collomatique_state_colloscopes::PeriodOp::AddAfter(
                                        *id, new_desc,
                                    )
                                }
                                None => {
                                    collomatique_state_colloscopes::PeriodOp::AddFront(new_desc)
                                }
                            },
                        ),
                        self.get_desc(),
                    )
                    .expect("Adding a period should never fail");
                match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => Ok(Some(id)),
                    _ => panic!("Unexpected result! {:?}", result),
                }
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(UpdatePeriodWeekCountError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                desc.resize(*week_count, desc.last().copied().unwrap_or(true));

                let result = match data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    self.get_desc(),
                ) {
                    Ok(r) => r,
                    Err(collomatique_state_colloscopes::Error::Period(
                        collomatique_state_colloscopes::PeriodError::InvalidPeriodId(_),
                    )) => {
                        panic!(
                                "Period Id {:?} should be valid at this point but InvalidPeriodId received", *period_id
                            )
                    }
                    Err(e) => {
                        panic!("Unexpected error for UpdatePeriodWeekCount! {:?}", e);
                    }
                };
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Enlever une référence à la période pour une matière".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Rule(
                                    collomatique_state_colloscopes::RuleOp::Update(
                                        *rule_id, new_rule,
                                    ),
                                ),
                                "Enlever une référence à la période pour une règle".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        *student_id,
                                        new_student,
                                    ),
                                ),
                                "Enlever une référence à la période pour un élève".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                if let Some(subject_map) = data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, _group_list_id) in subject_map {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                        *period_id,
                                        *subject_id,
                                        None,
                                    ),
                                ),
                                "Enlever une association de liste de groupes sur la période".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let Some(period_assignments) = data
                    .get_data()
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .cloned()
                else {
                    return Err(DeletePeriodError::InvalidPeriodId(*period_id).into());
                };

                for (subject_id, assigned_students) in period_assignments.subject_map {
                    for student_id in assigned_students {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Assignment(
                                    collomatique_state_colloscopes::AssignmentOp::Assign(
                                        *period_id, student_id, subject_id, false,
                                    ),
                                ),
                                "Valeur par défaut pour l'affectation d'un élève".into(),
                            )
                            .expect("All data should be valid at this point");

                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                        ),
                        "Suppression effective de la période".into(),
                    )
                    .expect("All data should be valid at this point");

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                *data = session.commit(self.get_desc());
                Ok(None)
            }
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(CutPeriodError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                if *new_week_count > desc.len() {
                    Err(CutPeriodError::RemainingWeekCountTooBig(
                        *new_week_count,
                        desc.len(),
                    ))?;
                }

                let new_desc = desc.split_off(*new_week_count);

                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(
                                *period_id, new_desc,
                            ),
                        ),
                        "Ajouter une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.insert(new_id.clone());
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Dupliquer l'état de la période découpée sur une matière".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.insert(new_id.clone());
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Rule(
                                    collomatique_state_colloscopes::RuleOp::Update(
                                        *rule_id, new_rule,
                                    ),
                                ),
                                "Dupliquer l'état de la période découpée sur une règle".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.insert(new_id.clone());
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        *student_id,
                                        new_student,
                                    ),
                                ),
                                "Dupliquer l'état de la période découpée sur un élève".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let period_assignments = data
                    .get_data()
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .expect("Period id should be valid at this point")
                    .clone();

                for (subject_id, assigned_students) in period_assignments.subject_map {
                    for student_id in assigned_students {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Assignment(
                                    collomatique_state_colloscopes::AssignmentOp::Assign(
                                        new_id, student_id, subject_id, true,
                                    ),
                                ),
                                "Dupliquer l'affectation d'un élève sur la période découpée".into(),
                            )
                            .expect("All data should be valid at this point");

                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                if let Some(subject_map) = data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, group_list_id) in subject_map {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                        new_id,
                                        *subject_id,
                                        Some(*group_list_id),
                                    ),
                                ),
                                "Dupliquer une association de liste de groupes sur la période découpée".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        "Raccourcir une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                *data = session.commit(self.get_desc());
                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(MergeWithPreviousPeriodError::InvalidPeriodId(*period_id))?;
                if pos == 0 {
                    Err(MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith)?;
                }
                let previous_id = data.get_data().get_periods().ordered_period_list[pos - 1].0;

                let mut prev_desc = data.get_data().get_periods().ordered_period_list[pos - 1]
                    .1
                    .clone();
                let desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                prev_desc.extend(desc);

                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(
                                previous_id,
                                prev_desc,
                            ),
                        ),
                        "Prolongement d'une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Enlever une référence à la période pour une matière".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Rule(
                                    collomatique_state_colloscopes::RuleOp::Update(
                                        *rule_id, new_rule,
                                    ),
                                ),
                                "Enlever une référence à la période pour une règle".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        *student_id,
                                        new_student,
                                    ),
                                ),
                                "Enlever une référence à la période pour un élève".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let period_assignments = data
                    .get_data()
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .expect("At this point, period_id should be valid")
                    .clone();

                for (subject_id, assigned_students) in period_assignments.subject_map {
                    for student_id in assigned_students {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Assignment(
                                    collomatique_state_colloscopes::AssignmentOp::Assign(
                                        *period_id, student_id, subject_id, false,
                                    ),
                                ),
                                "Valeur par défaut pour l'affectation d'un élève".into(),
                            )
                            .expect("All data should be valid at this point");

                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                if let Some(subject_map) = data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, _group_list_id) in subject_map {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                        *period_id,
                                        *subject_id,
                                        None,
                                    ),
                                ),
                                "Enlever une association de liste de groupes sur la période".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                        ),
                        "Suppression d'une période".into(),
                    )
                    .expect("All data should be valid at this point");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                *data = session.commit(self.get_desc());
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekStatusError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                if *week_num >= desc.len() {
                    Err(UpdateWeekStatusError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                desc[*week_num] = *state;

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, parameters should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
        }
    }
}
