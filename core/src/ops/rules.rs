use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone)]
pub enum RulesUpdateWarning {}

impl RulesUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum RulesUpdateOp {
    AddNewRule(String, collomatique_state_colloscopes::rules::LogicRule),
    UpdateRule(
        collomatique_state_colloscopes::RuleId,
        String,
        collomatique_state_colloscopes::rules::LogicRule,
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
        _data: &T,
    ) -> Option<CleaningOp<RulesUpdateWarning>> {
        None
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
                        (OpCategory::Rules, self.get_desc()),
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
                    .get_rules()
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
                        (OpCategory::Rules, self.get_desc()),
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
                        (OpCategory::Rules, self.get_desc()),
                    )
                    .expect("All data should be valid at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatusForRule(rule_id, period_id, new_status) => {
                if data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusForRuleError::InvalidPeriodId(*period_id))?;
                }

                let mut rule = data
                    .get_data()
                    .get_rules()
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
                        (OpCategory::Rules, self.get_desc()),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> String {
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
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        _data: &T,
    ) -> Vec<RulesUpdateWarning> {
        vec![]
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
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
                        (OpCategory::Rules, self.get_desc()),
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
                    .get_rules()
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
                        (OpCategory::Rules, self.get_desc()),
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
                        (OpCategory::Rules, self.get_desc()),
                    )
                    .expect("All data should be valid at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatusForRule(rule_id, period_id, new_status) => {
                if data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusForRuleError::InvalidPeriodId(*period_id))?;
                }

                let mut rule = data
                    .get_data()
                    .get_rules()
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
                        (OpCategory::Rules, self.get_desc()),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }
}
