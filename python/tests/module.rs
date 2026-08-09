//! The `collomatique` module, exercised from real python scripts
//!
//! The scripts live in `tests/scripts/` as `.py` files rather than as string
//! literals: they are the thing under test, and a real file is what a user
//! writes. The rust side passes inputs in and reads results out through the
//! script's globals, so the assertions stay here, where a failure says
//! something useful.

use std::collections::{BTreeSet, VecDeque};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

use pyo3::prelude::*;
use pyo3::types::PyDict;

use collomatique_python::{FileRequest, collomatique};
use collomatique_state_colloscopes::Data;
use collomatique_ui_text::rendering::{
    render_pairing_rule, render_slot_in_subject, render_slot_pairing_rule, render_subject,
};

static INIT: Once = Once::new();

/// One script at a time, whatever cargo does with its threads
///
/// The host a script sees is module-global, as it has to be — a script cannot
/// be handed one — so two scripts must not overlap: python releases the GIL
/// every few milliseconds, and a second `Python::attach` would otherwise run
/// its script in the middle of the first one's, under the first one's host.
static ONE_SCRIPT_AT_A_TIME: Mutex<()> = Mutex::new(());

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

/// Runs `script` on its own, the way a script started from a shell runs
fn run(script: &str, fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>) -> Py<PyDict> {
    run_in(script, None, None, fill)
}

/// Runs `script` inside `host`, as the gui's script editor does
fn run_hosted(
    script: &str,
    host: Option<Arc<dyn collomatique_python::Host>>,
    fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
) -> Py<PyDict> {
    run_in(script, host, None, fill)
}

/// Runs `script` inside `host`, with `dialogs` answering its file choosers
fn run_hosted_with_dialogs(
    script: &str,
    host: Option<Arc<dyn collomatique_python::Host>>,
    dialogs: Arc<dyn collomatique_python::Dialogs>,
    fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
) -> Py<PyDict> {
    run_in(script, host, Some(dialogs), fill)
}

/// Runs `script` with `dialogs` answering its file choosers
fn run_with_dialogs(
    script: &str,
    dialogs: Arc<dyn collomatique_python::Dialogs>,
    fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
) -> Py<PyDict> {
    run_in(script, None, Some(dialogs), fill)
}

/// Runs `script` in whatever surroundings it needs, and hands back the globals
/// it left
///
/// `fill` populates the namespace the script runs in, which is how a test hands
/// it paths and other inputs. The host and the dialogs are installed for the run
/// and cleared afterwards, so the scripts that check there is no host still see
/// none, and the one script that wants a real `rfd` still gets one.
fn run_in(
    script: &str,
    host: Option<Arc<dyn collomatique_python::Host>>,
    dialogs: Option<Arc<dyn collomatique_python::Dialogs>>,
    fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
) -> Py<PyDict> {
    // A test that panicked left this poisoned; that test has already failed, and
    // taking the lock anyway keeps the failure to itself.
    let _one_at_a_time = ONE_SCRIPT_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    interpreter();
    collomatique_python::set_host(host);
    collomatique_python::set_dialogs(dialogs);

    let globals = Python::attach(|py| {
        let globals = PyDict::new(py);
        fill(&globals).expect("the test inputs should convert to python");

        let script = std::ffi::CString::new(script).expect("the script has no interior nul");
        py.run(&script, Some(&globals), None).unwrap_or_else(|e| {
            e.print(py);
            panic!("the script should run")
        });

        globals.unbind()
    });

    collomatique_python::set_host(None);
    collomatique_python::set_dialogs(None);

    globals
}

/// Runs several scripts in one namespace, with rust doing something between them
///
/// The read surface ships no removes, so a script cannot make an entity go away
/// on its own — and staleness is exactly what happens when one does. So the
/// mutation happens here, between two stages: the first stage leaves in the
/// globals the handles it wants to outlive it, `between` applies a real
/// `UpdateOp` to the document it opened, and the next stage says what has become
/// of them.
///
/// `between` runs before every stage but the first, so a two-element `scripts`
/// makes it run once.
fn run_stages(
    scripts: &[&str],
    fill: impl FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
    mut between: impl FnMut(Python<'_>, &Bound<'_, PyDict>),
) -> Py<PyDict> {
    // A test that panicked left this poisoned; that test has already failed, and
    // taking the lock anyway keeps the failure to itself.
    let _one_at_a_time = ONE_SCRIPT_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    interpreter();
    collomatique_python::set_host(None);
    collomatique_python::set_dialogs(None);

    Python::attach(|py| {
        let globals = PyDict::new(py);
        fill(&globals).expect("the test inputs should convert to python");

        for (stage, script) in scripts.iter().enumerate() {
            if stage > 0 {
                between(py, &globals);
            }

            let script = std::ffi::CString::new(*script).expect("the script has no interior nul");
            py.run(&script, Some(&globals), None).unwrap_or_else(|e| {
                e.print(py);
                panic!("stage {stage} should run")
            });
        }

        globals.unbind()
    })
}

/// The document a stage left in its globals
fn document_of(globals: &Bound<'_, PyDict>) -> Py<collomatique_python::Document> {
    globals
        .get_item("doc")
        .expect("looking up a global should not fail")
        .expect("the stage before this one leaves the document it opened")
        .extract()
        .expect("`doc` is a collomatique document")
}

/// One global, extracted into the rust shape the test compares against
fn global<T>(globals: &Py<PyDict>, name: &str) -> T
where
    T: for<'a, 'py> FromPyObject<'a, 'py>,
{
    Python::attach(|py| {
        globals
            .bind(py)
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"))
            .extract()
            .unwrap_or_else(|_| panic!("`{name}` should convert to what the test compares it with"))
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

/// The date the colles start goes in from python and comes out on disk
///
/// The one write this api has so far, so it is also what says the write door
/// works: the op is applied, the document keeps it, and a save carries it into
/// the file. The dates travel in as `chrono` values, which is how the script
/// receives real `datetime.date`s rather than strings it would have to parse.
#[test]
fn the_first_week_is_written_read_back_and_cleared() {
    let dir = workspace("first-week");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("dated.collomatique");
    let cleared_target = dir.join("cleared.collomatique");

    let monday = chrono::NaiveDate::from_ymd_opt(2026, 9, 7).expect("7 September 2026 is a date");
    let tuesday = monday.succ_opt().expect("the day after exists");

    run(include_str!("scripts/first_week.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("cleared_target", &cleared_target)?;
        globals.set_item("monday", monday)?;
        globals.set_item("tuesday", tuesday)?;
        Ok(())
    });

    let dated = reload(&target);
    assert_eq!(
        dated.get_inner_data().params.periods.first_week,
        Some(collomatique_time::WeekStart::new(monday).expect("7 September 2026 is a monday"))
    );

    // The refused tuesday left nothing behind, and the clear really cleared.
    let cleared = reload(&cleared_target);
    assert_eq!(cleared.get_inner_data().params.periods.first_week, None);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// Writes are taken back one at a time, and put back again
///
/// The undone document is compared with the file it was loaded from: three
/// undos do not merely land on a document with the right start date, they land
/// on the document that was opened. The french labels are handed in from `ops`
/// rather than spelled out in the script, so this says python shows the
/// operation's own name.
#[test]
fn writes_are_undone_and_redone_one_at_a_time() {
    let dir = workspace("undo");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("undone.collomatique");

    let mondays: Vec<_> = [(2026, 9, 7), (2026, 9, 14), (2026, 9, 21)]
        .into_iter()
        .map(|(y, m, d)| {
            chrono::NaiveDate::from_ymd_opt(y, m, d).expect("these are dates, and mondays")
        })
        .collect();

    let label = |op: collomatique_ops::GeneralPlanningUpdateOp| op.get_desc().1;
    let update_label = label(collomatique_ops::GeneralPlanningUpdateOp::UpdateFirstWeek(
        collomatique_time::WeekStart::new(mondays[0]).expect("7 September 2026 is a monday"),
    ));
    let clear_label = label(collomatique_ops::GeneralPlanningUpdateOp::DeleteFirstWeek);

    run(include_str!("scripts/undo.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("first", mondays[0])?;
        globals.set_item("second", mondays[1])?;
        globals.set_item("third", mondays[2])?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("clear_label", &clear_label)?;
        Ok(())
    });

    assert_eq!(
        reload(&target).get_inner_data(),
        reload(&source).get_inner_data()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A block of writes is one step, and blocks really nest
///
/// The headline is the nesting: an inner block that rolls back takes only its
/// own writes, and the outer block that catches the exception keeps everything
/// it did before. The script checks that, and the seven other behaviours a
/// transaction has — the single slot, the rollback on an exception, `cancel()`,
/// the four refusals, the transaction that is never entered, `undo()` stopping
/// at the block's start, and the empty block's named step.
///
/// The labels are handed in from rust: the block names so that what the history
/// shows is the name it was opened with, and `update_label` from `ops` so that
/// a write made *outside* a block is seen to keep the operation's own name.
///
/// The final comparison is with the file the script opened, the way `undo.py`'s
/// is: the rollbacks land on the document that was loaded, not merely on one
/// carrying the right start date.
#[test]
fn a_transaction_makes_a_block_of_writes_one_step() {
    let dir = workspace("transaction");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("rolled-back.collomatique");

    let mondays: Vec<_> = [(2026, 9, 7), (2026, 9, 14), (2026, 9, 21), (2026, 9, 28)]
        .into_iter()
        .map(|(y, m, d)| {
            chrono::NaiveDate::from_ymd_opt(y, m, d).expect("these are dates, and mondays")
        })
        .collect();

    let update_label = collomatique_ops::GeneralPlanningUpdateOp::UpdateFirstWeek(
        collomatique_time::WeekStart::new(mondays[0]).expect("7 September 2026 is a monday"),
    )
    .get_desc()
    .1;

    run(include_str!("scripts/transaction.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("first", mondays[0])?;
        globals.set_item("second", mondays[1])?;
        globals.set_item("third", mondays[2])?;
        globals.set_item("fourth", mondays[3])?;
        globals.set_item("batch_label", "Import Pronote")?;
        globals.set_item("outer_label", "Outer block")?;
        globals.set_item("empty_label", "Nothing at all")?;
        globals.set_item("update_label", &update_label)?;
        Ok(())
    });

    assert_eq!(
        reload(&target).get_inner_data(),
        reload(&source).get_inner_data()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The application, for a script that is not really running inside one
///
/// It does what the rpc engine does — hands over the document it holds, and
/// takes whole documents back — and keeps every one of them, so a test can say
/// what crossed and in which order.
struct FakeHost {
    data: Data,
    sent: Mutex<Vec<Data>>,
}

impl collomatique_python::Host for FakeHost {
    fn data(&self) -> Data {
        self.data.clone()
    }

    fn send(&self, data: &Data) -> Result<(), String> {
        self.sent.lock().unwrap().push(data.clone());
        Ok(())
    }
}

/// A hosted script is handed a document, and sends documents back
///
/// The comparisons are against whole documents rather than against the date
/// alone: what crosses is the document, so a send that carried the right date
/// on the wrong colloscope would be caught. The count is part of the test —
/// three sends and no fourth — because a send happening on its own is exactly
/// what `docs/python/new_api_design.md` §9.2 refuses, and the script's undo at
/// the end would be the one to produce it.
#[test]
fn a_hosted_script_is_handed_a_document_and_sends_one_back() {
    let dir = workspace("hosted");
    let source = example_copy(&dir, "hosted.collomatique");
    let other_source = example_copy(&dir, "other.collomatique");

    let monday = chrono::NaiveDate::from_ymd_opt(2026, 9, 7).expect("7 September 2026 is a monday");
    let other_monday =
        chrono::NaiveDate::from_ymd_opt(2026, 9, 14).expect("14 September 2026 is a monday");

    let host = Arc::new(FakeHost {
        data: reload(&source),
        sent: Mutex::new(Vec::new()),
    });

    run_hosted(
        include_str!("scripts/hosted.py"),
        Some(host.clone()),
        |globals| {
            globals.set_item("other_source", &other_source)?;
            globals.set_item("monday", monday)?;
            globals.set_item("other_monday", other_monday)?;
            Ok(())
        },
    );

    // What the application would have ended up with, had it applied each send.
    let hosted_document_dated = |date| {
        let mut expected = host.data.get_inner_data().clone();
        expected.params.periods.first_week =
            Some(collomatique_time::WeekStart::new(date).expect("these dates are mondays"));
        expected
    };

    let sent = host.sent.lock().expect("no sender panicked");
    assert_eq!(sent.len(), 3);

    // `doc.save()` on the hosted document.
    assert_eq!(sent[0].get_inner_data(), &hosted_document_dated(monday));
    // A document the script loaded itself, which the application never gave it.
    assert_eq!(
        sent[1].get_inner_data(),
        &hosted_document_dated(other_monday)
    );
    // Sending twice is allowed, and this is the one that wins.
    assert_eq!(sent[2].get_inner_data(), &hosted_document_dated(monday));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
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

/// Which of the three dialogs a script asked for
#[derive(Debug, PartialEq, Eq)]
enum Dialog {
    Open,
    Save,
    Folder,
}

/// The desktop, for a test that cannot click a file chooser
///
/// It answers from a list written in the test, one entry per call and in order,
/// and keeps every request — so the test says both what the script was given
/// and what reached the backend on the way.
struct FakeDialogs {
    answers: Mutex<VecDeque<Result<Option<PathBuf>, String>>>,
    asked: Mutex<Vec<(Dialog, FileRequest)>>,
}

impl FakeDialogs {
    fn answering(
        answers: impl IntoIterator<Item = Result<Option<PathBuf>, String>>,
    ) -> FakeDialogs {
        FakeDialogs {
            answers: Mutex::new(answers.into_iter().collect()),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn answer(&self, dialog: Dialog, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        self.asked.lock().unwrap().push((dialog, request.clone()));
        self.answers
            .lock()
            .unwrap()
            .pop_front()
            .expect("the script should ask for exactly the dialogs the test answers")
    }
}

impl collomatique_python::Dialogs for FakeDialogs {
    fn open_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        self.answer(Dialog::Open, request)
    }

    fn save_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        self.answer(Dialog::Save, request)
    }

    fn pick_folder(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        self.answer(Dialog::Folder, request)
    }
}

/// The filters of a request, in a shape a test can write down
fn filters_of(request: &FileRequest) -> Vec<(&str, Vec<&str>)> {
    request
        .filters
        .iter()
        .map(|(description, extensions)| {
            (
                description.as_str(),
                extensions.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

/// A script asks for files and folders, and is told what was chosen
///
/// The script checks what it was handed — a `pathlib.Path`, a `None` for a
/// cancel, a `DialogUnavailable` for a machine that cannot show one — and rust
/// checks what reached the backend, which is the half no script can see: that
/// the title, the directory and the file name travel, and that the three ways
/// of writing an extension all arrive as the bare one.
///
/// The paths are made up rather than created: nothing here touches the disk, and
/// the fake desktop hands back whatever it is told to.
#[test]
fn a_script_asks_for_a_file_a_folder_and_a_place_to_write() {
    let start_dir = std::env::temp_dir();
    let chosen_file = start_dir.join("élèves.csv");
    let saved_file = start_dir.join("sortie.csv");
    let chosen_folder = start_dir.join("exports");
    let refusal = "this machine has no desktop to draw on";

    // One per call the script makes, in order. The call that raises `ValueError`
    // is not among them: it is refused before the backend hears about it, which
    // is what the count below says.
    let dialogs = Arc::new(FakeDialogs::answering([
        Ok(Some(chosen_file.clone())),
        Ok(Some(saved_file.clone())),
        Ok(Some(chosen_folder.clone())),
        Ok(None),
        Err(refusal.to_owned()),
        Ok(None),
    ]));

    run_with_dialogs(
        include_str!("scripts/dialogs.py"),
        dialogs.clone(),
        |globals| {
            globals.set_item("start_dir", &start_dir)?;
            globals.set_item("chosen_file", &chosen_file)?;
            globals.set_item("saved_file", &saved_file)?;
            globals.set_item("chosen_folder", &chosen_folder)?;
            globals.set_item("refusal", refusal)?;
            Ok(())
        },
    );

    let asked = dialogs.asked.lock().expect("no dialog panicked");
    let kinds: Vec<_> = asked.iter().map(|(dialog, _)| dialog).collect();
    assert_eq!(
        kinds,
        [
            &Dialog::Open,
            &Dialog::Save,
            &Dialog::Folder,
            &Dialog::Open,
            &Dialog::Open,
            &Dialog::Folder
        ]
    );

    // The whole of what an open dialog takes, `*.collomatique` included.
    let (_, open) = &asked[0];
    assert_eq!(open.title.as_deref(), Some("Ouvrir la liste des élèves"));
    assert_eq!(
        filters_of(open),
        [
            ("Fichiers collomatique", vec!["collomatique"]),
            ("Tous les fichiers", vec!["*"]),
        ]
    );
    assert_eq!(open.directory.as_deref(), Some(start_dir.as_path()));
    assert_eq!(open.file_name, None);

    // The name a save starts from, and the three spellings of one extension.
    let (_, save) = &asked[1];
    assert_eq!(save.file_name.as_deref(), Some("sortie.csv"));
    assert_eq!(filters_of(save), [("Tableur", vec!["csv", "csv", "xlsx"])]);
    assert_eq!(save.directory.as_deref(), Some(start_dir.as_path()));

    // A folder has nothing to filter by, so the request carries no filter even
    // though the same struct has room for one.
    let (_, folder) = &asked[2];
    assert_eq!(folder.title.as_deref(), Some("Choisir un dossier"));
    assert_eq!(folder.directory.as_deref(), Some(start_dir.as_path()));
    assert!(folder.filters.is_empty());

    // The bare call asks for nothing in particular.
    let (_, bare) = &asked[5];
    assert_eq!(bare.title, None);
    assert_eq!(bare.directory, None);
    assert_eq!(bare.file_name, None);
    assert!(bare.filters.is_empty());
}

/// The default document is the hosted one, then a path, then a chooser
///
/// Two runs, because the first link of the chain needs an application and the
/// rest need none, and the host is installed around a whole run rather than
/// around a call. The hosted run is where the order can actually go wrong: the
/// script is handed a real colloscope path as well, the stale argument the
/// order exists to ignore, and a desktop with no answer for anything — so a
/// chooser opened there fails the test instead of passing it quietly.
#[test]
fn the_default_document_is_the_hosted_one_then_a_path_then_a_dialog() {
    let dir = workspace("default-document");
    let source = example_copy(&dir, "source.collomatique");
    let other_source = example_copy(&dir, "other.collomatique");
    let chosen = example_copy(&dir, "chosen.collomatique");
    let missing = dir.join("nothing-here.collomatique");
    let refusal = "this machine has no desktop to draw on";

    let unused = Arc::new(FakeDialogs::answering([]));
    let host = Arc::new(FakeHost {
        data: reload(&source),
        sent: Mutex::new(Vec::new()),
    });

    run_hosted_with_dialogs(
        include_str!("scripts/default_document_hosted.py"),
        Some(host.clone()),
        unused.clone(),
        |globals| {
            globals.set_item("other_source", &other_source)?;
            Ok(())
        },
    );

    assert!(unused.asked.lock().expect("no dialog panicked").is_empty());
    // Nothing crossed back, either: this opens a document, it does not send one.
    assert!(host.sent.lock().expect("no sender panicked").is_empty());

    // One per chooser the standalone script gets as far as opening.
    let dialogs = Arc::new(FakeDialogs::answering([
        Ok(Some(chosen.clone())),
        Ok(None),
        Err(refusal.to_owned()),
    ]));

    run_with_dialogs(
        include_str!("scripts/default_document.py"),
        dialogs.clone(),
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("chosen", &chosen)?;
            globals.set_item("missing", &missing)?;
            globals.set_item("refusal", refusal)?;
            Ok(())
        },
    );

    // Three, and no fourth: neither the path nor `dialog=False` asked for one.
    let asked = dialogs.asked.lock().expect("no dialog panicked");
    let kinds: Vec<_> = asked.iter().map(|(dialog, _)| dialog).collect();
    assert_eq!(kinds, [&Dialog::Open, &Dialog::Open, &Dialog::Open]);

    // What the module asked the desktop for, which is the half no script can
    // see. The words are the application's own (`gtk4/src/tools/open_save.rs`),
    // because a user who meets both should meet the same ones.
    let (_, request) = &asked[0];
    assert_eq!(request.title.as_deref(), Some("Ouvrir"));
    assert_eq!(
        filters_of(request),
        [
            (
                "Fichiers collomatique (*.collomatique)",
                vec!["collomatique"]
            ),
            ("Tous les fichiers", vec!["*"]),
        ]
    );
    assert_eq!(request.directory, None);
    assert_eq!(request.file_name, None);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The calendar reads back period by period and week by week
///
/// The script walks `doc.periods` and `doc.weeks` and leaves what it saw; rust
/// compares it with the same document read straight from the model. What is
/// pinned is both the values — the annotations, the interrogation flags, the
/// dates — and the two orders they come in: periods in display order, weeks in
/// the model's own global walk.
///
/// The script does the rest on its own, because it is about what python sees:
/// the collection protocol, indexing by id and by handle alike, the handle that
/// has no constructor and no setters, and the handle of another document that
/// names nothing here.
#[test]
fn the_calendar_reads_back_period_by_period_and_week_by_week() {
    let dir = workspace("calendar");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/calendar.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let period_ids: Vec<_> = params.periods.period_ids().collect();
    let walk: Vec<_> = params.walk_weeks().collect();

    // The example is only worth reading if it has something to say: several
    // periods, several weeks each, and annotations that are sometimes absent.
    assert!(period_ids.len() > 1);
    assert!(walk.len() > period_ids.len());

    assert_eq!(
        global::<Vec<usize>>(&globals, "period_indices"),
        (0..period_ids.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "weeks_per_period"),
        period_ids
            .iter()
            .map(|period| params.weeks.week_count_for_period(*period).unwrap_or(0))
            .collect::<Vec<_>>()
    );

    assert_eq!(
        global::<Vec<usize>>(&globals, "week_indices"),
        (0..walk.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "week_interrogations"),
        walk.iter()
            .map(|(_period, _id, week)| week.interrogations)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "week_annotations"),
        walk.iter()
            .map(|(_period, _id, week)| week.annotation.as_ref().map(|text| text.to_string()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "week_period_indices"),
        walk.iter()
            .map(|(period, _id, _week)| period_ids
                .iter()
                .position(|id| id == period)
                .expect("a walked week names a live period"))
            .collect::<Vec<_>>()
    );

    // The dates are the export's own: consecutive weeks from the start date, in
    // global order (`xlsx/src/lib.rs`, `generate_week_dates_title`).
    let first_week = *params
        .periods
        .first_week
        .as_ref()
        .expect("the example starts on a date")
        .monday();
    assert_eq!(
        global::<Vec<chrono::NaiveDate>>(&globals, "week_mondays"),
        (0..walk.len())
            .map(|index| first_week
                .checked_add_days(chrono::Days::new(7 * index as u64))
                .expect("the example's weeks are datable"))
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// An id compares, hashes, orders and prints, and does nothing else
///
/// Most of this is the script's, because opacity is a statement about what
/// python *cannot* do: build one, turn one into a number, order one against
/// another kind. Rust checks the one thing a script cannot see for itself —
/// that the repr names the number the document really holds.
#[test]
fn ids_compare_hash_and_order_but_do_nothing_else() {
    use collomatique_state_colloscopes::ids::Id as _;

    let dir = workspace("ids");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/ids.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");
    let week = params.week_ids().next().expect("the example has weeks");

    assert_eq!(
        global::<String>(&globals, "period_id_repr"),
        format!("<PeriodId {}>", period.inner())
    );
    assert_eq!(
        global::<String>(&globals, "week_id_repr"),
        format!("<WeekId {}>", week.inner())
    );

    // The eleven kinds land together, and the script refused to build any of
    // them: the read surface hands ids out, it does not take them in.
    let names = global::<Vec<String>>(&globals, "id_class_names");
    assert_eq!(
        names.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "PeriodId",
            "WeekId",
            "SubjectId",
            "TeacherId",
            "StudentId",
            "WeekPatternId",
            "SlotId",
            "IncompatId",
            "GroupListId",
            "PairingRuleId",
            "SlotPairingRuleId",
        ]
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed period takes its handles down with it, loudly
///
/// The mutation cannot come from the script — the read surface ships no removes
/// — so it comes from rust, between the two halves: `DeletePeriodAndWeeks` kills
/// a period and every week in it in one blow, which is what makes both handle
/// kinds stale at once.
///
/// The second half is where the whole of §2.2 is pinned: `.id`, `==` and `hash`
/// keep working because they never read the state, every reading attribute
/// raises `StaleHandleError`, the repr says `(périmé)` instead of raising, and
/// the mapping conventions answer `None` / `False` / `KeyError`. The walk
/// started before the removal is in there too, for the promise that iteration
/// snapshots ids and mints handles as it goes.
#[test]
fn a_removed_period_makes_its_handles_stale() {
    let dir = workspace("stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same period the script is holding.
    let doomed = reload(&source)
        .get_inner_data()
        .params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");

    run_stages(
        &[
            include_str!("scripts/stale_before.py"),
            include_str!("scripts/stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::GeneralPlanning(
                        collomatique_ops::GeneralPlanningUpdateOp::DeletePeriodAndWeeks(doomed),
                    ),
                )
                .expect("the last period of the example is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The subjects read back, with their interrogation parameters
///
/// The script walks `doc.subjects` and leaves what it saw; rust compares it with
/// the same document read straight from the model — the names and the user order
/// they come in, and every field of the interrogation parameters for the
/// subjects that hold them.
///
/// The example is worth reading here because it carries both shapes: subjects
/// that run colles, and two that only take up room in the timetable and answer
/// `None`. What it does not carry is any subject skipping a period, or three of
/// the four periodicities — those are
/// [the_four_periodicities_read_back_value_by_value]'s document.
#[test]
fn the_subjects_read_back_with_their_interrogations() {
    let dir = workspace("subjects");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/subjects.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let subjects: Vec<_> = data
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .map(|(_id, subject)| subject.clone())
        .collect();

    // The example is only worth reading if it has something to say: several
    // subjects, and both shapes among them.
    assert!(subjects.len() > 1);
    let with_colles: Vec<_> = subjects
        .iter()
        .filter_map(|subject| subject.parameters.interrogation_parameters.as_ref())
        .collect();
    assert!(!with_colles.is_empty());
    assert!(with_colles.len() < subjects.len());

    assert_eq!(
        global::<Vec<usize>>(&globals, "subject_indices"),
        (0..subjects.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "subject_names"),
        subjects
            .iter()
            .map(|subject| subject.parameters.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "interrogation_present"),
        subjects
            .iter()
            .map(|subject| subject.parameters.interrogation_parameters.is_some())
            .collect::<Vec<_>>()
    );

    let bounds = |range: &collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32>| {
        (range.start().get(), range.end().get())
    };
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "students_per_group"),
        with_colles
            .iter()
            .map(|params| bounds(&params.students_per_group))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "groups_per_interrogation"),
        with_colles
            .iter()
            .map(|params| bounds(&params.groups_per_interrogation))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<u32>>(&globals, "durations"),
        with_colles
            .iter()
            .map(|params| params.duration.get().get())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "take_duration_into_account"),
        with_colles
            .iter()
            .map(|params| params.take_duration_into_account)
            .collect::<Vec<_>>()
    );

    // Which class each periodicity became. The example only ever uses two of the
    // four, which is why the other two need a document of their own.
    let expected_classes: Vec<_> = with_colles
        .iter()
        .map(|params| {
            use collomatique_state_colloscopes::SubjectPeriodicity as Model;
            match params.periodicity {
                Model::ExactlyPeriodic { .. } => "EveryNWeeks",
                Model::OnceForEveryBlockOfWeeks { .. } => "OncePerBlock",
                Model::AmountInYear { .. } => "CountInYear",
                Model::AmountForEveryArbitraryBlock { .. } => "CustomBlocks",
            }
        })
        .collect();
    assert_eq!(
        global::<Vec<String>>(&globals, "periodicity_class_names"),
        expected_classes
    );

    let period_ids: Vec<_> = data.get_inner_data().params.periods.period_ids().collect();
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "excluded_period_indices"),
        subjects
            .iter()
            .map(|subject| {
                let mut indices: Vec<_> = subject
                    .excluded_periods
                    .iter()
                    .map(|period| {
                        period_ids
                            .iter()
                            .position(|id| id == period)
                            .expect("an excluded period is a live one")
                    })
                    .collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A `(min, max)` the model stores as a range that counts from one
fn nonzero_range(
    (min, max): (u32, u32),
) -> collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32> {
    let bound = |value: u32| NonZeroU32::new(value).expect("the fixtures count from one");
    collomatique_state_colloscopes::NonEmptyRangeInclusive::new(bound(min)..=bound(max))
        .expect("the fixtures' ranges are non-empty")
}

/// A `(min, max)` the model stores as a range that may start at zero
fn plain_range(
    (min, max): (u32, u32),
) -> collomatique_state_colloscopes::NonEmptyRangeInclusive<u32> {
    collomatique_state_colloscopes::NonEmptyRangeInclusive::new(min..=max)
        .expect("the fixtures' ranges are non-empty")
}

/// One subject of the periodicity fixture
///
/// It takes every field rather than starting from a default, because the point
/// of the fixture is that no two subjects share a value: a conversion reading the
/// wrong field would still line up on a document built out of defaults.
fn periodicity_subject(
    name: &str,
    students_per_group: (u32, u32),
    groups_per_interrogation: (u32, u32),
    duration: u32,
    take_duration_into_account: bool,
    periodicity: collomatique_state_colloscopes::SubjectPeriodicity,
    excluded_periods: BTreeSet<collomatique_state_colloscopes::PeriodId>,
) -> collomatique_state_colloscopes::Subject {
    use collomatique_state_colloscopes::{
        Subject, SubjectInterrogationParameters, SubjectParameters,
    };

    Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range(students_per_group),
                groups_per_interrogation: nonzero_range(groups_per_interrogation),
                duration: collomatique_time::NonZeroMinutes::new(duration)
                    .expect("the fixtures' interrogations last a while"),
                take_duration_into_account,
                periodicity,
            }),
        },
        excluded_periods,
    }
}

/// A document written here rather than copied, holding all four periodicities
///
/// The example uses two of the four and excludes no period from any subject, so
/// the fourfold read needs a document of its own. It is built as an `InnerData`
/// through the sealed types' own constructors and passed through
/// `Data::from_inner_data`, so a fixture that breaks an invariant fails here
/// rather than halfway through the script
/// (`docs/python/handle_api.md` §6.2).
fn periodicity_document(path: &Path) {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::subjects::{Subjects, WeekBlock};
    use collomatique_state_colloscopes::{
        Data, InnerData, PeriodId, SubjectId, SubjectPeriodicity,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let periods = vec![period(1), period(2)];

    let subjects = vec![
        (
            unsafe { SubjectId::new(11) },
            periodicity_subject(
                "Périodique",
                (1, 1),
                (2, 4),
                45,
                false,
                SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(3).expect("three is not zero"),
                },
                BTreeSet::new(),
            ),
        ),
        (
            unsafe { SubjectId::new(12) },
            periodicity_subject(
                "Par bloc",
                (2, 3),
                (1, 1),
                60,
                true,
                SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block: NonZeroU32::new(4).expect("four is not zero"),
                    minimum_week_separation: NonZeroU32::new(2).expect("two is not zero"),
                },
                // The one subject that skips a period, so the frozenset of
                // handles has something in it.
                BTreeSet::from([period(2)]),
            ),
        ),
        (
            unsafe { SubjectId::new(13) },
            periodicity_subject(
                "Dans l'année",
                (3, 3),
                (1, 2),
                30,
                true,
                SubjectPeriodicity::AmountInYear {
                    interrogation_count_in_year: plain_range((2, 5)),
                    minimum_week_separation: 0,
                },
                BTreeSet::new(),
            ),
        ),
        (
            unsafe { SubjectId::new(14) },
            periodicity_subject(
                "Blocs sur mesure",
                (2, 2),
                (1, 1),
                90,
                false,
                SubjectPeriodicity::AmountForEveryArbitraryBlock {
                    blocks: vec![
                        WeekBlock {
                            delay_in_weeks: 0,
                            size_in_weeks: NonZeroU32::new(2).expect("two is not zero"),
                            interrogation_count_in_block: plain_range((1, 1)),
                        },
                        WeekBlock {
                            delay_in_weeks: 3,
                            size_in_weeks: NonZeroU32::new(4).expect("four is not zero"),
                            interrogation_count_in_block: plain_range((0, 2)),
                        },
                    ],
                    minimum_week_separation: 1,
                },
                BTreeSet::new(),
            ),
        ),
    ];

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The four periodicities read back, value by value
///
/// Everything here is the script's, because it is all python-facing: the four
/// classes, the ranges that are plain tuples, the leaf values a script builds to
/// compare against, and the `ValueError` construction raises on what the model
/// would refuse. Rust's half is the document — the example holds two of the four
/// periodicities and excludes no period, so the other two and the exclusion are
/// written here.
#[test]
fn the_four_periodicities_read_back_value_by_value() {
    let dir = workspace("periodicities");
    let source = dir.join("periodicities.collomatique");
    periodicity_document(&source);

    run(include_str!("scripts/periodicities.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A subject's colles go, then the subject does, and each takes its own view down
///
/// Three stages, two mutations, because the `Interrogation` sub-view has two
/// ways of dying and they mean different things: the subject stopped holding
/// interrogations, or the subject is gone. The first leaves the handle perfectly
/// alive — `subject.interrogation` simply answers `None` from then on — and only
/// the second takes it with it.
///
/// The mutations come from rust: the read surface ships no writes of its own, so
/// the ops layer applies them between the stages.
#[test]
fn switching_a_subject_off_then_removing_it_stales_the_view_then_the_handle() {
    let dir = workspace("subject-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same subject the script is holding. The
    // last one that runs colles, so what is removed sits after the survivor and
    // nothing renumbers under it.
    let data = reload(&source);
    let (doomed, doomed_params) = data
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, subject)| (id, subject.parameters.clone()))
        .last()
        .expect("the example has subjects that run colles");

    let mut without_colles = doomed_params;
    without_colles.interrogation_parameters = None;

    let mut stage = 0;
    run_stages(
        &[
            include_str!("scripts/subject_alive.py"),
            include_str!("scripts/subject_without_colles.py"),
            include_str!("scripts/subject_gone.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            let op = match stage {
                0 => collomatique_ops::SubjectsUpdateOp::UpdateSubject(
                    doomed,
                    without_colles.clone(),
                ),
                _ => collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed),
            };
            stage += 1;

            document_of(globals)
                .borrow_mut(py)
                .update(py, collomatique_ops::UpdateOp::Subjects(op))
                .expect("the example's last subject with colles is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// What python reads for a field the model types as an optional non-empty string
///
/// Written against `Display` rather than against the string type itself, so the
/// test file needs no dependency on the crate that type comes from: what is
/// being compared is the text, which is all python ever sees of it.
fn optional_text<T: std::fmt::Display>(value: &Option<T>) -> Option<String> {
    value.as_ref().map(|text| text.to_string())
}

/// The teachers and the students read back, person by person
///
/// The script walks `doc.teachers` and `doc.students` and leaves what it saw;
/// rust compares it with the same document read straight from the model — every
/// field of the card the two entities share, the subjects a teacher interrogates
/// in, and the id order the two collections iterate in.
///
/// The example is worth reading here because it carries teachers who shared a
/// number and teachers who did not, and students of both shapes too. What it
/// does not carry is anyone who shared neither, a teacher who interrogates in
/// nothing, or a student sitting a period out — those are
/// [a_person_who_shared_nothing_reads_as_none]'s document.
#[test]
fn the_teachers_and_the_students_read_back_person_by_person() {
    let dir = workspace("people");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/people.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // In id order, which is the order the two tables iterate in and the order
    // the script says it saw.
    let teachers: Vec<_> = params
        .teachers
        .teacher_map
        .iter()
        .map(|(_id, teacher)| teacher.clone())
        .collect();
    let students: Vec<_> = params
        .students
        .student_map
        .iter()
        .map(|(_id, student)| student.clone())
        .collect();

    // The example is only worth reading if it has something to say: several of
    // each, and both contact shapes among them.
    assert!(teachers.len() > 1);
    assert!(students.len() > 1);
    assert!(teachers.iter().any(|teacher| teacher.desc.tel.is_none()));
    assert!(teachers.iter().any(|teacher| teacher.desc.tel.is_some()));
    assert!(students.iter().any(|student| student.desc.email.is_none()));
    assert!(students.iter().any(|student| student.desc.email.is_some()));

    assert_eq!(
        global::<Vec<String>>(&globals, "teacher_surnames"),
        teachers
            .iter()
            .map(|teacher| teacher.desc.surname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "teacher_firstnames"),
        teachers
            .iter()
            .map(|teacher| teacher.desc.firstname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "teacher_tels"),
        teachers
            .iter()
            .map(|teacher| optional_text(&teacher.desc.tel))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "teacher_emails"),
        teachers
            .iter()
            .map(|teacher| optional_text(&teacher.desc.email))
            .collect::<Vec<_>>()
    );

    assert_eq!(
        global::<Vec<String>>(&globals, "student_surnames"),
        students
            .iter()
            .map(|student| student.desc.surname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "student_firstnames"),
        students
            .iter()
            .map(|student| student.desc.firstname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "student_tels"),
        students
            .iter()
            .map(|student| optional_text(&student.desc.tel))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "student_emails"),
        students
            .iter()
            .map(|student| optional_text(&student.desc.email))
            .collect::<Vec<_>>()
    );

    // The subjects a teacher interrogates in, read through the display positions
    // the handles in them answer with — a subject named by its place in the user
    // order.
    let subject_ids: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "teacher_subject_indices"),
        teachers
            .iter()
            .map(|teacher| {
                let mut indices: Vec<_> = teacher
                    .subjects
                    .iter()
                    .map(|subject| {
                        subject_ids
                            .iter()
                            .position(|id| id == subject)
                            .expect("a teacher's subject is a live one")
                    })
                    .collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );

    // Every student of the example sits every period, so all this says is that
    // python saw the sets empty — which is worth saying, since an exclusion set
    // read from the wrong student would show up here. The sets with something in
    // them are [a_person_who_shared_nothing_reads_as_none]'s document, and its
    // script is where they are read one by one.
    assert!(
        students
            .iter()
            .all(|student| student.excluded_periods.is_empty())
    );
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "student_excluded_period_indices"),
        vec![Vec::<usize>::new(); students.len()]
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// One person of the contact fixture, with the card the model keeps for them
///
/// The tel and the email are handed in as plain strings and become the model's
/// optional non-empty ones here, so the fixture reads as the four shapes it is
/// meant to be: both, one, the other, neither.
fn person(
    firstname: &str,
    surname: &str,
    tel: Option<&str>,
    email: Option<&str>,
) -> collomatique_state_colloscopes::PersonWithContact {
    // The model's optional non-empty string, reached without naming its crate:
    // the field says what the conversion lands in, so the fixture needs no
    // dependency of its own to build one.
    let contact = |text: Option<&str>| {
        text.map(|text| {
            text.to_owned()
                .try_into()
                .expect("the fixture's contact details are not empty")
        })
    };

    collomatique_state_colloscopes::PersonWithContact {
        surname: surname.to_owned(),
        firstname: firstname.to_owned(),
        tel: contact(tel),
        email: contact(email),
    }
}

/// A document written here rather than copied, holding every contact shape
///
/// The example has nobody who shared neither a number nor an email, no teacher
/// who interrogates in nothing, and no student sitting a period out — so the
/// four shapes of a card and the two extremes of a set need a document of their
/// own. It is built as an `InnerData` through the sealed types' own constructors
/// and passed through `Data::from_inner_data`, so a fixture that breaks an
/// invariant fails here rather than halfway through the script
/// (`docs/python/handle_api.md` §6.2).
fn contact_document(path: &Path) {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::teachers::{Teacher, Teachers};
    use collomatique_state_colloscopes::{
        Data, InnerData, PeriodId, StudentId, Subject, SubjectId, SubjectInterrogationParameters,
        SubjectParameters, SubjectPeriodicity, TeacherId,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let periods = vec![period(1), period(2)];

    // The subjects are here for the teachers to name. They run colles because a
    // teacher may only interrogate in a subject that has some, and what those
    // colles look like is [the_four_periodicities_read_back_value_by_value]'s
    // business rather than this fixture's — so the two share every parameter.
    let named_subject = |name: &str| Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range((2, 3)),
                groups_per_interrogation: nonzero_range((1, 1)),
                duration: collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(1).expect("one is not zero"),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    };
    let subjects = vec![
        (subject(11), named_subject("Sortilèges")),
        (subject(12), named_subject("Métamorphose")),
    ];

    // Both contact details, one, the other, neither — and, on the way, a teacher
    // who interrogates in two subjects, ones who interrogate in one, and one who
    // interrogates in nothing.
    let teachers = vec![
        (
            unsafe { TeacherId::new(21) },
            Teacher {
                desc: person(
                    "Minerva",
                    "McGonagall",
                    Some("0700000021"),
                    Some("mcgonagall@poudlard.fr"),
                ),
                subjects: BTreeSet::from([subject(11), subject(12)]),
            },
        ),
        (
            unsafe { TeacherId::new(22) },
            Teacher {
                desc: person("Severus", "Rogue", Some("0700000022"), None),
                subjects: BTreeSet::from([subject(11)]),
            },
        ),
        (
            unsafe { TeacherId::new(23) },
            Teacher {
                desc: person("Pomona", "Chourave", None, Some("chourave@poudlard.fr")),
                subjects: BTreeSet::from([subject(12)]),
            },
        ),
        (
            unsafe { TeacherId::new(24) },
            Teacher {
                desc: person("Cuthbert", "Binns", None, None),
                subjects: BTreeSet::new(),
            },
        ),
    ];

    // The same four shapes, with the exclusion sets running from empty to whole.
    let students = vec![
        (
            unsafe { StudentId::new(31) },
            Student {
                desc: person(
                    "Harry",
                    "Potter",
                    Some("0601020304"),
                    Some("harry.potter@poudlard.fr"),
                ),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            unsafe { StudentId::new(32) },
            Student {
                desc: person("Hermione", "Granger", Some("0605060708"), None),
                excluded_periods: BTreeSet::from([period(1)]),
            },
        ),
        (
            unsafe { StudentId::new(33) },
            Student {
                desc: person("Ron", "Weasley", None, Some("ron.weasley@poudlard.fr")),
                excluded_periods: BTreeSet::from([period(2)]),
            },
        ),
        (
            unsafe { StudentId::new(34) },
            Student {
                desc: person("Neville", "Londubat", None, None),
                excluded_periods: BTreeSet::from([period(1), period(2)]),
            },
        ),
    ];

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };
    // An id-keyed table takes the last of a duplicated id without a word, where
    // the ordered lists above refuse one. So the count is checked by hand: a
    // fixture that named a teacher twice would otherwise quietly ship one fewer
    // person than the script is about to read.
    let (teacher_count, student_count) = (teachers.len(), students.len());
    inner_data.params.teachers = Teachers {
        teacher_map: teachers.into_iter().collect(),
    };
    inner_data.params.students = Students {
        student_map: students.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.teachers.teacher_map.len(),
        teacher_count,
        "the fixture names each teacher once"
    );
    assert_eq!(
        inner_data.params.students.student_map.len(),
        student_count,
        "the fixture names each student once"
    );

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// A person who shared no number and no email reads as `None`, not as `""`
///
/// Everything here is the script's, because it is all python-facing: the four
/// shapes a card can have, the sets that are frozen and may be empty, and the
/// handles inside them that read this document rather than carrying names copied
/// out of it. Rust's half is the document — the example shares at least one
/// contact detail for everyone, gives every teacher a subject and excludes no
/// student from a period, so the missing shapes are written here.
///
/// The example comes along as a second document, because the two number their
/// people nowhere near each other: that is what lets the script hold an id that
/// is a perfectly good one and still names nothing where it is asked. Rust
/// checks the two id spaces really are disjoint, since the whole question the
/// script asks rests on it.
#[test]
fn a_person_who_shared_nothing_reads_as_none() {
    let dir = workspace("contacts");
    let source = dir.join("contacts.collomatique");
    contact_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let teacher_ids = |data: &Data| -> BTreeSet<_> {
        data.get_inner_data()
            .params
            .teachers
            .teacher_map
            .keys()
            .collect()
    };
    let student_ids = |data: &Data| -> BTreeSet<_> {
        data.get_inner_data()
            .params
            .students
            .student_map
            .keys()
            .collect()
    };

    let fixture = reload(&source);
    let example = reload(&other_source);
    assert!(teacher_ids(&fixture).is_disjoint(&teacher_ids(&example)));
    assert!(student_ids(&fixture).is_disjoint(&student_ids(&example)));

    run(include_str!("scripts/contacts.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The week patterns read back, with the weeks they switch off
///
/// The script walks `doc.week_patterns` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the names, the
/// exception sets read as global week positions, and the whole activity grid:
/// every week of the document against every pattern of it, `None` included.
///
/// The grid is the headline. `doc.is_week_active` must answer what
/// `Parameters::is_week_active` answers, because that is the one definition the
/// gui grid and the constraints layer read too — an api that merged the week's
/// flag with the pattern's set its own way would be describing a different
/// document. So the expected values are computed from the model rather than
/// written out, and the assertions above them say the example really exercises
/// the merge: weeks of both flags, patterns that exclude something, and a
/// pattern that switches off a week which holds no colles anyway.
#[test]
fn the_week_patterns_read_back_with_the_weeks_they_switch_off() {
    let dir = workspace("week-patterns");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/week_patterns.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // In id order, which is the order the table iterates in and the order the
    // script says it saw.
    let patterns: Vec<_> = params
        .week_patterns
        .week_pattern_map
        .iter()
        .map(|(id, pattern)| (id, pattern.clone()))
        .collect();
    let walk: Vec<_> = params.walk_weeks().collect();

    // The example is only worth reading if it has something to say: several
    // patterns, exception sets with something in them, weeks of both flags, and
    // — the case the model is explicit about — a pattern excluding a week that
    // holds no interrogations anyway.
    assert!(patterns.len() > 1);
    assert!(
        patterns
            .iter()
            .all(|(_id, pattern)| !pattern.excluded_weeks.is_empty())
    );
    assert!(walk.iter().any(|(_p, _id, week)| week.interrogations));
    assert!(walk.iter().any(|(_p, _id, week)| !week.interrogations));
    assert!(patterns.iter().any(|(_id, pattern)| {
        walk.iter().any(|(_p, week_id, week)| {
            !week.interrogations && pattern.excluded_weeks.contains(week_id)
        })
    }));

    assert_eq!(
        global::<Vec<String>>(&globals, "pattern_names"),
        patterns
            .iter()
            .map(|(_id, pattern)| pattern.name.clone())
            .collect::<Vec<_>>()
    );

    // The exception sets, read through the global positions of the weeks in them
    // — a week named by its place in the document's own walk.
    let position = |week: &collomatique_state_colloscopes::WeekId| {
        walk.iter()
            .position(|(_p, id, _week)| id == week)
            .expect("an excluded week is a live one")
    };
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "pattern_excluded_week_indices"),
        patterns
            .iter()
            .map(|(_id, pattern)| {
                let mut indices: Vec<_> = pattern.excluded_weeks.iter().map(position).collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );

    // One row per week, one column per pattern, and a first column for the
    // pattern-less question a slot without one asks.
    let columns: Vec<Option<_>> = std::iter::once(None)
        .chain(patterns.iter().map(|(id, _pattern)| Some(*id)))
        .collect();
    let expected: Vec<Vec<bool>> = walk
        .iter()
        .map(|(_p, week, _desc)| {
            columns
                .iter()
                .map(|pattern| params.is_week_active(*week, *pattern))
                .collect()
        })
        .collect();

    // A grid of all-the-same would compare equal without pinning anything: the
    // weeks must disagree among themselves, and a pattern must disagree with the
    // pattern-less column.
    assert!(expected.iter().any(|row| row[0]));
    assert!(expected.iter().any(|row| !row[0]));
    assert!(
        expected
            .iter()
            .any(|row| row.iter().any(|answer| *answer != row[0]))
    );

    assert_eq!(global::<Vec<Vec<bool>>>(&globals, "activity"), expected);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document written here rather than copied, holding the patterns the example
/// has not got
///
/// The example's two patterns both exclude every other week, both have a name,
/// and neither is empty — so the three ends of what a pattern can be (excluding
/// nothing, excluding everything, and having no name at all) need a document of
/// their own. Its weeks carry both flags, so that a `False` answer can be told
/// apart by its reason.
///
/// It is built as an `InnerData` through the sealed types' own constructors and
/// passed through `Data::from_inner_data`, so a fixture that breaks an invariant
/// fails here rather than halfway through the script
/// (`docs/python/handle_api.md` §6.2).
fn week_pattern_document(path: &Path) {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::week_patterns::{WeekPattern, WeekPatterns};
    use collomatique_state_colloscopes::weeks::{WeekDesc, Weeks};
    use collomatique_state_colloscopes::{Data, InnerData, PeriodId, WeekId, WeekPatternId};

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with. The patterns are
    // numbered nowhere near the example's, since the script holds ids of both
    // documents at once.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let week = |n: u64| unsafe { WeekId::new(n) };
    let pattern = |n: u64| unsafe { WeekPatternId::new(n) };

    let periods = vec![period(1), period(2)];
    let weeks = vec![
        (
            period(1),
            vec![
                (week(11), WeekDesc::new(true)),
                (week(12), WeekDesc::new(false)),
                (week(13), WeekDesc::new(true)),
            ],
        ),
        (
            period(2),
            vec![
                (week(14), WeekDesc::new(true)),
                (week(15), WeekDesc::new(true)),
                (week(16), WeekDesc::new(false)),
            ],
        ),
    ];

    let patterns = vec![
        (
            pattern(41),
            WeekPattern {
                name: "Toutes les semaines".to_owned(),
                excluded_weeks: BTreeSet::new(),
            },
        ),
        (
            pattern(42),
            WeekPattern {
                name: "Semaines paires".to_owned(),
                // The odd positions of the walk: one of them holds colles, and
                // two of them hold none anyway — so the set says something about
                // a week the flag has already settled.
                excluded_weeks: BTreeSet::from([week(12), week(14), week(16)]),
            },
        ),
        (
            pattern(43),
            WeekPattern {
                // A pattern the user never named. The model types the field as a
                // plain `String`, so this reads as `""` and not as `None`.
                name: String::new(),
                excluded_weeks: BTreeSet::from([week(11)]),
            },
        ),
        (
            pattern(44),
            WeekPattern {
                name: "Aucune semaine".to_owned(),
                excluded_weeks: (11..=16).map(week).collect(),
            },
        ),
    ];

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.weeks =
        Weeks::from_period_rows(weeks).expect("the fixture names each week once");
    // An id-keyed table takes the last of a duplicated id without a word, so the
    // count is checked by hand: a fixture that named a pattern twice would
    // otherwise quietly ship one fewer than the script is about to read.
    let pattern_count = patterns.len();
    inner_data.params.week_patterns = WeekPatterns {
        week_pattern_map: patterns.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.week_patterns.week_pattern_map.len(),
        pattern_count,
        "the fixture names each pattern once"
    );

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// A pattern that excludes nothing, one that excludes everything, and one with
/// no name
///
/// Everything here is the script's, because it is all python-facing: the sets
/// that are frozen and may be empty or whole, the `""` of a nameless pattern,
/// and the grid a script can write out by hand because the document was built
/// for it. Rust's half is that document, plus the disjointness the script's last
/// question rests on.
///
/// The example comes along as a second document, because the two number their
/// patterns nowhere near each other: that is what lets the script hold an id
/// that is a perfectly good `WeekPatternId` and still names nothing where it is
/// asked — a lookup and an argument, side by side, answering the two different
/// ways §2.4 says they must.
#[test]
fn a_pattern_excludes_no_week_every_week_or_the_ones_it_names() {
    let dir = workspace("exclusions");
    let source = dir.join("exclusions.collomatique");
    week_pattern_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let pattern_ids = |data: &Data| -> BTreeSet<_> {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .keys()
            .collect()
    };

    let fixture = reload(&source);
    let example = reload(&other_source);
    assert!(pattern_ids(&fixture).is_disjoint(&pattern_ids(&example)));

    run(include_str!("scripts/exclusions.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed pattern and a removed week make the predicate raise, not shrug
///
/// The mutations cannot come from the script — the read surface ships no removes
/// — so they come from rust, between the two halves: one week pattern goes, and
/// one whole period goes with every week in it.
///
/// The point is the divergence §2.4 asks for. The model is forgiving about a
/// reference it cannot resolve: `is_week_active` answers `false` for a week it
/// does not hold, and treats a pattern it does not hold as excluding nothing.
/// Those are the two answers pinned here on the model's side — and the script's
/// side is that python gives neither, because an argument naming nothing was
/// malformed before it had an answer.
#[test]
fn a_removed_week_or_pattern_makes_is_week_active_raise() {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::{WeekId, WeekPatternId};

    let dir = workspace("pattern-stale");
    let source = dir.join("exclusions.collomatique");
    week_pattern_document(&source);

    // What the model says about a reference it cannot resolve. The ids are made
    // up on purpose: a document that never held them is exactly the position the
    // script is in once its own are removed.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let live_week = params.week_ids().next().expect("the fixture has weeks");
    assert!(!params.is_week_active(unsafe { WeekId::new(9_001) }, None));
    assert!(params.is_week_active(live_week, Some(unsafe { WeekPatternId::new(9_002) })));

    run_stages(
        &[
            include_str!("scripts/pattern_alive.py"),
            include_str!("scripts/pattern_gone.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            let doc = document_of(globals);

            // « Semaines paires », then the second period and its three weeks.
            // The ids are the fixture's own, which the file keeps.
            for op in [
                collomatique_ops::UpdateOp::WeekPatterns(
                    collomatique_ops::WeekPatternsUpdateOp::DeleteWeekPattern(unsafe {
                        WeekPatternId::new(42)
                    }),
                ),
                collomatique_ops::UpdateOp::GeneralPlanning(
                    collomatique_ops::GeneralPlanningUpdateOp::DeletePeriodAndWeeks(unsafe {
                        collomatique_state_colloscopes::PeriodId::new(2)
                    }),
                ),
            ] {
                doc.borrow_mut(py)
                    .update(py, op)
                    .expect("the fixture's second period and pattern are removable");
            }
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The english name of a day, for the mapping the script builds from the members
///
/// The script cannot read a day's name off the class — the members are plain
/// class attributes, not an `enum.Enum` — so it names them itself, and this is
/// the other half of that dictionary. Written as a match so that the seven are
/// spelled out on both sides.
fn weekday_name(weekday: collomatique_time::Weekday) -> &'static str {
    match weekday.into_inner() {
        chrono::Weekday::Mon => "monday",
        chrono::Weekday::Tue => "tuesday",
        chrono::Weekday::Wed => "wednesday",
        chrono::Weekday::Thu => "thursday",
        chrono::Weekday::Fri => "friday",
        chrono::Weekday::Sat => "saturday",
        chrono::Weekday::Sun => "sunday",
    }
}

/// The slots read back, and the cells they can hold a colle in
///
/// The script walks `doc.slots` and leaves what it saw; rust compares it with
/// the same document read straight from the model — every field of a slot, the
/// order the walk comes in, and the whole possibility grid: every slot of the
/// document against every week of it.
///
/// The grid is the headline. `doc.is_interrogation_possible` must answer what
/// `Parameters::is_interrogation_possible` answers, because that is the one
/// definition the gui grid, the constraints layer and the file decoder read too
/// — an api that joined the slot, the subject and the pattern its own way would
/// be describing a different document. So the expected values are computed from
/// the model rather than written out, and the assertions above them say the
/// example really exercises the join.
///
/// The two orders are pinned side by side: the walk is the subjects in user
/// order, each followed by its own slots, and `.index` counts inside the
/// subject. The example is worth reading for that because at least one of its
/// subjects keeps its slots in an order that is not their ids'.
#[test]
fn the_slots_read_back_with_the_cells_they_can_fill() {
    let dir = workspace("slots");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/slots.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The walk the api promises: the subjects in user order — the one
    // `ordered_subject_list` keeps, which is the one `doc.subjects` walks —
    // each followed by its slots in theirs. This is the order the script says
    // it saw.
    let walk: Vec<_> = params
        .subjects
        .ordered_subject_list
        .keys()
        .flat_map(|subject_id| {
            params
                .slots
                .slots_for_subject(subject_id)
                .into_iter()
                .flatten()
        })
        .map(|(slot_id, slot)| (*slot_id, slot.clone()))
        .collect();
    let week_ids: Vec<_> = params.week_ids().collect();

    // The example is only worth reading if it has something to say: several
    // subjects with slots, a subject whose slots are not in id order, patterns
    // both carried and absent, and costs that are not all the same.
    let subjects_with_slots: Vec<_> = params.slots.subjects_with_slots().collect();
    assert!(subjects_with_slots.len() > 1);
    assert!(subjects_with_slots.iter().any(|subject| {
        let ids: Vec<_> = params
            .slots
            .slots_for_subject(*subject)
            .expect("a subject with slots has an ordering row")
            .map(|(slot_id, _slot)| *slot_id)
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        ids != sorted
    }));
    assert!(walk.iter().any(|(_id, slot)| slot.week_pattern.is_some()));
    assert!(walk.iter().any(|(_id, slot)| slot.week_pattern.is_none()));
    assert!(walk.iter().any(|(_id, slot)| slot.cost != walk[0].1.cost));

    assert_eq!(
        global::<Vec<usize>>(&globals, "slot_indices"),
        walk.iter()
            .map(|(slot_id, _slot)| params
                .slots
                .find_slot_subject_and_position(*slot_id)
                .expect("a walked slot is a live one")
                .1)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "slot_subject_indices"),
        walk.iter()
            .map(|(_slot_id, slot)| params
                .subjects
                .find_subject_position(slot.subject_id)
                .expect("a slot names a live subject"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "slot_teacher_surnames"),
        walk.iter()
            .map(|(_slot_id, slot)| params
                .teachers
                .teacher_map
                .get(&slot.teacher_id)
                .expect("a slot names a live teacher")
                .desc
                .surname
                .clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "slot_weekdays"),
        walk.iter()
            .map(|(_slot_id, slot)| weekday_name(slot.start_time.weekday).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<chrono::NaiveTime>>(&globals, "slot_start_times"),
        walk.iter()
            .map(|(_slot_id, slot)| *slot.start_time.start_time.inner())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "slot_extra_info"),
        walk.iter()
            .map(|(_slot_id, slot)| slot.extra_info.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<i32>>(&globals, "slot_costs"),
        walk.iter()
            .map(|(_slot_id, slot)| slot.cost)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "slot_pattern_names"),
        walk.iter()
            .map(|(_slot_id, slot)| {
                slot.week_pattern.map(|pattern_id| {
                    params
                        .week_patterns
                        .week_pattern_map
                        .get(&pattern_id)
                        .expect("a slot names a live pattern")
                        .name
                        .clone()
                })
            })
            .collect::<Vec<_>>()
    );

    // One row per slot, one column per week of the document.
    let expected: Vec<Vec<bool>> = walk
        .iter()
        .map(|(slot_id, _slot)| {
            week_ids
                .iter()
                .map(|week| params.is_interrogation_possible(*slot_id, *week))
                .collect()
        })
        .collect();

    // A grid of all-the-same would compare equal without pinning anything: the
    // weeks must disagree among themselves, and two slots must disagree about
    // some week — which is the pattern half of the join doing something.
    assert!(expected.iter().any(|row| row.iter().any(|answer| *answer)));
    assert!(expected.iter().any(|row| row.iter().any(|answer| !*answer)));
    assert!(expected.iter().any(|row| {
        row.iter()
            .zip(&expected[0])
            .any(|(here, first)| here != first)
    }));

    assert_eq!(global::<Vec<Vec<bool>>>(&globals, "possibility"), expected);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document written here rather than copied, holding a sparse assignments
/// table
///
/// The example's table is complete — every subject holds a row on every
/// period — so the absent-address shape, the empty frozenset of a valid pair
/// no row is stored for, needs a document of its own: three rows on six
/// possible pairs, one subject with no row at all, and a second period that
/// does not repeat the first's rows. It is built as an `InnerData` through the
/// sealed types' own constructors and passed through `Data::from_inner_data`,
/// so a fixture that breaks an invariant fails here rather than halfway
/// through the script (`docs/python/handle_api.md` §6.2).
fn assignments_document(path: &Path) {
    use collomatique_state_colloscopes::assignments::Assignments;
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::{
        Data, InnerData, PeriodId, StudentId, Subject, SubjectId, SubjectInterrogationParameters,
        SubjectParameters, SubjectPeriodicity,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let student = |n: u64| unsafe { StudentId::new(n) };

    let periods = vec![period(1), period(2)];

    // The subjects run colles, because a row for a subject that never runs any
    // would be a document the loader refuses; what those colles look like is
    // [the_four_periodicities_read_back_value_by_value]'s business rather than
    // this fixture's. None excludes a period, so every row's period is one the
    // subject runs on.
    let named_subject = |name: &str| Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range((2, 3)),
                groups_per_interrogation: nonzero_range((1, 1)),
                duration: collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(1).expect("one is not zero"),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    };
    let subjects = vec![
        (subject(11), named_subject("Sortilèges")),
        (subject(12), named_subject("Métamorphose")),
        (subject(13), named_subject("Potions")),
    ];

    // No student sits a period out, so every row's students are present on
    // its period.
    let students = vec![
        (
            student(31),
            Student {
                desc: person("Harry", "Potter", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(32),
            Student {
                desc: person("Hermione", "Granger", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(33),
            Student {
                desc: person("Ron", "Weasley", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(34),
            Student {
                desc: person("Neville", "Londubat", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
    ];

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };
    let student_count = students.len();
    inner_data.params.students = Students {
        student_map: students.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.students.student_map.len(),
        student_count,
        "the fixture names each student once"
    );

    // Three stored rows out of six possible pairs: the first subject on both
    // periods, the second on the second only, and the third on neither — so
    // the absent pairs are (1, 12), (1, 13) and (2, 13).
    inner_data.params.assignments = Assignments {
        map: [
            (
                (period(1), subject(11)),
                BTreeSet::from([student(31), student(32)]),
            ),
            (
                (period(2), subject(11)),
                BTreeSet::from([student(31), student(33)]),
            ),
            (
                (period(2), subject(12)),
                BTreeSet::from([student(33), student(34)]),
            ),
        ]
        .into_iter()
        .collect(),
    };

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The assignments read back, row by row
///
/// The script walks `doc.assignments` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the stored rows
/// themselves, and the key order they come in: `params.assignments.iter()`
/// over the `(PeriodId, SubjectId)` table.
///
/// The example's assignments table is complete — every subject holds a row on
/// every period, eight subjects across three periods — so the absent-address
/// shape, the empty frozenset of a valid pair no row is stored for, needs a
/// document of its own: [assignments_document] stores rows on three of its six
/// pairs. The script does the rest on its own, because it is about what python
/// sees: the total read, the address that must be a pair, the missing
/// `len`/`in`/`get`, and the address of another document.
#[test]
fn the_assignments_read_back_row_by_row() {
    let dir = workspace("assignments");
    let source = dir.join("assignments.collomatique");
    assignments_document(&source);

    let globals = run(include_str!("scripts/assignments.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The fixture is only worth reading if it has something to say: several
    // rows, on more than one period, and at least one valid address that
    // stores no row — the two shapes of a total read.
    let rows: Vec<_> = params.assignments.iter().collect();
    let periods: Vec<_> = params.periods.period_ids().collect();
    let subjects: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    assert!(rows.len() > 1);
    assert!(periods.len() > 1);
    assert!(subjects.len() > 1);
    let row_periods: BTreeSet<_> = rows
        .iter()
        .map(|(period, _subject, _students)| *period)
        .collect();
    assert!(row_periods.len() > 1);
    let stored: BTreeSet<_> = rows
        .iter()
        .map(|(period, subject, _students)| (*period, *subject))
        .collect();
    assert!(
        periods
            .iter()
            .flat_map(|period| subjects.iter().map(move |subject| (*period, *subject)))
            .any(|key| !stored.contains(&key))
    );

    assert_eq!(
        global::<Vec<usize>>(&globals, "row_period_indices"),
        rows.iter()
            .map(|(period, _subject, _students)| periods
                .iter()
                .position(|id| id == period)
                .expect("a row names a live period"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "row_subject_indices"),
        rows.iter()
            .map(|(_period, subject, _students)| subjects
                .iter()
                .position(|id| id == subject)
                .expect("a row names a live subject"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<String>>>(&globals, "row_student_surnames"),
        rows.iter()
            .map(|(_period, _subject, students)| {
                let mut names: Vec<_> = students
                    .iter()
                    .map(|student| {
                        params
                            .students
                            .student_map
                            .get(student)
                            .expect("a row names a live student")
                            .desc
                            .surname
                            .clone()
                    })
                    .collect();
                names.sort();
                names
            })
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed subject takes its rows with it, and the address raises
///
/// The mutation cannot come from the script — the read surface ships no
/// removes — so it comes from rust, between the two halves: the subject of
/// the last stored row, in key order, goes — the very row the script's first
/// half picked, so the two sides agree on what is doomed. The cascade repairs
/// the dangling rows away (`ops/src/subjects.rs`), which is what makes the
/// address dead rather than empty.
///
/// The second half pins §3.7's wrinkle: the address is an *argument*, so a
/// dead one raises `StaleHandleError` where the total read's empty frozenset
/// would have read as « nobody assigned ». And the survivors read exactly as
/// before, because what went was the subject, not the table.
#[test]
fn a_removed_address_makes_the_assignments_read_raise() {
    let dir = workspace("assignments-stale");
    let source = example_copy(&dir, "source.collomatique");

    // The first and the last stored row, in the model's key order — read from
    // the file rather than from the running document, like the other staleness
    // tests: ids are stored, so this copy names the same rows the script is
    // holding. The example's row subjects all repeat, but the first and the
    // last are different ones — the survivor's subject must not be the
    // doomed one, or the second stage's survivor read would raise too.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let rows: Vec<_> = params.assignments.iter().collect();
    assert!(rows.len() > 1);
    let (_doomed_period, doomed, _) = *rows.last().expect("the example has rows");
    let (_survivor_period, survivor, _) = rows[0];
    assert_ne!(survivor, doomed);

    run_stages(
        &[
            include_str!("scripts/assignments_stale_before.py"),
            include_str!("scripts/assignments_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            document_of(globals)
                .borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Subjects(
                        collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed),
                    ),
                )
                .expect("a subject of the example is removable");
        },
    );

    // What happened to the document is asserted by the second stage's script:
    // the dead address raises, the survivor reads exactly as before, and the
    // walk shows the doomed subject's rows and only them gone. That the
    // removal really clears the rows is the cascade's own contract, pinned by
    // the ops crate's tests (`ops/src/subjects.rs`).

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The incompatibilities read back, window by window
///
/// The script walks `doc.incompats` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the names, the
/// subjects, and every busy window of every incompatibility: the day, the time
/// and the duration of the model's `SlotWithDuration`. The windows also pin the
/// read half of the `TimeSlot` value: the `from_model` conversion of §2.6.
///
/// The example carries six incompatibilities across two subjects, one with a
/// single busy window, all bound to no week pattern — enough to pin the walk,
/// every field, and the `None` shape of `week_pattern`. An incompatibility
/// that carries a pattern stays out of this commit's tests: no fixture has
/// one, and the `Some` shape needs a synthetic document, which commit 13
/// builds.
#[test]
fn the_incompats_read_back_slot_by_slot() {
    let dir = workspace("incompats");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/incompats.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The example is only worth reading if it has something to say: several
    // incompatibilities, on more than one subject, one of them with more than
    // a single busy window — and all of them without a week pattern, which is
    // the shape the script's `None` assertions stand on.
    let incompats: Vec<_> = params.incompats.incompat_map.iter().collect();
    assert_eq!(
        incompats.len(),
        6,
        "the example holds six incompatibilities"
    );
    let subjects: BTreeSet<_> = incompats
        .iter()
        .map(|(_id, incompat)| incompat.subject_id)
        .collect();
    assert!(subjects.len() > 1);
    assert!(
        incompats
            .iter()
            .any(|(_id, incompat)| incompat.slots.len() > 1)
    );
    assert!(
        incompats
            .iter()
            .all(|(_id, incompat)| incompat.week_pattern_id.is_none())
    );

    assert_eq!(
        global::<Vec<String>>(&globals, "incompat_names"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "incompat_subject_names"),
        incompats
            .iter()
            .map(|(_id, incompat)| params
                .subjects
                .ordered_subject_list
                .get(&incompat.subject_id)
                .expect("an incompat names a live subject")
                .parameters
                .name
                .clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<(String, chrono::NaiveTime, u32)>>>(&globals, "incompat_slots"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat
                .slots
                .iter()
                .map(|slot| (
                    weekday_name(slot.start().weekday).to_owned(),
                    *slot.start().start_time.inner(),
                    slot.duration().get().get(),
                ))
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<u32>>(&globals, "minimum_free_slots"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat.minimum_free_slots.get())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "week_pattern_present"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat.week_pattern_id.is_some())
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// Constructing a `TimeSlot` validates what the model's own window type does
///
/// The whole point of a leaf value (§2.6): a script names the window it
/// expects and compares it, and a window the model would refuse to build
/// refuses to exist here too. The script builds the valid shapes — the plain
/// window, and the one ending exactly at midnight, which the model's
/// `SlotWithDuration::new` accepts — and asks for each refusal the model
/// knows: a zero-minute duration, a start time with seconds or microseconds,
/// and a window that crosses midnight into the next day.
#[test]
fn a_time_slot_refuses_what_the_model_refuses() {
    run(include_str!("scripts/time_slot.py"), |_| Ok(()));
}

/// A document written here rather than copied, holding both group-list shapes
///
/// The example has only prefilled lists and only unnamed groups, so the
/// automatic shape — `.groups = None`, a real exclusion set — and the named
/// half of `group_name` need a document of their own: one automatic list with
/// non-empty exclusions, one prefilled list whose groups carry names, and
/// associations reaching both. It is built as an `InnerData` through the
/// sealed types' own constructors — `GroupList::new` enforces the two
/// value-internal invariants (prefill count matching the names, no student in
/// two groups) — and passed through `Data::from_inner_data`, so a fixture that
/// breaks an invariant fails here rather than halfway through the script
/// (`docs/python/handle_api.md` §6.2).
fn group_lists_document(path: &Path) {
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
    };
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::{
        Data, GroupListId, InnerData, PeriodId, StudentId, Subject, SubjectId,
        SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let student = |n: u64| unsafe { StudentId::new(n) };
    let group_list = |n: u64| unsafe { GroupListId::new(n) };

    let periods = vec![period(1), period(2)];

    // Both subjects run colles and exclude no period, so every association the
    // fixture stores is one the loader accepts.
    let named_subject = |name: &str| Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range((2, 3)),
                groups_per_interrogation: nonzero_range((1, 1)),
                duration: collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(1).expect("one is not zero"),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    };
    let subjects = vec![
        (subject(11), named_subject("Sortilèges")),
        (subject(12), named_subject("Métamorphose")),
    ];

    let students = vec![
        (
            student(31),
            Student {
                desc: person("Harry", "Potter", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(32),
            Student {
                desc: person("Hermione", "Granger", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(33),
            Student {
                desc: person("Ron", "Weasley", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(34),
            Student {
                desc: person("Neville", "Londubat", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(35),
            Student {
                desc: person("Luna", "Lovegood", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
    ];

    // The non-empty group name, reached without naming its crate: the field
    // says what the conversion lands in, so the fixture needs no dependency of
    // its own to build one.
    let named = |text: &str| {
        text.to_owned()
            .try_into()
            .expect("the fixture's group names are not empty")
    };

    // The automatic list, with one excluded student — the shape the example
    // never shows.
    let automatic = GroupList::new(
        GroupListParameters {
            name: "Automatique".to_owned(),
            students_per_group: nonzero_range((1, 2)),
            group_names: vec![None, None, None, None],
        },
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::from([student(33)]),
        },
    )
    .expect("an automatic list is always internally consistent");

    // The prefilled list with named groups — the shape the example never
    // shows. Ron sits alone in the unnamed middle group, and Luna in no group
    // at all, which is a prefilled list's privilege.
    let prefilled = GroupList::new(
        GroupListParameters {
            name: "Maisons".to_owned(),
            students_per_group: nonzero_range((2, 3)),
            group_names: vec![Some(named("Aurore")), None, Some(named("Serdaigle"))],
        },
        GroupListFilling::Prefilled {
            groups: vec![
                PrefilledGroup {
                    students: BTreeSet::from([student(31), student(32)]),
                },
                PrefilledGroup {
                    students: BTreeSet::from([student(33)]),
                },
                PrefilledGroup {
                    students: BTreeSet::from([student(34)]),
                },
            ],
        },
    )
    .expect("the prefilled groups match the names and share no student");

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };
    let student_count = students.len();
    inner_data.params.students = Students {
        student_map: students.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.students.student_map.len(),
        student_count,
        "the fixture names each student once"
    );

    // Three associations on four possible pairs: the automatic list serves
    // two of them and the prefilled one the third, so the hop is pinned on
    // both shapes and the pair (1, 12) stays unassociated.
    inner_data.params.group_lists = GroupLists {
        group_list_map: [(group_list(51), automatic), (group_list(52), prefilled)]
            .into_iter()
            .collect(),
        subjects_associations: [
            ((period(1), subject(11)), group_list(51)),
            ((period(2), subject(11)), group_list(52)),
            ((period(2), subject(12)), group_list(51)),
        ]
        .into_iter()
        .collect(),
    };

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The group lists read back, list by list
///
/// The script walks `doc.group_lists` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the names, the
/// ranges, the raw group names, and the prefilled groups themselves, one
/// frozenset per group with the students the file puts in them.
///
/// The example carries two prefilled lists with every group unnamed and
/// eighteen associations across three periods and eight subjects, which pins
/// the collection protocol, the « Groupe N » fallback of `group_name`, and
/// both association reads — the total `association_for`, with an absent pair
/// and a foreign reference among its answers, and the stored rows of
/// `associations()`, in key order. What it cannot show — the automatic shape
/// and a named group — is
/// [the_two_filling_shapes_read_side_by_side]'s document.
#[test]
fn the_group_lists_read_back_list_by_list() {
    let dir = workspace("group-lists");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/group_lists.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The example is only worth reading if it has something to say: several
    // group lists, all prefilled and all unnamed (which is what makes the
    // fallback run all the way down), and an absent pair among its
    // associations.
    let lists: Vec<_> = params.group_lists.group_list_map.iter().collect();
    assert!(lists.len() > 1);
    assert!(
        lists
            .iter()
            .all(|(_id, group_list)| group_list.is_prefilled())
    );
    assert!(lists.iter().all(|(_id, group_list)| {
        group_list
            .params()
            .group_names
            .iter()
            .all(|name| name.is_none())
    }));
    let rows: Vec<_> = params.group_lists.subjects_associations.iter().collect();
    let periods: Vec<_> = params.periods.period_ids().collect();
    let subjects: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    let stored: BTreeSet<_> = rows
        .iter()
        .map(|((period, subject), _group_list)| (*period, *subject))
        .collect();
    assert!(
        periods
            .iter()
            .flat_map(|period| subjects.iter().map(move |subject| (*period, *subject)))
            .any(|key| !stored.contains(&key))
    );

    let bounds = |range: &collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32>| {
        (range.start().get(), range.end().get())
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "gl_names"),
        lists
            .iter()
            .map(|(_id, group_list)| group_list.params().name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "gl_students_per_group"),
        lists
            .iter()
            .map(|(_id, group_list)| bounds(&group_list.params().students_per_group))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "gl_group_counts"),
        lists
            .iter()
            .map(|(_id, group_list)| group_list.params().group_names.len())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<Option<String>>>>(&globals, "gl_group_names"),
        lists
            .iter()
            .map(|(_id, group_list)| group_list
                .params()
                .group_names
                .iter()
                .map(|name| name.as_ref().map(|name| name.to_string()))
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "gl_is_prefilled"),
        lists
            .iter()
            .map(|(_id, group_list)| group_list.is_prefilled())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<Vec<String>>>>(&globals, "gl_group_members"),
        lists
            .iter()
            .map(|(_id, group_list)| match group_list.filling() {
                collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                    groups,
                } => groups
                    .iter()
                    .map(|group| {
                        let mut names: Vec<_> = group
                            .students
                            .iter()
                            .map(|student| {
                                params
                                    .students
                                    .student_map
                                    .get(student)
                                    .expect("a prefilled group names a live student")
                                    .desc
                                    .surname
                                    .clone()
                            })
                            .collect();
                        names.sort();
                        names
                    })
                    .collect(),
                collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    ..
                } => Vec::new(),
            })
            .collect::<Vec<_>>()
    );

    // The fallback is the application's own — « Groupe 3 » for the third
    // group, the number always showing (`gtk4/src/editor/colloscope.rs`) —
    // and the repr names the list with the id the file really holds.
    use collomatique_state::ids::Id as _;
    let (first_id, first) = lists[0];
    assert_eq!(
        global::<String>(&globals, "first_repr"),
        format!(
            "<GroupList #{} '{}'>",
            first_id.inner(),
            first.params().name
        )
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "fallback_names"),
        lists
            .iter()
            .flat_map(|(_id, group_list)| {
                group_list
                    .params()
                    .group_names
                    .iter()
                    .enumerate()
                    .map(|(index, name)| match name {
                        Some(name) => name.to_string(),
                        None => format!("Groupe {}", index + 1),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    );

    assert_eq!(
        global::<Vec<usize>>(&globals, "row_period_indices"),
        rows.iter()
            .map(|((period, _subject), _group_list)| periods
                .iter()
                .position(|id| id == period)
                .expect("a row names a live period"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "row_subject_indices"),
        rows.iter()
            .map(|((_period, subject), _group_list)| subjects
                .iter()
                .position(|id| id == subject)
                .expect("a row names a live subject"))
            .collect::<Vec<_>>()
    );
    let list_ids: Vec<_> = params.group_lists.group_list_map.keys().collect();
    assert_eq!(
        global::<Vec<usize>>(&globals, "row_group_positions"),
        rows.iter()
            .map(|((_period, _subject), group_list)| list_ids
                .iter()
                .position(|id| id == *group_list)
                .expect("a row names a live group list"))
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The two filling shapes read side by side
///
/// The script reads [group_lists_document]'s two lists and leaves what it saw;
/// rust compares it with the same document read straight from the model. What
/// is pinned is the `None`-for-inapplicable rule — an automatic list answers
/// `.groups = None` and a real exclusion set, a prefilled one the groups and
/// `.excluded_students = None` — and the named half of `group_name`, which the
/// example's all-unnamed lists cannot show. The associations reach both
/// shapes, because the hop is not prefilled-only.
#[test]
fn the_two_filling_shapes_read_side_by_side() {
    let dir = workspace("group-lists-filling");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    let globals = run(include_str!("scripts/group_lists_filling.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let lists: Vec<_> = params.group_lists.group_list_map.iter().collect();
    assert_eq!(lists.len(), 2);
    let (automatic_id, automatic) = lists
        .iter()
        .find(|(_id, group_list)| !group_list.is_prefilled())
        .expect("the fixture holds an automatic list");
    let (_prefilled_id, prefilled) = lists
        .iter()
        .find(|(_id, group_list)| group_list.is_prefilled())
        .expect("the fixture holds a prefilled list");

    // The automatic list's exclusions, read from the model the way the script
    // reads them from python: the set, sorted by surname.
    let excluded: std::collections::BTreeSet<_> = match automatic.filling() {
        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
            excluded_students,
        } => excluded_students.clone(),
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { .. } => {
            unreachable!("the list was picked as automatic")
        }
    };
    let mut expected_excluded: Vec<_> = excluded
        .iter()
        .map(|student| {
            params
                .students
                .student_map
                .get(student)
                .expect("an excluded student is a live one")
                .desc
                .surname
                .clone()
        })
        .collect();
    expected_excluded.sort();
    assert_eq!(
        global::<Vec<String>>(&globals, "excluded_surnames"),
        expected_excluded
    );

    // The prefilled groups, one frozenset per group, in group order.
    let expected_members: Vec<Vec<String>> = match prefilled.filling() {
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
            groups
                .iter()
                .map(|group| {
                    let mut names: Vec<_> = group
                        .students
                        .iter()
                        .map(|student| {
                            params
                                .students
                                .student_map
                                .get(student)
                                .expect("a prefilled group names a live student")
                                .desc
                                .surname
                                .clone()
                        })
                        .collect();
                    names.sort();
                    names
                })
                .collect()
        }
        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic { .. } => {
            unreachable!("the list was picked as prefilled")
        }
    };
    assert_eq!(
        global::<Vec<Vec<String>>>(&globals, "prefilled_members"),
        expected_members
    );

    // group_name reads the stored name where there is one, and the fallback
    // where there is none — the exact strings, in group order.
    assert_eq!(
        global::<Vec<String>>(&globals, "shown_names"),
        prefilled
            .params()
            .group_names
            .iter()
            .enumerate()
            .map(|(index, name)| match name {
                Some(name) => name.to_string(),
                None => format!("Groupe {}", index + 1),
            })
            .collect::<Vec<_>>()
    );

    // The two lists' own fields.
    assert_eq!(
        global::<String>(&globals, "automatic_name"),
        automatic.params().name.clone()
    );
    assert_eq!(
        global::<String>(&globals, "prefilled_name"),
        prefilled.params().name.clone()
    );
    let bounds = |range: &collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32>| {
        (range.start().get(), range.end().get())
    };
    assert_eq!(
        global::<(u32, u32)>(&globals, "automatic_students_per_group"),
        bounds(&automatic.params().students_per_group)
    );
    assert_eq!(
        global::<(u32, u32)>(&globals, "prefilled_students_per_group"),
        bounds(&prefilled.params().students_per_group)
    );
    assert_eq!(
        global::<usize>(&globals, "automatic_group_count"),
        automatic.params().group_names.len()
    );
    assert_eq!(
        global::<usize>(&globals, "prefilled_group_count"),
        prefilled.params().group_names.len()
    );

    // Three associations on four possible pairs, both shapes served — the
    // count is the fixture's own, and it is what the script's
    // both-kinds-are-served assertions stand on.
    let rows: Vec<_> = params.group_lists.subjects_associations.iter().collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(global::<usize>(&globals, "row_count"), rows.len());
    assert!(
        rows.iter()
            .any(|((_period, _subject), group_list)| **group_list == *automatic_id)
    );
    assert!(
        rows.iter()
            .any(|((_period, _subject), group_list)| **group_list != *automatic_id)
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document written here rather than copied, holding pairing rules
///
/// The example has no subject pairing rules at all — its only pairings are the
/// two slot ones — so the shapes a rule can take need a document of their own:
/// one soft rule with a period excluded and one strict rule excluding nothing,
/// covering both `should_have` polarities on each side. It is built as an
/// `InnerData` through the sealed types' own constructors — `PairingRule::new`
/// enforces the one value-internal invariant (distinct subjects in the two
/// parts) — and passed through `Data::from_inner_data`, so a fixture that
/// breaks an invariant fails here rather than halfway through the script
/// (`docs/python/handle_api.md` §6.2).
fn pairings_document(path: &Path) {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::pairings::{PairingRule, Pairings, RulePart};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::{
        Data, InnerData, PairingRuleId, PeriodId, Subject, SubjectId,
        SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let periods = vec![period(1), period(2)];

    // Both subjects run colles — a rule naming a subject without
    // interrogations is vacuous, and the invariant gate says so — and none
    // excludes a period, so every exclusion the fixture stores is a live one.
    let named_subject = |name: &str| Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range((2, 3)),
                groups_per_interrogation: nonzero_range((1, 1)),
                duration: collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(1).expect("one is not zero"),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    };
    let subjects = vec![
        (subject(11), named_subject("Sortilèges")),
        (subject(12), named_subject("Métamorphose")),
    ];

    // Both `should_have` polarities on each side, soft both ways, one rule
    // excluding a period and one excluding none.
    let rules = vec![
        (
            unsafe { PairingRuleId::new(61) },
            PairingRule::new(
                RulePart {
                    subject_id: subject(11),
                    should_have: true,
                },
                RulePart {
                    subject_id: subject(12),
                    should_have: false,
                },
                BTreeSet::from([period(2)]),
                true,
            )
            .expect("the antecedent and the consequent name different subjects"),
        ),
        (
            unsafe { PairingRuleId::new(62) },
            PairingRule::new(
                RulePart {
                    subject_id: subject(12),
                    should_have: false,
                },
                RulePart {
                    subject_id: subject(11),
                    should_have: true,
                },
                BTreeSet::new(),
                false,
            )
            .expect("the antecedent and the consequent name different subjects"),
        ),
    ];

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };
    // An id-keyed table takes the last of a duplicated id without a word, so
    // the count is checked by hand: a fixture that named a rule twice would
    // otherwise quietly ship one fewer than the script is about to read.
    let rule_count = rules.len();
    inner_data.params.pairings = Pairings {
        pairing_rule_map: rules.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.pairings.pairing_rule_map.len(),
        rule_count,
        "the fixture names each pairing rule once"
    );

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The pairing rules read back, rule by rule
///
/// The script walks `doc.pairings` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the two parts of each
/// rule as live subject handles with their `should_have` flags, the exclusions
/// as period handles, the softness.
///
/// The reprs are pinned exactly, against `collomatique_ui_text::rendering`'s own notation: the
/// api names a rule the way the application does, like `group_name`'s
/// « Groupe N » fallback.
///
/// The example has no subject pairing rules at all — its only pairings are the
/// two slot ones, which are [the_slot_pairing_rules_read_back_rule_by_rule]'s
/// document — so this reads [pairings_document], whose two rules cover both
/// `should_have` polarities on each side, both softness values, and one
/// exclusion set.
#[test]
fn the_pairing_rules_read_back_rule_by_rule() {
    let dir = workspace("pairings");
    let source = dir.join("pairings.collomatique");
    pairings_document(&source);

    let globals = run(include_str!("scripts/pairings.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The fixture is only worth reading if it has something to say: two rules,
    // both polarities on each side, soft both ways, one exclusion set empty
    // and one not.
    let rules: Vec<_> = params.pairings.pairing_rule_map.iter().collect();
    assert_eq!(rules.len(), 2);
    assert!(rules.iter().any(|(_id, rule)| rule.soft()));
    assert!(rules.iter().any(|(_id, rule)| !rule.soft()));
    assert!(
        rules
            .iter()
            .any(|(_id, rule)| rule.excluded_periods().is_empty())
    );
    assert!(
        rules
            .iter()
            .any(|(_id, rule)| !rule.excluded_periods().is_empty())
    );

    let subject_name = |id: &collomatique_state_colloscopes::SubjectId| {
        params
            .subjects
            .ordered_subject_list
            .get(id)
            .expect("a rule names a live subject")
            .parameters
            .name
            .clone()
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "antecedent_subject_names"),
        rules
            .iter()
            .map(|(_id, rule)| subject_name(&rule.antecedent().subject_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "consequent_subject_names"),
        rules
            .iter()
            .map(|(_id, rule)| subject_name(&rule.consequent().subject_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "antecedent_should_have"),
        rules
            .iter()
            .map(|(_id, rule)| rule.antecedent().should_have)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "consequent_should_have"),
        rules
            .iter()
            .map(|(_id, rule)| rule.consequent().should_have)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "softs"),
        rules
            .iter()
            .map(|(_id, rule)| rule.soft())
            .collect::<Vec<_>>()
    );

    // The exclusions, read through the display positions of the periods in
    // them — a period named by its place in the document's own order.
    let period_ids: Vec<_> = params.periods.period_ids().collect();
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "excluded_period_indices"),
        rules
            .iter()
            .map(|(_id, rule)| {
                let mut indices: Vec<_> = rule
                    .excluded_periods()
                    .iter()
                    .map(|period| {
                        period_ids
                            .iter()
                            .position(|id| id == period)
                            .expect("an excluded period is a live one")
                    })
                    .collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );

    // The reprs name the rules the way the application does: the exact
    // `collomatique_ui_text::rendering` notation, quoted the way the other reprs quote names.
    use collomatique_state::ids::Id as _;
    let (first_id, first) = rules[0];
    assert_eq!(
        global::<String>(&globals, "first_repr"),
        format!(
            "<PairingRule #{} '{}'>",
            first_id.inner(),
            render_pairing_rule(&params.subjects, &params.pairings, first_id)
                .expect("the first rule's subjects are live"),
        )
    );
    assert_eq!(
        global::<String>(&globals, "first_side_repr"),
        format!(
            "<PairingRuleSide #{} (antécédent) '{}'>",
            first_id.inner(),
            render_subject(&params.subjects, first.antecedent().subject_id)
                .expect("the first rule's antecedent subject is live"),
        )
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The slot pairing rules read back, rule by rule
///
/// The script walks `doc.slot_pairings` and leaves what it saw; rust compares
/// it with the same document read straight from the model — the two parts of
/// each rule as live slot handles with their `should_have` flags, the empty
/// exclusions, the softness. The reprs are pinned exactly against
/// `collomatique_ui_text::rendering`'s notation, like the subject-level rules'.
///
/// The example carries two slot pairing rules, both strict, both excluding no
/// period, with a used antecedent and an unused consequent — the shape the
/// script's assertions stand on. What it does not carry is a subject pairing
/// rule, which is why [the_pairing_rules_read_back_rule_by_rule] reads a
/// document of its own.
#[test]
fn the_slot_pairing_rules_read_back_rule_by_rule() {
    let dir = workspace("slot-pairings");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/slot_pairings.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The example is only worth reading if it has something to say: two rules,
    // both strict, both excluding no period, a used antecedent against an
    // unused consequent.
    let rules: Vec<_> = params.slot_pairings.slot_pairing_rule_map.iter().collect();
    assert_eq!(rules.len(), 2, "the example holds two slot pairing rules");
    assert!(rules.iter().all(|(_id, rule)| !rule.soft()));
    assert!(
        rules
            .iter()
            .all(|(_id, rule)| rule.excluded_periods().is_empty())
    );
    assert!(
        rules
            .iter()
            .all(|(_id, rule)| rule.antecedent().should_have)
    );
    assert!(
        rules
            .iter()
            .all(|(_id, rule)| !rule.consequent().should_have)
    );

    assert_eq!(
        global::<Vec<bool>>(&globals, "antecedent_should_have"),
        rules
            .iter()
            .map(|(_id, rule)| rule.antecedent().should_have)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "consequent_should_have"),
        rules
            .iter()
            .map(|(_id, rule)| rule.consequent().should_have)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "softs"),
        rules
            .iter()
            .map(|(_id, rule)| rule.soft())
            .collect::<Vec<_>>()
    );

    // The antecedent slots, named by their start time — that is what pins the
    // part to a slot of the document rather than to a number that happens to
    // be there.
    let start_time = |id: collomatique_state_colloscopes::SlotId| {
        *params
            .slots
            .find_slot(id)
            .expect("a rule names a live slot")
            .start_time
            .start_time
            .inner()
    };
    assert_eq!(
        global::<Vec<chrono::NaiveTime>>(&globals, "antecedent_slot_start_times"),
        rules
            .iter()
            .map(|(_id, rule)| start_time(rule.antecedent().slot_id))
            .collect::<Vec<_>>()
    );

    // Both slots of a rule belong to one subject, and the rule is read under
    // that subject's name.
    let subject_name = |id: &collomatique_state_colloscopes::SubjectId| {
        params
            .subjects
            .ordered_subject_list
            .get(id)
            .expect("a rule names a live subject")
            .parameters
            .name
            .clone()
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "rule_subject_names"),
        rules
            .iter()
            .map(|(_id, rule)| {
                let (subject_id, _slot) = params
                    .slots
                    .find_slot_with_subject(rule.antecedent().slot_id)
                    .expect("a rule names a live slot");
                subject_name(&subject_id)
            })
            .collect::<Vec<_>>()
    );

    use collomatique_state::ids::Id as _;
    let (first_id, first) = rules[0];
    assert_eq!(
        global::<String>(&globals, "first_repr"),
        format!(
            "<SlotPairingRule #{} '{}'>",
            first_id.inner(),
            render_slot_pairing_rule(
                &params.subjects,
                &params.teachers,
                &params.slots,
                &params.slot_pairings,
                first_id,
            )
            .expect("the first rule's slots are live"),
        )
    );
    assert_eq!(
        global::<String>(&globals, "first_side_repr"),
        format!(
            "<SlotPairingRuleSide #{} (antécédent) '{}'>",
            first_id.inner(),
            render_slot_in_subject(&params.teachers, &params.slots, first.antecedent().slot_id)
                .expect("the first rule's antecedent slot is live"),
        )
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed pairing rule takes its handle and both its sides down, loudly
///
/// The mutation cannot come from the script — the read surface ships no removes
/// — so it comes from rust, between the two halves: the fixture's first rule
/// goes, and what the script's first half held of it — the handle and the two
/// side views — must all say so.
///
/// The second half is where the side views' contract is pinned: they are bound
/// to `(document, rule_id, side)`, so their `==` and `hash` keep working and
/// stay distinct once the rule is gone, while every reading attribute raises.
#[test]
fn a_removed_pairing_rule_takes_its_sides_with_it() {
    let dir = workspace("pairings-stale");
    let source = dir.join("pairings.collomatique");
    pairings_document(&source);

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same rule the script is holding.
    let doomed = reload(&source)
        .get_inner_data()
        .params
        .pairings
        .pairing_rule_map
        .keys()
        .next()
        .expect("the fixture has pairing rules");

    run_stages(
        &[
            include_str!("scripts/pairings_stale_before.py"),
            include_str!("scripts/pairings_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            document_of(globals)
                .borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Pairings(
                        collomatique_ops::PairingsUpdateOp::DeletePairingRule(doomed),
                    ),
                )
                .expect("the fixture's first pairing rule is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed slot pairing rule takes its handle and both its sides down, loudly
///
/// The mutation cannot come from the script — the read surface ships no removes
/// — so it comes from rust, between the two halves: the example's second slot
/// pairing rule goes, and what the script's first half held of it — the handle
/// and the two side views — must all say so.
#[test]
fn a_removed_slot_pairing_rule_takes_its_sides_with_it() {
    let dir = workspace("slot-pairings-stale");
    let source = example_copy(&dir, "source.collomatique");

    // The second of the example's two slot pairing rules, read from the file:
    // ids are stored, so this copy names the same rule the script is holding.
    let doomed = reload(&source)
        .get_inner_data()
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .keys()
        .nth(1)
        .expect("the example holds two slot pairing rules");

    run_stages(
        &[
            include_str!("scripts/slot_pairings_stale_before.py"),
            include_str!("scripts/slot_pairings_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            document_of(globals)
                .borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::SlotPairings(
                        collomatique_ops::SlotPairingsUpdateOp::DeleteSlotPairingRule(doomed),
                    ),
                )
                .expect("the example's second slot pairing rule is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The settings read back, entry by entry
///
/// The script walks `doc.settings` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the global entry,
/// Hermione's override (whose every field is set) and the resolved view over
/// her, Harry's resolved view inheriting the global entry, and the stored
/// rows in id order.
///
/// The example is worth reading here because it holds exactly one override
/// and one student without one, which is the whole resolution shape. What it
/// does not hold is an override with a `None` field masking a set global
/// limit — that is
/// [an_override_appearing_and_vanishing_tracks_through_limits_for]'s story,
/// whose masking entry is installed between stages.
#[test]
fn the_settings_read_back_entry_by_entry() {
    let dir = workspace("settings");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/settings.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let settings = &params.settings;

    let global_limits = &settings.global;
    let global_min = global_limits
        .interrogations_per_week_min
        .as_ref()
        .expect("the global entry sets a minimum");
    let global_max = global_limits
        .interrogations_per_week_max
        .as_ref()
        .expect("the global entry sets a maximum");
    let global_day = global_limits
        .max_interrogations_per_day
        .as_ref()
        .expect("the global entry sets a per-day limit");

    assert_eq!(
        global::<Vec<u32>>(&globals, "global_values"),
        vec![global_min.value, global_max.value, global_day.value.get()]
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "global_strict"),
        vec![!global_min.soft, !global_max.soft, !global_day.soft]
    );
    assert_eq!(
        global::<String>(&globals, "global_repr"),
        format!(
            "<Limits (global) interrogations_per_week_min={} interrogations_per_week_max={} \
             max_interrogations_per_day={}>",
            global_min.value,
            global_max.value,
            global_day.value.get(),
        )
    );

    // Hermione is the one student with an override; her resolved limits are
    // the override entry itself, and so is the stored row.
    let hermione = params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Granger")
        .expect("the example has Hermione")
        .0;
    let override_limits = settings
        .students
        .get(&hermione)
        .expect("Hermione has an override");
    let values_of = |limits: &collomatique_state_colloscopes::settings::Limits| {
        vec![
            limits
                .interrogations_per_week_min
                .as_ref()
                .expect("the entry sets a minimum")
                .value,
            limits
                .interrogations_per_week_max
                .as_ref()
                .expect("the entry sets a maximum")
                .value,
            limits
                .max_interrogations_per_day
                .as_ref()
                .expect("the entry sets a per-day limit")
                .value
                .get(),
        ]
    };
    let strict_of = |limits: &collomatique_state_colloscopes::settings::Limits| {
        vec![
            !limits
                .interrogations_per_week_min
                .as_ref()
                .expect("the entry sets a minimum")
                .soft,
            !limits
                .interrogations_per_week_max
                .as_ref()
                .expect("the entry sets a maximum")
                .soft,
            !limits
                .max_interrogations_per_day
                .as_ref()
                .expect("the entry sets a per-day limit")
                .soft,
        ]
    };
    assert_eq!(
        global::<Vec<u32>>(&globals, "hermione_values"),
        values_of(override_limits)
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "hermione_strict"),
        strict_of(override_limits)
    );
    assert_eq!(
        global::<Vec<u32>>(&globals, "override_values"),
        values_of(override_limits)
    );
    use collomatique_state::ids::Id as _;
    assert_eq!(
        global::<String>(&globals, "hermione_repr"),
        format!(
            "<Limits #{} {} {} {}>",
            hermione.inner(),
            repr_limit(
                override_limits
                    .interrogations_per_week_min
                    .as_ref()
                    .expect("set"),
                "interrogations_per_week_min",
            ),
            repr_limit(
                override_limits
                    .interrogations_per_week_max
                    .as_ref()
                    .expect("set"),
                "interrogations_per_week_max",
            ),
            repr_nonzero_limit(
                override_limits
                    .max_interrogations_per_day
                    .as_ref()
                    .expect("set"),
                "max_interrogations_per_day",
            ),
        )
    );

    // Harry inherits the global entry: no row of his own, and the model's own
    // resolution answer.
    let harry = params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Potter")
        .expect("the example has Harry")
        .0;
    assert!(settings.students.get(&harry).is_none());
    assert_eq!(
        global::<Vec<u32>>(&globals, "harry_values"),
        values_of(settings.limits_for(harry))
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A limit as its repr writes it — the word is the attribute name, so the two
/// shapes are written out rather than given a helper that could drift apart.
fn repr_limit(
    soft: &collomatique_state_colloscopes::settings::SoftParam<u32>,
    word: &str,
) -> String {
    format!("{word}={}", soft.value)
}

/// The same, for a count the model stores non-zero
fn repr_nonzero_limit(
    soft: &collomatique_state_colloscopes::settings::SoftParam<std::num::NonZeroU32>,
    word: &str,
) -> String {
    format!("{word}={}", soft.value)
}

/// The balancing read back, entry by entry
///
/// The script walks `doc.balancing` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the global entry,
/// Métamorphose's override (which hardens a rotation the global entry does not
/// pursue at all, the whole-entry verbatim rule), a subject inheriting the
/// global entry, and the stored rows in id order.
///
/// The example is worth reading here because the three states of a rotation
/// goal — not pursued, objective, strict — all appear across its entries.
#[test]
fn the_balancing_read_back_entry_by_entry() {
    let dir = workspace("balancing");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/balancing.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let balancing = &params.balancing;
    let subjects = &params.subjects;

    let global_options = &balancing.global;
    let enforcement = |soft: bool| if soft { "OBJECTIVE" } else { "STRICT" };
    assert_eq!(
        global::<Vec<bool>>(&globals, "global_rotation_objectives"),
        vec![
            global_options
                .teacher_rotation
                .as_ref()
                .expect("pursued")
                .soft,
            global_options.slot_rotation.as_ref().expect("pursued").soft,
        ]
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "global_bools"),
        vec![
            global_options.year_teacher_rotation,
            global_options.period_teacher_rotation
        ]
    );
    assert_eq!(
        global::<String>(&globals, "global_repr"),
        format!(
            "<BalancingOptions (global) teacher_rotation=Enforcement.{} \
             slot_rotation=Enforcement.{} avoid_twice_in_a_row=None \
             year_teacher_rotation={} period_teacher_rotation={}>",
            enforcement(
                global_options
                    .teacher_rotation
                    .as_ref()
                    .expect("pursued")
                    .soft
            ),
            enforcement(global_options.slot_rotation.as_ref().expect("pursued").soft),
            global_options.year_teacher_rotation,
            global_options.period_teacher_rotation,
        )
    );

    // Métamorphose's override wins verbatim: its avoid_twice_in_a_row is
    // strict where the global entry does not pursue the goal at all, and its
    // year switch is on where the global one is off.
    let metamorphose = subjects
        .ordered_subject_list
        .iter()
        .find(|(_id, subject)| subject.parameters.name == "Métamorphose")
        .expect("the example has Métamorphose")
        .0;
    let options = balancing.options_for(metamorphose);
    let objective = |soft: &Option<collomatique_state_colloscopes::settings::SoftParam<()>>| {
        soft.as_ref().is_some_and(|soft| soft.soft)
    };
    assert_eq!(
        global::<Vec<bool>>(&globals, "metamorphose_objectives"),
        vec![
            objective(&options.teacher_rotation),
            objective(&options.slot_rotation),
            objective(&options.avoid_twice_in_a_row),
        ]
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "metamorphose_bools"),
        vec![
            options.year_teacher_rotation,
            options.period_teacher_rotation
        ]
    );
    use collomatique_state::ids::Id as _;
    assert_eq!(
        global::<String>(&globals, "metamorphose_repr"),
        format!(
            "<BalancingOptions #{} teacher_rotation=Enforcement.{} \
             slot_rotation=Enforcement.{} avoid_twice_in_a_row=Enforcement.{} \
             year_teacher_rotation={} period_teacher_rotation={}>",
            metamorphose.inner(),
            enforcement(options.teacher_rotation.as_ref().expect("pursued").soft),
            enforcement(options.slot_rotation.as_ref().expect("pursued").soft),
            enforcement(options.avoid_twice_in_a_row.as_ref().expect("pursued").soft),
            options.year_teacher_rotation,
            options.period_teacher_rotation,
        )
    );

    // The stored rows, in id order: the overridden subjects named as the
    // application names them.
    assert_eq!(
        global::<Vec<String>>(&globals, "override_subject_names"),
        balancing
            .subjects
            .keys()
            .map(|subject| subjects
                .find_subject(subject)
                .expect("an overridden subject is a live one")
                .parameters
                .name
                .clone())
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// An override appearing and vanishing tracks through a held `limits_for` view
///
/// The mutation cannot come from the script — the read surface ships no writes
/// — so it comes from rust, between the three halves: an override for Harry is
/// installed and then removed. The same [Limits] view the first half held must
/// follow both changes — reading the override while it stands, its `None`
/// fields masking the set global limits (the verbatim whole-entry rule the
/// model's own tests pin), and falling back to the global entry when it is
/// gone. The raw view minted while the override stood is bound to its entry,
/// so it dies with it, loudly.
#[test]
fn an_override_appearing_and_vanishing_tracks_through_limits_for() {
    let dir = workspace("settings-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same student the script is holding.
    let harry = reload(&source)
        .get_inner_data()
        .params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Potter")
        .expect("the example has Harry")
        .0;

    // The whole-entry override installed between the first two stages: a
    // minimum of four a week, objective rather than strict, and the two other
    // fields unset — which is the point, they must disable the set global
    // limits rather than inherit them.
    let override_limits = collomatique_state_colloscopes::settings::Limits {
        interrogations_per_week_min: Some(collomatique_state_colloscopes::settings::SoftParam {
            soft: true,
            value: 4,
        }),
        interrogations_per_week_max: None,
        max_interrogations_per_day: None,
    };

    let mut stage = 0;
    run_stages(
        &[
            include_str!("scripts/settings_stale_before.py"),
            include_str!("scripts/settings_stale_override.py"),
            include_str!("scripts/settings_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            let op = match stage {
                0 => collomatique_ops::SettingsUpdateOp::UpdateStudentLimits(
                    harry,
                    override_limits.clone(),
                ),
                _ => collomatique_ops::SettingsUpdateOp::RemoveStudentLimits(harry),
            };
            stage += 1;

            document_of(globals)
                .borrow_mut(py)
                .update(py, collomatique_ops::UpdateOp::Settings(op))
                .expect("Harry's override is settable and removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// Removing a student takes the limits views of them down with it
///
/// The mutation comes from rust, between the two halves: Hermione goes, and
/// the cascade takes her override with her. Both views the first half held —
/// the resolved one, bound to the student, and the raw one, bound to the
/// entry — must say so loudly, and a fresh ask about the dead student is a
/// stale argument, not the model's forgiving answer.
#[test]
fn removing_a_student_takes_its_limits_views_with_it() {
    let dir = workspace("settings-student-gone");
    let source = example_copy(&dir, "source.collomatique");

    let hermione = reload(&source)
        .get_inner_data()
        .params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Granger")
        .expect("the example has Hermione")
        .0;

    run_stages(
        &[
            include_str!("scripts/settings_student_gone_before.py"),
            include_str!("scripts/settings_student_gone_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            document_of(globals)
                .borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Students(
                        collomatique_ops::StudentsUpdateOp::DeleteStudent(hermione),
                    ),
                )
                .expect("Hermione is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// Removing a balancing override stales only the raw view
///
/// The mutation comes from rust, between the two halves: Métamorphose's
/// balancing override goes. The resolved view the first half held re-resolves
/// to the global entry — `options_for` is live in the strong sense, like
/// `limits_for` — while the raw view dies with its entry, loudly.
#[test]
fn removing_a_balancing_override_stales_only_the_raw_view() {
    let dir = workspace("balancing-stale");
    let source = example_copy(&dir, "source.collomatique");

    let metamorphose = reload(&source)
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .find(|(_id, subject)| subject.parameters.name == "Métamorphose")
        .expect("the example has Métamorphose")
        .0;

    run_stages(
        &[
            include_str!("scripts/balancing_stale_before.py"),
            include_str!("scripts/balancing_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            document_of(globals)
                .borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Balancing(
                        collomatique_ops::BalancingUpdateOp::RemoveSubjectOptions(metamorphose),
                    ),
                )
                .expect("Métamorphose's balancing override is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document written here rather than copied, holding a filled colloscope
///
/// The example was never resolved, so its colloscope has nothing to read —
/// the cells and the placements need a document of their own: two subjects
/// with slots across two periods, one automatic group list and one prefilled
/// one, a few stored cells (one of them with several groups), one cell a
/// resolution could have filled and left empty, one week switched off
/// entirely, and placements for the automatic list. It is built as an
/// `InnerData` through the sealed types' own constructors and passed through
/// `Data::from_inner_data`, so a fixture that breaks an invariant — a cell on
/// a week the slot's subject does not run, a group number past the
/// associated list's bound, a placement for an excluded student — fails here
/// rather than halfway through the script (`docs/python/handle_api.md`
/// §6.2).
fn colloscope_document(path: &Path) {
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
    };
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::slots::{Slot, Slots};
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::teachers::{Teacher, Teachers};
    use collomatique_state_colloscopes::weeks::{WeekDesc, Weeks};
    use collomatique_state_colloscopes::{
        Data, GroupListId, InnerData, PeriodId, SlotId, StudentId, Subject, SubjectId,
        SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity, TeacherId, WeekId,
    };

    // Ids nothing else in this document issues: it is written by hand from
    // end to end, so there is no issuer to keep in step with. The slots, the
    // group lists and the students are numbered nowhere near the example's
    // (137+, 166+ and 100+), so the script's foreign-handle question is a
    // clean one. The weeks are the decoder's own synthesis in walk order on
    // the other side, so their numbers have nothing to be disjoint from —
    // and the script only ever asks about weeks with handles, which name
    // their document.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let week = |n: u64| unsafe { WeekId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let teacher = |n: u64| unsafe { TeacherId::new(n) };
    let slot = |n: u64| unsafe { SlotId::new(n) };
    let student = |n: u64| unsafe { StudentId::new(n) };
    let group_list = |n: u64| unsafe { GroupListId::new(n) };

    let periods = vec![period(1), period(2)];

    // Both periods hold colles, except the fixture's last week, which is
    // switched off entirely — the impossible half of the empty-cell shape.
    let weeks = vec![
        (
            period(1),
            vec![
                (week(81), WeekDesc::new(true)),
                (week(82), WeekDesc::new(true)),
            ],
        ),
        (
            period(2),
            vec![
                (week(83), WeekDesc::new(true)),
                (week(84), WeekDesc::new(false)),
            ],
        ),
    ];

    // Both subjects run colles and exclude no period, so every cell's
    // coordinates and every association are live ones.
    let named_subject = |name: &str| Subject {
        parameters: SubjectParameters {
            name: name.to_owned(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: nonzero_range((2, 3)),
                groups_per_interrogation: nonzero_range((1, 1)),
                duration: collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(1).expect("one is not zero"),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    };
    let subjects = vec![
        (subject(11), named_subject("Sortilèges")),
        (subject(12), named_subject("Métamorphose")),
    ];

    let teachers = vec![
        (
            teacher(21),
            Teacher {
                desc: person("Minerva", "McGonagall", None, None),
                subjects: BTreeSet::from([subject(11), subject(12)]),
            },
        ),
        (
            teacher(22),
            Teacher {
                desc: person("Severus", "Rogue", None, None),
                subjects: BTreeSet::from([subject(12)]),
            },
        ),
    ];

    // One subject with two slots, one with a single one. No week pattern on
    // any of them, so only the weeks' own flags switch cells off — which is
    // what makes the grid a one-dimensional question here.
    let slot_start = |weekday, hour, minute| collomatique_time::SlotStart {
        weekday,
        start_time: collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("a clock time"),
        )
        .expect("a whole minute"),
    };
    let slots = vec![
        (
            subject(11),
            vec![
                (
                    slot(71),
                    Slot {
                        subject_id: subject(11),
                        teacher_id: teacher(21),
                        start_time: slot_start(
                            collomatique_time::Weekday(chrono::Weekday::Mon),
                            9,
                            0,
                        ),
                        extra_info: String::new(),
                        week_pattern: None,
                        cost: 0,
                    },
                ),
                (
                    slot(72),
                    Slot {
                        subject_id: subject(11),
                        teacher_id: teacher(21),
                        start_time: slot_start(
                            collomatique_time::Weekday(chrono::Weekday::Tue),
                            10,
                            0,
                        ),
                        extra_info: String::new(),
                        week_pattern: None,
                        cost: -1,
                    },
                ),
            ],
        ),
        (
            subject(12),
            vec![(
                slot(73),
                Slot {
                    subject_id: subject(12),
                    teacher_id: teacher(22),
                    start_time: slot_start(collomatique_time::Weekday(chrono::Weekday::Wed), 14, 0),
                    extra_info: String::new(),
                    week_pattern: None,
                    cost: 0,
                },
            )],
        ),
    ];

    // No student sits a period out, so every placement's student is present
    // on every period.
    let students = vec![
        (
            student(31),
            Student {
                desc: person("Harry", "Potter", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(32),
            Student {
                desc: person("Hermione", "Granger", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(33),
            Student {
                desc: person("Ron", "Weasley", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
        (
            student(34),
            Student {
                desc: person("Neville", "Londubat", None, None),
                excluded_periods: BTreeSet::new(),
            },
        ),
    ];

    // The non-empty group name, reached without naming its crate: the field
    // says what the conversion lands in, so the fixture needs no dependency
    // of its own to build one.
    let named = |text: &str| {
        text.to_owned()
            .try_into()
            .expect("the fixture's group names are not empty")
    };

    // The automatic list the solver filled, with one excluded student — the
    // list a placements row belongs to. And the prefilled list, which never
    // appears in the colloscope: its groups are its own.
    let automatic = GroupList::new(
        GroupListParameters {
            name: "Automatique".to_owned(),
            students_per_group: nonzero_range((1, 2)),
            group_names: vec![None, None, None],
        },
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::from([student(33)]),
        },
    )
    .expect("an automatic list is always internally consistent");

    let prefilled = GroupList::new(
        GroupListParameters {
            name: "Maisons".to_owned(),
            students_per_group: nonzero_range((2, 3)),
            group_names: vec![Some(named("Aurore")), None],
        },
        GroupListFilling::Prefilled {
            groups: vec![
                PrefilledGroup {
                    students: BTreeSet::from([student(31), student(32)]),
                },
                PrefilledGroup {
                    students: BTreeSet::from([student(34)]),
                },
            ],
        },
    )
    .expect("the prefilled groups match the names and share no student");

    let mut inner_data = InnerData::default();
    inner_data.params.periods =
        collomatique_state_colloscopes::periods::Periods::from_ordered_ids(None, periods)
            .expect("the fixture names each period once");
    inner_data.params.weeks =
        Weeks::from_period_rows(weeks).expect("the fixture names each week once");
    inner_data.params.subjects = Subjects {
        ordered_subject_list: subjects
            .try_into()
            .expect("the fixture names each subject once"),
    };
    // An id-keyed table takes the last of a duplicated id without a word,
    // where the ordered lists above refuse one. So the counts are checked by
    // hand: a fixture that named a teacher or a student twice would otherwise
    // quietly ship one fewer than the script is about to read.
    let (teacher_count, student_count) = (teachers.len(), students.len());
    inner_data.params.teachers = Teachers {
        teacher_map: teachers.into_iter().collect(),
    };
    inner_data.params.students = Students {
        student_map: students.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.teachers.teacher_map.len(),
        teacher_count,
        "the fixture names each teacher once"
    );
    assert_eq!(
        inner_data.params.students.student_map.len(),
        student_count,
        "the fixture names each student once"
    );
    inner_data.params.slots =
        Slots::from_subject_rows(slots).expect("the fixture names each slot once");

    // The automatic list serves every pair a cell stands on, and the
    // prefilled one serves nothing at all.
    inner_data.params.group_lists = GroupLists {
        group_list_map: [(group_list(51), automatic), (group_list(52), prefilled)]
            .into_iter()
            .collect(),
        subjects_associations: [
            ((period(1), subject(11)), group_list(51)),
            ((period(2), subject(11)), group_list(51)),
            ((period(2), subject(12)), group_list(51)),
        ]
        .into_iter()
        .collect(),
    };

    // The colloscope itself, written through the canonical sparse writers:
    // four cells on three slots and three weeks — one of them carrying two
    // groups — and the automatic list filled. Every stored cell is possible:
    // its subject runs on the week's period, the week holds colles, and the
    // groups fit the associated list's three. The cell `(slot 71, week 82)`
    // is possible and left empty, and `(slot 71, week 84)` is impossible —
    // the two shapes of the single `None` answer.
    inner_data
        .colloscope
        .set_interrogation(slot(71), week(81), BTreeSet::from([0, 2]));
    inner_data
        .colloscope
        .set_interrogation(slot(71), week(83), BTreeSet::from([1]));
    inner_data
        .colloscope
        .set_interrogation(slot(72), week(82), BTreeSet::from([0]));
    inner_data
        .colloscope
        .set_interrogation(slot(73), week(83), BTreeSet::from([2]));
    inner_data.colloscope.set_group_list(
        group_list(51),
        [(student(31), 0), (student(32), 2), (student(34), 1)]
            .into_iter()
            .collect(),
    );

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The colloscope reads back, cell by cell
///
/// The script walks `doc.colloscope` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the stored cells
/// themselves, in key order, and the placements rows, in key order too.
///
/// The example was never resolved, so its colloscope has nothing to read —
/// the cells and the placements need a document of its own:
/// [colloscope_document]. The script does the rest on its own, because it is
/// about what python sees: the single `None` of an empty cell, the
/// placements as a read-only `mappingproxy`, the `None` of a prefilled list,
/// and the foreign-handle arguments that must raise.
#[test]
fn the_colloscope_reads_back_cell_by_cell() {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::{GroupListId, SlotId};

    let dir = workspace("colloscope");
    let source = dir.join("colloscope.collomatique");
    colloscope_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let globals = run(include_str!("scripts/colloscope.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let colloscope = &data.get_inner_data().colloscope;

    // The fixture is only worth reading if it has something to say: several
    // cells, on more than one slot and more than one week, one of them with
    // several groups — and placements for the automatic list.
    let cells: Vec<_> = colloscope.iter().collect();
    let placements: Vec<_> = colloscope.group_lists_iter().collect();
    assert!(cells.len() > 1);
    assert!(
        cells
            .iter()
            .map(|((slot, _week), _groups)| slot)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    );
    assert!(
        cells
            .iter()
            .map(|((_slot, week), _groups)| week)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    );
    assert!(cells.iter().any(|(_key, groups)| groups.len() > 1));
    assert_eq!(placements.len(), 1, "the fixture fills exactly one list");

    // The cells, named by the positions a script reads them in: the slot's
    // position within its subject, the week's global index, and the sorted
    // group numbers.
    let week_ids: Vec<_> = params.week_ids().collect();
    let expected_cells: Vec<(usize, usize, Vec<u32>)> = cells
        .iter()
        .map(|((slot, week), groups)| {
            let slot_index = params
                .slots
                .find_slot_subject_and_position(*slot)
                .expect("a stored cell names a live slot")
                .1;
            let week_index = week_ids
                .iter()
                .position(|id| id == week)
                .expect("a stored cell names a live week");
            (slot_index, week_index, groups.iter().copied().collect())
        })
        .collect();
    assert_eq!(
        global::<Vec<(usize, usize, Vec<u32>)>>(&globals, "cell_reads"),
        expected_cells
    );

    // Every stored cell is one the model could have accepted — a fixture that
    // shipped a cell the resolver could never have produced would make the
    // script's grid promises vacuous.
    assert!(
        cells
            .iter()
            .all(|((slot, week), _groups)| { params.is_interrogation_possible(*slot, *week) })
    );

    // The two empty shapes the script read: the possible cell nobody filled,
    // and the week that is switched off entirely. The slot id is the
    // fixture's own, which the file keeps; the week ids are not stored in the
    // file — the decoder re-synthesizes them in walk order — so the reloaded
    // document's own walk names the second and the last week.
    let slot71 = unsafe { SlotId::new(71) };
    let week82 = week_ids[1];
    let week84 = *week_ids.last().expect("the fixture has weeks");
    assert!(params.is_interrogation_possible(slot71, week82));
    assert!(!params.is_interrogation_possible(slot71, week84));
    assert!(colloscope.interrogation(slot71, week82).is_none());
    assert!(colloscope.interrogation(slot71, week84).is_none());
    assert_eq!(
        global::<(Option<Vec<u32>>, Option<Vec<u32>>)>(&globals, "empty_cell_reads"),
        (None, None)
    );

    // The placements, read from the model the way the script reads them from
    // python: student by surname, group by number.
    let gl51 = unsafe { GroupListId::new(51) };
    let placed = colloscope
        .group_list(gl51)
        .expect("the fixture fills its automatic list");
    let mut expected_placements: Vec<(String, u32)> = placed
        .iter()
        .map(|(student, group)| {
            (
                params
                    .students
                    .student_map
                    .get(student)
                    .expect("a placement names a live student")
                    .desc
                    .surname
                    .clone(),
                *group,
            )
        })
        .collect();
    expected_placements.sort();
    assert_eq!(
        global::<Vec<(String, u32)>>(&globals, "placement_items"),
        expected_placements
    );

    // The stored rows, in key order, each named the way the script names
    // them: the list's position in `doc.group_lists`, and the placements.
    let list_ids: Vec<_> = params.group_lists.group_list_map.keys().collect();
    let mut expected_rows: Vec<(usize, Vec<(String, u32)>)> = placements
        .iter()
        .map(|(group_list, placed)| {
            let index = list_ids
                .iter()
                .position(|id| id == group_list)
                .expect("a stored row names a live group list");
            let mut items: Vec<(String, u32)> = placed
                .iter()
                .map(|(student, group)| {
                    (
                        params
                            .students
                            .student_map
                            .get(student)
                            .expect("a placement names a live student")
                            .desc
                            .surname
                            .clone(),
                        *group,
                    )
                })
                .collect();
            items.sort();
            (index, items)
        })
        .collect();
    expected_rows.sort_by_key(|(index, _)| *index);
    assert_eq!(
        global::<Vec<(usize, Vec<(String, u32)>)>>(&globals, "group_list_rows"),
        expected_rows
    );

    // The script's foreign-handle question rests on the two documents not
    // sharing the ids of the kinds it asks about. The weeks are the decoder's
    // own synthesis in walk order, so they have nothing to be disjoint from —
    // and the script only ever asks about weeks with handles, which name
    // their document.
    let fixture = reload(&source);
    let example = reload(&other_source);
    // The model keeps no single slot table to read ids from, so the walk the
    // `doc.slots` view makes is composed here too: each subject, then its own
    // slots.
    let slots_of = |data: &Data| -> BTreeSet<_> {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .keys()
            .flat_map(|subject| {
                data.get_inner_data()
                    .params
                    .slots
                    .slots_for_subject(subject)
                    .into_iter()
                    .flatten()
                    .map(|(slot, _desc)| *slot)
            })
            .collect()
    };
    let group_lists_of = |data: &Data| -> BTreeSet<_> {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .collect()
    };
    assert!(slots_of(&fixture).is_disjoint(&slots_of(&example)));
    assert!(group_lists_of(&fixture).is_disjoint(&group_lists_of(&example)));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The real `rfd` chooser, on a machine with someone in front of it
///
/// Everything above answers the dialogs itself, so this is the only test that
/// says the `rfd` side works at all — that the runtime is one `zbus` can spawn
/// onto, and that the portal hands a path back. It cannot be answered by a test
/// runner, hence the `#[ignore]`.
///
/// It asks for them one after another on purpose. A per-dialog runtime shows
/// the first chooser and then hangs on the second, because the connection
/// `ashpd` caches outlives the runtime that was driving it — one dialog would
/// have said everything was fine. The last of them is `default_document`'s, the
/// one the module opens on the script's behalf.
#[test]
#[ignore = "opens four real file choosers: run with --ignored --nocapture, and answer them"]
fn real_file_choosers_open_one_after_another() {
    let globals = run(include_str!("scripts/real_dialog.py"), |_| Ok(()));

    for name in ["opened", "saved", "folder", "document_path"] {
        let chosen: Option<PathBuf> = Python::attach(|py| {
            globals
                .bind(py)
                .get_item(name)
                .expect("looking up a global should not fail")
                .expect("the script sets one global per dialog")
                .extract()
                .expect("a dialog answers with a path or with nothing")
        });

        // Cancelling is a perfectly good answer, and the only one available on a
        // machine where the chooser cannot be reached.
        match chosen {
            Some(path) => println!("{name}: {}", path.display()),
            None => println!("{name}: cancelled"),
        }
    }
}
