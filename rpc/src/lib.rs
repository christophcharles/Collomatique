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

pub mod solver_msg;
pub use solver_msg::{
    IlpSolveRequest, SerializedIlpProblem, SolverIncumbentInfo, SolverMsg, SolverProgressData,
    SolverResultData, SolverStatus,
};

pub mod strategy_msg;
pub use strategy_msg::{
    SerializedStrategyProgress, SerializedStrategyRequest, StrategyMsg, StrategyProgressRaw,
    StrategyResultData, StrategyStatus,
};

/// Failure to decode an RPC message from its wire form.
///
/// Both variants carry the raw payload that could not be decoded (the framing markers stripped),
/// matching the historical behaviour where the undecodable data was itself surfaced as the error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RpcDecodeError {
    /// The RPC frame markers were malformed; carries the data with markers stripped.
    #[error("Trame RPC mal formée : {0}")]
    MalformedFrame(String),
    /// The message body was not valid JSON for the target type; carries the raw body.
    #[error("Message RPC indécodable : {0}")]
    InvalidJson(String),
}

impl RpcDecodeError {
    /// The raw payload, for callers that need to suppress empty/no-detail errors.
    pub fn payload(&self) -> &str {
        match self {
            Self::MalformedFrame(s) | Self::InvalidJson(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMsg {
    RunPythonScript(String),
    SolveColloscope,
    SolveIlp(SerializedIlpProblem),
    RunStrategy(SerializedStrategyRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDataStream {
    serialized: String,
}

// The data stream carries the storage crate's file-format (spec-2) JSON, not a
// raw serde dump of the in-memory data. The file format is decoupled from the
// in-memory types (it goes through the storage layer's own `format` structs), so
// it is guaranteed serializable no matter how the in-memory representation
// evolves — it does not depend on any `Serialize` impl of `Data` or the tables
// it contains.
//
// The conversion is `To`/`From Data` (not `InnerData`) on purpose: `Data`
// carries the "is valid" invariant, whereas an arbitrary `InnerData` is not
// guaranteed to be a valid document. Writing a valid document can still fail
// on one thing the model allows and the file format does not — an id above
// the format's ceiling — which this panics on for now, like every other
// consumer of the writer.
impl From<&collomatique_state_colloscopes::Data> for InternalDataStream {
    fn from(value: &collomatique_state_colloscopes::Data) -> Self {
        InternalDataStream {
            serialized: collomatique_storage::serialize_data(value)
                .expect("document ids exceed the file-format ceiling"),
        }
    }
}

impl From<InternalDataStream> for collomatique_state_colloscopes::Data {
    fn from(value: InternalDataStream) -> Self {
        // Round-tripping our own writer's output must always succeed; any
        // caveats only arise for foreign or newer-version files, never here.
        let (data, _caveats) = collomatique_storage::deserialize_data(&value.serialized)
            .expect("data from our own data stream should always be deserializable");
        data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultMsg {
    InvalidMsg,
    Ack(Option<collomatique_state_colloscopes::NewId>),
    AckGui(GuiAnswer),
    Data(InternalDataStream),
    GlobalError(String),
    SolverControl(bool),
    StrategyControl(bool),
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

impl InitMsg {
    fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    fn to_text_msg(&self) -> String {
        serde_json::to_string_pretty(self).expect("Serializing to JSON should not fail")
    }
}

impl ResultMsg {
    fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    fn to_text_msg(&self) -> String {
        serde_json::to_string_pretty(self).expect("Serializing to JSON should not fail")
    }
}

impl CompleteCmdMsg {
    fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    fn to_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

#[derive(Clone, Debug)]
pub struct EncodedMsg {
    msg: String,
}

const RPC_MSG_MARKER: &str = "%%COLLOMATIQUE-RPC-MSG%%";
const RPC_CONTINUE_MARKER: &str = "%%COLLOMATIQUE-RPC-CON%%";
const RPC_END_MARKER: &str = "%%COLLOMATIQUE-RPC-END%%";
const NEW_LINE: &str = "\n";
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

    pub fn receive() -> Result<Self, RpcDecodeError> {
        Self::from_raw_string(Self::wait_for_raw_msg())
    }

    pub fn encode(self) -> String {
        Self::bundle_msg(self.msg)
    }

    pub fn from_raw_string(raw: String) -> Result<Self, RpcDecodeError> {
        let msg = Self::strip_msg(raw)?;
        Ok(Self { msg })
    }

    pub fn send_and_get_response(self) -> Result<Self, RpcDecodeError> {
        self.send();
        Self::receive()
    }

    pub fn send(self) {
        let bundled = Self::bundle_msg(self.msg);
        Self::send_raw_msg(&bundled);
    }

    pub fn send_rpc(cmd: CmdMsg) -> Result<ResultMsg, RpcDecodeError> {
        let msg: Self = CompleteCmdMsg::CmdMsg(cmd).into();
        let answer = msg.send_and_get_response()?;
        answer.try_into()
    }
}

impl From<InitMsg> for EncodedMsg {
    fn from(value: InitMsg) -> Self {
        EncodedMsg {
            msg: value.to_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for InitMsg {
    type Error = RpcDecodeError;
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
            msg: value.to_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for CompleteCmdMsg {
    type Error = RpcDecodeError;
    fn try_from(value: EncodedMsg) -> Result<Self, Self::Error> {
        CompleteCmdMsg::from_text_msg(&value.msg)
    }
}

impl From<ResultMsg> for EncodedMsg {
    fn from(value: ResultMsg) -> Self {
        EncodedMsg {
            msg: value.to_text_msg(),
        }
    }
}

impl TryFrom<EncodedMsg> for ResultMsg {
    type Error = RpcDecodeError;
    fn try_from(value: EncodedMsg) -> Result<Self, RpcDecodeError> {
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

    fn strip_msg(data: String) -> Result<String, RpcDecodeError> {
        let naked_data = data
            .replace(RPC_MSG_MARKER, "")
            .replace(RPC_CONTINUE_MARKER, "")
            .replace(RPC_END_MARKER, "");
        let malformed = || RpcDecodeError::MalformedFrame(naked_data.clone());
        let mut stripped = String::new();
        let mut reached_last = false;
        let mut first_run = true;
        for line in data.lines() {
            if reached_last {
                return Err(malformed());
            }
            if line.starts_with(RPC_END_MARKER) {
                if line != RPC_END_MARKER {
                    return Err(malformed());
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
                    None => return Err(malformed()),
                };
            } else if line.starts_with(RPC_CONTINUE_MARKER) {
                if first_run {
                    return Err(malformed());
                }
                stripped += match line.strip_prefix(RPC_CONTINUE_MARKER) {
                    Some(d) => d,
                    None => return Err(malformed()),
                };
            } else {
                return Err(malformed());
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
