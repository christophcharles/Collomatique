use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulesUpdateWarning {
    LooseColloscopeLinkWithRule(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::RuleId,
    ),
}

impl RulesUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LooseColloscopeLinkWithRule(colloscope_id, rule_id) => {
                let Some(colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return None;
                };
                let Some(rule) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .rules
                    .rule_map
                    .get(rule_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de la possibilité de mettre à jour le colloscope \"{}\" pour la règle \"{}\"",
                    colloscope.name,
                    rule.name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RulesUpdateOp {
    AddNewRule(
        String,
        collomatique_state_colloscopes::rules::LogicRule<collomatique_state_colloscopes::SlotId>,
    ),
    UpdateRule(
        collomatique_state_colloscopes::RuleId,
        String,
        collomatique_state_colloscopes::rules::LogicRule<collomatique_state_colloscopes::SlotId>,
    ),
    DeleteRule(collomatique_state_colloscopes::RuleId),
    UpdatePeriodStatusForRule(
        collomatique_state_colloscopes::RuleId,
        collomatique_state_colloscopes::PeriodId,
        bool,
    ),
}

#[derive(Debug, Error)]
pub enum RulesUpdateError {
    #[error(transparent)]
    AddNewRule(#[from] AddNewRuleError),
    #[error(transparent)]
    UpdateRule(#[from] UpdateRuleError),
    #[error(transparent)]
    DeleteRule(#[from] DeleteRuleError),
    #[error(transparent)]
    UpdatePeriodStatusForRule(#[from] UpdatePeriodStatusForRuleError),
}

#[derive(Debug, Error)]
pub enum AddNewRuleError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
}

#[derive(Debug, Error)]
pub enum UpdateRuleError {
    #[error("Rule ID {0:?} is invalid")]
    InvalidRuleId(collomatique_state_colloscopes::RuleId),
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
}

#[derive(Debug, Error)]
pub enum DeleteRuleError {
    #[error("Rule ID {0:?} is invalid")]
    InvalidRuleId(collomatique_state_colloscopes::RuleId),
}

#[derive(Debug, Error)]
pub enum UpdatePeriodStatusForRuleError {
    #[error("Rule ID {0:?} is invalid")]
    InvalidRuleId(collomatique_state_colloscopes::RuleId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl RulesUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<RulesUpdateWarning>> {
        match self {
            Self::AddNewRule(_name, _desc) => None,
            Self::UpdateRule(_rule_id, _name, _desc) => None,
            Self::UpdatePeriodStatusForRule(_rule_id, _period_id, _status) => None,
            Self::DeleteRule(rule_id) => {
                for (colloscope_id, colloscope) in
                    &data.get_data().get_inner_data().colloscopes.colloscope_map
                {
                    if colloscope.id_maps.rules.contains_key(rule_id) {
                        let mut new_colloscope = colloscope.clone();
                        new_colloscope.id_maps.rules.remove(rule_id);

                        return Some(CleaningOp {
                            warning: RulesUpdateWarning::LooseColloscopeLinkWithRule(
                                *colloscope_id,
                                *rule_id,
                            ),
                            op: UpdateOp::Colloscopes(ColloscopesUpdateOp::UpdateColloscope(
                                *colloscope_id,
                                new_colloscope,
                            )),
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
    ) -> Result<Option<collomatique_state_colloscopes::RuleId>, RulesUpdateError> {
        match self {
            Self::AddNewRule(name, rule) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Rule(
                            collomatique_state_colloscopes::RuleOp::Add(
                                collomatique_state_colloscopes::rules::Rule {
                                    name: name.clone(),
                                    excluded_periods: BTreeSet::new(),
                                    desc: rule.clone(),
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Rule(re) = e {
                            match re {
                                collomatique_state_colloscopes::RuleError::InvalidSlotId(id) => {
                                    AddNewRuleError::InvalidSlotId(id)
                                }
                                _ => panic!("Unexpected subject error during AddNewRule: {:?}", re),
                            }
                        } else {
                            panic!("Unexpected error during AddNewRule: {:?}", e);
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::RuleId(new_id)) = result else {
                    panic!("Unexpected result from RuleOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateRule(rule_id, name, rule) => {
                let current_rule = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .rules
                    .rule_map
                    .get(rule_id)
                    .ok_or(UpdateRuleError::InvalidRuleId(*rule_id))?;

                let excluded_periods = current_rule.excluded_periods.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Rule(
                            collomatique_state_colloscopes::RuleOp::Update(
                                *rule_id,
                                collomatique_state_colloscopes::rules::Rule {
                                    name: name.clone(),
                                    excluded_periods,
                                    desc: rule.clone(),
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Rule(re) = e {
                            match re {
                                collomatique_state_colloscopes::RuleError::InvalidRuleId(_id) => {
                                    panic!("Rule ID should be valid at this point")
                                }
                                collomatique_state_colloscopes::RuleError::InvalidSlotId(id) => {
                                    UpdateRuleError::InvalidSlotId(id)
                                }
                                _ => panic!("Unexpected subject error during UpdateRule: {:?}", re),
                            }
                        } else {
                            panic!("Unexpected error during UpdateRule: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteRule(rule_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Rule(
                            collomatique_state_colloscopes::RuleOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Rule(re) = e {
                            match re {
                                collomatique_state_colloscopes::RuleError::InvalidRuleId(id) => {
                                    DeleteRuleError::InvalidRuleId(id)
                                }
                                _ => panic!("Unexpected subject error during DeleteRule: {:?}", re),
                            }
                        } else {
                            panic!("Unexpected error during DeleteRule: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatusForRule(rule_id, period_id, new_status) => {
                if data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusForRuleError::InvalidPeriodId(*period_id))?;
                }

                let mut rule = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .rules
                    .rule_map
                    .get(rule_id)
                    .ok_or(UpdatePeriodStatusForRuleError::InvalidRuleId(*rule_id))?
                    .clone();

                if *new_status {
                    rule.excluded_periods.remove(period_id);
                } else {
                    rule.excluded_periods.insert(*period_id);
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Rule(
                            collomatique_state_colloscopes::RuleOp::Update(*rule_id, rule),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Rules,
            match self {
                RulesUpdateOp::AddNewRule(_desc, _rule) => "Ajouter une règle".into(),
                RulesUpdateOp::UpdateRule(_id, _desc, _rule) => "Modifier une règle".into(),
                RulesUpdateOp::DeleteRule(_id) => "Supprimer une règle".into(),
                Self::UpdatePeriodStatusForRule(_rule_id, _period_id, status) => {
                    if *status {
                        "Activer une règle sur une période".into()
                    } else {
                        "Désactiver une règle sur une période".into()
                    }
                }
            },
        )
    }
}
