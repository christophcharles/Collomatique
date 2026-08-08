//! The `collomatique` module, exercised from real python scripts
//!
//! The scripts live in `tests/scripts/` as `.py` files rather than as string
//! literals: they are the thing under test, and a real file is what a user
//! writes. The rust side passes inputs in and reads results out through the
//! script's globals, so the assertions stay here, where a failure says
//! something useful.

use std::path::{Path, PathBuf};
use std::sync::Once;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use collomatique_python::collomatique;

static INIT: Once = Once::new();

/// Registers the module and starts the interpreter, at most once per process
///
/// The interpreter is process-global while cargo runs the tests of a file on
/// several threads, so this goes behind a `Once`. `append_to_inittab!` has to
/// run before `Python::initialize()`, which is why the two sit together — the
/// same pairing `collomatique_python_runner::initialize()` makes, since a test
/// binary has no runner in it.
fn interpreter() {
    INIT.call_once(|| {
        pyo3::append_to_inittab!(collomatique);
        Python::initialize();
    });
}

/// Runs `script` and hands back the globals it left behind
///
/// `fill` populates the namespace the script runs in, which is how a test
/// hands it paths and other inputs.
fn run(script: &str, fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>) -> Py<PyDict> {
    interpreter();
    Python::attach(|py| {
        let globals = PyDict::new(py);
        fill(&globals).expect("the test inputs should convert to python");

        let script = std::ffi::CString::new(script).expect("the script has no interior nul");
        py.run(&script, Some(&globals), None).unwrap_or_else(|e| {
            e.print(py);
            panic!("the script should run")
        });

        globals.unbind()
    })
}

/// `collomatique.__version__` is the package version
///
/// The two sides of the comparison do not come from the same crate:
/// `__version__` is built from `collomatique_settings::current_version()`,
/// i.e. the *settings* crate's `CARGO_PKG_VERSION`, while the `env!` here is
/// this crate's. So this pins two things at once — that the module exposes the
/// version, and that the workspace really does speak with one version number.
#[test]
fn the_module_reports_the_package_version() {
    let globals = run(include_str!("scripts/version.py"), |_| Ok(()));

    let version: String = Python::attach(|py| {
        globals
            .bind(py)
            .get_item("version")
            .expect("looking up a global should not fail")
            .expect("the script sets `version`")
            .extract()
            .expect("`__version__` is a string")
    });

    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

/// A private directory for one test to write in, emptied first
///
/// The document scripts write files, and `doc.save()` writes back to the file
/// the document came from — so they must work on a copy, never on anything in
/// the repository. A per-process, per-test name keeps two runs of the suite out
/// of each other's way without a `tempfile` dependency.
fn workspace(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("collomatique-python-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory should be creatable");
    dir
}

/// Reads a colloscope file the way the application does
///
/// Used on the files the script wrote: it says the output is a colloscope
/// document and not merely some bytes that happen to be there.
fn reload(path: &Path) -> collomatique_state_colloscopes::Data {
    let content = std::fs::read_to_string(path).expect("the script wrote this file");
    let (inner_data, _caveats) =
        collomatique_storage::deserialize_data(&content).expect("the written file should decode");
    collomatique_state_colloscopes::Data::from_inner_data(inner_data)
        .expect("the written document should satisfy the in-memory invariants")
}

/// A document survives a load and both shapes of save
///
/// The script does the whole trip — open a real colloscope, write it back to
/// its origin, write it somewhere else, and check what `source_path` says
/// throughout. Rust checks what landed on disk: the two files it wrote are the
/// same bytes, and they read back as a document.
#[test]
fn a_document_loads_saves_and_remembers_where_it_came_from() {
    let dir = workspace("document");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("target.collomatique");

    run(include_str!("scripts/document.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        Ok(())
    });

    // `save()` and `save(target)` write the same document, so they write the
    // same bytes: the destination is the only thing that differs between them.
    let written = std::fs::read(&source).expect("save() rewrote its origin");
    let saved_as = std::fs::read(&target).expect("save(target) wrote the target");
    assert_eq!(written, saved_as);

    let reloaded = reload(&target);
    assert_eq!(reloaded.get_inner_data(), reload(&source).get_inner_data());

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The name of the block the caveated fixture cannot read
const UNKNOWN_BLOCK: &str = "YouShouldReallyNeverCallAnEntryThisWay";

/// A copy of the example colloscope, in `dir`, safe to write over
///
/// `doc.save()` writes back to the file the document came from, so a script
/// must never be handed the file in `examples/`.
fn example_copy(dir: &Path, name: &str) -> PathBuf {
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/hogwarts.collomatique")
        .canonicalize()
        .expect("the example colloscope should be in the repository");

    let copy = dir.join(name);
    std::fs::copy(&example, &copy).expect("the example should be copyable");
    copy
}

/// `compacted()` hands back a renumbered copy, and leaves its document alone
///
/// The comparison is against the compaction rust does itself: it says the
/// python side really ran `compact_ids` on the document it was called on, and
/// wrote *that* out. Whether the example's ids were dense already does not
/// matter — the two sides agree either way, and the file the original saved is
/// still the original.
#[test]
fn compacting_a_document_writes_a_renumbered_copy() {
    let dir = workspace("compacted");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("compacted.collomatique");
    let original = dir.join("original.collomatique");

    let caveated_source = dir.join("caveated.collomatique");
    let (content, _newer, _spec) = caveated_file();
    std::fs::write(&caveated_source, &content).expect("the fixture should be writable");

    run(include_str!("scripts/compacted.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("original", &original)?;
        globals.set_item("caveated_source", &caveated_source)?;
        Ok(())
    });

    let before = reload(&source);
    let compacted = reload(&target);
    assert_eq!(
        compacted.get_inner_data(),
        &before.get_inner_data().clone().compact_ids()
    );

    // Compaction copies, so what the document itself saved is unrenumbered.
    assert_eq!(reload(&original).get_inner_data(), before.get_inner_data());

    // The caveated file refused the bare `save()` of the compacted copy too, so
    // the block this build cannot read is still in it.
    let untouched = std::fs::read_to_string(&caveated_source).expect("the fixture is still there");
    assert_eq!(untouched, content);
    assert!(untouched.contains(UNKNOWN_BLOCK));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document a caveat-free build of this version cannot read whole
///
/// Written by hand rather than copied from `examples/`, because that is what
/// makes it caveated: the header claims a version above this one, and the one
/// entry claims a spec level this build does not know while declaring itself
/// unneeded — the two shapes `storage/tests/header_check.rs` and
/// `storage/tests/general_entries_check.rs` build for the same reason. What is
/// *in* the document does not matter here; that it decodes with both caveats
/// does.
fn caveated_file() -> (String, collomatique_settings::Version, u32) {
    let current = collomatique_settings::current_version();
    let newer =
        collomatique_settings::Version::new(current.major, current.minor + 1, current.patch);
    let spec = collomatique_storage::CURRENT_SPEC_VERSION + 1;

    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": "{newer}",
        "file_content": "Colloscope"
    }},
    "entries": [
        {{
            "minimum_spec_version": {spec},
            "needed_entry": false,
            "content": {{
                "{UNKNOWN_BLOCK}": {{
                    "some_data_this_version_cannot_fathom": [42, 43]
                }}
            }}
        }}
    ]
}}"#
    );

    (content, newer, spec)
}

/// A caveated file hands the script what it could not read, and is not
/// overwritten behind its back
///
/// The script does most of the checking, because caveats are a python-facing
/// vocabulary — what matters is that a script can name the caveat it expects
/// and compare. Rust builds the file, says which values went into it, and then
/// looks at what the writes the script *did* make left on disk.
#[test]
fn a_caveated_file_says_what_it_could_not_read() {
    let dir = workspace("caveats");
    let source = dir.join("caveated.collomatique");
    let target = dir.join("copy.collomatique");

    let (content, newer, spec) = caveated_file();
    std::fs::write(&source, &content).expect("the fixture should be writable");

    run(include_str!("scripts/caveats.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("newer_version", newer.to_string())?;
        globals.set_item("block_name", UNKNOWN_BLOCK)?;
        globals.set_item("spec_version", spec)?;
        Ok(())
    });

    // The script did eventually overwrite the origin on purpose, and the block
    // this build could not read is gone from the file — which is the loss the
    // refusal of the bare `save()` was about.
    let rewritten = std::fs::read_to_string(&source).expect("the script rewrote its origin");
    assert!(!rewritten.contains(UNKNOWN_BLOCK));
    let (_inner, caveats) = collomatique_storage::deserialize_data(&rewritten)
        .expect("the rewritten file should decode");
    assert!(caveats.is_empty());

    // The copy written elsewhere is the same document, so the same bytes.
    let copy = std::fs::read_to_string(&target).expect("save(target) wrote the copy");
    assert_eq!(copy, rewritten);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}
