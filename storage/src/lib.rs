//! Storage crate for collomatique
//!
//! This crate implements storage of the colloscopes data into a (JSON) file
//!
//! This crate provides two main utility functions: [deserialize_data] and [serialize_data].
//! Their goal is to allow translation of the in-memory data described in
//! [collomatique_state_colloscopes::Data] and a in-file representation.
//!
//! The actual representation is done in JSON. [deserialize_data] and [serialize_data] do
//! not actually handle reading and writing from a file. You can use [load_data_from_file]
//! and [save_data_to_file] for this.

mod decode;
mod encode;
mod json;

pub use decode::{Caveat, DecodeError};
pub use json::Version;

use collomatique_state_colloscopes::Data;
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use thiserror::Error;

/// Error type when deserializing a file
///
/// This error type describes error that happen when interpreting the file content.
#[derive(Debug, Error)]
pub enum DeserializationError {
    /// The JSON structure does not match the normal structure
    ///
    /// Except for programming errors, this means either the
    /// file is corrupted or it is ill-formed (which usually means
    /// it is not a colloscope file)
    #[error("Invalid JSON structure in colloscope file: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// Well-formed JSON structure but issues when decoding it
    #[error("Error whild decoding the colloscope file: {0}")]
    Decode(#[from] DecodeError),
}

/// Deserialize the content of a colloscope file
///
/// This function takes the content of a colloscope file
/// represented as a UTF8-string and deserialize it into a valid
/// in-memory [Data] representation.
///
/// This can fail for numerous reasons, described by [DeserializeError].
///
/// Even in case of success, the deserialization might only be partial. This
/// can happen for instance if we try to open a file from a newer version
/// of Collomatique. The type [Caveats] list possible issues in this situation.
pub fn deserialize_data(
    file_content: &str,
) -> Result<(Data, BTreeSet<Caveat>), DeserializationError> {
    let json_data = serde_json::from_str::<json::JsonData>(file_content)?;
    Ok(decode::decode(&json_data)?)
}

/// Serialize the content of a colloscope file
///
/// This function takes an in-memory [Data] representation
/// and serialize it into the content of a colloscope file
/// represented as a UTF-8 string.
///
/// This cannot fail as [Data] is always a valid representation.
pub fn serialize_data(data: &Data) -> String {
    let json_data = encode::encode(data);
    serde_json::to_string_pretty(&json_data).expect("Serializing to JSON should not fail")
}

/// Errors when loading data from a file
///
/// There are two main possibilities of errors:
/// - I/O errors: when there is a problem with access to the file or the
///   file cannot be read as a UTF-8 string
/// - deserialization errors: the obtained UTF-8 string cannot be parsed
///   properly.
#[derive(Error, Debug)]
pub enum LoadError {
    #[error("Error while reading/writing file: {0}")]
    IO(#[from] io::Error),

    #[error("Error while deserializing: {0}")]
    Deserialization(#[from] DeserializationError),
}

/// Load [Data] from an existing file
///
/// This is a convenience function encapsulating [deserialize_data].
///
/// Even in case of success, the deserialization might only be partial. This
/// can happen for instance if we try to open a file from a newer version
/// of Collomatique. The type [Caveats] list possible issues in this situation.
pub async fn load_data_from_file(file_path: &Path) -> Result<(Data, BTreeSet<Caveat>), LoadError> {
    use tokio::fs;
    let content = fs::read_to_string(file_path).await?;
    Ok(deserialize_data(&content)?)
}

/// Save [Data] to a file
///
/// No checks are done on the existence of the file. If the file
/// exists it will be overwritten. If it doesn't, it will be created.
///
/// The method can fail for various reasons like wrong permissions.
/// This will be reported as an [io::Error].
///
/// This is a convenience function encapsulating [deserialize_data].
pub async fn save_data_to_file(data: &Data, file_path: &Path) -> Result<(), io::Error> {
    use tokio::fs;
    let content = serialize_data(data);
    fs::write(file_path, content.as_bytes()).await
}
