use super::*;

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
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::WeekPatternId>, WeekPatternsUpdateError>
    {
        match self {
            Self::AddNewWeekPattern(week_pattern) => {
                let result = session
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
                        match &e {
                            // The pre-op state was valid, so any pattern->week
                            // dangle in the set was introduced by this Add: it
                            // is a made-up week id of its own exclusion set. The
                            // cascade cannot take it back out either — the
                            // pattern went back with the rolled-back op, so the
                            // map finds no pattern excluding that week and the
                            // target is convicted.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
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
                let result = session
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
                            Error, FixableInvariant, InvalidOp, PrecheckError, Reference,
                            WeekPatternPrecheckError, WeekRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::WeekPattern(
                                wpe,
                            ))) => match wpe {
                                WeekPatternPrecheckError::InvalidWeekPatternId(id) => {
                                    UpdateWeekPatternError::InvalidWeekPatternId(*id)
                                }
                                WeekPatternPrecheckError::WeekPatternIdAlreadyExists(_) => panic!(
                                    "Unexpected WeekPatternPrecheckError during UpdateWeekPattern: {e:?}"
                                ),
                            },
                            Error::BrokenInvariants(set) => {
                                // Same shape as the Add: the excluded week ids
                                // are the payload's own, and the pattern in the
                                // state still holds its old set, so there is
                                // nothing there for the map to take out.
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Week {
                                        target,
                                        site: WeekRefSite::WeekPatternExcludedWeek(_),
                                    }) = inv
                                    {
                                        return UpdateWeekPatternError::WeekPatternExcludesInvalidWeek(*target);
                                    }
                                }
                                // The old body's colloscope cleaning is gone
                                // with the cleaning phase: a cell on a week the
                                // new exclusion set disables breaks
                                // `InterrogationOnInactiveWeek`, and the cascade
                                // clears exactly that cell, never returned here.
                                panic!("Unexpected invariant breaks during UpdateWeekPattern: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateWeekPattern: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteWeekPattern(week_pattern_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Remove(*week_pattern_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, WeekPatternPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::WeekPattern(
                                wpe,
                            ))) => match wpe {
                                WeekPatternPrecheckError::InvalidWeekPatternId(id) => {
                                    DeleteWeekPatternError::InvalidWeekPatternId(*id)
                                }
                                WeekPatternPrecheckError::WeekPatternIdAlreadyExists(_) => panic!(
                                    "Unexpected WeekPatternPrecheckError during DeleteWeekPattern: {e:?}"
                                ),
                            },
                            // The old `BrokenInvariants` arm — one
                            // « … should be cleaned before removing week
                            // patterns » panic per reference site — is gone, and
                            // what replaces it is the step's one deliberate
                            // divergence from the legacy cleaning: both sites
                            // hold the pattern in an `Option` whose `None` is a
                            // legal value meaning "every week", so the cascade
                            // clears the reference and the slot and the
                            // incompatibility *survive*, where the old cleaning
                            // deleted them (and the slot's colloscope data with
                            // it).
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

#[cfg(test)]
mod tests {
    //! The family where the new world and the old one disagree on purpose.
    //!
    //! Two things point at a week pattern: a slot, which follows it to know
    //! which weeks it runs on, and an incompatibility, which follows it to know
    //! which weeks it applies on. Both hold it in an `Option` whose `None` is a
    //! legal, documented value meaning « toutes les semaines ». So when the
    //! pattern goes, the reference can go alone: the map clears it and the row
    //! survives, running every week from then on. The old cleaning *deleted*
    //! both rows instead — and the slot's colloscope data with them. That is
    //! the step's one recorded divergence from the legacy behaviour, ruled on
    //! July 28 2026, and the delete fixture below is where `ops/` pins it.
    //!
    //! The other half of the family is the update. Excluding a week from a
    //! pattern makes interrogations impossible on that week for every slot
    //! following it, so the colles already written there contradict the new
    //! pattern — the checker's `InterrogationOnInactiveWeek`. The old body
    //! scanned for those cells and cleaned them one at a time; now the cascade
    //! clears exactly the same cells and logs each one.
    //!
    //! The frozen hogwarts base carries both patterns and a slot on each, but
    //! no incompatibility follows a pattern and there is no colloscope at all.
    //! Those two shapes are set up by applying one elementary op on top of the
    //! base, in plain sight at the top of the fixture that needs it.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        ColloscopeOp, Fix, IncompatOp, Op, SlotOp, WeekPatternOp,
        ids::{Id, IncompatId, SlotId, WeekId, WeekPatternId},
        incompats::Incompatibility,
        slots::Slot,
        week_patterns::WeekPattern,
    };
    use std::collections::BTreeSet;

    fn week_pattern_by_name(data: &Data, name: &str) -> WeekPatternId {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .iter()
            .find(|(_id, week_pattern)| week_pattern.name == name)
            .map(|(id, _week_pattern)| id)
            .unwrap_or_else(|| panic!("the fixture should have a week pattern named {name}"))
    }

    fn incompat_by_name(data: &Data, name: &str) -> IncompatId {
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .iter()
            .find(|(_id, incompat)| incompat.name == name)
            .map(|(id, _incompat)| id)
            .unwrap_or_else(|| panic!("the fixture should have an incompatibility named {name}"))
    }

    fn week_pattern_of(data: &Data, week_pattern: WeekPatternId) -> WeekPattern {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&week_pattern)
            .expect("the fixture's week pattern should be live")
            .clone()
    }

    fn slot_of(data: &Data, slot: SlotId) -> Slot {
        data.get_inner_data()
            .params
            .slots
            .find_slot(slot)
            .expect("the fixture's slot should be live")
            .clone()
    }

    fn incompat_of(data: &Data, incompat: IncompatId) -> Incompatibility {
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .get(&incompat)
            .expect("the fixture's incompatibility should be live")
            .clone()
    }

    /// The slots following `week_pattern`, in id order — which is the order the
    /// cascade meets them, the reference site carrying the slot id.
    fn slots_following(data: &Data, week_pattern: WeekPatternId) -> Vec<SlotId> {
        let mut slots: Vec<_> = data
            .get_inner_data()
            .params
            .slots
            .all_slots()
            .filter(|(_id, slot)| slot.week_pattern == Some(week_pattern))
            .map(|(id, _slot)| *id)
            .collect();
        slots.sort();

        slots
    }

    /// The `n`-th week in global week order.
    fn week_at(data: &Data, index: usize) -> WeekId {
        data.get_inner_data()
            .params
            .week_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} weeks", index + 1))
    }

    /// Ids no document ever issued.
    fn dangling_week_pattern() -> WeekPatternId {
        unsafe { WeekPatternId::new(1u64 << 40) }
    }

    fn dangling_week() -> WeekId {
        unsafe { WeekId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the cascade to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::WeekPatterns, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &WeekPatternsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// A brand new pattern is followed by nobody, so it can contradict nothing:
    /// the id comes back and the warning log stays empty.
    #[test]
    fn adding_a_week_pattern_creates_it_and_warns_about_nothing() {
        let base = hogwarts();
        let second_week = week_at(base.get_data(), 1);
        let pattern = WeekPattern {
            name: "Semaines de contrôle".into(),
            excluded_weeks: BTreeSet::from([second_week]),
        };

        let mut session = CascadeSession::new(base.clone());
        let op = WeekPatternsUpdateOp::AddNewWeekPattern(pattern.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a live week is all this pattern names");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a week pattern returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(week_pattern_of(state.get_data(), new_id), pattern);
    }

    /// Renaming a pattern leaves its weeks alone, so nothing that follows it
    /// changes meaning: the update lands by itself.
    #[test]
    fn renaming_a_week_pattern_warns_about_nothing() {
        let base = hogwarts();
        let pattern = week_pattern_by_name(base.get_data(), "Semaines paires");

        let mut renamed = week_pattern_of(base.get_data(), pattern);
        renamed.name = "Semaines A".into();

        let op = WeekPatternsUpdateOp::UpdateWeekPattern(pattern, renamed.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::WeekPattern(WeekPatternOp::Update(pattern, renamed))],
            )
            .get_data(),
        );
    }

    /// The divergence, pinned. « Semaines paires » is followed by one slot; the
    /// setup op puts one incompatibility on it too, a shape hogwarts does not
    /// carry. Deleting the pattern used to delete both of them — the two
    /// « … should be cleaned before removing week patterns » panics of the old
    /// body were the guard on that cleaning. Now the cascade merely takes the
    /// reference out of each row: the colle slot stays, and runs every week
    /// from now on, and so does the incompatibility.
    #[test]
    fn deleting_a_pattern_keeps_the_slot_and_the_incompat_running_every_week() {
        let mut base = hogwarts();
        let pattern = week_pattern_by_name(base.get_data(), "Semaines paires");
        let incompat = incompat_by_name(base.get_data(), "Lundi Midi");

        let mut following_incompat = incompat_of(base.get_data(), incompat);
        following_incompat.week_pattern_id = Some(pattern);
        base.apply(
            Op::Incompat(IncompatOp::Update(incompat, following_incompat)),
            (OpCategory::Incompatibilities, "Préparation".into()),
        )
        .expect("an incompatibility may follow a live pattern");

        let slots = slots_following(base.get_data(), pattern);
        assert_eq!(
            slots.len(),
            1,
            "the fixture should have exactly one slot on « Semaines paires »"
        );
        let slot = slots[0];

        let mut freed_slot = slot_of(base.get_data(), slot);
        freed_slot.week_pattern = None;
        let mut freed_incompat = incompat_of(base.get_data(), incompat);
        freed_incompat.week_pattern_id = None;

        let op = WeekPatternsUpdateOp::DeleteWeekPattern(pattern);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::ClearSlotWeekPattern {
                    slot,
                    rebuilt: freed_slot.clone(),
                },
                Fix::ClearIncompatWeekPattern {
                    incompat,
                    rebuilt: freed_incompat.clone(),
                },
            ],
        );
        // What the legacy cleaning would have taken with the pattern.
        assert_eq!(slot_of(state.get_data(), slot), freed_slot);
        assert_eq!(incompat_of(state.get_data(), incompat), freed_incompat);
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Slot(SlotOp::Update(slot, freed_slot)),
                    Op::Incompat(IncompatOp::Update(incompat, freed_incompat)),
                    Op::WeekPattern(WeekPatternOp::Remove(pattern)),
                ],
            )
            .get_data(),
        );
    }

    /// Excluding a week from a pattern contradicts every colle already written
    /// on that week for a slot following it — the checker's
    /// `InterrogationOnInactiveWeek`. Hogwarts has no colloscope, so the setup
    /// op writes the one cell this is about, on the first week the slot is
    /// actually active on. The cascade clears exactly that cell; the pattern's
    /// other weeks, and every other cell, are untouched.
    #[test]
    fn excluding_a_week_clears_the_colles_written_on_it() {
        let mut base = hogwarts();
        let pattern = week_pattern_by_name(base.get_data(), "Semaines paires");
        let slot = slots_following(base.get_data(), pattern)[0];
        let week = base
            .get_data()
            .get_inner_data()
            .params
            .week_ids()
            .find(|week| {
                base.get_data()
                    .get_inner_data()
                    .params
                    .is_interrogation_possible(slot, *week)
            })
            .expect("the slot should be active on at least one week of its pattern");

        base.apply(
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0]),
            )),
            (OpCategory::Colloscope, "Préparation".into()),
        )
        .expect("a group of the associated list may be placed on an active week");

        let mut narrowed = week_pattern_of(base.get_data(), pattern);
        narrowed.excluded_weeks.insert(week);

        let op = WeekPatternsUpdateOp::UpdateWeekPattern(pattern, narrowed.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::ClearInterrogationCell { slot, week }]
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(slot, week, BTreeSet::new())),
                    Op::WeekPattern(WeekPatternOp::Update(pattern, narrowed)),
                ],
            )
            .get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing pattern. A rejected op changes nothing and logs nothing: the
    /// engine put the document back before the error came out.
    #[test]
    fn a_dead_week_pattern_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let dangling = dangling_week_pattern();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            WeekPatternsUpdateOp::UpdateWeekPattern(
                dangling,
                WeekPattern {
                    name: "Semaines fantômes".into(),
                    excluded_weeks: BTreeSet::new(),
                },
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            WeekPatternsUpdateError::UpdateWeekPattern(
                UpdateWeekPatternError::InvalidWeekPatternId(dangling)
            ),
        );
        assert_eq!(
            WeekPatternsUpdateOp::DeleteWeekPattern(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            WeekPatternsUpdateError::DeleteWeekPattern(
                DeleteWeekPatternError::InvalidWeekPatternId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::WeekPatterns, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A week id the payload made up dangles the moment the op lands. No
    /// pattern in the state excludes that week — the payload went back with the
    /// rolled-back op — so no repair can help: the map answers nothing, the
    /// engine convicts the op, and the scan turns the break back into the bad
    /// input it came from.
    #[test]
    fn a_dead_week_id_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let pattern = week_pattern_by_name(base.get_data(), "Semaines paires");
        let dangling = dangling_week();

        let mut excluding_a_ghost = week_pattern_of(base.get_data(), pattern);
        excluding_a_ghost.excluded_weeks.insert(dangling);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            WeekPatternsUpdateOp::AddNewWeekPattern(WeekPattern {
                name: "Semaines fantômes".into(),
                excluded_weeks: BTreeSet::from([dangling]),
            })
            .apply_to_session(&mut session)
            .unwrap_err(),
            WeekPatternsUpdateError::AddNewWeekPattern(
                AddNewWeekPatternError::WeekPatternExcludesInvalidWeek(dangling)
            ),
        );
        assert_eq!(
            WeekPatternsUpdateOp::UpdateWeekPattern(pattern, excluding_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            WeekPatternsUpdateError::UpdateWeekPattern(
                UpdateWeekPatternError::WeekPatternExcludesInvalidWeek(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
