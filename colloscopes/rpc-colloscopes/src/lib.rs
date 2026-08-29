//! The colloscope application's half of the RPC protocol.
//!
//! [`collomatique_rpc`] owns the transport, the ILP and strategy jobs, and the
//! envelopes that carry them. Everything below mentions a colloscope document,
//! which is exactly why it is not there.
//!
//! What rides on it is the hosted Python script: the host says which script to
//! run, and the script asks for the document and hands one back. The
//! interactive console is the same thing with the lines typed by the user
//! coming down as they are asked for.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// The application half of the protocol, for a channel that hosts a Python script.
///
/// The `Debug`/`Clone`/`PartialEq`/`Eq` impls are what the envelopes' derived
/// impls ask of their parameter; a protocol marker is a unit type, so they cost
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColloProtocol;

impl collomatique_rpc::AppProtocol for ColloProtocol {
    type Init = AppInitMsg;
    type Cmd = AppCmdMsg;
    type Answer = AppAnswerMsg;
}

/// What the host opens the channel with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppInitMsg {
    RunPythonScript(String),
    StartPythonRepl,
}

/// What the script, or the console, asks of the host.
///
/// [`AppCmdMsg::ReadLine`] and [`AppCmdMsg::ReplaceData`] only make sense for
/// the console: the script has no user to type at it, and it hands its document
/// over without asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppCmdMsg {
    GetData,
    SetData(InternalDataStream),
    ReadLine {
        prompt: String,
    },
    /// `token` is what [`InternalDataStream::token`] gave for the document the
    /// console read, and `None` when it never read one.
    ReplaceData {
        data: InternalDataStream,
        token: Option<u64>,
    },
}

/// What the host answers, when the answer is not one of the generic ones
/// ([`collomatique_rpc::ResultMsg::Ack`] and friends).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppAnswerMsg {
    Data(InternalDataStream),
    Line(String),
    /// The document now held by the host, as a token.
    ReplaceDone {
        token: u64,
    },
    /// The user declined the replacement.
    ReplaceRefused,
}

impl AppAnswerMsg {
    pub fn generate_data_msg(data: &collomatique_state_colloscopes::Data) -> AppAnswerMsg {
        AppAnswerMsg::Data(data.into())
    }
}

// So consumers write `ColloCmdMsg`, not `collomatique_rpc::CmdMsg<ColloProtocol>`.
pub type ColloInitMsg = collomatique_rpc::InitMsg<ColloProtocol>;
pub type ColloCmdMsg = collomatique_rpc::CmdMsg<ColloProtocol>;
pub type ColloResultMsg = collomatique_rpc::ResultMsg<ColloProtocol>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDataStream {
    serialized: String,
}

impl InternalDataStream {
    /// Identity token of this exact document, for comparing two of them across
    /// the channel. Never persisted, never sent to another build.
    pub fn token(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.serialized.hash(&mut hasher);
        hasher.finish()
    }
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
