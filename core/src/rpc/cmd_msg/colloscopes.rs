use crate::rpc::error_msg::ColloscopesError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColloscopesCmdMsg {
    AddEmptyColloscope(String),
    CopyColloscope(MsgColloscopeId, String),
    UpdateColloscope(MsgColloscopeId, ColloscopeMsg),
    DeleteColloscope(MsgColloscopeId),
}

impl ColloscopesCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::ColloscopesUpdateOp, ColloscopesError> {
        use crate::ops::ColloscopesUpdateOp;
        Ok(match self {
            ColloscopesCmdMsg::AddEmptyColloscope(new_name) => {
                ColloscopesUpdateOp::AddEmptyColloscope(new_name)
            }
            ColloscopesCmdMsg::CopyColloscope(id, new_name) => {
                let Some(colloscope_id) = data
                    .get_inner_data()
                    .colloscopes
                    .validate_colloscope_id(id.0)
                else {
                    return Err(error_msg::CopyColloscopeError::InvalidColloscopeId(id).into());
                };
                ColloscopesUpdateOp::CopyColloscope(colloscope_id, new_name)
            }
            ColloscopesCmdMsg::UpdateColloscope(_id, _colloscope) => {
                todo!("Must implement ColloscopeMsg first to encode Colloscope description across rpc boundary")
            }
            ColloscopesCmdMsg::DeleteColloscope(id) => {
                let Some(colloscope_id) = data
                    .get_inner_data()
                    .colloscopes
                    .validate_colloscope_id(id.0)
                else {
                    return Err(error_msg::DeleteColloscopeError::InvalidColloscopeId(id).into());
                };
                ColloscopesUpdateOp::DeleteColloscope(colloscope_id)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeMsg {}
