//! rpc module
//!
//! This module contains the code to run an rpc server
//! as well as the necessary RCP messages

use std::io::Write;

use serde::{Deserialize, Serialize};

pub mod cmd_msg;
pub use cmd_msg::CmdMsg;

pub mod gui_answer;
pub use gui_answer::GuiAnswer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    RunPythonScript(String),
    SolveColloscope,
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
pub enum ResultMsg {
    InvalidMsg,
    Ack(Option<collomatique_state_colloscopes::NewId>),
    AckGui(GuiAnswer),
    Data(InternalDataStream),
    Error(collomatique_ops::UpdateError),
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

const RPC_MSG_MARKER: &'static str = "%%COLLOMATIQUE-RPC-MSG%%";

impl CompleteCmdMsg {
    pub fn check_if_msg(data: &str) -> bool {
        data.starts_with(RPC_MSG_MARKER)
    }

    pub fn from_text_msg(data: &str) -> Result<Self, String> {
        let Some(deprefixed) = data.strip_prefix(RPC_MSG_MARKER) else {
            return Err(trim_newline(data.to_string()));
        };

        match serde_json::from_str::<Self>(deprefixed) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(trim_newline(data.to_string())),
        }
    }

    pub fn into_text_msg(&self) -> String {
        RPC_MSG_MARKER.to_string()
            + &serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

pub fn wait_for_msg() -> String {
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
