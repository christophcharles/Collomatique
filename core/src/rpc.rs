//! rpc module
//!
//! This module contains the code to run an rpc server
//! as well as the necessary RCP messages

use anyhow::anyhow;
use std::io::Write;

use serde::{Deserialize, Serialize};

pub mod cmd_msg;
pub use cmd_msg::{CmdMsg, UpdateMsg};

pub mod error_msg;
pub use error_msg::ErrorMsg;

pub mod gui_answer;
pub use gui_answer::GuiAnswer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    RunPythonScript(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDataStream {
    serialized: String,
}

impl From<&collomatique_state_colloscopes::InnerData> for InternalDataStream {
    fn from(value: &collomatique_state_colloscopes::InnerData) -> Self {
        InternalDataStream {
            serialized: serde_json::to_string(value)
                .expect("Serialization of InnerData should never fail"),
        }
    }
}

impl From<InternalDataStream> for collomatique_state_colloscopes::InnerData {
    fn from(value: InternalDataStream) -> Self {
        serde_json::from_str::<collomatique_state_colloscopes::InnerData>(&value.serialized)
            .expect("Data from data stream should always be deserializable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewId {
    PeriodId(cmd_msg::MsgPeriodId),
    StudentId(cmd_msg::MsgStudentId),
    SubjectId(cmd_msg::MsgSubjectId),
    TeacherId(cmd_msg::MsgTeacherId),
    WeekPatternId(cmd_msg::MsgWeekPatternId),
    SlotId(cmd_msg::MsgSlotId),
    IncompatId(cmd_msg::MsgIncompatId),
    GroupListId(cmd_msg::MsgGroupListId),
    RuleId(cmd_msg::MsgRuleId),
    ColloscopeId(cmd_msg::MsgColloscopeId),
}

impl From<collomatique_state_colloscopes::NewId> for NewId {
    fn from(value: collomatique_state_colloscopes::NewId) -> Self {
        match value {
            collomatique_state_colloscopes::NewId::PeriodId(id) => NewId::PeriodId(id.into()),
            collomatique_state_colloscopes::NewId::StudentId(id) => NewId::StudentId(id.into()),
            collomatique_state_colloscopes::NewId::SubjectId(id) => NewId::SubjectId(id.into()),
            collomatique_state_colloscopes::NewId::TeacherId(id) => NewId::TeacherId(id.into()),
            collomatique_state_colloscopes::NewId::WeekPatternId(id) => {
                NewId::WeekPatternId(id.into())
            }
            collomatique_state_colloscopes::NewId::SlotId(id) => NewId::SlotId(id.into()),
            collomatique_state_colloscopes::NewId::IncompatId(id) => NewId::IncompatId(id.into()),
            collomatique_state_colloscopes::NewId::GroupListId(id) => NewId::GroupListId(id.into()),
            collomatique_state_colloscopes::NewId::RuleId(id) => NewId::RuleId(id.into()),
            collomatique_state_colloscopes::NewId::ColloscopeId(id) => {
                NewId::ColloscopeId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultMsg {
    InvalidMsg,
    Ack(Option<collomatique_state_colloscopes::NewId>),
    AckGui(GuiAnswer),
    Data(InternalDataStream),
    Error(crate::ops::UpdateError),
}

impl ResultMsg {
    pub fn generate_data_msg(data: &collomatique_state_colloscopes::Data) -> ResultMsg {
        ResultMsg::Data(data.get_inner_data().into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompleteCmdMsg {
    CmdMsg(CmdMsg),
    GracefulExit,
}

fn trim_newline(mut s: String) -> String {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    s
}

impl InitMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(trim_newline(data.to_string())),
        }
    }

    pub fn into_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

impl ResultMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(trim_newline(data.to_string())),
        }
    }

    pub fn into_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

impl CompleteCmdMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(trim_newline(data.to_string())),
        }
    }

    pub fn into_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

fn wait_for_msg() -> String {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    stdin.read_line(&mut buffer).expect("no error on reading");
    buffer
}

pub fn send_raw_msg(msg: &str) -> Result<ResultMsg, String> {
    print!("{}\r\n", msg);
    std::io::stdout().flush().expect("no error on flush");
    let data = wait_for_msg();
    ResultMsg::from_text_msg(&data)
}

pub fn send_rpc(cmd: CmdMsg) -> Result<ResultMsg, String> {
    let msg = CompleteCmdMsg::CmdMsg(cmd).into_text_msg();
    send_raw_msg(&msg)
}

pub fn wait_for_init_msg() -> Result<InitMsg, String> {
    let data = wait_for_msg();
    InitMsg::from_text_msg(&data)
}

pub fn send_exit() {
    print!("{}\r\n", CompleteCmdMsg::GracefulExit.into_text_msg());
    std::io::stdout().flush().expect("no error on flush");
}

/// Main RPC Engine function
///
/// Runs the RPC engine through stdin/stdout
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    eprintln!("Waiting for initial payload...");
    let init_msg = match wait_for_init_msg() {
        Ok(x) => x,
        Err(e) => return Err(anyhow!("Unknown initial payload: {}", e)),
    };
    eprintln!("Payload received!");

    match init_msg {
        InitMsg::RunPythonScript(script) => {
            crate::python::initialize();
            crate::python::run_python_script(script)?;
        }
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
