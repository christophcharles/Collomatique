//! The sentences the command line's script mode writes on stderr
//!
//! `--python`/`--python-file` with a document (`[FILE]` or `--new`) is a run
//! with a human at the terminal, so the things it has to say, it says in the
//! application's own vocabulary. The caveat sentences themselves come from
//! [`crate::caveats::caveat_text`]; [`load_caveats_intro_text`] is only the
//! line that introduces them.
//!
//! The other two are both about work the script sent that no file will keep,
//! and they are two because the reasons are two: with
//! [`lost_modifications_text`] there was nowhere to write it, no `--out` having
//! been given, and naming that option is the useful advice; with
//! [`interrupted_modifications_text`] there was somewhere, but the script
//! failed before the end, and a run that stopped halfway is not written out.

/// The line introducing the load caveats of the file a script was given
///
/// One line; the caveats follow it, one [`crate::caveats::caveat_text`]
/// sentence per line, and the `- ` bullet is the caller's own layout.
pub fn load_caveats_intro_text(path: &std::path::Path) -> String {
    format!(
        "Certains points du fichier « {} » nécessitent votre attention :",
        path.display()
    )
}

/// The warning when a script sent modifications and no `--out` will keep them
pub fn lost_modifications_text() -> &'static str {
    "Attention : le script a envoyé des modifications, mais aucun fichier de sortie \
     n'a été indiqué (option --out). Ces modifications sont perdues."
}

/// The warning when a script that had sent modifications failed before the end
///
/// The destination was given here; what is missing is a run that finished, and
/// a document left halfway is not written out.
pub fn interrupted_modifications_text() -> &'static str {
    "Attention : le script s'est interrompu ; les modifications qu'il avait envoyées \
     ne sont pas enregistrées."
}

#[cfg(test)]
mod tests;
