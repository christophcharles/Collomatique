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
/// raises `StaleHandleError`, the repr says `(stale)` instead of raising, and
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
