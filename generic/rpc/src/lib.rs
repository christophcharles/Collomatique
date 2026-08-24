//! rpc module
//!
//! This module contains the code to run an rpc server
//! as well as the necessary RCP messages
//!
//! What lives here is the half of the protocol that knows nothing about any
//! particular problem: the transport, the ILP and strategy jobs, and the
//! envelopes that carry them. An application plugs its own messages in through
//! [`AppProtocol`].

use std::fmt::Debug;
use std::sync::{Mutex, OnceLock};

use serde::de::{self, DeserializeOwned};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod channel;

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

/// The application half of the protocol, plugged into the generic envelopes.
///
/// A channel carries exactly one job from start to finish, so this parameter also
/// records what kind of job it is. A channel that runs an ILP or a strategy and
/// nothing else uses [`NoApp`], whose message type is uninhabited.
///
/// The two ends of a channel need not agree on `P`, and in practice they do not:
/// the GUI spawns an ILP worker as `NoApp` while the engine binary reads its init
/// as the application's protocol, because the same binary also hosts Python
/// scripts. That works because serde's external tagging makes the non-`App`
/// variants serialize identically for every `P` — their payload types never
/// mention it.
///
/// The associated types are `Send + 'static` because an init message is handed to
/// the host's channel thread, and answers are built from reader threads.
pub trait AppProtocol {
    type Init: Serialize + DeserializeOwned + Clone + Debug + PartialEq + Eq + Send + 'static;
    type Cmd: Serialize + DeserializeOwned + Clone + Debug + PartialEq + Eq + Send + 'static;
    type Answer: Serialize + DeserializeOwned + Clone + Debug + PartialEq + Eq + Send + 'static;
}

/// The application half of a channel that carries no application traffic.
///
/// The `Debug`/`Clone`/`PartialEq`/`Eq` impls are what the envelopes' derived
/// impls ask of their parameter; a protocol marker is a unit type, so they cost
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoApp;

/// Nameable, never constructible: an `App` frame cannot exist on a [`NoApp`] channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoAppMsg {}

impl AppProtocol for NoApp {
    type Init = NoAppMsg;
    type Cmd = NoAppMsg;
    type Answer = NoAppMsg;
}

// Written out rather than derived, so that the failure message is ours. The
// serialize side is unreachable by construction; the deserialize side is what a
// peer sending an `App` frame down a `NoApp` channel would hit.
impl Serialize for NoAppMsg {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl<'de> Deserialize<'de> for NoAppMsg {
    fn deserialize<D: Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(de::Error::custom(
            "this channel carries no application messages",
        ))
    }
}

// The envelopes. `bound = ""` because the bounds serde would infer are on `P`
// itself, whereas what the bodies need is on its associated types, and
// `AppProtocol` already guarantees that much.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum InitMsg<P: AppProtocol> {
    SolveIlp(SerializedIlpProblem),
    RunStrategy(SerializedStrategyRequest),
    App(P::Init),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum CmdMsg<P: AppProtocol> {
    Solver(SolverMsg),
    Strategy(StrategyMsg),
    App(P::Cmd),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum ResultMsg<P: AppProtocol> {
    InvalidMsg,
    Ack,
    GlobalError(String),
    SolverControl(bool),
    StrategyControl(bool),
    App(P::Answer),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum CompleteCmdMsg<P: AppProtocol> {
    CmdMsg(CmdMsg<P>),
    GracefulExit,
}

// The three message bodies, in the form they travel in. Compact JSON throughout:
// the pretty form only ever existed because the old transport chopped a message
// into lines and needed them short enough for a terminal not to wrap.

impl<P: AppProtocol> InitMsg<P> {
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

impl<P: AppProtocol> ResultMsg<P> {
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

impl<P: AppProtocol> CompleteCmdMsg<P> {
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
///
/// Nothing here is parameterized by [`AppProtocol`]: the slot holds untyped
/// frame halves, and the protocol only appears in the signatures of the
/// functions that encode and decode.
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
pub fn receive_init<P: AppProtocol>() -> Result<InitMsg<P>, RpcError> {
    with_channel(|connection| {
        let body = connection.reader.recv()?.ok_or(RpcError::Closed)?;
        Ok(InitMsg::<P>::from_text_msg(&body)?)
    })
}

/// One round trip: send a command, block until its answer comes back.
pub fn send_command<P: AppProtocol>(cmd: CmdMsg<P>) -> Result<ResultMsg<P>, RpcError> {
    with_channel(|connection| {
        connection
            .writer
            .send(&CompleteCmdMsg::CmdMsg(cmd).to_text_msg())?;
        let body = connection.reader.recv()?.ok_or(RpcError::Closed)?;
        Ok(ResultMsg::<P>::from_text_msg(&body)?)
    })
}

/// Announce a clean end of work. The only message with no answer.
pub fn send_graceful_exit() -> Result<(), RpcError> {
    with_channel(|connection| {
        connection
            .writer
            .send(&CompleteCmdMsg::<NoApp>::GracefulExit.to_text_msg())?;
        Ok(())
    })
}
