use collomatique_state_colloscopes::PromoteIncompatError;

use crate::rpc::error_msg::incompatibilities::IncompatibilitiesError;
use std::num::NonZeroU32;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncompatibilitiesCmdMsg {
    AddNewIncompat(IncompatMsg),
    UpdateIncompat(MsgIncompatId, IncompatMsg),
    DeleteIncompat(MsgIncompatId),
}

impl IncompatibilitiesCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::IncompatibilitiesUpdateOp, IncompatibilitiesError> {
        use crate::ops::IncompatibilitiesUpdateOp;
        Ok(match self {
            IncompatibilitiesCmdMsg::AddNewIncompat(incompat) => {
                let external_incompat = match incompat.try_into() {
                    Ok(i) => i,
                    Err(IncompatMsgDecodeError::SlotOverlapsWithNextDay) => {
                        return Err(error_msg::AddNewIncompatError::SlotOverlapsWithNextDay.into());
                    }
                    Err(IncompatMsgDecodeError::TimeNotToTheMinute) => {
                        panic!("Invalid incompat received");
                    }
                };
                let new_incompat = match data.promote_incompat(external_incompat) {
                    Ok(i) => i,
                    Err(PromoteIncompatError::InvalidSubjectId(id)) => {
                        return Err(error_msg::AddNewIncompatError::InvalidSubjectId(
                            MsgSubjectId(id),
                        )
                        .into());
                    }
                    Err(PromoteIncompatError::InvalidWeekPatternId(id)) => {
                        return Err(error_msg::AddNewIncompatError::InvalidWeekPatternId(
                            MsgWeekPatternId(id),
                        )
                        .into());
                    }
                };
                IncompatibilitiesUpdateOp::AddNewIncompat(new_incompat)
            }
            IncompatibilitiesCmdMsg::UpdateIncompat(id, incompat) => {
                let Some(incompat_id) = data.validate_incompat_id(id.0) else {
                    return Err(error_msg::UpdateIncompatError::InvalidIncompatId(id).into());
                };
                let external_incompat = match incompat.try_into() {
                    Ok(i) => i,
                    Err(IncompatMsgDecodeError::SlotOverlapsWithNextDay) => {
                        return Err(error_msg::UpdateIncompatError::SlotOverlapsWithNextDay.into());
                    }
                    Err(IncompatMsgDecodeError::TimeNotToTheMinute) => {
                        panic!("Invalid incompat received");
                    }
                };
                let new_incompat = match data.promote_incompat(external_incompat) {
                    Ok(i) => i,
                    Err(PromoteIncompatError::InvalidSubjectId(id)) => {
                        return Err(error_msg::UpdateIncompatError::InvalidSubjectId(
                            MsgSubjectId(id),
                        )
                        .into());
                    }
                    Err(PromoteIncompatError::InvalidWeekPatternId(id)) => {
                        return Err(error_msg::UpdateIncompatError::InvalidWeekPatternId(
                            MsgWeekPatternId(id),
                        )
                        .into());
                    }
                };
                IncompatibilitiesUpdateOp::UpdateIncompat(incompat_id, new_incompat)
            }
            IncompatibilitiesCmdMsg::DeleteIncompat(id) => {
                let Some(incompat_id) = data.validate_incompat_id(id.0) else {
                    return Err(error_msg::DeleteIncompatError::InvalidIncompatId(id).into());
                };
                IncompatibilitiesUpdateOp::DeleteIncompat(incompat_id)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompatMsg {
    pub subject_id: MsgSubjectId,
    pub name: String,
    pub slots: Vec<IncompatSlotMsg>,
    pub minimum_free_slots: NonZeroU32,
    pub week_pattern_id: Option<MsgWeekPatternId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompatSlotMsg {
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub duration: std::num::NonZeroU32,
}

pub enum IncompatMsgDecodeError {
    SlotOverlapsWithNextDay,
    TimeNotToTheMinute,
}

impl TryFrom<IncompatMsg>
    for collomatique_state_colloscopes::incompats::IncompatibilityExternalData
{
    type Error = IncompatMsgDecodeError;

    fn try_from(value: IncompatMsg) -> Result<Self, IncompatMsgDecodeError> {
        let mut slots = vec![];
        for incompat_slot_msg in value.slots {
            let start = collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(incompat_slot_msg.start_day),
                start_time: collomatique_time::TimeOnMinutes::new(incompat_slot_msg.start_time)
                    .ok_or(IncompatMsgDecodeError::TimeNotToTheMinute)?,
            };
            let slot = match collomatique_time::SlotWithDuration::new(
                start,
                incompat_slot_msg.duration.into(),
            ) {
                Some(s) => s,
                None => {
                    return Err(IncompatMsgDecodeError::SlotOverlapsWithNextDay);
                }
            };
            slots.push(slot);
        }

        Ok(
            collomatique_state_colloscopes::incompats::IncompatibilityExternalData {
                subject_id: value.subject_id.0,
                name: value.name,
                slots,
                minimum_free_slots: value.minimum_free_slots,
                week_pattern_id: value.week_pattern_id.map(|x| x.0),
            },
        )
    }
}
