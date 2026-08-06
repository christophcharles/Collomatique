//! Encode submodule
//!
//! This module contains the logic that builds a file document from an
//! [InnerData]:
//! [spec2::encode] produces the spec-2 format. The shared file header is
//! built by [generate_header].

pub(crate) mod spec2;

use super::*;
use json::*;

fn generate_header() -> Header {
    Header {
        file_type: FileType::ValidFileType(ValidFileType::Collomatique),
        produced_with_version: current_version(),
        file_content: FileContent::ValidFileContent(ValidFileContent::Colloscope),
    }
}
