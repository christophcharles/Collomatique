use collomatique_state_colloscopes::PromoteIncompatError;

use crate::rpc::error_msg::incompatibilities::IncompatibilitiesError;

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
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub duration: std::num::NonZeroU32,
    pub week_pattern_id: Option<MsgWeekPatternId>,
}

pub enum IncompatMsgDecodeError {
    SlotOverlapsWithNextDay,
}

impl TryFrom<IncompatMsg>
    for collomatique_state_colloscopes::incompats::IncompatibilityExternalData
{
    type Error = IncompatMsgDecodeError;

    fn try_from(value: IncompatMsg) -> Result<Self, IncompatMsgDecodeError> {
        let start = collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(value.start_day),
            start_time: value.start_time,
        };
        let slot = match collomatique_time::SlotWithDuration::new(start, value.duration.into()) {
            Some(s) => s,
            None => {
                return Err(IncompatMsgDecodeError::SlotOverlapsWithNextDay);
            }
        };
        Ok(
            collomatique_state_colloscopes::incompats::IncompatibilityExternalData {
                subject_id: value.subject_id.0,
                slot,
                week_pattern_id: value.week_pattern_id.map(|x| x.0),
            },
        )
    }
}
