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
mod format;
mod json;

pub use decode::{Caveat, DecodeError};
pub use json::{CURRENT_SPEC_VERSION, Version};

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
    /// The entries declare an unsupported combination of minimum spec versions
    ///
    /// Entries from the retired pre-alpha format (spec 1) cannot be mixed with
    /// spec 2 (or later) entries, and spec version 0 does not exist.
    #[error(
        "Unsupported combination of minimum spec versions in entries ({versions:?}): entries from the pre-alpha format (spec 1) cannot be mixed with spec 2 or later entries, and spec version 0 does not exist"
    )]
    UnsupportedSpecVersions { versions: BTreeSet<u32> },
}

/// The two decoding pipelines a file can be routed to
///
/// The legacy pipeline (spec 1, a raw dump of the in-memory data) is
/// kept alive only during the transition to spec 2 and will be retired
/// once existing files have been bulk-converted.
enum SpecFamily {
    Legacy,
    Spec2,
}

fn detect_spec_family(entries: &[json::RawEntry]) -> Result<SpecFamily, DeserializationError> {
    if entries.is_empty() {
        // An empty entry list is a valid blank spec-2 document
        return Ok(SpecFamily::Spec2);
    }
    if entries.iter().all(|e| e.minimum_spec_version == 1) {
        return Ok(SpecFamily::Legacy);
    }
    if entries.iter().all(|e| e.minimum_spec_version >= 2) {
        return Ok(SpecFamily::Spec2);
    }
    Err(DeserializationError::UnsupportedSpecVersions {
        versions: entries.iter().map(|e| e.minimum_spec_version).collect(),
    })
}

/// Deserialize the content of a colloscope file
///
/// This function takes the content of a colloscope file
/// represented as a UTF8-string and deserialize it into a valid
/// in-memory [Data] representation.
///
/// This can fail for numerous reasons, described by [DeserializationError].
///
/// Even in case of success, the deserialization might only be partial. This
/// can happen for instance if we try to open a file from a newer version
/// of Collomatique. The type [Caveat] list possible issues in this situation.
pub fn deserialize_data(
    file_content: &str,
) -> Result<(Data, BTreeSet<Caveat>), DeserializationError> {
    let raw_data = serde_json::from_str::<json::RawJsonData>(file_content)?;

    // The header check is path-independent: it must run before dispatch so
    // that e.g. an unknown file content is reported the same way whatever
    // the spec version of the entries.
    let mut caveats = BTreeSet::new();
    decode::check_header(&raw_data.header, &mut caveats)?;

    match detect_spec_family(&raw_data.entries)? {
        SpecFamily::Legacy => {
            // Deliberate re-parse of the original string: this keeps the
            // legacy pipeline (due for retirement) byte-for-byte identical,
            // including its quirks. The double parse is negligible.
            let json_data = serde_json::from_str::<json::JsonData>(file_content)?;
            let (data, mut legacy_caveats) = decode::decode(json_data)?;
            legacy_caveats.append(&mut caveats);
            Ok((data, legacy_caveats))
        }
        SpecFamily::Spec2 => todo!("spec-2 read path (commit 3)"),
    }
}

/// Serialize the content of a colloscope file
///
/// This function takes an in-memory [Data] representation
/// and serialize it into the content of a colloscope file
/// represented as a UTF-8 string.
///
/// If `legacy` is `true`, the file is written in the pre-alpha
/// format (spec 1, a raw dump of the in-memory data); otherwise in
/// the spec-2 format. The parameter only exists for the transition
/// period and will be retired (along with the legacy writer) once
/// existing files have been bulk-converted to spec 2.
///
/// This cannot fail as [Data] is always a valid representation.
pub fn serialize_data(data: &Data, legacy: bool) -> String {
    if legacy {
        let json_data = encode::encode(data);
        serde_json::to_string_pretty(&json_data).expect("Serializing to JSON should not fail")
    } else {
        todo!("spec-2 write path (commit 3)")
    }
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
/// of Collomatique. The type [Caveat] list possible issues in this situation.
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
/// The `legacy` flag has the same meaning as in [serialize_data]
/// and will be retired with it.
///
/// This is a convenience function encapsulating [serialize_data].
pub async fn save_data_to_file(
    data: &Data,
    file_path: &Path,
    legacy: bool,
) -> Result<(), io::Error> {
    use tokio::fs;
    let content = serialize_data(data, legacy);
    fs::write(file_path, content.as_bytes()).await
}
