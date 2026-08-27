use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupListsUpdateOp {
    AddNewGroupList(collomatique_state_colloscopes::group_lists::GroupList),
    /// Replaces a whole group list — parameters *and* filling — with the
    /// sealed value the caller supplies.
    UpdateGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupList,
    ),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    AssignGroupListToSubject(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::GroupListId>,
    ),
    DuplicatePreviousPeriod(collomatique_state_colloscopes::PeriodId),
    /// Removes every group list the document holds, one `DeleteGroupList` at a
    /// time. Everything that named a list goes with it — the associations, the
    /// colloscope placement rows, the colles left out of range — all of it the
    /// cascade's business, exactly as for the single removal.
    DeleteAllGroupLists,
    /// Drops every `(this period, subject) → group list` association, leaving
    /// the lists themselves alone: a list no association names any more is an
    /// ordinary document.
    ClearPeriodAssociations(collomatique_state_colloscopes::PeriodId),
    /// Lands the output of the automatic group-list generation as one undo
    /// slot: each entry is a sealed group list plus the `(period, subject)`
    /// coordinates it must be associated to. Associations overwrite whatever
    /// the coordinate held; a list orphaned that way is kept, not deleted.
    AddGeneratedGroupLists(
        Vec<(
            collomatique_state_colloscopes::group_lists::GroupList,
            std::collections::BTreeSet<(
                collomatique_state_colloscopes::PeriodId,
                collomatique_state_colloscopes::SubjectId,
            )>,
        )>,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupListsUpdateError {
    #[error(transparent)]
    AddNewGroupList(#[from] AddNewGroupListError),
    #[error(transparent)]
    UpdateGroupList(#[from] UpdateGroupListError),
    #[error(transparent)]
    DeleteGroupList(#[from] DeleteGroupListError),
    #[error(transparent)]
    AssignGroupListToSubject(#[from] AssignGroupListToSubjectError),
    #[error(transparent)]
    DuplicatePreviousPeriod(#[from] DuplicatePreviousPeriodAssociationsError),
    #[error(transparent)]
    ClearPeriodAssociations(#[from] ClearPeriodAssociationsError),
    #[error(transparent)]
    AddGeneratedGroupLists(#[from] AddGeneratedGroupListsError),
}

/// The payload carries a filling, which can name students, so adding a list
/// can fail on a dangling student id.
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewGroupListError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateGroupListError {
    #[error("Group list id ({0:?}) is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteGroupListError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignGroupListToSubjectError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} has no interrogation and does not need a group list")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuplicatePreviousPeriodAssociationsError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    /// trying to override first period
    #[error("given period ({0:?}) is the first period")]
    FirstPeriodHasNoPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

/// The one way clearing a period can be wrong. Everything else the coordinates
/// could be blamed for — a subject with no interrogations, a subject excluded
/// from the period — is implied by there *being* an association to clear.
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClearPeriodAssociationsError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

/// The five ways the generated payload can be wrong, all of them address
/// facts: a filling naming a student the document does not have, or a coverage
/// pair naming a coordinate no association can be written at.
///
/// There is no `InvalidGroupListId` arm: every association the composite writes
/// names the list the session has just issued, so a caller cannot name a bad
/// one.
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddGeneratedGroupListsError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} has no interrogation and does not need a group list")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

/// Every student id a filling names, whichever variant it is.
///
/// `GroupListFilling::iter_students` deliberately covers the prefilled groups
/// only, but a student-existence sweep must also see the excluded set of an
/// automatic filling.
fn students_of(
    filling: &collomatique_state_colloscopes::group_lists::GroupListFilling,
) -> impl Iterator<Item = collomatique_state_colloscopes::StudentId> + '_ {
    filling
        .iter_students()
        .chain(filling.excluded_students().iter().copied())
}

impl GroupListsUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::GroupListId>, GroupListsUpdateError> {
        match self {
            Self::AddNewGroupList(group_list) => {
                // The payload is a sealed `GroupList`, so its internal
                // consistency is already settled; only student *existence* — a
                // state-dependent fact — is left to check here.
                for student_id in students_of(group_list.filling()) {
                    if !session
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .contains(&student_id)
                    {
                        return Err(AddNewGroupListError::InvalidStudentId(student_id).into());
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Add(group_list.clone()),
                        ),
                        self.get_desc(),
                    )
                    // A brand new list is named by nobody: no subject is
                    // associated with an id that did not exist a moment ago, and
                    // the colloscope holds no placement row for it either, so
                    // none of the four predicates watching a list has anything
                    // to look at. The only ids the payload carries are its
                    // students', checked just above.
                    .expect("a list nothing names yet contradicts nothing");
                let Some(collomatique_state_colloscopes::NewId::GroupListId(new_id)) = result
                else {
                    panic!("Unexpected result from GroupListOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateGroupList(group_list_id, group_list) => {
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains(group_list_id)
                {
                    return Err(UpdateGroupListError::InvalidGroupListId(*group_list_id).into());
                }

                for student_id in students_of(group_list.filling()) {
                    if !session
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .contains(&student_id)
                    {
                        return Err(UpdateGroupListError::InvalidStudentId(student_id).into());
                    }
                }

                // No reshaping, no rebuild, no arity assert: the payload is a
                // sealed value that already carries both halves.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Update(
                                *group_list_id,
                                group_list.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    // Everything a new payload can contradict is material that
                    // was already there, so the cascade repairs all of it: the
                    // colloscope placement of a student it now excludes, the
                    // placements and the interrogation groups the new group
                    // count no longer has, and the whole placement row if the
                    // list becomes prefilled. What the payload says about the
                    // list *itself* is the caller's own edit and lands verbatim
                    // — the four cleaning scans the old body ran here never
                    // looked at it either.
                    .expect("the cascade repairs whatever a new payload contradicts");
                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteGroupList(group_list_id) => {
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains(group_list_id)
                {
                    return Err(DeleteGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Remove(*group_list_id),
                        ),
                        self.get_desc(),
                    )
                    // The old body panicked here when a subject association
                    // still named the list — five cleaning scans were supposed
                    // to have emptied the way, three of them undoing the doomed
                    // list's own filling first, which the removal takes with it
                    // anyway. Both sites a removal leaves dangling are the
                    // cascade's business now: every association goes, the
                    // colloscope placement row goes, and each dropped
                    // association takes the group numbers of the colles at its
                    // coordinate with it.
                    .expect("the cascade repairs everything a removed list leaves behind");
                assert!(result.is_none());

                Ok(None)
            }
            Self::AssignGroupListToSubject(period_id, subject_id, group_list_id_opt) => {
                let Some(subject) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return Err(AssignGroupListToSubjectError::InvalidSubjectId(*subject_id).into());
                };

                if subject.parameters.interrogation_parameters.is_none() {
                    return Err(AssignGroupListToSubjectError::SubjectHasNoInterrogation(
                        *subject_id,
                    )
                    .into());
                }

                if subject.excluded_periods.contains(period_id) {
                    return Err(AssignGroupListToSubjectError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    )
                    .into());
                }

                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignGroupListToSubjectError::InvalidPeriodId(*period_id).into());
                }

                if let Some(group_list_id) = group_list_id_opt
                    && !session
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .group_list_map
                        .contains(group_list_id)
                {
                    return Err(
                        AssignGroupListToSubjectError::InvalidGroupListId(*group_list_id).into(),
                    );
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                *period_id,
                                *subject_id,
                                *group_list_id_opt,
                            ),
                        ),
                        self.get_desc(),
                    )
                    // The three ids the entry carries are checked just above,
                    // and the two predicates watching an association — a
                    // subject with no interrogations, a subject that does not
                    // run on the period — are the two prechecks between them.
                    // What is left is the colles already written at this
                    // coordinate: a list with fewer groups than they name (or no
                    // list at all, which puts the bound at zero) leaves them out
                    // of range, and the cascade trims them one group at a time.
                    .expect("the cascade trims whatever colles the new bound leaves out of range");
                assert!(result.is_none());

                Ok(None)
            }
            Self::DuplicatePreviousPeriod(period_id) => {
                let Some(position) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(DuplicatePreviousPeriodAssociationsError::InvalidPeriodId(
                        *period_id,
                    )
                    .into());
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(
                            *period_id,
                        )
                        .into(),
                    );
                }

                let previous_period_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let previous_period_assignments: std::collections::BTreeMap<_, _> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), group_list)| {
                        (period == previous_period_id).then_some((subject, *group_list))
                    })
                    .collect();

                // Read once, before the loop: nothing a cascade can answer
                // touches the subject list or the previous period's
                // associations — no fix creates a subject, excludes one from a
                // period or removes a group list — so what the loop plans
                // against the pre-state stays true op after op (the frame
                // rule).
                let subjects = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();

                for (subject_id, subject) in subjects.iter() {
                    let subject_id = &subject_id;
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if subject.excluded_periods.contains(&previous_period_id) {
                        continue;
                    }
                    if subject.parameters.interrogation_parameters.is_none() {
                        continue;
                    }

                    let previous_group_list_id =
                        previous_period_assignments.get(subject_id).cloned();

                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                    *period_id,
                                    *subject_id,
                                    previous_group_list_id,
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The same reasoning as the single assignment above,
                        // with the three prechecks replaced by the loop's own
                        // filters: the subject runs on both periods and holds
                        // interrogations, and the group list comes out of a live
                        // association. Only the colles of the target period can
                        // be left out of range, and those the cascade trims.
                        .expect(
                            "the cascade trims whatever colles the new bound leaves out of range",
                        );
                    assert!(result.is_none());
                }

                Ok(None)
            }
            Self::DeleteAllGroupLists => {
                // Only the lists that are there need removing, and the map
                // yields exactly those, so nothing here can fail. The ids are
                // collected up front so the mutable `session.apply` loop does
                // not overlap the shared read borrow.
                let group_list_ids: Vec<_> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .keys()
                    .collect();

                for group_list_id in group_list_ids {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::Remove(group_list_id),
                            ),
                            self.get_desc(),
                        )
                        // The same reasoning as the single removal above: both
                        // sites a removal leaves dangling are the cascade's.
                        .expect("the cascade repairs everything a removed list leaves behind");
                    assert!(result.is_none());
                }

                Ok(None)
            }
            Self::ClearPeriodAssociations(period_id) => {
                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(ClearPeriodAssociationsError::InvalidPeriodId(*period_id).into());
                }

                // Read once, before the loop (the frame rule): no repair a
                // cascade can make creates or removes an association, so the
                // coordinates planned against the pre-state stay true op after
                // op.
                let subject_ids: Vec<_> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), _group_list)| {
                        (period == *period_id).then_some(subject)
                    })
                    .collect();

                for subject_id in subject_ids {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                    *period_id, subject_id, None,
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The two predicates watching an association — a
                        // subject with no interrogations, a subject that does
                        // not run on the period — are implied by there being an
                        // association to clear. What is left is the colles
                        // written at the coordinate: taking the list away puts
                        // their bound at zero, and the cascade empties them.
                        .expect(
                            "the cascade trims whatever colles the new bound leaves out of range",
                        );
                    assert!(result.is_none());
                }

                Ok(None)
            }
            Self::AddGeneratedGroupLists(entries) => {
                // The whole payload is prechecked before anything lands, and
                // the answers survive the loop below (the frame rule): adding a
                // list and overwriting an association never touch students,
                // subjects or periods, and the only cascade an association can
                // trigger trims colles at its own coordinate.
                for (group_list, coverage) in entries {
                    for student_id in students_of(group_list.filling()) {
                        if !session
                            .get_data()
                            .get_inner_data()
                            .params
                            .students
                            .student_map
                            .contains(&student_id)
                        {
                            return Err(
                                AddGeneratedGroupListsError::InvalidStudentId(student_id).into()
                            );
                        }
                    }

                    for (period_id, subject_id) in coverage {
                        // The same four checks, in the same order, as the
                        // single assignment above — minus the group-list one,
                        // whose id the session issues itself a few lines down.
                        let Some(subject) = session
                            .get_data()
                            .get_inner_data()
                            .params
                            .subjects
                            .find_subject(*subject_id)
                        else {
                            return Err(
                                AddGeneratedGroupListsError::InvalidSubjectId(*subject_id).into()
                            );
                        };

                        if subject.parameters.interrogation_parameters.is_none() {
                            return Err(AddGeneratedGroupListsError::SubjectHasNoInterrogation(
                                *subject_id,
                            )
                            .into());
                        }

                        if subject.excluded_periods.contains(period_id) {
                            return Err(AddGeneratedGroupListsError::SubjectDoesNotRunOnPeriod(
                                *subject_id,
                                *period_id,
                            )
                            .into());
                        }

                        if session
                            .get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .find_period_position(*period_id)
                            .is_none()
                        {
                            return Err(
                                AddGeneratedGroupListsError::InvalidPeriodId(*period_id).into()
                            );
                        }
                    }
                }

                for (group_list, coverage) in entries {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::Add(
                                    group_list.clone(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The same reasoning as the single add above: a brand
                        // new list is named by nobody, and the only ids its
                        // payload carries are its students', swept in the
                        // precheck.
                        .expect("a list nothing names yet contradicts nothing");
                    let Some(collomatique_state_colloscopes::NewId::GroupListId(new_id)) = result
                    else {
                        panic!("Unexpected result from GroupListOp::Add");
                    };

                    for (period_id, subject_id) in coverage {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                        *period_id,
                                        *subject_id,
                                        Some(new_id),
                                    ),
                                ),
                                self.get_desc(),
                            )
                            // The same reasoning as the single assignment
                            // above, with the group list supplied by the
                            // session itself. Only the colles already written
                            // at this coordinate can be left out of range by
                            // the new bound — pre-state material, so the
                            // rendering corollary holds.
                            .expect(
                                "the cascade trims whatever colles the new bound leaves out of range",
                            );
                        assert!(result.is_none());
                    }
                }

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::GroupLists,
            match self {
                GroupListsUpdateOp::AddNewGroupList(_group_list) => {
                    "Ajouter une liste de groupes".into()
                }
                GroupListsUpdateOp::UpdateGroupList(_id, _group_list) => {
                    "Modifier une liste de groupes".into()
                }
                GroupListsUpdateOp::DeleteGroupList(_id) => "Supprimer une liste de groupes".into(),
                GroupListsUpdateOp::AssignGroupListToSubject(
                    _period_id,
                    _subject_id,
                    group_list_id,
                ) => {
                    if group_list_id.is_some() {
                        "Affecter une liste de groupes à une matière".into()
                    } else {
                        "Supprimer l'affectation d'une liste de groupes à une matière".into()
                    }
                }
                GroupListsUpdateOp::DuplicatePreviousPeriod(_period_id) => {
                    "Dupliquer les listes de groupes d'une période".into()
                }
                GroupListsUpdateOp::DeleteAllGroupLists => {
                    "Supprimer toutes les listes de groupes".into()
                }
                GroupListsUpdateOp::ClearPeriodAssociations(_period_id) => {
                    "Effacer les listes de groupes d'une période".into()
                }
                GroupListsUpdateOp::AddGeneratedGroupLists(_entries) => {
                    "Ajouter des listes de groupes générées automatiquement".into()
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! A group list is the one entity of the document that sits between two
    //! worlds. Above it, the *parameters*: how many groups there are and who is
    //! in them, which the caller describes whole. Below it, everything the
    //! colloscope wrote against those groups: the students placed in them and
    //! the group numbers the colles name. The two are what every fixture below
    //! separates.
    //!
    //! The caller's half is silent. Since the merge of July 31 2026 the payload
    //! is one sealed [`GroupList`] carrying parameters *and* filling, so a group
    //! they deleted and a student they took out of a group are their own edits:
    //! the op lands them verbatim and says nothing. The old split ops had to
    //! guess, and warned.
    //!
    //! The colloscope's half is the cascade's, and it is where the eleven
    //! cleaning scans of the old module went. Four of them are one convergence
    //! each — a placement out of the new count's range, a colle group out of
    //! range, a placement of a newly-excluded student, a placement row on a list
    //! that just became prefilled — and the remaining two dangle arms answer the
    //! removal. The panic the old removal kept for an association it had failed
    //! to clean (« Associated subjects should be properly cleaned ») has no
    //! reachable input left: the associations are dropped by the cascade now.
    //!
    //! Two shapes of the base document shape the fixtures. Its two group lists
    //! are **prefilled**, and a prefilled list may hold no colloscope placement
    //! row at all, so every fixture that needs a placement builds an automatic
    //! list of its own on top, in plain sight at its head. And it carries no
    //! colloscope, so the colles a fixture is about are written there too.
    //!
    //! One order is worth reading twice, in
    //! [deleting_a_list_takes_the_colles_its_associations_bounded_with_it]: the
    //! colles of a coordinate die *before* the association that bounded them,
    //! because dropping the association is what makes them out of range and the
    //! engine lands a repair's own repairs first. One group at a time, which is
    //! the case §3.13 of the plan looked at and deliberately left alone — here
    //! the user asked for the list to go, so the colles going with it is no
    //! surprise.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, ColloscopeOp, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op, SubjectOp,
        group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
        ids::{GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId, WeekId},
        subjects::Subject,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

    fn subject_by_name(data: &Data, name: &str) -> SubjectId {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .map(|(id, _subject)| id)
            .unwrap_or_else(|| panic!("the fixture should have a subject named {name}"))
    }

    fn student_by_name(data: &Data, surname: &str, firstname: &str) -> StudentId {
        data.get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .find(|(_id, student)| {
                student.desc.surname == surname && student.desc.firstname == firstname
            })
            .map(|(id, _student)| id)
            .unwrap_or_else(|| {
                panic!("the fixture should have a student named {firstname} {surname}")
            })
    }

    fn group_list_by_name(data: &Data, name: &str) -> GroupListId {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .iter()
            .find(|(_id, group_list)| group_list.params().name == name)
            .map(|(id, _group_list)| id)
            .unwrap_or_else(|| panic!("the fixture should have a group list named {name}"))
    }

    fn list_of(data: &Data, group_list: GroupListId) -> GroupList {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(&group_list)
            .expect("the fixture's group list should be live")
            .clone()
    }

    /// The `(period, subject)` coordinates `group_list` is used at, in key
    /// order — the order the reference site carries.
    fn associations_of(data: &Data, group_list: GroupListId) -> Vec<(PeriodId, SubjectId)> {
        data.get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .iter()
            .filter(|(_coordinate, assigned)| **assigned == group_list)
            .map(|(coordinate, _assigned)| coordinate)
            .collect()
    }

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    /// The subject's slots, in display order.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        data.get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect()
    }

    /// The weeks of `period` a colle may be written on for `slot`, in id order.
    fn writable_weeks(data: &Data, slot: SlotId, period: PeriodId) -> Vec<WeekId> {
        let params = &data.get_inner_data().params;
        let mut weeks: Vec<_> = params
            .week_ids()
            .filter(|week| {
                params.weeks.week_position(*week).map(|(p, _pos)| p) == Some(period)
                    && params.is_interrogation_possible(slot, *week)
            })
            .collect();
        weeks.sort();

        weeks
    }

    /// Group-list parameters with `count` unnamed groups.
    fn list_params(name: &str, count: usize) -> GroupListParameters {
        GroupListParameters {
            name: name.into(),
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            )
            .expect("statically non-empty"),
            group_names: vec![None; count],
        }
    }

    fn automatic_list(name: &str, count: usize, excluded: BTreeSet<StudentId>) -> GroupList {
        GroupList::new(
            list_params(name, count),
            GroupListFilling::Automatic {
                excluded_students: excluded,
            },
        )
        .expect("an automatic filling never constrains the group count")
    }

    fn prefilled_list(name: &str, groups: Vec<BTreeSet<StudentId>>) -> GroupList {
        GroupList::new(
            list_params(name, groups.len()),
            GroupListFilling::Prefilled {
                groups: groups
                    .into_iter()
                    .map(|students| PrefilledGroup { students })
                    .collect(),
            },
        )
        .expect("the group count is read off the group list itself")
    }

    /// Ids no document ever issued.
    fn dangling_group_list() -> GroupListId {
        unsafe { GroupListId::new(1u64 << 40) }
    }

    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the composite to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::GroupLists, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Applies one preparation op to the base a fixture builds on.
    fn prepare(base: &mut AppState<Data, Desc>, op: Op) {
        base.apply(op.clone(), (OpCategory::GroupLists, "Préparation".into()))
            .unwrap_or_else(|e| panic!("the preparation op {op:?} should land, got {e:?}"));
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &GroupListsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// The number of groups the automatic list of [placed_list] offers.
    const AUTOMATIC_GROUPS: usize = 3;

    /// The corner the base document does not carry, since both of its lists are
    /// prefilled: an **automatic** list — the only shape that may hold a
    /// colloscope placement row — used by Divination on the first period in
    /// place of the base's own list, with two students placed in it and one
    /// colle written on two of its groups.
    struct PlacedList {
        base: AppState<Data, Desc>,
        list: GroupListId,
        subject: SubjectId,
        period: PeriodId,
        slot: SlotId,
        week: WeekId,
        harry: StudentId,
        ron: StudentId,
    }

    fn placed_list() -> PlacedList {
        let mut base = hogwarts();
        let subject = subject_by_name(base.get_data(), "Divination");
        let period = period_at(base.get_data(), 0);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");

        let list = match base.apply(
            Op::GroupList(GroupListOp::Add(automatic_list(
                "Liste automatique",
                AUTOMATIC_GROUPS,
                BTreeSet::new(),
            ))),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(period, subject, Some(list))),
        );

        let slot = slots_of_subject(base.get_data(), subject)[0];
        let week = writable_weeks(base.get_data(), slot, period)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetGroupList(
                list,
                BTreeMap::from([(harry, 0), (ron, 1)]),
            )),
        );
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0, 2]),
            )),
        );

        PlacedList {
            base,
            list,
            subject,
            period,
            slot,
            week,
            harry,
            ron,
        }
    }

    /// A list nothing names yet cannot cost anything: the id comes back, the
    /// log stays empty, and the filling the caller described lands untouched —
    /// which is the whole point of the widened payload, the old op having
    /// forced every new list to be automatic.
    #[test]
    fn adding_a_list_lands_its_filling_verbatim_and_warns_about_nothing() {
        let base = hogwarts();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");
        let payload = prefilled_list(
            "Liste de rattrapage",
            vec![BTreeSet::from([harry]), BTreeSet::from([ron])],
        );

        let mut session = CascadeSession::new(base.clone());
        let op = GroupListsUpdateOp::AddNewGroupList(payload.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a fresh list names nothing but its students");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a group list returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(list_of(state.get_data(), new_id), payload);
    }

    /// The two writing ops share one student-existence sweep, and it has to see
    /// **both** halves of a filling: `GroupListFilling::iter_students` walks the
    /// prefilled groups only, so an automatic list's excluded set needs its own
    /// pass. Also the order between the two checks of the update: the list's own
    /// id is answered before anything the payload says.
    #[test]
    fn both_writing_ops_report_a_dead_id_whichever_part_of_the_payload_names_it() {
        let base = hogwarts();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let live = group_list_by_name(base.get_data(), "Divination");
        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            GroupListsUpdateOp::AddNewGroupList(prefilled_list(
                "Liste",
                vec![BTreeSet::from([dangling_student()])],
            ))
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::AddNewGroupList(AddNewGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::AddNewGroupList(automatic_list(
                "Liste",
                2,
                BTreeSet::from([dangling_student()]),
            ))
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::AddNewGroupList(AddNewGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                live,
                prefilled_list("Liste", vec![BTreeSet::from([dangling_student()])]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                live,
                automatic_list("Liste", 2, BTreeSet::from([dangling_student()])),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        // A payload naming a dead student, aimed at a dead list: the list wins.
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                dangling_group_list(),
                prefilled_list("Liste", vec![BTreeSet::from([dangling_student()])]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidGroupListId(
                dangling_group_list()
            )),
        );
        // And a removal, whose only way of being wrong is that one.
        assert_eq!(
            GroupListsUpdateOp::DeleteGroupList(dangling_group_list())
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DeleteGroupList(DeleteGroupListError::InvalidGroupListId(
                dangling_group_list()
            )),
        );
        // A live list with a live filling still passes both sweeps.
        GroupListsUpdateOp::UpdateGroupList(
            live,
            automatic_list("Liste", 2, BTreeSet::from([harry])),
        )
        .apply_to_session(&mut session)
        .expect("a live list and a live student are all the sweeps ask for");
    }

    /// The merge's own rule, on the new path: the payload is the caller's whole
    /// description of the list, so a group they deleted and the students they
    /// took out of it are their own edit. The list lands exactly as given and
    /// the log stays empty — no scan compares the old filling with the new one.
    #[test]
    fn replacing_a_list_lands_verbatim_and_says_nothing_about_what_the_caller_dropped() {
        let base = hogwarts();
        let list = group_list_by_name(base.get_data(), "Divination");
        let (params, filling) = list_of(base.get_data(), list).into_parts();
        let GroupListFilling::Prefilled { groups } = filling else {
            panic!("the fixture's Divination list should be prefilled");
        };
        assert_eq!(
            groups.len(),
            5,
            "the fixture's list should hold five groups"
        );

        // One group less, and the three students it held simply absent from the
        // payload.
        let payload = GroupList::new(
            GroupListParameters {
                group_names: params.group_names[..4].to_vec(),
                ..params
            },
            GroupListFilling::Prefilled {
                groups: groups[..4].to_vec(),
            },
        )
        .expect("four groups and four group names");

        let op = GroupListsUpdateOp::UpdateGroupList(list, payload.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::GroupList(GroupListOp::Update(list, payload))],
            )
            .get_data(),
        );
    }

    /// What the caller *cannot* see, and so must be told about: down to a
    /// single group, both a colle naming group 2 and Ron's placement in group 1
    /// are out of range. The colle goes first — the interrogation-row predicate
    /// is declared ahead of the placement one — and what still fits (group 0,
    /// and Harry in it) is left alone.
    #[test]
    fn shrinking_a_list_trims_the_colles_and_the_placements_the_dropped_groups_held() {
        let placed = placed_list();
        let payload = automatic_list("Liste automatique", 1, BTreeSet::new());

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveGroupsFromInterrogationCell {
                    slot: placed.slot,
                    week: placed.week,
                    groups: BTreeSet::from([2]),
                    rebuilt: BTreeSet::from([0]),
                },
                Fix::RemoveStudentColloscopePlacement {
                    group_list: placed.list,
                    student: placed.ron,
                    rebuilt: BTreeMap::from([(placed.harry, 0)]),
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::from([0]),
                    )),
                    Op::Colloscope(ColloscopeOp::SetGroupList(
                        placed.list,
                        BTreeMap::from([(placed.harry, 0)]),
                    )),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// Excluding a student the colloscope already placed: the placement goes,
    /// the other one stays, and the colles — which name groups, not students —
    /// are untouched.
    #[test]
    fn excluding_a_placed_student_takes_their_placement_out_of_the_colloscope() {
        let placed = placed_list();
        let payload = automatic_list(
            "Liste automatique",
            AUTOMATIC_GROUPS,
            BTreeSet::from([placed.ron]),
        );

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveStudentColloscopePlacement {
                group_list: placed.list,
                student: placed.ron,
                rebuilt: BTreeMap::from([(placed.harry, 0)]),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetGroupList(
                        placed.list,
                        BTreeMap::from([(placed.harry, 0)]),
                    )),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// A prefilled list holds no colloscope placement row at all, so turning a
    /// placed list prefilled retires the whole row — **one** repair, where the
    /// old cleaning walked it one student at a time and warned once per
    /// student. The row is the offending thing here, and there is no single
    /// placement to blame for it.
    #[test]
    fn turning_a_placed_list_prefilled_clears_its_whole_placement_row_at_once() {
        let placed = placed_list();
        let payload = prefilled_list(
            "Liste automatique",
            vec![
                BTreeSet::from([placed.harry]),
                BTreeSet::from([placed.ron]),
                BTreeSet::new(),
            ],
        );

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::ClearColloscopeGroupListRow {
                group_list: placed.list,
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetGroupList(placed.list, BTreeMap::new())),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// The removal's first dangle site, on the base's own document: a list used
    /// by one subject on all three periods. Every association goes, in the order
    /// the reference site carries — and the list's own filling goes with the row
    /// it lives in, which is why the old body's three pre-cleaning scans of it
    /// have no successor here.
    #[test]
    fn deleting_a_list_unassigns_every_subject_that_used_it() {
        let base = hogwarts();
        let list = group_list_by_name(base.get_data(), "Divination");
        let divination = subject_by_name(base.get_data(), "Divination");
        let coordinates = associations_of(base.get_data(), list);
        assert_eq!(
            coordinates,
            (0..3)
                .map(|index| (period_at(base.get_data(), index), divination))
                .collect::<Vec<_>>(),
            "the fixture's Divination list should serve its subject on all three periods",
        );

        let op = GroupListsUpdateOp::DeleteGroupList(list);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            coordinates
                .iter()
                .map(|(period, subject)| Fix::UnassignGroupList {
                    period: *period,
                    subject: *subject,
                })
                .collect::<Vec<_>>(),
        );
        let mut expected_ops: Vec<_> = coordinates
            .iter()
            .map(|(period, subject)| {
                Op::GroupList(GroupListOp::AssignToSubject(*period, *subject, None))
            })
            .collect();
        expected_ops.push(Op::GroupList(GroupListOp::Remove(list)));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The removal's second dangle site, and the order that is worth reading
    /// twice. Dropping the association is what takes the group bound of that
    /// coordinate to zero, so the colles written there become out of range —
    /// and the engine lands a repair's own repairs before the repair itself.
    /// The colles therefore die *first*, each cell emptied of all its groups in
    /// one go, then the association, then the placement row, and only then the
    /// list.
    #[test]
    fn deleting_a_list_takes_the_colles_its_associations_bounded_with_it() {
        let placed = placed_list();

        let op = GroupListsUpdateOp::DeleteGroupList(placed.list);
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveGroupsFromInterrogationCell {
                    slot: placed.slot,
                    week: placed.week,
                    groups: BTreeSet::from([0, 2]),
                    rebuilt: BTreeSet::new(),
                },
                Fix::UnassignGroupList {
                    period: placed.period,
                    subject: placed.subject,
                },
                Fix::ClearColloscopeGroupListRow {
                    group_list: placed.list,
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::new(),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        None,
                    )),
                    Op::Colloscope(ColloscopeOp::SetGroupList(placed.list, BTreeMap::new())),
                    Op::GroupList(GroupListOp::Remove(placed.list)),
                ],
            )
            .get_data(),
        );
    }

    /// Swapping in a shorter list: the colles that named a group the new list
    /// does not have are trimmed, the ones that fit are left alone.
    #[test]
    fn assigning_a_shorter_list_trims_the_colles_that_overflow_it() {
        let placed = placed_list();
        let mut base = placed.base;
        let short = match base.apply(
            Op::GroupList(GroupListOp::Add(automatic_list(
                "Petite liste",
                2,
                BTreeSet::new(),
            ))),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };

        let op = GroupListsUpdateOp::AssignGroupListToSubject(
            placed.period,
            placed.subject,
            Some(short),
        );
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot: placed.slot,
                week: placed.week,
                groups: BTreeSet::from([2]),
                rebuilt: BTreeSet::from([0]),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::from([0]),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        Some(short),
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// Taking the list away outright takes the bound to zero, so every group of
    /// every colle at that coordinate is out of range and the cell empties in a
    /// single fix naming all of them: here it reads the user's own edit back to
    /// them.
    #[test]
    fn unassigning_a_list_empties_the_colles_it_bounded_in_one_go() {
        let placed = placed_list();

        let op = GroupListsUpdateOp::AssignGroupListToSubject(placed.period, placed.subject, None);
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot: placed.slot,
                week: placed.week,
                groups: BTreeSet::from([0, 2]),
                rebuilt: BTreeSet::new(),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::new(),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        None,
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// The assignment's five ops-level prechecks, in the order they run — which
    /// is the surface: a call wrong in two ways at once gets the first answer.
    /// The subject comes before the period, and the two predicates the state
    /// layer would break on (a subject with no interrogations, a subject that
    /// does not run on the period) are pre-empted here rather than translated,
    /// because the cascade's answer to either would be to undo the caller's own
    /// association.
    #[test]
    fn the_assignment_op_reports_every_way_its_coordinates_can_be_wrong() {
        let mut base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let list = group_list_by_name(base.get_data(), "Liste principale");
        let first = period_at(base.get_data(), 0);
        let last = period_at(base.get_data(), 2);

        // A subject that skips a period: the enrolments and the association it
        // holds there have to go before the exclusion is legal.
        prepare(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(last, divination, BTreeSet::new())),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(last, divination, None)),
        );
        let excluding = Subject {
            excluded_periods: BTreeSet::from([last]),
            ..base
                .get_data()
                .get_inner_data()
                .params
                .subjects
                .find_subject(divination)
                .expect("the fixture's Divination subject should be live")
                .clone()
        };
        prepare(
            &mut base,
            Op::Subject(SubjectOp::Update(divination, excluding)),
        );

        let mut session = CascadeSession::new(base.clone());
        for (op, expected) in [
            (
                GroupListsUpdateOp::AssignGroupListToSubject(first, dangling_subject(), Some(list)),
                AssignGroupListToSubjectError::InvalidSubjectId(dangling_subject()),
            ),
            // Wrong in two ways: the subject is checked first, so it answers.
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    dangling_subject(),
                    Some(list),
                ),
                AssignGroupListToSubjectError::InvalidSubjectId(dangling_subject()),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(first, quidditch, Some(list)),
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(quidditch),
            ),
            // Likewise between the interrogation check and the period one.
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    quidditch,
                    Some(list),
                ),
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(quidditch),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(last, divination, Some(list)),
                AssignGroupListToSubjectError::SubjectDoesNotRunOnPeriod(divination, last),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    divination,
                    Some(list),
                ),
                AssignGroupListToSubjectError::InvalidPeriodId(dangling_period()),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    first,
                    divination,
                    Some(dangling_group_list()),
                ),
                AssignGroupListToSubjectError::InvalidGroupListId(dangling_group_list()),
            ),
        ] {
            assert_eq!(
                op.apply_to_session(&mut session).unwrap_err(),
                GroupListsUpdateError::AssignGroupListToSubject(expected),
                "{op:?}",
            );
        }
    }

    /// The composite: one assignment per subject that runs on both periods and
    /// holds interrogations, copying what the previous period says — including
    /// when what it says is « no list at all ». The fixture perturbs three
    /// coordinates and lets the duplication put two of them back.
    ///
    /// One of the loop's three filters is structurally shadowed, and no fixture
    /// can catch it: dropping the `interrogation_parameters.is_none()` skip
    /// changes nothing. A subject without interrogations may hold no
    /// association on *any* period — that is
    /// `Conv:AssociationForSubjectWithoutInterrogations` — so the previous
    /// period has nothing to copy for it and the assignment it would then write
    /// is `None` onto a coordinate that is already empty: a perfect no-op. The
    /// filter is kept because the old body had it, not because it guards
    /// anything.
    #[test]
    fn duplicating_copies_the_previous_periods_associations_including_the_absent_ones() {
        let mut base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let second = period_at(base.get_data(), 1);
        let potions = subject_by_name(base.get_data(), "Potions");
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let divination = subject_by_name(base.get_data(), "Divination");
        let main = group_list_by_name(base.get_data(), "Liste principale");
        let divination_list = group_list_by_name(base.get_data(), "Divination");

        // Arithmancie uses no list on the first period, so the duplication has
        // to *remove* the second period's; the two others are copied back onto
        // coordinates the preparation moved away.
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(first, arithmancie, None)),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, potions, None)),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, divination, Some(main))),
        );

        let op = GroupListsUpdateOp::DuplicatePreviousPeriod(second);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        // The six subjects with interrogations, in list order: the composite
        // writes every one of them, even where the value does not move.
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::GroupList(GroupListOp::AssignToSubject(second, potions, Some(main))),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Défense contre les forces du Mal"),
                        Some(main),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Métamorphose"),
                        Some(main),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(second, arithmancie, None)),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        divination,
                        Some(divination_list),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Potions - TP"),
                        Some(main),
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// The composite's assignments cascade like any other: a copied list with
    /// fewer groups than the colles of the target period name trims them.
    #[test]
    fn duplicating_trims_the_colles_the_copied_lists_no_longer_bound() {
        let mut base = hogwarts();
        let second = period_at(base.get_data(), 1);
        let divination = subject_by_name(base.get_data(), "Divination");
        let main = group_list_by_name(base.get_data(), "Liste principale");
        let divination_list = group_list_by_name(base.get_data(), "Divination");

        // Divination runs on the eight-group list for the second period, and a
        // colle there names its group 6. The first period's five-group list is
        // what the duplication copies back.
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, divination, Some(main))),
        );
        let slot = slots_of_subject(base.get_data(), divination)[0];
        let week = writable_weeks(base.get_data(), slot, second)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([1, 6]),
            )),
        );

        let op = GroupListsUpdateOp::DuplicatePreviousPeriod(second);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot,
                week,
                groups: BTreeSet::from([6]),
                rebuilt: BTreeSet::from([1]),
            }],
        );
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .colloscope
                .interrogation(slot, week),
            Some(&BTreeSet::from([1])),
        );
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .subjects_associations
                .get(&(second, divination)),
            Some(&divination_list),
        );
    }

    /// The composite's two ops-level prechecks: a period that does not exist,
    /// and the first period, which has no previous one to copy.
    #[test]
    fn duplicating_the_first_period_or_a_dead_one_is_refused() {
        let base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            GroupListsUpdateOp::DuplicatePreviousPeriod(dangling_period())
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodAssociationsError::InvalidPeriodId(dangling_period())
            ),
        );
        assert_eq!(
            GroupListsUpdateOp::DuplicatePreviousPeriod(first)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(first)
            ),
        );
    }

    /// The wholesale removal, on the fixture that carries every dangle site at
    /// once: three lists, associations on all three periods, a placement row
    /// and colles written against it. What comes out is what the same removals
    /// one at a time come out as, which is the whole claim — and, read on the
    /// document itself, a base with no list, no association and no placement.
    #[test]
    fn deleting_every_list_leaves_no_association_and_no_placement() {
        let placed = placed_list();
        let base = placed.base;
        let list_ids: Vec<_> = base
            .get_data()
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .collect();
        assert_eq!(
            list_ids.len(),
            3,
            "the fixture should carry the base's two prefilled lists and its own automatic one",
        );

        let op = GroupListsUpdateOp::DeleteAllGroupLists;
        let (state, warnings) = apply_alone(&base, &op);

        // The reference is the single removal run once per list, in the order
        // the map yields them — not a replay of elementary `Remove` ops, which
        // would leave every association dangling: the cleaning up is the
        // cascade's, and only a session runs it.
        let mut expected = base.clone();
        let mut expected_fixes = Vec::new();
        for group_list_id in &list_ids {
            let (next, step_warnings) = apply_alone(
                &expected,
                &GroupListsUpdateOp::DeleteGroupList(*group_list_id),
            );
            expected = next;
            expected_fixes.extend(fixes(&step_warnings));
        }

        assert_eq!(fixes(&warnings), expected_fixes);
        assert_eq!(state.get_data(), expected.get_data());
        let params = &state.get_data().get_inner_data().params;
        assert!(params.group_lists.group_list_map.is_empty());
        assert!(params.group_lists.subjects_associations.is_empty());
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .colloscope
                .group_list(placed.list),
            None,
        );
        assert!(
            expected_fixes.contains(&Fix::ClearColloscopeGroupListRow {
                group_list: placed.list,
            }),
            "the fixture should exercise the placement-row dangle site: {expected_fixes:?}",
        );
    }

    /// Clearing one period: its associations go, the other periods keep theirs,
    /// and every list survives — an orphaned list is legal state, and nothing
    /// here deletes one.
    #[test]
    fn clearing_a_period_leaves_the_other_periods_and_the_lists_alone() {
        let base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let second = period_at(base.get_data(), 1);
        let cleared: Vec<_> = base
            .get_data()
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .iter()
            .filter_map(|((period, subject), _list)| (period == first).then_some(subject))
            .collect();
        assert!(
            !cleared.is_empty(),
            "the fixture's first period should hold associations to clear",
        );
        let lists_before = base
            .get_data()
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .clone();

        let op = GroupListsUpdateOp::ClearPeriodAssociations(first);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                cleared
                    .iter()
                    .map(|subject| Op::GroupList(GroupListOp::AssignToSubject(
                        first, *subject, None
                    )))
                    .collect(),
            )
            .get_data(),
        );
        let group_lists = &state.get_data().get_inner_data().params.group_lists;
        assert!(
            !group_lists
                .subjects_associations
                .keys()
                .any(|(period, _subject)| period == first),
        );
        assert!(
            group_lists
                .subjects_associations
                .keys()
                .any(|(period, _subject)| period == second),
            "the second period should keep its own associations",
        );
        assert_eq!(group_lists.group_list_map, lists_before);
    }

    /// The composite's one ops-level precheck.
    #[test]
    fn clearing_a_dead_period_is_refused() {
        let base = hogwarts();
        let mut session = CascadeSession::new(base);

        assert_eq!(
            GroupListsUpdateOp::ClearPeriodAssociations(dangling_period())
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::ClearPeriodAssociations(
                ClearPeriodAssociationsError::InvalidPeriodId(dangling_period())
            ),
        );
    }

    /// Replays one generated entry on `expected`: the add, whose issued id the
    /// caller cannot know beforehand, then the associations that name it.
    ///
    /// Both sides of a comparison clone the same base, and the id issuer clones
    /// with it, so as long as neither side retries an add the ids agree.
    fn replay_generated_entry(
        expected: &mut AppState<Data, Desc>,
        group_list: GroupList,
        coverage: &[(PeriodId, SubjectId)],
    ) {
        let new_id = match expected.apply(
            Op::GroupList(GroupListOp::Add(group_list)),
            (OpCategory::GroupLists, "Expected".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };
        for (period, subject) in coverage {
            expected
                .apply(
                    Op::GroupList(GroupListOp::AssignToSubject(
                        *period,
                        *subject,
                        Some(new_id),
                    )),
                    (OpCategory::GroupLists, "Expected".into()),
                )
                .expect("the coordinate is live and the list was added a moment ago");
        }
    }

    /// The generation's own composite, on the base document — which carries no
    /// colloscope, so nothing here can cascade. Each entry's list is added
    /// first and the coordinates it covers are then written to it, one subject
    /// spanning two periods with a single list. The whole thing is one undo
    /// slot, which is what the caller asked for: the user accepted a
    /// generation, not fourteen edits.
    ///
    /// The second entry takes Divination's three coordinates away from the
    /// list that held them, which leaves that list named by nobody. It stays:
    /// an orphaned list is legal state, and no part of this op deletes one.
    #[test]
    fn generated_lists_land_with_their_associations_as_one_undo_slot() {
        let base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let second = period_at(base.get_data(), 1);
        let third = period_at(base.get_data(), 2);
        let potions = subject_by_name(base.get_data(), "Potions");
        let divination = subject_by_name(base.get_data(), "Divination");
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");
        let displaced = group_list_by_name(base.get_data(), "Divination");
        assert_eq!(
            associations_of(base.get_data(), displaced).len(),
            3,
            "the fixture's Divination list should serve its subject on all three periods",
        );

        let for_potions = prefilled_list(
            "Potions (générée)",
            vec![BTreeSet::from([harry]), BTreeSet::from([ron])],
        );
        let for_divination =
            prefilled_list("Divination (générée)", vec![BTreeSet::from([harry, ron])]);
        let potions_coverage = [(first, potions), (second, potions)];
        let divination_coverage = [
            (first, divination),
            (second, divination),
            (third, divination),
        ];

        let op = GroupListsUpdateOp::AddGeneratedGroupLists(vec![
            (for_potions.clone(), potions_coverage.into()),
            (for_divination.clone(), divination_coverage.into()),
        ]);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        let mut expected = base.clone();
        replay_generated_entry(&mut expected, for_potions, &potions_coverage);
        replay_generated_entry(&mut expected, for_divination, &divination_coverage);
        assert_eq!(state.get_data(), expected.get_data());

        // The orphan rule, read off the result rather than through the replay.
        assert!(
            state
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .group_list_map
                .contains(&displaced),
            "a list this op displaces is kept, not deleted",
        );
        assert_eq!(associations_of(state.get_data(), displaced), vec![]);
    }

    /// The composite's associations cascade like any other. The generated list
    /// has two groups where the colle at that coordinate names group 2, so the
    /// colle is trimmed — and the repair lands before the association that
    /// needed it, after the add that the association names.
    #[test]
    fn generated_lists_trim_the_colles_their_new_bound_leaves_out_of_range() {
        let placed = placed_list();
        let payload = prefilled_list(
            "Liste générée",
            vec![BTreeSet::from([placed.harry]), BTreeSet::from([placed.ron])],
        );
        let coverage = [(placed.period, placed.subject)];

        let op =
            GroupListsUpdateOp::AddGeneratedGroupLists(vec![(payload.clone(), coverage.into())]);
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot: placed.slot,
                week: placed.week,
                groups: BTreeSet::from([2]),
                rebuilt: BTreeSet::from([0]),
            }],
        );

        // Written out rather than replayed with the helper: the trimming sits
        // between the add and the association.
        let mut expected = placed.base.clone();
        let new_id = match expected.apply(
            Op::GroupList(GroupListOp::Add(payload)),
            (OpCategory::GroupLists, "Expected".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };
        prepare(
            &mut expected,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                placed.slot,
                placed.week,
                BTreeSet::from([0]),
            )),
        );
        prepare(
            &mut expected,
            Op::GroupList(GroupListOp::AssignToSubject(
                placed.period,
                placed.subject,
                Some(new_id),
            )),
        );
        assert_eq!(state.get_data(), expected.get_data());
    }

    /// The composite's precheck surface: the student sweep of the add, and the
    /// four coordinate checks of the assignment — the fifth, the group-list id,
    /// has no input here since the session issues it.
    ///
    /// The order is the surface too, and it is read three ways: entries answer
    /// in payload order, an entry's filling answers before its coverage, and
    /// within a pair the subject answers before the period.
    #[test]
    fn the_generated_lists_op_reports_every_way_its_payload_can_be_wrong() {
        let mut base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let first = period_at(base.get_data(), 0);
        let last = period_at(base.get_data(), 2);

        // A subject that skips a period, as in the single assignment's test:
        // the enrolments and the association it holds there have to go before
        // the exclusion is legal.
        prepare(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(last, divination, BTreeSet::new())),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(last, divination, None)),
        );
        let excluding = Subject {
            excluded_periods: BTreeSet::from([last]),
            ..base
                .get_data()
                .get_inner_data()
                .params
                .subjects
                .find_subject(divination)
                .expect("the fixture's Divination subject should be live")
                .clone()
        };
        prepare(
            &mut base,
            Op::Subject(SubjectOp::Update(divination, excluding)),
        );

        let sound = || automatic_list("Liste générée", 2, BTreeSet::new());
        let with_dead_student =
            || prefilled_list("Liste générée", vec![BTreeSet::from([dangling_student()])]);

        let mut session = CascadeSession::new(base.clone());
        for (op, expected) in [
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    with_dead_student(),
                    BTreeSet::new(),
                )]),
                AddGeneratedGroupListsError::InvalidStudentId(dangling_student()),
            ),
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    sound(),
                    BTreeSet::from([(first, dangling_subject())]),
                )]),
                AddGeneratedGroupListsError::InvalidSubjectId(dangling_subject()),
            ),
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    sound(),
                    BTreeSet::from([(first, quidditch)]),
                )]),
                AddGeneratedGroupListsError::SubjectHasNoInterrogation(quidditch),
            ),
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    sound(),
                    BTreeSet::from([(last, divination)]),
                )]),
                AddGeneratedGroupListsError::SubjectDoesNotRunOnPeriod(divination, last),
            ),
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    sound(),
                    BTreeSet::from([(dangling_period(), divination)]),
                )]),
                AddGeneratedGroupListsError::InvalidPeriodId(dangling_period()),
            ),
            // The entries are swept in payload order, so the first entry's
            // dead student answers before the second entry's dead subject.
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![
                    (with_dead_student(), BTreeSet::new()),
                    (sound(), BTreeSet::from([(first, dangling_subject())])),
                ]),
                AddGeneratedGroupListsError::InvalidStudentId(dangling_student()),
            ),
            // And within one entry, the filling answers before the coverage.
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    with_dead_student(),
                    BTreeSet::from([(first, dangling_subject())]),
                )]),
                AddGeneratedGroupListsError::InvalidStudentId(dangling_student()),
            ),
            // A pair wrong in two ways: the subject is checked first.
            (
                GroupListsUpdateOp::AddGeneratedGroupLists(vec![(
                    sound(),
                    BTreeSet::from([(dangling_period(), dangling_subject())]),
                )]),
                AddGeneratedGroupListsError::InvalidSubjectId(dangling_subject()),
            ),
        ] {
            assert_eq!(
                op.apply_to_session(&mut session).unwrap_err(),
                GroupListsUpdateError::AddGeneratedGroupLists(expected),
                "{op:?}",
            );
        }
    }

    /// The two degenerate payloads, both legal: a generation that produced
    /// nothing changes nothing, and an entry covering no coordinate is simply a
    /// list added and left unused.
    #[test]
    fn an_empty_payload_is_a_no_op_and_an_uncovered_entry_just_adds_its_list() {
        let base = hogwarts();

        let (state, warnings) =
            apply_alone(&base, &GroupListsUpdateOp::AddGeneratedGroupLists(vec![]));
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(state.get_data(), base.get_data());

        let payload = automatic_list("Liste orpheline", 2, BTreeSet::new());
        let op =
            GroupListsUpdateOp::AddGeneratedGroupLists(vec![(payload.clone(), BTreeSet::new())]);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        let new_id = group_list_by_name(state.get_data(), "Liste orpheline");
        assert_eq!(list_of(state.get_data(), new_id), payload);
        assert_eq!(associations_of(state.get_data(), new_id), vec![]);
    }
}
