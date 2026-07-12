//! Encode submodule
//!
//! This module contains the logic that builds a file document from a
//! [Data]: [self::encode] for the legacy (spec 1) format, and
//! [spec2::encode] for the spec-2 format.

pub(crate) mod spec2;

use super::*;
use json::*;

fn generate_header() -> Header {
    Header {
        file_type: FileType::Collomatique,
        produced_with_version: Version::current(),
        file_content: FileContent::ValidFileContent(ValidFileContent::Colloscope),
    }
}

fn generate_inner_data_dump(data: &Data) -> collomatique_state_colloscopes::InnerData {
    data.get_inner_data().clone()
}

pub fn encode(data: &Data) -> JsonData {
    let header = generate_header();

    let inner_data_dump_entry = ValidEntry::InnerDataDump(generate_inner_data_dump(data));

    JsonData {
        header,
        entries: vec![inner_data_dump_entry]
            .into_iter()
            .map(|x| Entry {
                minimum_spec_version: x.minimum_spec_version(),
                needed_entry: x.needed_entry(),
                content: EntryContent::ValidEntry(Box::new(x)),
            })
            .collect(),
    }
}
