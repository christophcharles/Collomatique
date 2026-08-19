//! rpc module
//!
//! This module contains the code to run an rpc server
//! as well as the necessary RCP messages

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

pub mod channel;

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
/// A frame either arrives whole or does not arrive at all ([`channel`] guarantees that much),
/// so the only thing left to go wrong is the body itself. It is carried in the error, matching
/// the historical behaviour where the undecodable data was surfaced as the error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RpcDecodeError {
    /// The message body was not valid JSON for the target type; carries the raw body.
    #[error("Message RPC indécodable : {0}")]
    InvalidJson(String),
}

impl RpcDecodeError {
    /// The raw payload, for callers that need to suppress empty/no-detail errors.
    pub fn payload(&self) -> &str {
        match self {
            Self::InvalidJson(s) => s,
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
// guaranteed to be a valid document, and only valid documents should cross a
// process boundary. The storage layer itself works on `InnerData`, so both
// directions bridge explicitly: the write direction hands it the inner
// document, and the read direction runs the invariant gate. It only ever sees
// documents this very writer produced, so a rejection would be a bug and is
// treated as one. Writing a valid document can still fail on one thing the
// model allows and the file format does not — an id above the format's
// ceiling — which this panics on for now, like every other consumer of the
// writer.
impl From<&collomatique_state_colloscopes::Data> for InternalDataStream {
    fn from(value: &collomatique_state_colloscopes::Data) -> Self {
        InternalDataStream {
            serialized: collomatique_storage::serialize_data(value.get_inner_data())
                .expect("document ids exceed the file-format ceiling"),
        }
    }
}

impl From<InternalDataStream> for collomatique_state_colloscopes::Data {
    fn from(value: InternalDataStream) -> Self {
        // Round-tripping our own writer's output must always succeed; any
        // caveats only arise for foreign or newer-version files, never here.
        let (inner_data, _caveats) = collomatique_storage::deserialize_data(&value.serialized)
            .expect("data from our own data stream should always be deserializable");
        collomatique_state_colloscopes::Data::from_inner_data(inner_data)
            .expect("our own writer only serializes valid documents")
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

// The three message bodies, in the form they travel in. Compact JSON throughout:
// the pretty form only ever existed because the old transport chopped a message
// into lines and needed them short enough for a terminal not to wrap.

impl InitMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    pub fn to_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

impl ResultMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    pub fn to_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

impl CompleteCmdMsg {
    pub fn from_text_msg(data: &str) -> Result<Self, RpcDecodeError> {
        match serde_json::from_str::<Self>(data) {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(RpcDecodeError::InvalidJson(data.to_string())),
        }
    }

    pub fn to_text_msg(&self) -> String {
        serde_json::to_string(self).expect("Serializing to JSON should not fail")
    }
}

/// Failure of an RPC call made by a worker on its channel.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error(transparent)]
    Channel(#[from] channel::ChannelError),
    #[error(transparent)]
    Decode(#[from] RpcDecodeError),
    #[error("Le canal RPC n'a pas été ouvert")]
    NotConnected,
    #[error("Le canal RPC est déjà ouvert")]
    AlreadyConnected,
    #[error("L'hôte a fermé le canal RPC")]
    Closed,
}

/// The worker's end of the channel, connected once and used from anywhere afterwards.
///
/// The two halves live behind one lock, so a round trip is atomic: a request and
/// its answer cannot be interleaved with another thread's. The old transport gave
/// no such guarantee — it was a bare `print!` followed by a bare `read_line` on the
/// process's own stdio — and a hosted Python script can perfectly well call into
/// the RPC from two threads at once.
static CHANNEL: OnceLock<Mutex<Connection>> = OnceLock::new();

struct Connection {
    reader: channel::FrameReader,
    writer: channel::FrameWriter,
}

/// Join the channel named in the environment. Call once, before anything else.
pub fn connect_channel() -> Result<(), RpcError> {
    let (reader, writer) = channel::Endpoint::connect_from_env()?.split();
    CHANNEL
        .set(Mutex::new(Connection { reader, writer }))
        .map_err(|_| RpcError::AlreadyConnected)
}

fn with_channel<T>(f: impl FnOnce(&mut Connection) -> Result<T, RpcError>) -> Result<T, RpcError> {
    let mutex = CHANNEL.get().ok_or(RpcError::NotConnected)?;
    let mut connection = mutex.lock().unwrap();
    f(&mut connection)
}

/// Wait for the host's opening message, which says what this worker is for.
pub fn receive_init() -> Result<InitMsg, RpcError> {
    with_channel(|connection| {
        let body = connection.reader.recv()?.ok_or(RpcError::Closed)?;
        Ok(InitMsg::from_text_msg(&body)?)
    })
}

/// One round trip: send a command, block until its answer comes back.
pub fn send_command(cmd: CmdMsg) -> Result<ResultMsg, RpcError> {
    with_channel(|connection| {
        connection
            .writer
            .send(&CompleteCmdMsg::CmdMsg(cmd).to_text_msg())?;
        let body = connection.reader.recv()?.ok_or(RpcError::Closed)?;
        Ok(ResultMsg::from_text_msg(&body)?)
    })
}

/// Announce a clean end of work. The only message with no answer.
pub fn send_graceful_exit() -> Result<(), RpcError> {
    with_channel(|connection| {
        connection
            .writer
            .send(&CompleteCmdMsg::GracefulExit.to_text_msg())?;
        Ok(())
    })
}
