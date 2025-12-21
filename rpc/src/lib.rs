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

impl InitMsg {
    fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(&data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(data.to_string()),
        }
    }

    fn into_text_msg(&self) -> String {
        serde_json::to_string_pretty(self).expect("Serializing to JSON should not fail")
    }
}

impl ResultMsg {
    fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(data.to_string()),
        }
    }

    fn into_text_msg(&self) -> String {
        serde_json::to_string_pretty(self).expect("Serializing to JSON should not fail")
    }
}

impl CompleteCmdMsg {
    fn from_text_msg(data: &str) -> Result<Self, String> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(data.to_string()),
        }
    }

    fn into_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

#[derive(Clone, Debug)]
pub struct EncodedMsg {
    msg: String,
}

const RPC_MSG_MARKER: &'static str = "%%COLLOMATIQUE-RPC-MSG%%";
const RPC_CONTINUE_MARKER: &'static str = "%%COLLOMATIQUE-RPC-CON%%";
const RPC_END_MARKER: &'static str = "%%COLLOMATIQUE-RPC-END%%";
const NEW_LINE: &'static str = "\n";
const MAX_LINE_LEN: usize = 80;

impl EncodedMsg {
    pub fn check_if_msg(data: &str) -> bool {
        data.starts_with(RPC_MSG_MARKER)
            || data.starts_with(RPC_CONTINUE_MARKER)
            || data.starts_with(RPC_END_MARKER)
    }

    pub fn check_if_end(data: &str) -> bool {
        data.starts_with(RPC_END_MARKER)
    }

    pub fn receive() -> Result<Self, String> {
        Self::from_raw_string(Self::wait_for_raw_msg())
    }

    pub fn encode(self) -> String {
        Self::bundle_msg(self.msg)
    }

    pub fn from_raw_string(raw: String) -> Result<Self, String> {
        let msg = Self::strip_msg(raw)?;
        Ok(Self { msg })
    }

    pub fn send_and_get_response(self) -> Result<Self, String> {
        self.send();
        Self::receive()
    }

    pub fn send(self) {
        let bundled = Self::bundle_msg(self.msg);
        Self::send_raw_msg(&bundled);
    }

    pub fn send_rpc(cmd: CmdMsg) -> Result<ResultMsg, String> {
        let msg: Self = CompleteCmdMsg::CmdMsg(cmd).into();
        let answer = msg.send_and_get_response()?;
        answer.try_into()
    }
}

impl From<InitMsg> for EncodedMsg {
    fn from(value: InitMsg) -> Self {
        EncodedMsg {
            msg: value.into_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for InitMsg {
    type Error = String;
    fn try_from(value: EncodedMsg) -> Result<Self, Self::Error> {
        InitMsg::from_text_msg(&value.msg)
    }
}

impl From<CmdMsg> for EncodedMsg {
    fn from(value: CmdMsg) -> Self {
        Self::from(CompleteCmdMsg::CmdMsg(value))
    }
}

impl From<CompleteCmdMsg> for EncodedMsg {
    fn from(value: CompleteCmdMsg) -> Self {
        EncodedMsg {
            msg: value.into_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for CompleteCmdMsg {
    type Error = String;
    fn try_from(value: EncodedMsg) -> Result<Self, Self::Error> {
        CompleteCmdMsg::from_text_msg(&value.msg)
    }
}

impl From<ResultMsg> for EncodedMsg {
    fn from(value: ResultMsg) -> Self {
        EncodedMsg {
            msg: value.into_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for ResultMsg {
    type Error = String;
    fn try_from(value: EncodedMsg) -> Result<Self, String> {
        ResultMsg::from_text_msg(&value.msg)
    }
}

impl EncodedMsg {
    fn bundle_msg(data: String) -> String {
        let mut output = String::new();
        for line in data.lines() {
            output += RPC_MSG_MARKER;

            let mut remaining_line_opt = Some(line);
            while let Some(mut remaining_line) = remaining_line_opt.take() {
                if remaining_line.len() > MAX_LINE_LEN {
                    let target_len = remaining_line.floor_char_boundary(MAX_LINE_LEN);
                    let (start, end) = remaining_line.split_at(target_len);
                    remaining_line = start;
                    remaining_line_opt = Some(end);
                }
                output += remaining_line;
                if remaining_line_opt.is_some() {
                    output += NEW_LINE;
                    output += RPC_CONTINUE_MARKER;
                }
            }

            output += NEW_LINE;
        }
        output += RPC_END_MARKER;
        output += NEW_LINE;
        output
    }

    fn strip_msg(data: String) -> Result<String, String> {
        let naked_data = data
            .replace(RPC_MSG_MARKER, "")
            .replace(RPC_CONTINUE_MARKER, "")
            .replace(RPC_END_MARKER, "");
        let mut stripped = String::new();
        let mut reached_last = false;
        let mut first_run = true;
        for line in data.lines() {
            if reached_last {
                return Err(naked_data);
            }
            if line.starts_with(RPC_END_MARKER) {
                if line != RPC_END_MARKER {
                    return Err(naked_data);
                }
                reached_last = true;
                continue;
            }
            if line.starts_with(RPC_MSG_MARKER) {
                if !first_run {
                    stripped += NEW_LINE;
                }
                stripped += match line.strip_prefix(RPC_MSG_MARKER) {
                    Some(d) => d,
                    None => return Err(naked_data),
                };
            } else if line.starts_with(RPC_CONTINUE_MARKER) {
                if first_run {
                    return Err(naked_data);
                }
                stripped += match line.strip_prefix(RPC_CONTINUE_MARKER) {
                    Some(d) => d,
                    None => return Err(naked_data),
                };
            } else {
                return Err(naked_data);
            }
            first_run = false;
        }
        Ok(stripped)
    }

    fn wait_for_raw_msg() -> String {
        let mut output = String::new();
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        loop {
            buffer.clear();
            stdin.read_line(&mut buffer).expect("no error on reading");
            output += &buffer;
            if buffer.starts_with(RPC_END_MARKER) {
                break;
            }
        }
        output
    }

    fn send_raw_msg(msg: &str) {
        print!("{}", msg);
        std::io::stdout().flush().expect("no error on flush");
    }
}
