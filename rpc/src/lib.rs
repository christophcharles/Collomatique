//! Collomatique-rpc crate
//!
//! This crate contains the code to run an rpc server
//! as well as the necessary RCP messages

use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    RpcTest,
    RunPythonScript(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutMsg {
    Ack,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Success,
    Warning,
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

impl OutMsg {
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

pub fn send_raw_msg(msg: &str) -> Result<OutMsg, String> {
    print!("{}\r\n", msg);
    std::io::stdout().flush().expect("no error on flush");
    let data = wait_for_msg();
    OutMsg::from_text_msg(&data)
}

pub fn send_rpc(cmd: CmdMsg) -> Result<OutMsg, String> {
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
