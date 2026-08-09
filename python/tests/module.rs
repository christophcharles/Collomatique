//! The `collomatique` module, exercised from real python scripts
//!
//! The scripts live in `tests/scripts/` as `.py` files rather than as string
//! literals: they are the thing under test, and a real file is what a user
//! writes. The rust side passes inputs in and reads results out through the
//! script's globals, so the assertions stay here, where a failure says
//! something useful.

use std::collections::VecDeque;
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

/// The real `rfd` chooser, on a machine with someone in front of it
///
/// Everything above answers the dialogs itself, so this is the only test that
/// says the `rfd` side works at all — that the runtime it builds is one `zbus`
/// can spawn onto, and that the portal hands a path back. It cannot be answered
/// by a test runner, hence the `#[ignore]`.
#[test]
#[ignore = "opens a real file chooser: run with --ignored --nocapture, and answer it"]
fn a_real_file_chooser_opens() {
    let globals = run(include_str!("scripts/real_dialog.py"), |_| Ok(()));

    let chosen: Option<PathBuf> = Python::attach(|py| {
        globals
            .bind(py)
            .get_item("chosen")
            .expect("looking up a global should not fail")
            .expect("the script sets `chosen`")
            .extract()
            .expect("a dialog answers with a path or with nothing")
    });

    // Cancelling is a perfectly good answer, and the only one available on a
    // machine where the chooser cannot be reached. What must not happen is a
    // path that is not a file.
    match chosen {
        Some(path) => {
            println!("the dialog answered with {}", path.display());
            assert!(path.is_file(), "the chooser handed back an existing file");
        }
        None => println!("the dialog was cancelled"),
    }
}
