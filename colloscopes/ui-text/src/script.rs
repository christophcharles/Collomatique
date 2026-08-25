//! The sentences the command line's script mode writes on stderr
//!
//! `--python`/`--python-file` with a document (`[FILE]` or `--new`) is a run
//! with a human at the terminal, so the two things it has to say, it says in
//! the application's own vocabulary. The caveat sentences themselves come from
//! [`crate::caveats::caveat_text`]; [`load_caveats_intro_text`] is only the
//! line that introduces them. [`lost_modifications_text`] is the warning for
//! dropped work: the script sent a document back, but no `--out` was given,
//! so nothing will be written.

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

#[cfg(test)]
mod tests;
