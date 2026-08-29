//! What the script-mode sentences say, pinned the way `caveats`'s are.

use super::*;

#[test]
fn the_caveats_intro_names_the_file() {
    let text = load_caveats_intro_text(std::path::Path::new("dossier/exemple.collomatique"));
    assert!(
        text.contains("« dossier/exemple.collomatique »"),
        "names the file: {text}"
    );
    assert!(!text.contains('\n'), "a single line: {text}");
}

#[test]
fn the_lost_modifications_warning_names_the_option() {
    let text = lost_modifications_text();
    assert!(text.contains("--out"), "names the option: {text}");
    assert!(!text.contains('\n'), "a single line: {text}");
}

/// The interrupted-run warning is for a run that had a destination, so naming
/// `--out` there would be advice for a mistake the user did not make.
#[test]
fn the_interrupted_warning_does_not_name_the_option() {
    let text = interrupted_modifications_text();
    assert!(
        !text.contains("--out"),
        "the option is not the problem: {text}"
    );
    assert!(!text.contains('\n'), "a single line: {text}");
}

#[test]
fn the_two_warnings_are_different_sentences() {
    assert_ne!(lost_modifications_text(), interrupted_modifications_text());
}
