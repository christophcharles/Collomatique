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

pub use decode::{Caveat, DecodeError, IdKind, RowKey};
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
    /// The file uses the retired pre-alpha format (spec 1)
    ///
    /// Spec 1 was a raw dump of the in-memory data, used before any
    /// release. It is permanently retired: such files can no longer be
    /// opened. This is the tombstone described in `docs/file_format.md` —
    /// a spec-1 file (any entry declaring `minimum_spec_version: 1`) is
    /// rejected with this clear error rather than a generic decode failure.
    #[error(
        "This file uses the retired pre-alpha format (spec 1), which is no longer supported and cannot be opened"
    )]
    RetiredSpec1Format,
    /// The entries declare a spec version that cannot exist
    ///
    /// Spec version 0 does not exist.
    #[error("Unsupported spec versions in entries ({versions:?}): spec version 0 does not exist")]
    UnsupportedSpecVersions { versions: BTreeSet<u32> },
}

/// Rejects entries the current reader cannot decode, based only on their
/// declared `minimum_spec_version`, before any payload interpretation.
///
/// Spec 1 (the pre-alpha dump format) is permanently retired: any file
/// carrying a spec-1 entry is rejected with [DeserializationError::RetiredSpec1Format]
/// (the tombstone). Spec version 0 never existed. Everything else —
/// spec 2 and later — is routed to the spec-2 pipeline, which applies
/// the forward-compatibility rules.
fn reject_retired_or_invalid_spec_versions(
    entries: &[json::RawEntry],
) -> Result<(), DeserializationError> {
    if entries.iter().any(|e| e.minimum_spec_version == 1) {
        return Err(DeserializationError::RetiredSpec1Format);
    }
    if entries.iter().any(|e| e.minimum_spec_version == 0) {
        return Err(DeserializationError::UnsupportedSpecVersions {
            versions: entries.iter().map(|e| e.minimum_spec_version).collect(),
        });
    }
    Ok(())
}

/// Options for [deserialize_data_with_options]
///
/// The default value is the plain behaviour of [deserialize_data]: read
/// the document exactly as the file writes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeserializeOptions {
    /// When true, all ids are regenerated right after parsing: every id
    /// of the document is renumbered densely from 0, in ascending order
    /// of the old values, before any id-space check runs
    ///
    /// This rescues a file whose ids sit above the format's ceiling of
    /// 2^63 - 1 (spec §3) — which older versions of the application could
    /// write, since nothing stopped them. The document that comes out
    /// differs from the file by its ids alone.
    ///
    /// It rescues nothing else. The renumbering is injective, so an id
    /// that two entities shared is still shared afterwards
    /// ([DecodeError::DuplicatedIdInBlock] or
    /// [DecodeError::DuplicatedIdAcrossBlocks]), and a reference to an id
    /// no entity defines still points nowhere
    /// ([DecodeError::DanglingReference]) — such a file is ambiguous or
    /// incomplete, not merely misnumbered. Note that those errors then
    /// name the renumbered id, not the one written in the file.
    ///
    /// It is off by default because renumbering breaks any id an outside
    /// party remembers.
    pub regenerate_ids: bool,
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
    deserialize_data_with_options(file_content, &DeserializeOptions::default())
}

/// Deserialize the content of a colloscope file, with options
///
/// This is [deserialize_data] with the extra behaviours of
/// [DeserializeOptions] available; see that type for what they do.
pub fn deserialize_data_with_options(
    file_content: &str,
    options: &DeserializeOptions,
) -> Result<(Data, BTreeSet<Caveat>), DeserializationError> {
    let raw_data = serde_json::from_str::<json::RawJsonData>(file_content)?;

    // The header check is path-independent: it must run before the
    // spec-version check so that e.g. an unknown file content is reported
    // the same way whatever the spec version of the entries.
    let mut caveats = BTreeSet::new();
    decode::check_header(&raw_data.header, &mut caveats)?;

    // Retired (spec 1) and impossible (spec 0) versions are rejected here,
    // before any payload interpretation. Everything else is spec 2 or later.
    reject_retired_or_invalid_spec_versions(&raw_data.entries)?;

    let data = decode::spec2::decode(
        &raw_data.entries,
        &raw_data.header.produced_with_version,
        &mut caveats,
        options,
    )?;
    Ok((data, caveats))
}

/// Error type when encoding data into a file
///
/// A valid [Data] is almost always writable — the one thing the file
/// format forbids and the in-memory model does not is an id above the
/// format's ceiling.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    /// The document holds an id above the file format's ceiling of
    /// 2^63 - 1 (spec §3) and cannot be written faithfully
    ///
    /// The in-memory id issuer has no upper bound, so this is reachable:
    /// by a very long editing history, or by a single operation on a
    /// document loaded from a file whose ids already sat at the ceiling.
    #[error("id {id} exceeds the file-format ceiling (2^63 - 1); the file cannot be written")]
    IdAboveCeiling { id: u64 },
}

/// Options for [serialize_data_with_options]
///
/// The default value is the plain behaviour of [serialize_data]: write
/// the document exactly as it is in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SerializeOptions {
    /// When true, all ids are regenerated before writing: every id of the
    /// document is renumbered densely from 0, in ascending order of the
    /// old values, so their relative order — and therefore the canonical
    /// form's sort orders — is preserved
    ///
    /// The in-memory [Data] is not modified; only the written file uses
    /// the new ids, so reloading the file yields a document that differs
    /// from the one in memory by its ids alone.
    ///
    /// This is the way out of [EncodeError::IdAboveCeiling]: a document
    /// that grew an id above the format's ceiling holds far fewer than
    /// 2^63 entities, so dense ids always fit. It is off by default
    /// because renumbering breaks any id an outside party remembers.
    pub regenerate_ids: bool,
}

/// Serialize the content of a colloscope file
///
/// This function takes an in-memory [Data] representation
/// and serialize it into the content of a colloscope file
/// represented as a UTF-8 string. The file is written in the
/// current (spec-2) format.
///
/// This fails only when the document cannot be represented in the file
/// format at all — see [EncodeError], which has a single cause: an id
/// above the format's ceiling.
pub fn serialize_data(data: &Data) -> Result<String, EncodeError> {
    serialize_data_with_options(data, &SerializeOptions::default())
}

/// Serialize the content of a colloscope file, with options
///
/// This is [serialize_data] with the extra behaviours of
/// [SerializeOptions] available; see that type for what they do.
pub fn serialize_data_with_options(
    data: &Data,
    options: &SerializeOptions,
) -> Result<String, EncodeError> {
    let document = encode::spec2::encode(data, options)?;
    Ok(serde_json::to_string_pretty(&document).expect("Serializing to JSON should not fail"))
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

/// Errors when saving data to a file
///
/// There are two main possibilities of errors:
/// - I/O errors: when there is a problem with access to the file
/// - encoding errors: the data cannot be represented in the file format
#[derive(Error, Debug)]
pub enum SaveError {
    #[error("Error while reading/writing file: {0}")]
    IO(#[from] io::Error),

    #[error("Error while encoding: {0}")]
    Encode(#[from] EncodeError),
}

/// Save [Data] to a file
///
/// No checks are done on the existence of the file. If the file
/// exists it will be overwritten. If it doesn't, it will be created.
///
/// The method can fail for various reasons like wrong permissions.
/// This will be reported as a [SaveError::IO]. It can also fail because
/// the data cannot be written in the file format at all
/// ([SaveError::Encode]).
///
/// This is a convenience function encapsulating [serialize_data].
pub async fn save_data_to_file(data: &Data, file_path: &Path) -> Result<(), SaveError> {
    use tokio::fs;
    let content = serialize_data(data)?;
    fs::write(file_path, content.as_bytes()).await?;
    Ok(())
}
