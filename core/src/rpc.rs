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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    RunPythonScript(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDataStream {
    serialized: String,
}

impl From<&collomatique_state_colloscopes::Data> for InternalDataStream {
    fn from(value: &collomatique_state_colloscopes::Data) -> Self {
        InternalDataStream {
            serialized: collomatique_storage::serialize_data(value),
        }
    }
}

impl From<InternalDataStream> for collomatique_state_colloscopes::Data {
    fn from(value: InternalDataStream) -> Self {
        let (data, caveats) = collomatique_storage::deserialize_data(&value.serialized)
            .expect("Correctly formatted serialization");
        if !caveats.is_empty() {
            panic!("There should be no caveats when decoding data stream from RPC");
        }
        data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultMsg {
    InvalidMsg,
    Ack,
    Data(InternalDataStream),
    Error(ErrorMsg),
}

impl ResultMsg {
    pub fn generate_data_msg(data: &collomatique_state_colloscopes::Data) -> ResultMsg {
        ResultMsg::Data(data.into())
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
