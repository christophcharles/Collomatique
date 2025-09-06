use collomatique_state_colloscopes::colloscope_params::PromoteSlotError;

use crate::rpc::error_msg::slots::SlotsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotsCmdMsg {
    AddNewSlot(MsgSubjectId, SlotMsg),
    UpdateSlot(MsgSlotId, SlotMsg),
    DeleteSlot(MsgSlotId),
    MoveSlotUp(MsgSlotId),
    MoveSlotDown(MsgSlotId),
}

impl SlotsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::SlotsUpdateOp, SlotsError> {
        use crate::ops::SlotsUpdateOp;
        Ok(match self {
            SlotsCmdMsg::AddNewSlot(id, slot) => {
                let Some(subject_id) = data.get_inner_data().main_params.validate_subject_id(id.0)
                else {
                    return Err(error_msg::AddNewSlotError::InvalidSubjectId(id).into());
                };
                let slot = match slot.try_into() {
                    Ok(s) => s,
                    Err(SlotMsgDecodeError::TimeNotToTheMinute) => {
                        panic!("Invalid slot received");
                    }
                };
                let new_slot = match data.get_inner_data().main_params.promote_slot(slot) {
                    Ok(s) => s,
                    Err(PromoteSlotError::InvalidTeacherId(id)) => {
                        return Err(
                            error_msg::AddNewSlotError::InvalidTeacherId(MsgTeacherId(id)).into(),
                        )
                    }
                    Err(PromoteSlotError::InvalidWeekPatternId(id)) => {
                        return Err(error_msg::AddNewSlotError::InvalidWeekPatternId(
                            MsgWeekPatternId(id),
                        )
                        .into())
                    }
                };
                SlotsUpdateOp::AddNewSlot(subject_id, new_slot)
            }
            SlotsCmdMsg::UpdateSlot(id, slot) => {
                let Some(slot_id) = data.get_inner_data().main_params.validate_slot_id(id.0) else {
                    return Err(error_msg::UpdateSlotError::InvalidSlotId(id).into());
                };
                let slot = match slot.try_into() {
                    Ok(s) => s,
                    Err(SlotMsgDecodeError::TimeNotToTheMinute) => {
                        panic!("Invalid slot received");
                    }
                };
                let new_slot = match data.get_inner_data().main_params.promote_slot(slot) {
                    Ok(s) => s,
                    Err(PromoteSlotError::InvalidTeacherId(id)) => {
                        return Err(
                            error_msg::UpdateSlotError::InvalidTeacherId(MsgTeacherId(id)).into(),
                        )
                    }
                    Err(PromoteSlotError::InvalidWeekPatternId(id)) => {
                        return Err(error_msg::UpdateSlotError::InvalidWeekPatternId(
                            MsgWeekPatternId(id),
                        )
                        .into())
                    }
                };
                SlotsUpdateOp::UpdateSlot(slot_id, new_slot)
            }
            SlotsCmdMsg::DeleteSlot(id) => {
                let Some(slot_id) = data.get_inner_data().main_params.validate_slot_id(id.0) else {
                    return Err(error_msg::DeleteSlotError::InvalidSlotId(id).into());
                };
                SlotsUpdateOp::DeleteSlot(slot_id)
            }
            SlotsCmdMsg::MoveSlotUp(id) => {
                let Some(slot_id) = data.get_inner_data().main_params.validate_slot_id(id.0) else {
                    return Err(error_msg::MoveSlotUpError::InvalidSlotId(id).into());
                };
                SlotsUpdateOp::MoveSlotUp(slot_id)
            }
            SlotsCmdMsg::MoveSlotDown(id) => {
                let Some(slot_id) = data.get_inner_data().main_params.validate_slot_id(id.0) else {
                    return Err(error_msg::MoveSlotDownError::InvalidSlotId(id).into());
                };
                SlotsUpdateOp::MoveSlotDown(slot_id)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotMsg {
    pub teacher_id: MsgTeacherId,
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub extra_info: String,
    pub week_pattern: Option<MsgWeekPatternId>,
    pub cost: i32,
}

pub enum SlotMsgDecodeError {
    TimeNotToTheMinute,
}

impl TryFrom<SlotMsg> for collomatique_state_colloscopes::slots::SlotExternalData {
    type Error = SlotMsgDecodeError;

    fn try_from(value: SlotMsg) -> Result<Self, SlotMsgDecodeError> {
        Ok(collomatique_state_colloscopes::slots::SlotExternalData {
            teacher_id: value.teacher_id.0,
            start_time: collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(value.start_day),
                start_time: collomatique_time::TimeOnMinutes::new(value.start_time)
                    .ok_or(SlotMsgDecodeError::TimeNotToTheMinute)?,
            },
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.0),
            cost: value.cost,
        })
    }
}
