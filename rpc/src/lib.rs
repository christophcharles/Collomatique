//! Collomatique-rpc crate
//!
//! This crate contains the code to run an rpc server
//! as well as the necessary RCP messages

use std::io::Write;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    Greetings,
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

fn send_raw_msg(msg: &str) -> Result<OutMsg, String> {
    eprint!("{}\r\n", msg);
    std::io::stderr().flush().expect("no error on flush");
    let data = wait_for_msg();
    OutMsg::from_text_msg(&data)
}

fn send_rpc(cmd: CmdMsg) -> Result<OutMsg, String> {
    let msg = CompleteCmdMsg::CmdMsg(cmd).into_text_msg();
    send_raw_msg(&msg)
}

fn wait_for_init_msg() -> Result<InitMsg, String> {
    let data = wait_for_msg();
    InitMsg::from_text_msg(&data)
}

fn send_exit() {
    eprint!("{}\r\n", CompleteCmdMsg::GracefulExit.into_text_msg());
}

/// Main RPC Engine function
///
/// Runs the RPC engine through stdin/stderr
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    println!("Waiting for initial payload...");
    let init_msg = match wait_for_init_msg() {
        Ok(x) => x,
        Err(e) => return Err(anyhow!("Unknown initial payload: {}", e)),
    };
    println!("Payload received!");

    match init_msg {
        InitMsg::Greetings => run_rpc_test()?,
    }

    send_exit();

    Ok(())
}

fn run_rpc_test() -> Result<(), anyhow::Error> {
    println!("Running RPC test...");
    for i in 1..=100 {
        println!("Msg {}", i);
        match i % 3 {
            1 => match send_rpc(CmdMsg::Success) {
                Ok(rep) => {
                    if rep != OutMsg::Ack {
                        return Err(anyhow!("Invalid response to valid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
            2 => match send_rpc(CmdMsg::Warning) {
                Ok(rep) => {
                    if rep != OutMsg::Ack {
                        return Err(anyhow!("Invalid response to valid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
            _ => match send_raw_msg("This is not a valid command...") {
                Ok(rep) => {
                    if rep != OutMsg::Invalid {
                        return Err(anyhow!("Ack response to invalid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    println!("End of RPC test");
    Ok(())
}
