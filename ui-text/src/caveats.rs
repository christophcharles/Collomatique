//! The sentence a load caveat is shown as
//!
//! One function renders a [`collomatique_storage::Caveat`] the way the
//! application speaks about it — the sentences the gtk4 caveat dialog writes
//! (`gtk4/src/dialogs/file_caveats.rs`), kept here so the python module's
//! `str()` on a caveat says the same thing the dialog does. The dialog's `- `
//! bullet and its line breaks are its own layout; the sentence itself is flat
//! here, so a script printing one gets a single line.

/// The sentence a caveat is shown as
///
/// `L'entrée « … » n'a pas pu être décodée …` for a skipped block, and
/// `Fichier généré avec la version … de Collomatique…` for a newer writer.
pub fn caveat_text(caveat: &collomatique_storage::Caveat) -> String {
    match caveat {
        collomatique_storage::Caveat::UnknownEntry {
            block_name,
            minimum_spec_version,
        } => format!(
            "L'entrée « {block_name} » n'a pas pu être décodée \
             (elle demande la version {minimum_spec_version} du format)"
        ),
        collomatique_storage::Caveat::CreatedWithNewerVersion { version } => format!(
            "Fichier généré avec la version {} de Collomatique. Il est préférable \
             d'utiliser une version plus récente de Collomatique.",
            version
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::caveat_text;
    use collomatique_storage::Caveat;

    #[test]
    fn unknown_entry_names_the_block_and_the_spec_version() {
        let text = caveat_text(&Caveat::UnknownEntry {
            block_name: "colloscope".to_string(),
            minimum_spec_version: 3,
        });
        assert!(text.contains("« colloscope »"), "names the block: {text}");
        assert!(text.contains("version 3"), "names the spec version: {text}");
        assert!(!text.contains('\n'), "a single line: {text}");
    }

    #[test]
    fn created_with_newer_version_names_the_version() {
        let version = collomatique_storage::Version::new(9, 0, 0);
        let text = caveat_text(&Caveat::CreatedWithNewerVersion {
            version: version.clone(),
        });
        assert!(
            text.contains(&version.to_string()),
            "names the version: {text}"
        );
        assert!(!text.contains('\n'), "a single line: {text}");
    }
}
