//! The `collomatique` module, exercised from real python scripts
//!
//! The scripts live in `tests/scripts/` as `.py` files rather than as string
//! literals: they are the thing under test, and a real file is what a user
//! writes. The rust side passes inputs in and reads results out through the
//! script's globals, so the assertions stay here, where a failure says
//! something useful.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

use pyo3::prelude::*;
use pyo3::types::PyDict;

use collomatique_python::data::{
    BalancingData, ColloscopeData, DocumentData, ExportColloscopeConfigData, ExportConfigData,
    ExportGlobalConfigData, ExportGroupListConfigData, ExportStudentGroupsConfigData,
    GroupListData, IncompatData, InterrogationData, LimitsData, PairingRuleData,
    PairingRuleSideData, SlotData, SlotPairingRuleData, SlotPairingRuleSideData, StudentData,
    SubjectData, TeacherData, WeekData, WeekPatternData,
};
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

/// The exception one refused write raised, put where the script can see it
///
/// The write surface publishes nothing the model can refuse yet, so the test
/// drives the door the mirror will drive — the raw `Document::update` — and
/// hands the script what it raised.
fn refused_write(
    py: Python<'_>,
    globals: &Bound<'_, PyDict>,
    name: &str,
    op: collomatique_ops::UpdateOp,
) {
    let doc = document_of(globals);
    let error = match doc.borrow_mut(py).update(py, op) {
        Ok(_) => panic!("`{name}` is a write the model must refuse"),
        Err(error) => error,
    };

    globals
        .set_item(name, error.value(py))
        .expect("the exception should go into the namespace");
}

/// The result one applied write handed back, put where the script can see it
///
/// The mirror of [refused_write]. The write surface publishes no cascading op
/// yet, so the test drives the door the families will drive — the raw
/// `Document::update` — and hands the script the `OpResult` it answered.
fn applied_write(
    py: Python<'_>,
    globals: &Bound<'_, PyDict>,
    name: &str,
    op: collomatique_ops::UpdateOp,
) {
    let doc = document_of(globals);
    let result = doc
        .borrow_mut(py)
        .update(py, op)
        .unwrap_or_else(|_| panic!("`{name}` is a write the model must accept"));

    globals
        .set_item(name, Py::new(py, result).expect("an OpResult converts"))
        .expect("the result should go into the namespace");
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

/// One value a script built, extracted the way a mutator will extract it
///
/// The inbound half of the boundary has no python-facing caller until the ops
/// mirror lands, so the test drives the door the mirror will drive:
/// `Value::from_py`, against the document the script left in its globals.
fn extracted<V: collomatique_python::data::Value>(globals: &Py<PyDict>, name: &str) -> V::Model {
    Python::attach(|py| {
        let globals = globals.bind(py);
        let doc = document_of(globals);
        let value = globals
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"));

        V::from_py(&doc, &value).unwrap_or_else(|e| {
            e.print(py);
            panic!("`{name}` should extract")
        })
    })
}

/// The values one global holds, each extracted
///
/// A list of values is what a walk over a collection leaves behind, so this is
/// the shape most of the round trip is written in.
fn extracted_all<V: collomatique_python::data::Value>(
    globals: &Py<PyDict>,
    name: &str,
) -> Vec<V::Model> {
    Python::attach(|py| {
        let globals = globals.bind(py);
        let doc = document_of(globals);
        let values = globals
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"));

        values
            .try_iter()
            .unwrap_or_else(|_| panic!("`{name}` is a list of values"))
            .map(|value| {
                V::from_py(&doc, &value.expect("iterating a list should not fail")).unwrap_or_else(
                    |e| {
                        e.print(py);
                        panic!("every value in `{name}` should extract")
                    },
                )
            })
            .collect()
    })
}

/// How extracting one value the script built was refused
///
/// The exception's class name and its message, both of them: the refusals of
/// §2.4 are `ValueError`s that name the class and the field, and a test that
/// only checked the class would not notice a message naming the wrong field.
fn refused<V: collomatique_python::data::Value>(
    globals: &Py<PyDict>,
    name: &str,
) -> (String, String)
where
    V::Model: std::fmt::Debug,
{
    Python::attach(|py| {
        let globals = globals.bind(py);
        let doc = document_of(globals);
        let value = globals
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"));

        let error =
            V::from_py(&doc, &value).expect_err("this value is one the boundary must refuse");

        (
            error
                .get_type(py)
                .name()
                .expect("an exception class has a name")
                .to_string(),
            error.value(py).to_string(),
        )
    })
}

/// `collomatique.__version__` is the program's version
///
/// The comparison is against `collomatique_settings::current_version()`, and
/// not against this crate's `env!("CARGO_PKG_VERSION")`. Those two are not the
/// same string. maturin wants PEP 440, so `Cargo.toml` next door carries the
/// workspace version with the dev counter cut off: `0.1.0-alpha.1.99` is
/// written `0.1.0-alpha.1` there. The program's version is the whole one, so
/// that is the one the module must report.
///
/// What is left still pins something real: `__version__` has to be there, it
/// has to be a string, and it has to be the settings version rather than this
/// crate's truncated one. The assertion tells those two apart whenever a dev
/// counter is on — at a release they are the same string, and it only says the
/// module exposes a version at all.
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

    assert_eq!(
        version,
        collomatique_settings::current_version().to_string()
    );
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
        .join("../../examples/hogwarts.collomatique")
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
/// unneeded — the two shapes `colloscopes/storage/tests/header_check.rs` and
/// `colloscopes/storage/tests/general_entries_check.rs` build for the same reason. What is
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

/// The periods and the weeks are added, resized, cut, merged and removed
///
/// The fifteenth and last family of the ops mirror, and the calendar every
/// other family stands on: `doc.periods` gains `add`, `set_week_count`,
/// `remove_with_weeks`, `cut` and `merge_with_previous` beside the first-week
/// pair that opened the mirror, and `doc.weeks` gains `set_status` and
/// `set_annotation`.
///
/// The example holds no colle at all, so the three the cascades need are
/// written by the script itself, through the surface piece 13 published — one
/// on a week whose colles it switches off, one on a week the cut hands over,
/// and one on a week a shrink drops. What rust asserts here is that the example
/// really carries the shapes the script leans on: three periods, weeks enough
/// in the second to cut it after the sixth, a week each pattern pair leaves out
/// exactly once, and a subject with an enrolment row on the third period and no
/// group list there — the one the script excludes, so that the removal cascade
/// has an exclusion to repair beside its rows and its associations.
///
/// Rust reads back the file the script saved after the cut and the merge, and
/// before the removal: the year the script left, period by period and week by
/// week. The second period's own week ids, in their own order, are what says the
/// cut and the merge really cancelled out.
#[test]
fn periods_and_weeks_are_added_resized_cut_merged_and_removed() {
    use collomatique_ops::{ColloscopeUpdateOp, GeneralPlanningUpdateOp, SubjectsUpdateOp};
    use collomatique_state_colloscopes::weeks::WeekDesc;
    use collomatique_state_colloscopes::{PeriodId, WeekId};

    let dir = workspace("calendar-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let weeks_of = |params: &collomatique_state_colloscopes::colloscope_params::Parameters,
                    period: PeriodId| {
        params
            .weeks
            .weeks_for_period(period)
            .into_iter()
            .flatten()
            .map(|(week_id, _week)| *week_id)
            .collect::<Vec<_>>()
    };

    let period_ids: Vec<_> = params.periods.period_ids().collect();
    assert_eq!(period_ids.len(), 3, "the script names three periods");
    let original: Vec<Vec<WeekId>> = period_ids
        .iter()
        .map(|period| weeks_of(params, *period))
        .collect();
    assert!(
        original[1].len() > 7,
        "the script cuts the second period after its sixth week, and needs a tail"
    );

    // Every week of the example is left out by exactly one of its two patterns,
    // which is what makes each week the script drops free exactly one exclusion
    // — the assertion its `pattern_skipping` makes on its own side.
    for week in params.week_ids() {
        assert_eq!(
            params
                .week_patterns
                .week_pattern_map
                .values()
                .filter(|pattern| pattern.excluded_weeks.contains(&week))
                .count(),
            1,
            "exactly one of the example's patterns should skip each week"
        );
    }

    // A colle can only be written where the model allows one *and* the slot's
    // subject uses a group list with a group to name — the script's
    // `writable_cell`, and the predicate the shrink point below is read off.
    let writable = |week: WeekId| {
        let Some((period, _position)) = params.weeks.week_position(week) else {
            return false;
        };
        params.slots.subjects_with_slots().any(|subject| {
            let bound = params
                .group_lists
                .subjects_associations
                .get(&(period, subject))
                .and_then(|group_list| params.group_lists.group_list_map.get(group_list))
                .map(|group_list| group_list.params().group_names.len())
                .unwrap_or(0);
            bound > 0
                && params
                    .slots
                    .slots_for_subject(subject)
                    .into_iter()
                    .flatten()
                    .any(|(slot, _slot_desc)| params.is_interrogation_possible(*slot, week))
        })
    };

    // The script shrinks the third period to just before the last week a colle
    // can stand on, so that the colle it wrote there goes with the weeks.
    let kept = original[2]
        .iter()
        .rposition(|week| writable(*week))
        .expect("the third period holds a week a colle can be written on");
    assert!(
        original[2].len() - kept > 1,
        "the shrink drops more than one week, so the removals' order shows"
    );
    assert!(
        original[1].iter().take(6).any(|week| writable(*week))
            && original[1].iter().skip(6).any(|week| writable(*week)),
        "the second period holds a writable week on each side of the cut"
    );

    // The subject the script takes off the third period: one the example gives
    // an enrolment row there and no group list, so its exclusion drops the row
    // and nothing else — and the period removal that follows has an exclusion
    // to repair beside its rows and its associations.
    let spare = params
        .subjects
        .ordered_subject_list
        .keys()
        .find(|subject| {
            params
                .assignments
                .students(period_ids[2], *subject)
                .is_some()
                && !params
                    .group_lists
                    .subjects_associations
                    .contains(&(period_ids[2], *subject))
        })
        .expect("the example gives a subject a row on the third period and no group list");
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .all(|subject| subject.excluded_periods.is_empty()),
        "no subject of the example excludes a period, so the script's is the only one"
    );

    // The french labels this family's operations carry, so that the script's
    // undo assertions pin the operations' own names and not merely some
    // strings. Only the variant is read, so the payloads below are the nearest
    // ones to hand — and the two ops that name their own direction get one
    // label each way.
    let label = |op: GeneralPlanningUpdateOp| op.get_desc().1;
    let some_period = period_ids[0];
    let add_label = label(GeneralPlanningUpdateOp::AddNewPeriod(1));
    let week_count_label = label(GeneralPlanningUpdateOp::UpdatePeriodWeekCount(
        some_period,
        1,
    ));
    let remove_label = label(GeneralPlanningUpdateOp::DeletePeriodAndWeeks(some_period));
    let cut_label = label(GeneralPlanningUpdateOp::CutPeriod(some_period, 1));
    let merge_label = label(GeneralPlanningUpdateOp::MergeWithPreviousPeriod(
        some_period,
    ));
    let status_off_label = label(GeneralPlanningUpdateOp::UpdateWeekStatus(
        some_period,
        0,
        false,
    ));
    let annotate_label = label(GeneralPlanningUpdateOp::UpdateWeekAnnotation(
        some_period,
        0,
        Some(
            "Vacances"
                .to_owned()
                .try_into()
                .expect("a word is not the empty string"),
        ),
    ));
    let clear_annotation_label = label(GeneralPlanningUpdateOp::UpdateWeekAnnotation(
        some_period,
        0,
        None,
    ));

    // The two writes of other families the script makes, each carrying its own
    // family's name: the colles the cascades need, and the exclusion the
    // removal repairs.
    let colle_label = ColloscopeUpdateOp::UpdateColloscopeInterrogation(
        params
            .slots
            .subjects_with_slots()
            .flat_map(|subject| {
                params
                    .slots
                    .slots_for_subject(subject)
                    .into_iter()
                    .flatten()
            })
            .map(|(slot, _slot_desc)| *slot)
            .next()
            .expect("the example holds slots"),
        original[0][0],
        BTreeSet::new(),
    )
    .get_desc()
    .1;
    let exclude_label = SubjectsUpdateOp::UpdatePeriodStatus(spare, period_ids[2], false)
        .get_desc()
        .1;

    run(include_str!("scripts/calendar_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("week_count_label", &week_count_label)?;
        globals.set_item("remove_label", &remove_label)?;
        globals.set_item("cut_label", &cut_label)?;
        globals.set_item("merge_label", &merge_label)?;
        globals.set_item("status_off_label", &status_off_label)?;
        globals.set_item("annotate_label", &annotate_label)?;
        globals.set_item("clear_annotation_label", &clear_annotation_label)?;
        globals.set_item("colle_label", &colle_label)?;
        globals.set_item("exclude_label", &exclude_label)?;
        Ok(())
    });

    // The year the script left: the three periods it opened with, then the one
    // it added and the empty one after it.
    let written = reload(&target);
    let after = &written.get_inner_data().params;
    let after_periods: Vec<_> = after.periods.period_ids().collect();

    assert_eq!(after_periods.len(), period_ids.len() + 2);
    assert_eq!(after_periods[..3], period_ids[..]);
    assert_eq!(weeks_of(after, after_periods[0]), original[0]);
    assert_eq!(
        weeks_of(after, after_periods[1]),
        original[1],
        "the cut and the merge cancelled out, week for week and in order",
    );
    assert_eq!(weeks_of(after, after_periods[2]), original[2][..kept]);

    // The period the script grew to five weeks and shrank back to three: the
    // two it kept are the ones it was made with, and the third is the one it
    // annotated and switched the colles off on.
    let fresh = weeks_of(after, after_periods[3]);
    assert_eq!(fresh.len(), 3);
    assert!(fresh.iter().all(|week| !original.concat().contains(week)));
    assert_eq!(
        after
            .weeks
            .weeks_desc_vec_for_period(after_periods[3])
            .expect("the added period holds weeks"),
        vec![
            WeekDesc::new(true),
            WeekDesc::new(true),
            WeekDesc {
                interrogations: false,
                annotation: Some(
                    "Vacances"
                        .to_owned()
                        .try_into()
                        .expect("a word is not the empty string")
                ),
            },
        ],
    );
    assert!(weeks_of(after, after_periods[4]).is_empty());

    // Two of the three colles are still there — the third stood on a week the
    // shrink dropped — and the one the cut handed over and the merge brought
    // back is on the tail of the second period, where it was written.
    let cells: Vec<_> = written.get_inner_data().colloscope.iter().collect();
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().all(
        |((_slot, week), groups)| *groups == &BTreeSet::from([0]) && original[1].contains(week)
    ));
    assert!(
        cells
            .iter()
            .any(|((_slot, week), _groups)| original[1][6..].contains(week)),
        "the colle the cut carried is on a week of the tail",
    );

    // The exclusion the script made is what the file holds, and the row it
    // dropped is gone with it.
    assert_eq!(
        after
            .subjects
            .ordered_subject_list
            .get(&spare)
            .expect("the subject is still there")
            .excluded_periods,
        BTreeSet::from([period_ids[2]]),
    );
    assert!(after.assignments.students(period_ids[2], spare).is_none());

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
    // see. The words are the application's own (`colloscopes/gtk4/src/tools/open_save.rs`),
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
    // global order (`colloscopes/xlsx/src/lib.rs`, `generate_week_dates_title`).
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

/// A refused write names its family, its op and its case
///
/// The typed update errors of `docs/python/new_api_design.md` §6, end to end:
/// the class comes from the family, and the three attributes from the two
/// levels under it. None of it is written out variant by variant — the mapping
/// is walked off the model's own serde shape — so this test is what says the
/// walk finds the same thing the model put there.
///
/// The four refusals cover what the walk has to get right: two families, so the
/// class table is exercised rather than assumed; a case carrying one id, which
/// must reach the script as the very `PeriodId` it is holding; a case carrying
/// nothing; and a case carrying two numbers, which must come in the model's own
/// order.
///
/// They are applied from here rather than from the script because no python
/// mutator can be refused yet — the two first-week ops are the whole write
/// surface, and neither can fail.
#[test]
fn a_refused_write_names_its_family_its_op_and_its_case() {
    let dir = workspace("refused");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let first = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");
    let doomed = params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");
    let first_week_count = params
        .weeks
        .week_count_for_period(first)
        .expect("the first period has weeks");
    let doomed_subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .next()
        .expect("the example has subjects");

    // A refusal needs something the model can say no to, and the surest one is
    // an entity that is not there any more.
    assert_ne!(first, doomed, "the example has more than one period");

    run_stages(
        &[
            include_str!("scripts/refused_before.py"),
            include_str!("scripts/refused_after.py"),
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

            refused_write(
                py,
                globals,
                "dead_period",
                collomatique_ops::UpdateOp::GeneralPlanning(
                    collomatique_ops::GeneralPlanningUpdateOp::UpdatePeriodWeekCount(doomed, 3),
                ),
            );
            refused_write(
                py,
                globals,
                "no_previous",
                collomatique_ops::UpdateOp::GeneralPlanning(
                    collomatique_ops::GeneralPlanningUpdateOp::MergeWithPreviousPeriod(first),
                ),
            );
            refused_write(
                py,
                globals,
                "too_long",
                collomatique_ops::UpdateOp::GeneralPlanning(
                    collomatique_ops::GeneralPlanningUpdateOp::CutPeriod(
                        first,
                        first_week_count + 5,
                    ),
                ),
            );

            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Subjects(
                        collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed_subject),
                    ),
                )
                .expect("a subject of the example is removable");
            refused_write(
                py,
                globals,
                "dead_subject",
                collomatique_ops::UpdateOp::Subjects(
                    collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed_subject),
                ),
            );
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A cascade hands back every repair it made, and what needed it
///
/// The piece §5 promises beyond the sentence: `kind` and `details` are the
/// repair as structured data — the model's own name for it and its coordinates,
/// as ids — and `parent` is the repair that needed this one, so the warning list
/// reads as the tree it came from.
///
/// The write is applied from here rather than from the script because no python
/// mutator cascades yet: the two first-week ops are the whole write surface.
/// Deleting a subject of the example is the write that produces every shape at
/// once — slots that go with it, teachers that stop interrogating in it (a
/// repair carrying a `rebuilt` teacher the script must not see), and a slot
/// pairing rule that goes because one of the slots did, which is the parent link.
#[test]
fn a_cascade_reports_every_repair_and_what_needed_it() {
    let dir = workspace("cascade");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let paired: BTreeSet<_> = params
        .slot_pairings
        .slot_pairing_rule_map
        .values()
        .flat_map(|rule| [rule.antecedent().slot_id, rule.consequent().slot_id])
        .collect();

    // The subject whose removal exercises the whole shape: several slots, one of
    // them named by a slot pairing rule that must go with it. Handed to the
    // script as its place in the user order, since a script reads its own ids
    // off the document and rust cannot mint one for it.
    let (doomed_index, doomed) = params
        .subjects
        .ordered_subject_list
        .keys()
        .enumerate()
        .find(|(_index, subject)| {
            let slots: Vec<_> = params
                .slots
                .slots_for_subject(*subject)
                .into_iter()
                .flatten()
                .map(|(slot_id, _slot)| *slot_id)
                .collect();
            slots.len() > 1 && slots.iter().any(|slot| paired.contains(slot))
        })
        .expect("the example has a subject with several slots, one of them paired");

    run_stages(
        &[
            include_str!("scripts/cascade_before.py"),
            include_str!("scripts/cascade_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("doomed_index", doomed_index)?;
            Ok(())
        },
        |py, globals| {
            applied_write(
                py,
                globals,
                "result",
                collomatique_ops::UpdateOp::Subjects(
                    collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed),
                ),
            );
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The incompatibilities are added, rewritten and removed from python
///
/// The first family of the ops mirror, and the debut of `AddResult`: an `add`
/// answers the subclass carrying the handle of what it made, while the two
/// other ops answer the plain `OpResult` — with no `created` at all rather than
/// one holding `None`.
///
/// The family was picked first because nothing in the document points at an
/// incompatibility: no write of it cascades, so what this test says is about
/// the wiring and not about the model. Its refusals are the two the surface
/// itself owns — a dead or foreign argument, and a value naming an entity this
/// document does not hold — because those are the only ones the three ops have
/// (`crate::collections::incompats`).
///
/// Rust reads back the file the script saved after its last write: the one
/// incompatibility the example did not have is the one the script asked for,
/// field by field.
#[test]
fn incompatibilities_are_added_rewritten_and_removed() {
    let dir = workspace("incompats-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let before: BTreeSet<_> = params.incompats.incompat_map.keys().collect();

    // The two entities the script names — its first subject and its first week
    // pattern, in the order the script walks them.
    let subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .next()
        .expect("the example has subjects");
    let pattern = params
        .week_patterns
        .week_pattern_map
        .keys()
        .next()
        .expect("the example has week patterns");

    let at = |hour: u32, minute: u32| {
        collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("a real time of day"),
        )
        .expect("a whole minute")
    };
    let monday_at = |hour: u32| {
        collomatique_time::SlotWithDuration::new(
            collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
                start_time: at(hour, 0),
            },
            collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
        )
        .expect("an hour from a whole hour is a window")
    };

    // What the script's last write asked for, built here so that the comparison
    // is with the incompatibility a reader of this test can see written out.
    let written_out = collomatique_state_colloscopes::incompats::Incompatibility {
        subject_id: subject,
        name: "Lundi Midi (par id)".to_owned(),
        slots: vec![monday_at(12), monday_at(13)],
        minimum_free_slots: NonZeroU32::new(2).expect("two is not zero"),
        week_pattern_id: Some(pattern),
    };

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::IncompatibilitiesUpdateOp| op.get_desc().1;
    let some_incompat = *before.iter().next().expect("the example has incompats");
    let add_label = label(collomatique_ops::IncompatibilitiesUpdateOp::AddNewIncompat(
        written_out.clone(),
    ));
    let update_label = label(collomatique_ops::IncompatibilitiesUpdateOp::UpdateIncompat(
        some_incompat,
        written_out.clone(),
    ));
    let remove_label = label(collomatique_ops::IncompatibilitiesUpdateOp::DeleteIncompat(
        some_incompat,
    ));

    run(include_str!("scripts/incompats_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // The document the script saved holds everything it opened with, plus the
    // one incompatibility it wrote — and that one is what it asked for.
    let written = reload(&target);
    let after = &written.get_inner_data().params.incompats.incompat_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _incompat)| !before.contains(id))
        .map(|(_id, incompat)| incompat.clone())
        .collect();

    assert_eq!(added, vec![written_out]);
    assert_eq!(after.len(), before.len() + 1);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The pairing rules are added, rewritten and removed from python
///
/// The second family of the ops mirror, and the twin of
/// [incompatibilities_are_added_rewritten_and_removed]: nothing points at a
/// pairing rule either, so a write of this family repairs nothing and the three
/// ops are again wiring rather than model.
///
/// What this family has and the incompatibilities' has not is a refusal of its
/// own — a rule about a subject that runs no interrogations is vacuous — so
/// this is also the first family test that meets its own `PairingsError` and
/// reads the op, the case and the subject the model named off it. The example
/// holds subjects of both kinds, which is why the write test reads it rather
/// than [pairings_document], the fixture the read test needed for lack of any
/// rule at all.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// one rule the example did not have is the one the script asked for, field by
/// field.
#[test]
fn pairing_rules_are_added_rewritten_and_removed() {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::pairings::{PairingRule, RulePart};

    let dir = workspace("pairings-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let before: BTreeSet<_> = params.pairings.pairing_rule_map.keys().collect();

    // The subjects the script names, in the order it walks them: the three
    // first that run colles, and the first that runs none. Both kinds have to
    // be there — the second is the whole of what `PairingsError` is asserted on
    // — so the fixture is checked before the script leans on it.
    let with_colles: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, _subject)| id)
        .collect();
    assert!(
        with_colles.len() >= 3,
        "the script names three subjects that run colles"
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .any(|subject| subject.parameters.interrogation_parameters.is_none()),
        "the script needs a subject that runs no interrogations"
    );

    let first_period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");

    // What the script's last accepted write asked for, built here so that the
    // comparison is with the rule a reader of this test can see written out.
    let written_out = PairingRule::new(
        RulePart {
            subject_id: with_colles[2],
            should_have: false,
        },
        RulePart {
            subject_id: with_colles[0],
            should_have: true,
        },
        BTreeSet::from([first_period]),
        true,
    )
    .expect("the antecedent and the consequent name different subjects");

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::PairingsUpdateOp| op.get_desc().1;
    let some_rule = unsafe { collomatique_state_colloscopes::PairingRuleId::new(1) };
    let add_label = label(collomatique_ops::PairingsUpdateOp::AddNewPairingRule(
        written_out.clone(),
    ));
    let update_label = label(collomatique_ops::PairingsUpdateOp::UpdatePairingRule(
        some_rule,
        written_out.clone(),
    ));
    let remove_label = label(collomatique_ops::PairingsUpdateOp::DeletePairingRule(
        some_rule,
    ));

    run(include_str!("scripts/pairings_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // The document the script saved holds everything it opened with, plus the
    // one pairing rule it wrote — and that one is what it asked for.
    let written = reload(&target);
    let after = &written.get_inner_data().params.pairings.pairing_rule_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _rule)| !before.contains(id))
        .map(|(_id, rule)| rule.clone())
        .collect();

    assert_eq!(added, vec![written_out]);
    assert_eq!(after.len(), before.len() + 1);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The slot pairing rules are added, rewritten and removed from python
///
/// The third family of the ops mirror, and the twin of
/// [pairing_rules_are_added_rewritten_and_removed] one level down: nothing
/// points at a slot pairing rule either, so a write of this family repairs
/// nothing, and the family keeps exactly one refusal for the model — the two
/// slots of a rule must be on the same subject, which is what the script's
/// `SlotPairingsError` assertions read the op, the case and the two slots off.
///
/// The example is what both this and [the_slot_pairing_rules_read_back_rule_by_rule]
/// read: it carries slot pairing rules of its own, so the foreign-handle
/// refusal here is sharper than the subject-level one could be — the rule
/// `other` hands out carries an id this document really does hold, and it is
/// refused all the same.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// one rule the example did not have is the one the script asked for, field by
/// field.
#[test]
fn slot_pairing_rules_are_added_rewritten_and_removed() {
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::slot_pairings::{SlotPairingRule, SlotRulePart};

    let dir = workspace("slot-pairings-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let before: BTreeSet<_> = params.slot_pairings.slot_pairing_rule_map.keys().collect();

    // The slots the script names, picked the way it picks them: the first three
    // of the first subject that runs at least three. A slot of some other
    // subject has to exist too — it is the whole of what `SlotPairingsError` is
    // asserted on — so the fixture is checked before the script leans on it.
    let slots_of = |subject_id| -> Vec<_> {
        params
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
            .map(|(slot_id, _slot)| *slot_id)
            .collect()
    };
    let subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .find(|subject_id| slots_of(*subject_id).len() >= 3)
        .expect("the script needs a subject running three slots");
    let slots = slots_of(subject);
    assert!(
        params
            .subjects
            .ordered_subject_list
            .keys()
            .any(|other| other != subject && !slots_of(other).is_empty()),
        "the script needs a slot of another subject"
    );

    let first_period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");

    // What the script's last accepted write asked for, built here so that the
    // comparison is with the rule a reader of this test can see written out.
    let written_out = SlotPairingRule::new(
        SlotRulePart {
            slot_id: slots[2],
            should_have: false,
        },
        SlotRulePart {
            slot_id: slots[0],
            should_have: true,
        },
        BTreeSet::from([first_period]),
        true,
    )
    .expect("the antecedent and the consequent name different slots");

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::SlotPairingsUpdateOp| op.get_desc().1;
    let some_rule = unsafe { collomatique_state_colloscopes::SlotPairingRuleId::new(1) };
    let add_label =
        label(collomatique_ops::SlotPairingsUpdateOp::AddNewSlotPairingRule(written_out.clone()));
    let update_label = label(
        collomatique_ops::SlotPairingsUpdateOp::UpdateSlotPairingRule(
            some_rule,
            written_out.clone(),
        ),
    );
    let remove_label =
        label(collomatique_ops::SlotPairingsUpdateOp::DeleteSlotPairingRule(some_rule));

    run(include_str!("scripts/slot_pairings_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // The document the script saved holds everything it opened with, plus the
    // one slot pairing rule it wrote — and that one is what it asked for.
    let written = reload(&target);
    let after = &written
        .get_inner_data()
        .params
        .slot_pairings
        .slot_pairing_rule_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _rule)| !before.contains(id))
        .map(|(_id, rule)| rule.clone())
        .collect();

    assert_eq!(added, vec![written_out]);
    assert_eq!(after.len(), before.len() + 1);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The limits are set, overridden and un-overridden from python
///
/// The fourth family of the ops mirror, and the first whose entities are not
/// entities at all: the global entry and the per-student overrides are records
/// the document holds one of, so the three ops are whole-entry writes and there
/// is nothing to add or to count. What the script pins on that is the
/// whole-entry rule itself — a field left at `None` in an override *disables*
/// the global limit rather than inheriting it — read back through the resolved
/// view it holds across every write.
///
/// The family keeps one refusal for the model, and it is the one op whose
/// address may be right and whose request may still be wrong: removing an
/// override a student does not have. That is what the script's `SettingsError`
/// assertions read the op, the case and the student off; a student this
/// document does not hold never reaches the model at all.
///
/// The example is what [the_settings_read_back_entry_by_entry] reads too: one
/// override (Hermione's) and one student without one (Harry), which is the
/// whole resolution shape and exactly what the two writes need.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// global entry it installed, the override it gave Harry, and Hermione's own,
/// which no write of the script ever named.
#[test]
fn the_limits_are_set_overridden_and_un_overridden() {
    use collomatique_state_colloscopes::settings::{Limits, SoftParam};

    let dir = workspace("settings-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same students the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let student_named = |surname: &str| {
        params
            .students
            .student_map
            .iter()
            .find(|(_id, student)| student.desc.surname == surname)
            .unwrap_or_else(|| panic!("the example has {surname}"))
            .0
    };
    let harry = student_named("Potter");
    let hermione = student_named("Granger");

    // The one override the example ships, and the one student without one:
    // the fixture is checked before the script leans on it.
    let hermione_before = params
        .settings
        .students
        .get(&hermione)
        .expect("Hermione has an override")
        .clone();
    assert!(
        params.settings.students.get(&harry).is_none(),
        "the script needs a student inheriting the global entry"
    );
    let before = params.settings.students.len();

    // What the script's two last accepted writes asked for, built here so that
    // the comparison is with the entries a reader of this test can see written
    // out. Both leave fields at `None`, which is the whole-entry rule: they
    // disable those limits rather than inheriting them.
    let new_global = Limits {
        interrogations_per_week_min: None,
        interrogations_per_week_max: Some(SoftParam {
            soft: false,
            value: 3,
        }),
        max_interrogations_per_day: None,
    };
    let harry_override = Limits {
        interrogations_per_week_min: Some(SoftParam {
            soft: false,
            value: 1,
        }),
        interrogations_per_week_max: None,
        max_interrogations_per_day: Some(SoftParam {
            soft: true,
            value: std::num::NonZeroU32::new(2).expect("two is not zero"),
        }),
    };

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::SettingsUpdateOp| op.get_desc().1;
    let global_label = label(collomatique_ops::SettingsUpdateOp::UpdateGlobalLimits(
        new_global.clone(),
    ));
    let student_label = label(collomatique_ops::SettingsUpdateOp::UpdateStudentLimits(
        harry,
        harry_override.clone(),
    ));
    let remove_label = label(collomatique_ops::SettingsUpdateOp::RemoveStudentLimits(
        harry,
    ));

    run(include_str!("scripts/settings_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("global_label", &global_label)?;
        globals.set_item("student_label", &student_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // The document the script saved holds the two entries it wrote, and the one
    // it never named is untouched.
    let written = reload(&target);
    let settings = &written.get_inner_data().params.settings;

    assert_eq!(settings.global, new_global);
    assert_eq!(settings.students.get(&harry), Some(&harry_override));
    assert_eq!(settings.students.get(&hermione), Some(&hermione_before));
    assert_eq!(settings.students.len(), before + 1);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The balancing options are set, overridden and un-overridden from python
///
/// The fifth family of the ops mirror, and the settings' twin: the global entry
/// and the per-subject overrides are records the document holds one of, so the
/// three ops are whole-entry writes and there is nothing to add or to count.
/// The whole-entry rule is what the script pins on that — a goal left at `None`
/// in an override is *not pursued*, rather than inherited from the global entry
/// — read back through the resolved view it holds across every write.
///
/// Where the twin has one refusal for the model, this family has two, one per
/// addressed op, and both need a subject the document already holds: only a
/// subject that runs interrogations may carry an override at all, and removing
/// an override a subject does not have is refused rather than quietly done.
/// That is what the script's two `BalancingError` assertions read the op, the
/// case and the subject off; a subject this document does not hold never
/// reaches the model at all.
///
/// The example is what [the_balancing_read_back_entry_by_entry] reads too, and
/// it has all three subjects this needs: Métamorphose with an override, which
/// no write of the script names, Arithmancie without one, and the Quidditch
/// training, which runs no interrogations and so can never have one.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// global entry it installed, the override it gave Arithmancie, and
/// Métamorphose's own.
#[test]
fn the_balancing_options_are_set_overridden_and_un_overridden() {
    use collomatique_state_colloscopes::balancing::BalancingOptions;
    use collomatique_state_colloscopes::settings::SoftParam;

    let dir = workspace("balancing-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same subjects the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let subject_named = |name: &str| {
        params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .unwrap_or_else(|| panic!("the example has {name}"))
            .0
    };
    let metamorphose = subject_named("Métamorphose");
    let arithmancie = subject_named("Arithmancie");
    let quidditch = subject_named("Entrainement de Quidditch");

    // One of the overrides the example ships, the subject without one, and the
    // subject that runs no interrogations: the fixture is checked before the
    // script leans on it.
    let metamorphose_before = params
        .balancing
        .subjects
        .get(&metamorphose)
        .expect("Métamorphose has an override")
        .clone();
    assert!(
        params.balancing.subjects.get(&arithmancie).is_none(),
        "the script needs a subject inheriting the global entry"
    );
    assert!(
        params
            .subjects
            .find_subject(quidditch)
            .expect("the Quidditch training is a subject")
            .parameters
            .interrogation_parameters
            .is_none(),
        "the script needs a subject that cannot carry an override"
    );
    let before = params.balancing.subjects.len();

    // What the script's two last accepted writes asked for, built here so that
    // the comparison is with the entries a reader of this test can see written
    // out. Both leave goals at `None`, which is the whole-entry rule: those
    // goals are not pursued rather than inherited. A `SoftParam` that is `soft`
    // is the `OBJECTIVE` spelling, and one that is not is `STRICT`.
    let strict = || {
        Some(SoftParam {
            soft: false,
            value: (),
        })
    };
    let new_global = BalancingOptions {
        teacher_rotation: None,
        slot_rotation: strict(),
        avoid_twice_in_a_row: None,
        year_teacher_rotation: true,
        period_teacher_rotation: false,
    };
    let arithmancie_override = BalancingOptions {
        teacher_rotation: strict(),
        slot_rotation: None,
        avoid_twice_in_a_row: None,
        year_teacher_rotation: false,
        period_teacher_rotation: true,
    };

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::BalancingUpdateOp| op.get_desc().1;
    let global_label = label(collomatique_ops::BalancingUpdateOp::UpdateGlobalOptions(
        new_global.clone(),
    ));
    let subject_label = label(collomatique_ops::BalancingUpdateOp::UpdateSubjectOptions(
        arithmancie,
        arithmancie_override.clone(),
    ));
    let remove_label = label(collomatique_ops::BalancingUpdateOp::RemoveSubjectOptions(
        arithmancie,
    ));

    run(include_str!("scripts/balancing_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("global_label", &global_label)?;
        globals.set_item("subject_label", &subject_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // The document the script saved holds the two entries it wrote, and the one
    // it never named is untouched.
    let written = reload(&target);
    let balancing = &written.get_inner_data().params.balancing;

    assert_eq!(balancing.global, new_global);
    assert_eq!(
        balancing.subjects.get(&arithmancie),
        Some(&arithmancie_override)
    );
    assert_eq!(
        balancing.subjects.get(&metamorphose),
        Some(&metamorphose_before)
    );
    assert_eq!(balancing.subjects.len(), before + 1);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The export configuration is rewritten section by section from python
///
/// The sixth family of the ops mirror, and the last of its leaves: eleven
/// setters over one atom of pure value data. Nothing here names an entity, so
/// there is no argument convention to exercise and no refusal for the model to
/// make — `ExportConfigUpdateError` has no variants at all — and the only way a
/// call of this family fails is the value boundary, which the script drives
/// through every shape the configuration holds.
///
/// What the script pins instead is the granularity: one setter writes one
/// field of the configuration and leaves the ten others alone, and above all
/// the flags sit *beside* the sections they gate, so switching a sheet off
/// keeps everything its section holds. The document it starts from is
/// [export_config_document], away from the default on every field, so a write
/// that did nothing could not pass for one that did.
///
/// Rust reads back the file the script saved after its eleven accepted writes
/// and compares the whole configuration with the one written out here.
#[test]
fn the_export_configuration_is_rewritten_section_by_section() {
    use collomatique_ops::ExportConfigUpdateOp;
    use collomatique_state_colloscopes::export_config::{
        ColloscopeConfig, Color, ExportConfig, GlobalConfig, PageOrientation, PerGroupListConfig,
        PerStudentGroupsConfig,
    };

    let dir = workspace("export-config-write");
    let source = dir.join("export_config.collomatique");
    export_config_document(&source);
    let target = dir.join("written.collomatique");

    // The french labels the eleven operations carry, in the order the script
    // writes them, so that its undo assertions pin the operations' own names
    // and not merely some strings. Only the variant is read, so the payloads
    // are the emptiest ones to hand.
    let label = |op: ExportConfigUpdateOp| op.get_desc().1;
    let labels = vec![
        label(ExportConfigUpdateOp::UpdateGlobalConfig(
            GlobalConfig::default(),
        )),
        label(ExportConfigUpdateOp::UpdateColloscopeEnabled(true)),
        label(ExportConfigUpdateOp::UpdateAllGroupsEnabled(true)),
        label(ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(false)),
        label(ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(false)),
        label(ExportConfigUpdateOp::UpdatePerGroupListEnabled(true)),
        label(ExportConfigUpdateOp::UpdateColloscopeConfig(
            ColloscopeConfig::default(),
        )),
        label(ExportConfigUpdateOp::UpdateAllGroupsConfig(
            PerStudentGroupsConfig::default_all_groups(),
        )),
        label(ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(
            PerStudentGroupsConfig::default_automatic_groups(),
        )),
        label(ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(
            PerStudentGroupsConfig::default_prefilled_groups(),
        )),
        label(ExportConfigUpdateOp::UpdatePerGroupListConfig(
            PerGroupListConfig::default(),
        )),
    ];

    // The eleven operations are eleven undo slots, so the labels must tell them
    // apart: a script undoing its way back through them would not notice two
    // of them sharing a sentence.
    assert_eq!(
        labels.iter().collect::<BTreeSet<_>>().len(),
        labels.len(),
        "the eleven operations name themselves apart"
    );

    run(include_str!("scripts/export_config_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("labels", &labels)?;
        Ok(())
    });

    // What the script's eleven writes asked for, written out here so that the
    // comparison is with a configuration a reader of this test can see whole.
    let expected = ExportConfig {
        global: GlobalConfig {
            background_color: Color {
                red: 16,
                green: 17,
                blue: 18,
            },
            stripes_color_enabled: true,
            stripes_color: Color {
                red: 19,
                green: 20,
                blue: 21,
            },
        },
        colloscope_enabled: true,
        all_groups_enabled: true,
        automatic_groups_enabled: false,
        prefilled_groups_enabled: false,
        per_group_list_enabled: true,
        colloscope_config: ColloscopeConfig {
            sheet_name: "Colles".into(),
            extra_info_column_enabled: true,
            extra_info_column_name: "Remarques".into(),
            teacher_email_enabled: true,
            teacher_email: "Courriel".into(),
            teacher_tel_enabled: false,
            teacher_tel: "Téléphone".into(),
            orientation: PageOrientation::Landscape,
            display_week_dates: true,
            display_annotations: true,
            no_interrogation_color: Color {
                red: 22,
                green: 23,
                blue: 24,
            },
            annotation_color_enabled: true,
            annotation_color: Color {
                red: 25,
                green: 26,
                blue: 27,
            },
            // The map the value carried is the whole of the sheet's afterwards:
            // the label the fixture ships is gone, and these two are all there
            // is.
            extra_colors: BTreeMap::from([
                (
                    "Vacances".to_owned(),
                    Color {
                        red: 28,
                        green: 29,
                        blue: 30,
                    },
                ),
                (
                    "Examens".to_owned(),
                    Color {
                        red: 31,
                        green: 32,
                        blue: 33,
                    },
                ),
            ]),
        },
        all_groups_config: PerStudentGroupsConfig {
            sheet_name: "Groupes".into(),
            orientation: Some(PageOrientation::Portrait),
            show_emails: true,
            show_tel: false,
        },
        automatic_groups_config: PerStudentGroupsConfig::default_automatic_groups(),
        // The one the script wrote with the *all-groups* default: the setter is
        // the address, and the name a value carries is a field like any other.
        prefilled_groups_config: PerStudentGroupsConfig::default_all_groups(),
        per_group_list_config: PerGroupListConfig {
            orientation: PageOrientation::Portrait,
            show_emails: true,
            show_tel: true,
            center_vertically: false,
        },
    };

    // Every field of it is away from what the document opened with, so no write
    // of the eleven could have been a no-op that passed for one.
    let opened = reload(&source).get_inner_data().export_config.clone();
    assert_ne!(expected.global, opened.global);
    assert_ne!(expected.colloscope_enabled, opened.colloscope_enabled);
    assert_ne!(expected.all_groups_enabled, opened.all_groups_enabled);
    assert_ne!(
        expected.automatic_groups_enabled,
        opened.automatic_groups_enabled
    );
    assert_ne!(
        expected.prefilled_groups_enabled,
        opened.prefilled_groups_enabled
    );
    assert_ne!(
        expected.per_group_list_enabled,
        opened.per_group_list_enabled
    );
    assert_ne!(expected.colloscope_config, opened.colloscope_config);
    assert_ne!(expected.all_groups_config, opened.all_groups_config);
    assert_ne!(
        expected.automatic_groups_config,
        opened.automatic_groups_config
    );
    assert_ne!(
        expected.prefilled_groups_config,
        opened.prefilled_groups_config
    );
    assert_ne!(expected.per_group_list_config, opened.per_group_list_config);

    let written = reload(&target);
    assert_eq!(&written.get_inner_data().export_config, &expected);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The teachers are added, rewritten and removed from python
///
/// The seventh family of the ops mirror, and the first whose writes cascade: a
/// slot names the teacher who holds it and cannot do without one, so both the
/// `update` that drops a subject from a teacher's set and the `remove` that
/// takes the teacher away leave slots with nobody to hold them, and the model
/// deletes those slots. This is therefore the first family test that reads
/// piece B's structured warnings off a family's own surface rather than off a
/// removal rust made behind the script's back.
///
/// The example is picked over a fixture of its own because it already holds
/// every shape this needs: teachers with slots in two subjects, a teacher two
/// of whose slots are related by a slot pairing rule — the parent link — and
/// subjects that run no colles at all, which is what `TeachersError` is
/// asserted on. Each of those is checked here before the script leans on it.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// two teachers the example did not have are the two the script asked for,
/// field by field.
#[test]
fn teachers_are_added_rewritten_and_removed() {
    use collomatique_state_colloscopes::PersonWithContact;
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::teachers::Teacher;

    let dir = workspace("teachers-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let before: BTreeSet<_> = params.teachers.teacher_map.keys().collect();

    // The subjects the script names, in the order it walks them: the two first
    // that run colles, and the first that runs none. Both kinds have to be
    // there — the second is the whole of what `TeachersError` is asserted on.
    let with_colles: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, _subject)| id)
        .collect();
    assert!(
        with_colles.len() >= 2,
        "the script names two subjects that run colles"
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .any(|subject| subject.parameters.interrogation_parameters.is_none()),
        "the script needs a subject that runs no interrogations"
    );

    let paired: BTreeSet<_> = params
        .slot_pairings
        .slot_pairing_rule_map
        .values()
        .flat_map(|rule| [rule.antecedent().slot_id, rule.consequent().slot_id])
        .collect();

    // What each teacher holds, as the script sees it: the subjects their slots
    // sit in, and whether any of those slots is named by a slot pairing rule.
    let slots_of = |teacher_id| {
        params
            .slots
            .all_slots()
            .filter(move |(_slot_id, slot)| slot.teacher_id == teacher_id)
    };

    // The teacher the script prunes: two subjects, slots in each, and none of
    // those slots paired — so the `update` that drops one subject deletes
    // exactly that subject's slots and the warning list is one flat row of
    // `DeleteSlot`.
    let (pruned_index, pruned) = params
        .teachers
        .teacher_map
        .keys()
        .enumerate()
        .find(|(_index, teacher_id)| {
            let held: BTreeSet<_> = slots_of(*teacher_id)
                .map(|(_slot_id, slot)| slot.subject_id)
                .collect();
            held.len() >= 2 && slots_of(*teacher_id).all(|(id, _slot)| !paired.contains(id))
        })
        .expect("the example has a teacher holding slots in two subjects, none of them paired");

    // The teacher the script removes: slots of their own, one of them named by
    // a slot pairing rule that must go with it — the parent link the warning
    // tree is asserted on. Never the pruned one, whose slots the script has
    // already thinned by then.
    let (doomed_index, _doomed) = params
        .teachers
        .teacher_map
        .keys()
        .enumerate()
        .find(|(_index, teacher_id)| {
            *teacher_id != pruned && slots_of(*teacher_id).any(|(id, _slot)| paired.contains(id))
        })
        .expect("the example has another teacher holding a paired slot");

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::TeachersUpdateOp| op.get_desc().1;
    let some_teacher = unsafe { collomatique_state_colloscopes::TeacherId::new(1) };
    let add_label = label(collomatique_ops::TeachersUpdateOp::AddNewTeacher(
        Teacher::default(),
    ));
    let update_label = label(collomatique_ops::TeachersUpdateOp::UpdateTeacher(
        some_teacher,
        Teacher::default(),
    ));
    let remove_label = label(collomatique_ops::TeachersUpdateOp::DeleteTeacher(
        some_teacher,
    ));

    run(include_str!("scripts/teachers_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("pruned_index", pruned_index)?;
        globals.set_item("doomed_index", doomed_index)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // What the script's two adds asked for, as they stood when it saved: the
    // first was rewritten twice after being created, the second never touched.
    let written_out = vec![
        Teacher {
            desc: PersonWithContact {
                surname: "Noether".to_owned(),
                firstname: "Emmy".to_owned(),
                tel: None,
                email: None,
            },
            subjects: BTreeSet::from([with_colles[0]]),
        },
        Teacher {
            desc: PersonWithContact {
                surname: "Rusard".to_owned(),
                firstname: "Argus".to_owned(),
                tel: None,
                email: None,
            },
            subjects: BTreeSet::new(),
        },
    ];

    // The document the script saved holds everything it opened with, plus the
    // two teachers it wrote — and those two are what it asked for.
    let written = reload(&target);
    let after = &written.get_inner_data().params.teachers.teacher_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _teacher)| !before.contains(id))
        .map(|(_id, teacher)| teacher.clone())
        .collect();

    assert_eq!(added, written_out);
    assert_eq!(after.len(), before.len() + 2);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The students are added, rewritten and removed from python
///
/// The eighth family of the ops mirror, and the widest cascade so far: a
/// student's name is written down in the assignment rows they sit in, in the
/// prefilled groups that hold them, and in their own entry of the settings, and
/// removing them takes it out of every one of those at once. None of those
/// sites dies of it — a row that lost a name is still a row — so the warning
/// list is flat where the teachers' was a tree, and this test says both things.
/// The `update` cascades too, and without anybody being removed: a student who
/// now sits a period out cannot be assigned in it.
///
/// Unlike the teachers next door, this family keeps no refusal for the model:
/// the two things `StudentsUpdateOp` can object to — a student id and an
/// excluded period id — are both caught above the write, so the script asserts
/// them as a stale handle and not as a `StudentsError`.
///
/// The example is picked over a fixture of its own because it already holds
/// every shape this needs: one student with a limits override, students in
/// prefilled group lists, and assignment rows over several periods. Each of
/// those is checked here before the script leans on it.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// two students the example did not have are the two the script asked for,
/// field by field.
#[test]
fn students_are_added_rewritten_and_removed() {
    use collomatique_state_colloscopes::PersonWithContact;
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::students::Student;

    let dir = workspace("students-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let before: BTreeSet<_> = params.students.student_map.keys().collect();

    // The last period, which the script's second add excludes: it is written
    // out, so rust has to name the same one the script did.
    let last_period = params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");

    // Where each student is assigned, as the script reads it off
    // `doc.assignments`: the periods of the rows naming them, and how many rows
    // that is.
    let rows_of = |student_id| {
        params
            .assignments
            .iter()
            .filter(move |(_period, _subject, students)| students.contains(&student_id))
    };
    let periods_of = |student_id| {
        rows_of(student_id)
            .map(|(period, _subject, _students)| period)
            .collect::<BTreeSet<_>>()
    };

    // The student the script removes: the one the settings hold an override
    // for, which the example has exactly one of. They must also be held by a
    // prefilled group list and sit in assignment rows — the three kinds of site
    // whose repairs the script reads apart.
    let mut overridden = params.settings.students.keys();
    let doomed = overridden
        .next()
        .expect("the example holds one per-student limits override");
    assert!(
        overridden.next().is_none(),
        "the script reads the override count down to zero, so there is one"
    );
    let doomed_index = params
        .students
        .student_map
        .keys()
        .position(|id| id == doomed)
        .expect("the overridden student is a student");
    assert!(
        params
            .group_lists
            .group_list_map
            .values()
            .any(|group_list| group_list.filling().contains_student(doomed)),
        "the removed student is held by a prefilled group list"
    );
    assert!(
        rows_of(doomed).next().is_some(),
        "the removed student sits in assignment rows"
    );

    // The student the script excludes from a period: assigned over at least two
    // periods, so that the repair can be seen to take one period's rows and
    // leave the others alone. Never the removed one, whose rows the script
    // asserts whole.
    let (excluded_index, excluded) = params
        .students
        .student_map
        .keys()
        .enumerate()
        .find(|(_index, student_id)| *student_id != doomed && periods_of(*student_id).len() >= 2)
        .expect("the example has a student assigned over two periods");
    assert!(
        params
            .group_lists
            .group_list_map
            .values()
            .any(|group_list| group_list.filling().contains_student(excluded)),
        "the excluded student is held by a group list the exclusion must leave alone"
    );

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operation's own name and not merely some string. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: collomatique_ops::StudentsUpdateOp| op.get_desc().1;
    let some_student = unsafe { collomatique_state_colloscopes::StudentId::new(1) };
    let add_label = label(collomatique_ops::StudentsUpdateOp::AddNewStudent(
        Student::default(),
    ));
    let update_label = label(collomatique_ops::StudentsUpdateOp::UpdateStudent(
        some_student,
        Student::default(),
    ));
    let remove_label = label(collomatique_ops::StudentsUpdateOp::DeleteStudent(
        some_student,
    ));

    run(include_str!("scripts/students_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("excluded_index", excluded_index)?;
        globals.set_item("doomed_index", doomed_index)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        Ok(())
    });

    // What the script's two adds asked for, as they stood when it saved: the
    // first was rewritten twice after being created, the second never touched.
    let written_out = vec![
        Student {
            desc: PersonWithContact {
                surname: "Tonks".to_owned(),
                firstname: "Nymphadora".to_owned(),
                tel: None,
                email: None,
            },
            excluded_periods: BTreeSet::new(),
        },
        Student {
            desc: PersonWithContact {
                surname: "Black".to_owned(),
                firstname: "Sirius".to_owned(),
                tel: None,
                email: None,
            },
            excluded_periods: BTreeSet::from([last_period]),
        },
    ];

    // The document the script saved holds everything it opened with, plus the
    // two students it wrote — and those two are what it asked for.
    let written = reload(&target);
    let after = &written.get_inner_data().params.students.student_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _student)| !before.contains(id))
        .map(|(_id, student)| student.clone())
        .collect();

    assert_eq!(added, written_out);
    assert_eq!(after.len(), before.len() + 2);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The assignments are written one student, one row and one period at a time
///
/// The ninth family of the ops mirror, and the only one with no value class at
/// all: a row is the three ids it is made of, so `set`, `set_all` and
/// `duplicate_previous_period` are argument-convention wiring and nothing else.
/// Nothing in the document points *at* a row either, so every one of those
/// writes answers an empty warning list — which is what this test says of each
/// of them, family by family being the point at which such a claim is worth
/// making.
///
/// The three refusals the model keeps are all here. Two are reachable from the
/// write surface as it stands — a student the period excludes, and the first
/// period having nothing before it — and the third is not: a subject stops
/// running on a period through the subjects family, which is a later piece. So
/// this runs in two stages, with rust switching that period off in between, the
/// way [a_refused_write_names_its_family_its_op_and_its_case] does.
///
/// The example is picked over a fixture of its own because it already holds
/// what this needs: three periods, a row for every (period, subject) pair on
/// the first two of them, and subjects that only some of the students take.
/// Each of those is checked here before the script leans on it.
///
/// Rust reads back the file the script saved after its last accepted write, and
/// compares the whole assignments table with one it computes itself from the
/// document the script opened.
#[test]
fn the_assignments_are_written_one_student_a_row_and_a_period_at_a_time() {
    use collomatique_ops::{AssignmentsUpdateOp, StudentsUpdateOp, SubjectsUpdateOp, UpdateOp};
    use collomatique_state_colloscopes::students::Student;
    use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};

    let dir = workspace("assignments-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let period_ids: Vec<_> = params.periods.period_ids().collect();
    assert!(
        period_ids.len() >= 3,
        "the script writes on two periods and leaves a third for the second stage"
    );
    let first_period = period_ids[0];
    let second_period = period_ids[1];
    let last_period = *period_ids.last().expect("the example has periods");

    let students: BTreeSet<_> = params.students.student_map.keys().collect();
    let subject_ids: Vec<_> = params.subjects.ordered_subject_list.keys().collect();

    // The row at one address, absent rows included — an absent row is the empty
    // one, which is exactly what python reads there.
    let row = |period, subject| {
        params
            .assignments
            .students(period, subject)
            .cloned()
            .unwrap_or_default()
    };

    // The subject the script rewrites: some students take it on the first
    // period and some do not, so both directions of `set` have something to do
    // there.
    let (partial_index, partial) = subject_ids
        .iter()
        .enumerate()
        .map(|(index, subject)| (index, *subject))
        .find(|(_index, subject)| {
            let members = row(first_period, *subject);
            !members.is_empty() && members.len() < students.len()
        })
        .expect("the example has a subject not every student takes");

    // `duplicate_previous_period` rewrites the rows the period already has, so
    // the script's whole-table assertion needs one everywhere on both periods.
    for subject in &subject_ids {
        assert!(
            !row(first_period, *subject).is_empty(),
            "every subject has a row on the first period"
        );
        assert!(
            !row(second_period, *subject).is_empty(),
            "every subject has a row on the second period"
        );
    }

    // The student the script excludes from the second period: one the partial
    // row holds there, so that the exclusion has something to take away and the
    // copy afterwards has something to leave alone.
    let (excluded_index, excluded) = params
        .students
        .student_map
        .keys()
        .enumerate()
        .find(|(_index, student)| row(second_period, partial).contains(student))
        .expect("the partial row of the second period holds somebody");

    // The french labels the operations carry, so that the script's undo
    // assertions pin the operations' own names and not merely some strings.
    // Only the variant and the flag are read, so the ids below are the nearest
    // ones to hand.
    let label = |op: AssignmentsUpdateOp| op.get_desc().1;
    let assign_label = label(AssignmentsUpdateOp::Assign(
        first_period,
        excluded,
        partial,
        true,
    ));
    let unassign_label = label(AssignmentsUpdateOp::Assign(
        first_period,
        excluded,
        partial,
        false,
    ));
    let assign_all_label = label(AssignmentsUpdateOp::AssignAll(first_period, partial, true));
    let unassign_all_label = label(AssignmentsUpdateOp::AssignAll(first_period, partial, false));
    let duplicate_label = label(AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period));
    // The one write of another family the script makes: the exclusion it needs
    // before it can be refused over an absent student.
    let student_update_label = StudentsUpdateOp::UpdateStudent(excluded, Student::default())
        .get_desc()
        .1;

    run_stages(
        &[
            include_str!("scripts/assignments_write_before.py"),
            include_str!("scripts/assignments_write_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("target", &target)?;
            globals.set_item("partial_index", partial_index)?;
            globals.set_item("excluded_index", excluded_index)?;
            globals.set_item("assign_label", &assign_label)?;
            globals.set_item("unassign_label", &unassign_label)?;
            globals.set_item("assign_all_label", &assign_all_label)?;
            globals.set_item("unassign_all_label", &unassign_all_label)?;
            globals.set_item("duplicate_label", &duplicate_label)?;
            globals.set_item("student_update_label", &student_update_label)?;
            Ok(())
        },
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                        partial,
                        last_period,
                        false,
                    )),
                )
                .expect("a subject of the example can stop running on a period");
        },
    );

    // What the script asked for, as it stood when it saved: the first period's
    // partial row holds every student, the second period's rows are the first
    // period's minus the one student it excluded there, and the third period is
    // untouched.
    let mut written_out: BTreeMap<(PeriodId, SubjectId), BTreeSet<StudentId>> = params
        .assignments
        .iter()
        .map(|(period, subject, members)| ((period, subject), members.clone()))
        .collect();
    written_out.insert((first_period, partial), students.clone());
    for subject in &subject_ids {
        let mut copied = written_out
            .get(&(first_period, *subject))
            .cloned()
            .unwrap_or_default();
        copied.remove(&excluded);
        assert!(
            !copied.is_empty(),
            "a copied row keeps students, so none of them is stored as absent"
        );
        written_out.insert((second_period, *subject), copied);
    }

    let written = reload(&target);
    let after: BTreeMap<_, _> = written
        .get_inner_data()
        .params
        .assignments
        .iter()
        .map(|(period, subject, members)| ((period, subject), members.clone()))
        .collect();

    assert_eq!(after, written_out);

    // And the exclusion the script wrote through the students family, which is
    // what made the second period's rows differ from the first's.
    assert_eq!(
        written
            .get_inner_data()
            .params
            .students
            .student_map
            .get(&excluded)
            .expect("the excluded student is still in the document")
            .excluded_periods,
        BTreeSet::from([second_period]),
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The week patterns are added, rewritten and removed from python
///
/// The tenth family of the ops mirror, and the one whose removal *keeps* what
/// pointed at it: a slot follows a pattern to know which weeks it runs on and
/// an incompatibility follows one to know which weeks it applies on, and both
/// hold it in a field whose empty value is the legal « toutes les semaines ».
/// So the cascade clears the reference and the two rows survive, running every
/// week from then on — the divergence from the legacy cleaning that `colloscopes/ops/` pins
/// from the other side, said here from a script's.
///
/// The `update` cascades the other way about, into the colloscope: excluding a
/// week makes interrogations impossible on it for every slot following the
/// pattern, so the colles already written there go. The example has no
/// colloscope and the write surface has no way to make one yet — that family is
/// a later piece — so this runs in two stages, with rust writing the one colle
/// in between, the way [a_cascade_reports_every_repair_and_what_needed_it]
/// does.
///
/// Nothing else needs a fixture of its own: the example holds two patterns with
/// one slot each, which is what the removal is about, and no incompatibility
/// follows a pattern, which the script fixes for itself through `doc.incompats`
/// — a published mutator, and the write whose undo slot carries another
/// family's name.
///
/// Rust reads back the file the script saved after its last accepted write: the
/// two patterns the example did not have are the two the script asked for,
/// field by field.
#[test]
fn week_patterns_are_added_rewritten_and_removed() {
    use collomatique_ops::{
        ColloscopeUpdateOp, IncompatibilitiesUpdateOp, UpdateOp, WeekPatternsUpdateOp,
    };
    use collomatique_state_colloscopes::week_patterns::WeekPattern;

    let dir = workspace("week-patterns-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let pattern_ids: Vec<_> = params.week_patterns.week_pattern_map.keys().collect();
    assert!(
        pattern_ids.len() >= 2,
        "the script removes one pattern and narrows another"
    );

    // The slots following each pattern. The script finds them for itself, by
    // reading `slot.week_pattern`; what rust asserts is that there is exactly
    // one to find, since that is what the script's counting leans on.
    let followers = |pattern| {
        params
            .slots
            .all_slots()
            .filter(move |(_id, slot)| slot.week_pattern == Some(pattern))
            .map(|(id, _slot)| *id)
            .collect::<Vec<_>>()
    };
    for pattern in &pattern_ids {
        assert_eq!(
            followers(*pattern).len(),
            1,
            "every pattern of the example is followed by exactly one slot"
        );
    }
    assert!(
        params
            .incompats
            .incompat_map
            .values()
            .all(|incompat| incompat.week_pattern_id.is_none()),
        "no incompatibility of the example follows a pattern, so the script points one at it"
    );

    // The pattern the script removes, and the one it narrows in the second
    // stage. Handed over as places in the patterns' own order, since a script
    // reads its own ids off the document and rust cannot mint one for it.
    let followed_index = 0usize;
    let cell_pattern_index = 1usize;
    let cell_pattern = pattern_ids[cell_pattern_index];
    let cell_slot = followers(cell_pattern)[0];

    // The cell rust writes between the stages: the first week that slot is
    // really active on, which is where a colle may be written at all.
    let (cell_week_index, cell_week) = params
        .week_ids()
        .enumerate()
        .find(|(_index, week)| params.is_interrogation_possible(cell_slot, *week))
        .expect("the slot is active on at least one week of its pattern");

    // The first two weeks, which the script's `add` switches off, and the third,
    // which its first rewrite switches off instead.
    let early_weeks: Vec<_> = params.week_ids().take(3).collect();
    assert_eq!(
        early_weeks.len(),
        3,
        "the script names the first three weeks"
    );

    // The french labels the three operations carry, so that the script's undo
    // assertions pin the operations' own names and not merely some strings. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: WeekPatternsUpdateOp| op.get_desc().1;
    let some_pattern = pattern_ids[0];
    let blank = WeekPattern {
        name: String::new(),
        excluded_weeks: BTreeSet::new(),
    };
    let add_label = label(WeekPatternsUpdateOp::AddNewWeekPattern(blank.clone()));
    let update_label = label(WeekPatternsUpdateOp::UpdateWeekPattern(
        some_pattern,
        blank.clone(),
    ));
    let remove_label = label(WeekPatternsUpdateOp::DeleteWeekPattern(some_pattern));

    // The one write of another family the script makes: the incompatibility it
    // points at the doomed pattern, so that the removal has both of its kinds of
    // site to repair.
    let (some_incompat, incompat) = params
        .incompats
        .incompat_map
        .iter()
        .next()
        .expect("the example holds incompatibilities");
    let incompat_label = IncompatibilitiesUpdateOp::UpdateIncompat(some_incompat, incompat.clone())
        .get_desc()
        .1;

    run_stages(
        &[
            include_str!("scripts/week_patterns_write_before.py"),
            include_str!("scripts/week_patterns_write_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("target", &target)?;
            globals.set_item("followed_index", followed_index)?;
            globals.set_item("cell_pattern_index", cell_pattern_index)?;
            globals.set_item("cell_week_index", cell_week_index)?;
            globals.set_item("add_label", &add_label)?;
            globals.set_item("update_label", &update_label)?;
            globals.set_item("remove_label", &remove_label)?;
            globals.set_item("incompat_label", &incompat_label)?;
            Ok(())
        },
        |py, globals| {
            applied_write(
                py,
                globals,
                "prepared",
                UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                    cell_slot,
                    cell_week,
                    BTreeSet::from([0]),
                )),
            );
        },
    );

    // What the script's two adds asked for, as they stood when it saved: the
    // first was rewritten twice after being created, the second never touched.
    let written_out = vec![
        WeekPattern {
            name: "Semaines de rentrée".to_owned(),
            excluded_weeks: BTreeSet::from([early_weeks[0], early_weeks[1]]),
        },
        blank,
    ];

    // The document the script saved holds everything it opened with, plus the
    // two patterns it wrote — and those two are what it asked for.
    let written = reload(&target);
    let after = &written
        .get_inner_data()
        .params
        .week_patterns
        .week_pattern_map;
    let added: Vec<_> = after
        .iter()
        .filter(|(id, _pattern)| !pattern_ids.contains(id))
        .map(|(_id, pattern)| pattern.clone())
        .collect();

    assert_eq!(added, written_out);
    assert_eq!(after.len(), pattern_ids.len() + 2);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The slots are added, rewritten, moved and removed from python
///
/// The eleventh family of the ops mirror, and the first whose value is larger
/// than what its ops carry: a `SlotData` names its subject, and a slot cannot
/// change subject. So the `add` reads that field — `AddNewSlot` takes the
/// subject beside the slot payload — and the `update` refuses a value naming
/// another one, which is the `ValueError` the script pins beside the model's own
/// refusals. It is also the first family with a position, so it brings two
/// mutators nothing else has: `move_up` and `move_down`, whose ends of the list
/// refuse.
///
/// The removal cascade needs a colle, and a colloscope cell is written through
/// the colloscope family, which is a later piece — so this runs in two stages,
/// with rust writing the one colle in between, the way
/// [a_cascade_reports_every_repair_and_what_needed_it] does. The slot it writes
/// on is the antecedent of the example's first slot pairing rule, so the second
/// stage has a cell and a rule on one slot: the removal repairs both, and the
/// `update` that puts the slot on a narrowing pattern repairs the cell alone.
///
/// The first stage needs no fixture of its own: it finds the subjects that run
/// colles and the one that runs none, a teacher of the first and a stranger to
/// it, off the document. What rust asserts here is that the example really holds
/// those shapes.
///
/// Rust reads back the file the script saved after its last accepted write of
/// the first stage: the slot the example did not have is the one the script
/// asked for, field by field, and it sits last among its subject's slots.
#[test]
fn slots_are_added_rewritten_moved_and_removed() {
    use collomatique_ops::{ColloscopeUpdateOp, SlotsUpdateOp, UpdateOp};
    use collomatique_state_colloscopes::slots::Slot;

    let dir = workspace("slots-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The subjects the script names, in the order it walks them: the two first
    // that run colles, and the first that runs none — the second is what the
    // `SubjectHasNoInterrogation` refusal is asserted on.
    let with_colles: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, _subject)| id)
        .collect();
    assert!(
        with_colles.len() >= 2,
        "the script names two subjects that run colles"
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .any(|subject| subject.parameters.interrogation_parameters.is_none()),
        "the script needs a subject that runs no interrogations"
    );

    let held = with_colles[0];
    let slots_of_held: Vec<_> = params
        .slots
        .slots_for_subject(held)
        .into_iter()
        .flatten()
        .map(|(slot_id, _slot)| *slot_id)
        .collect();
    assert!(
        slots_of_held.len() >= 2,
        "the script moves a slot around this subject's list and refuses at both ends"
    );

    // The teacher the script builds its slot around, and the stranger to that
    // subject the `TeacherDoesNotTeachInSubject` refusal is asserted on. Both
    // are the first of their kind, which is what the script picks too.
    let teaches = params
        .teachers
        .teacher_map
        .iter()
        .find(|(_id, teacher)| teacher.subjects.contains(&held))
        .map(|(id, _teacher)| id)
        .expect("the example has a teacher of its first subject with colles");
    assert!(
        params
            .teachers
            .teacher_map
            .values()
            .any(|teacher| !teacher.subjects.contains(&held)),
        "the script needs a teacher who is a stranger to that subject"
    );

    let pattern = params
        .week_patterns
        .week_pattern_map
        .keys()
        .next()
        .expect("the example has week patterns");

    // The slot the second stage is about: the antecedent of the first slot
    // pairing rule. It must be named by that rule and by no other, since the
    // script asserts the removal's warning list entry by entry, and it must
    // carry no pattern of its own, since the script puts one on it.
    let cell_slot = params
        .slot_pairings
        .slot_pairing_rule_map
        .values()
        .next()
        .map(|rule| rule.antecedent().slot_id)
        .expect("the example has slot pairing rules");
    assert_eq!(
        params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .filter(|(_id, other)| other.antecedent().slot_id == cell_slot
                || other.consequent().slot_id == cell_slot)
            .count(),
        1,
        "exactly one rule names that slot, so the removal repairs exactly one",
    );
    assert_eq!(
        params
            .slots
            .find_slot(cell_slot)
            .expect("a rule names a live slot")
            .week_pattern,
        None,
        "the second stage puts a pattern on that slot, so it starts with none",
    );

    // The cell rust writes between the stages: the first week that slot is
    // really active on, which is where a colle may be written at all.
    let (cell_week_index, cell_week) = params
        .week_ids()
        .enumerate()
        .find(|(_index, week)| params.is_interrogation_possible(cell_slot, *week))
        .expect("the slot is active on at least one week");

    // The french labels the five operations carry, so that the script's undo
    // assertions pin the operations' own names and not merely some strings. Only
    // the variant is read, so the payloads below are the nearest ones to hand.
    let label = |op: SlotsUpdateOp| op.get_desc().1;
    let blank = params
        .slots
        .find_slot(cell_slot)
        .expect("a rule names a live slot")
        .clone();
    let add_label = label(SlotsUpdateOp::AddNewSlot(held, blank.clone()));
    let update_label = label(SlotsUpdateOp::UpdateSlot(cell_slot, blank));
    let remove_label = label(SlotsUpdateOp::DeleteSlot(cell_slot));
    let move_up_label = label(SlotsUpdateOp::MoveSlotUp(cell_slot));
    let move_down_label = label(SlotsUpdateOp::MoveSlotDown(cell_slot));

    run_stages(
        &[
            include_str!("scripts/slots_write_before.py"),
            include_str!("scripts/slots_write_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("target", &target)?;
            globals.set_item("cell_week_index", cell_week_index)?;
            globals.set_item("add_label", &add_label)?;
            globals.set_item("update_label", &update_label)?;
            globals.set_item("remove_label", &remove_label)?;
            globals.set_item("move_up_label", &move_up_label)?;
            globals.set_item("move_down_label", &move_down_label)?;
            Ok(())
        },
        |py, globals| {
            applied_write(
                py,
                globals,
                "prepared",
                UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                    cell_slot,
                    cell_week,
                    BTreeSet::from([0]),
                )),
            );
        },
    );

    // What the script's one add asked for, as it stood when it saved: created
    // with one value and rewritten twice, then moved up and back down again.
    let written_out = Slot {
        subject_id: held,
        teacher_id: teaches,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(8, 0, 0).expect("a real time of day"),
            )
            .expect("a whole minute"),
        },
        extra_info: String::new(),
        week_pattern: Some(pattern),
        cost: -2,
    };

    // The document the script saved holds everything it opened with, plus the
    // one slot it wrote — last in its subject's list, which is where the two
    // moves left it.
    let written = reload(&target);
    let after: Vec<_> = written
        .get_inner_data()
        .params
        .slots
        .slots_for_subject(held)
        .into_iter()
        .flatten()
        .map(|(slot_id, slot)| (*slot_id, slot.clone()))
        .collect();

    assert_eq!(after.len(), slots_of_held.len() + 1);
    let (added_id, added) = after.last().expect("the subject holds slots").clone();
    assert!(!slots_of_held.contains(&added_id));
    assert_eq!(added, written_out);
    assert_eq!(
        after
            .iter()
            .take(slots_of_held.len())
            .map(|(id, _slot)| *id)
            .collect::<Vec<_>>(),
        slots_of_held,
        "the two moves cancelled out, so the subject's own slots are where they were",
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
///.
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

/// The subject values go out through `to_data()` and come back in unchanged
///
/// The headline is the round trip: what each handle handed the script,
/// extracted again, is the subject the document holds — the name, the whole
/// interrogation record and the exclusions, in one comparison. The sub-view's
/// own values ride beside it, and so do the same fields as python saw them, so
/// that a conversion wrong in both directions at once cannot cancel itself out.
///
/// The rest is the other kinds this milestone's tests are made of: values
/// written out by hand, the defaults pinned against the model's own, and the
/// refusals with the sentence each one raises. The example carries both shapes
/// — subjects that run colles and two that do not — which is what makes the
/// `None` half of `interrogation` a real case here.
#[test]
fn the_subject_values_carry_the_interrogation_out_and_back() {
    let dir = workspace("subject-values");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/subject_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let subjects: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .map(|(_id, subject)| subject.clone())
        .collect();
    let with_colles: Vec<_> = subjects
        .iter()
        .filter_map(|subject| subject.parameters.interrogation_parameters.clone())
        .collect();

    // The example is only worth reading if it has something to say: both
    // shapes among the subjects, and nothing here relying on that by accident.
    assert!(!with_colles.is_empty());
    assert!(with_colles.len() < subjects.len());

    // Out and back, whole.
    assert_eq!(
        extracted_all::<SubjectData>(&globals, "subject_values"),
        subjects
    );
    assert_eq!(
        extracted_all::<InterrogationData>(&globals, "interrogation_values"),
        with_colles
    );

    // And the same fields as python saw them.
    assert_eq!(
        global::<Vec<String>>(&globals, "value_names"),
        subjects
            .iter()
            .map(|subject| subject.parameters.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "value_holds_colles"),
        subjects
            .iter()
            .map(|subject| subject.parameters.interrogation_parameters.is_some())
            .collect::<Vec<_>>()
    );
    let bounds = |range: &collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32>| {
        (range.start().get(), range.end().get())
    };
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "value_students_per_group"),
        with_colles
            .iter()
            .map(|params| bounds(&params.students_per_group))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "value_groups_per_interrogation"),
        with_colles
            .iter()
            .map(|params| bounds(&params.groups_per_interrogation))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<u32>>(&globals, "value_durations"),
        with_colles
            .iter()
            .map(|params| params.duration.get().get())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "value_take_duration"),
        with_colles
            .iter()
            .map(|params| params.take_duration_into_account)
            .collect::<Vec<_>>()
    );

    // Built by hand: the value the script wrote out is the subject expected of
    // it, whether the period was named by handle, by id, or in a list. Nothing
    // but the name and the exclusion is given, so the interrogation record that
    // comes out is the model's own default.
    let first_period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");
    let spe_maths = collomatique_state_colloscopes::Subject {
        parameters: collomatique_state_colloscopes::SubjectParameters {
            name: "Spé maths".to_owned(),
            ..Default::default()
        },
        excluded_periods: BTreeSet::from([first_period]),
    };
    for name in ["by_handle", "by_id", "by_list"] {
        assert_eq!(extracted::<SubjectData>(&globals, name), spe_maths);
    }

    // One written out from end to end, so that no field can pass by being left
    // at the value the model would have put there anyway.
    assert_eq!(
        extracted::<SubjectData>(&globals, "written_out"),
        periodicity_subject(
            "Options",
            (1, 2),
            (2, 2),
            90,
            false,
            collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year: plain_range((0, 4)),
                minimum_week_separation: 3,
            },
            BTreeSet::from([first_period]),
        )
    );

    // And the subject that holds no colles at all, which is what an explicit
    // `interrogation=None` means.
    assert_eq!(
        extracted::<SubjectData>(&globals, "no_colles"),
        collomatique_state_colloscopes::Subject {
            parameters: collomatique_state_colloscopes::SubjectParameters {
                name: "Quidditch".to_owned(),
                interrogation_parameters: None,
            },
            excluded_periods: BTreeSet::new(),
        }
    );

    // The defaults, pinned against the model's own: with the one required field
    // set to what the model's empty subject holds, the whole value is the
    // model's `Default` — colles included, since that is what it holds. These
    // are the assertions that stop the python-side defaults drifting from the
    // rust ones.
    assert_eq!(
        extracted::<SubjectData>(&globals, "bare_subject"),
        collomatique_state_colloscopes::Subject::default()
    );
    assert_eq!(
        extracted::<InterrogationData>(&globals, "bare_interrogation"),
        collomatique_state_colloscopes::SubjectInterrogationParameters::default()
    );

    // The refusals, each with the sentence it raises. The class a message names
    // is the one the script wrote down, and the field is the path from it — a
    // duration nested in a subject is named through the subject.
    assert_eq!(
        refused::<SubjectData>(&globals, "zero_duration"),
        (
            "ValueError".to_owned(),
            "a SubjectData's interrogation.duration is at least 1, and 0 was given".to_owned(),
        )
    );
    assert_eq!(
        refused::<SubjectData>(&globals, "inverted_range"),
        (
            "ValueError".to_owned(),
            "a SubjectData's interrogation.students_per_group is a (min, max) range, \
             and 3 is above 2"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<SubjectData>(&globals, "empty_group"),
        (
            "ValueError".to_owned(),
            "a SubjectData's interrogation.students_per_group counts from 1 at both ends, \
             and 0 was given"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<SubjectData>(&globals, "not_a_periodicity"),
        (
            "TypeError".to_owned(),
            "a SubjectData's interrogation.periodicity is a Periodicity, and 3 is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<SubjectData>(&globals, "not_a_name"),
        (
            "TypeError".to_owned(),
            "a SubjectData's name is a string, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<SubjectData>(&globals, "not_an_interrogation"),
        (
            "TypeError".to_owned(),
            "a SubjectData is expected here, and 3 has no interrogation.students_per_group"
                .to_owned(),
        )
    );

    // The same field, refused on the class handed over whole: what the script
    // wrote there is an `InterrogationData`, so that is what the message names
    // — and english gets its article right.
    assert_eq!(
        refused::<InterrogationData>(&globals, "bare_zero_duration"),
        (
            "ValueError".to_owned(),
            "an InterrogationData's duration is at least 1, and 0 was given".to_owned(),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<SubjectData>(&globals, "foreign_period");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// All four periodicities travel back in, and the exclusion with them
///
/// The example holds two of the four kinds and excludes no period from any
/// subject, so this is
/// [the_four_periodicities_read_back_value_by_value]'s document again — the one
/// place where `SubjectData.excluded_periods`, the field §2.0 of the design is
/// about, has something in it to carry.
///
/// Both halves are here: the round trip, which says a periodicity read out of a
/// document goes back in as the same one, and four values written from scratch
/// in python, which says a script can build each kind without ever having read
/// one.
#[test]
fn the_subject_values_carry_all_four_periodicities() {
    let dir = workspace("subject-values-periodicities");
    let source = dir.join("periodicities.collomatique");
    periodicity_document(&source);

    let globals = run(
        include_str!("scripts/subject_data_periodicities.py"),
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
    );

    let data = reload(&source);
    let subjects: Vec<_> = data
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .map(|(_id, subject)| subject.clone())
        .collect();

    // The fixture is worth reading here because no two of its subjects share a
    // periodicity, and one of them skips a period.
    assert_eq!(subjects.len(), 4);
    assert!(
        subjects
            .iter()
            .any(|subject| !subject.excluded_periods.is_empty())
    );

    assert_eq!(
        extracted_all::<SubjectData>(&globals, "subject_values"),
        subjects
    );

    // The four written out by hand carry the same periodicities the document
    // holds, which is what says the conversion in is a conversion and not a
    // copy of the object that came out.
    let periodicities: Vec<_> = subjects
        .iter()
        .map(|subject| {
            subject
                .parameters
                .interrogation_parameters
                .as_ref()
                .expect("every subject of the fixture holds colles")
                .periodicity
                .clone()
        })
        .collect();
    assert_eq!(
        extracted_all::<InterrogationData>(&globals, "hand_built")
            .into_iter()
            .map(|params| params.periodicity)
            .collect::<Vec<_>>(),
        periodicities
    );

    // A block list may be empty, as it may in the model — a subject nobody is
    // ever interrogated in, which is odd rather than wrong.
    assert_eq!(
        extracted::<InterrogationData>(&globals, "no_block").periodicity,
        collomatique_state_colloscopes::SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks: Vec::new(),
            minimum_week_separation: 0,
        }
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed period stales the values that name it, and the two deaths of a
/// sub-view each stale its `to_data()`
///
/// Three removals between the two stages, because there are three failures to
/// tell apart: a value holding a dead period's id names nothing and is refused
/// on extraction; a `Subject` handle whose subject is gone refuses the read
/// `to_data()` is; and an `Interrogation` sub-view refuses it in each of its
/// own two ways — the subject removed, and the subject merely no longer holding
/// colles. The last one is the case where the *subject's* value keeps working
/// and answers `None`.
///
/// The mutations come from rust: the write surface does not exist yet, and this
/// is what `run_stages` is for.
#[test]
fn a_removed_period_stales_the_subject_values_that_name_it() {
    let dir = workspace("subject-values-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let with_colles: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, subject)| (id, subject.parameters.clone()))
        .collect();
    assert!(
        with_colles.len() > 1,
        "the two subjects this test kills differently must not be the same one"
    );

    // The last one that runs colles is removed, and the first one keeps its
    // place while losing its colles.
    let doomed_index = with_colles.len() - 1;
    let switched_off_index = 0;
    let doomed = with_colles[doomed_index].0;
    let mut without_colles = with_colles[switched_off_index].1.clone();
    without_colles.interrogation_parameters = None;
    let switched_off = with_colles[switched_off_index].0;

    let doomed_period = params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");

    let globals = run_stages(
        &[
            include_str!("scripts/subject_data_stale_before.py"),
            include_str!("scripts/subject_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("doomed_index", doomed_index)?;
            globals.set_item("switched_off_index", switched_off_index)?;
            Ok(())
        },
        // Two scripts, so this runs once — and it makes all three changes,
        // because the failures they cause are the halves of one test.
        |py, globals| {
            let doc = document_of(globals);
            let apply = |op| {
                doc.borrow_mut(py)
                    .update(py, op)
                    .expect("the example takes these three changes");
            };

            apply(collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed),
            ));
            apply(collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::UpdateSubject(
                    switched_off,
                    without_colles.clone(),
                ),
            ));
            apply(collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::DeletePeriodAndWeeks(doomed_period),
            ));
        },
    );

    // The two values naming the dead period no longer name anything, and it
    // makes no difference whether the script wrote the handle or the id.
    for name in ["naming_the_dead_by_handle", "naming_the_dead_by_id"] {
        let (kind, _message) = refused::<SubjectData>(&globals, name);
        assert_eq!(kind, "StaleHandleError", "`{name}` should be refused");
    }

    // And the one naming a period that survived still extracts.
    let still_good = extracted::<SubjectData>(&globals, "naming_the_living");
    assert_eq!(still_good.excluded_periods.len(), 1);

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
///.
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

/// The people values go out through `to_data()` and come back in unchanged
///
/// The headline is the round trip: what each handle handed the script,
/// extracted again, is the teacher the document holds — every field, in one
/// comparison. Beside it the same fields are compared as python saw them, so
/// that a conversion wrong in both directions at once cannot cancel itself out.
///
/// The rest is the other four kinds this milestone's tests are made of: a value
/// written out by hand, the defaults pinned against the model's own, the
/// refusals with the sentence each one raises, and the entity field taking a
/// handle and an id alike.
#[test]
fn the_people_values_carry_the_card_out_and_back() {
    let dir = workspace("people-values");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/people_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

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

    // Out and back, whole.
    assert_eq!(
        extracted_all::<TeacherData>(&globals, "teacher_values"),
        teachers
    );
    assert_eq!(
        extracted_all::<StudentData>(&globals, "student_values"),
        students
    );

    // And the same fields as python saw them.
    assert_eq!(
        global::<Vec<String>>(&globals, "value_firstnames"),
        teachers
            .iter()
            .map(|teacher| teacher.desc.firstname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "value_surnames"),
        teachers
            .iter()
            .map(|teacher| teacher.desc.surname.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "value_tels"),
        teachers
            .iter()
            .map(|teacher| optional_text(&teacher.desc.tel))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "value_emails"),
        teachers
            .iter()
            .map(|teacher| optional_text(&teacher.desc.email))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "student_value_tels"),
        students
            .iter()
            .map(|student| optional_text(&student.desc.tel))
            .collect::<Vec<_>>()
    );

    // The example is only worth reading if it has something to say: an empty
    // firstname, a teacher with no number, a teacher with one.
    assert!(
        teachers
            .iter()
            .any(|teacher| teacher.desc.firstname.is_empty())
    );
    assert!(teachers.iter().any(|teacher| teacher.desc.tel.is_none()));
    assert!(teachers.iter().any(|teacher| teacher.desc.tel.is_some()));

    // Built by hand: the value the script wrote out is the teacher expected of
    // it, whether the subject was named by handle, by id, or in a list.
    let first_subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .next()
        .expect("the example has subjects");
    let noether = collomatique_state_colloscopes::teachers::Teacher {
        desc: person("Emmy", "Noether", None, Some("noether@lycee.fr")),
        subjects: BTreeSet::from([first_subject]),
    };
    assert_eq!(extracted::<TeacherData>(&globals, "by_handle"), noether);
    assert_eq!(extracted::<TeacherData>(&globals, "by_id"), noether);
    assert_eq!(extracted::<TeacherData>(&globals, "by_list"), noether);

    // The defaults, pinned against the model's own: with the two required
    // fields set to what the model's empty card holds, the whole value is the
    // model's own `Default`. This is the assertion that stops the python-side
    // defaults drifting from the rust ones.
    assert_eq!(
        extracted::<TeacherData>(&globals, "bare_teacher"),
        collomatique_state_colloscopes::teachers::Teacher::default()
    );
    assert_eq!(
        extracted::<StudentData>(&globals, "bare_student"),
        collomatique_state_colloscopes::students::Student::default()
    );

    // The refusals, each with the sentence it raises. `''` is not `None`
    // wherever the model types the field as an optional non-empty string, and
    // the message names the class and the field so a script knows which line to
    // look at.
    assert_eq!(
        refused::<TeacherData>(&globals, "empty_tel"),
        (
            "ValueError".to_owned(),
            "a TeacherData's tel is a non-empty string or None, and '' is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<TeacherData>(&globals, "empty_email"),
        (
            "ValueError".to_owned(),
            "a TeacherData's email is a non-empty string or None, and '' is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<StudentData>(&globals, "empty_student_tel"),
        (
            "ValueError".to_owned(),
            "a StudentData's tel is a non-empty string or None, and '' is neither".to_owned(),
        )
    );

    // A field that is not the kind it says it is fails before anything else
    // does, and says which field it was.
    let (kind, message) = refused::<TeacherData>(&globals, "not_a_name");
    assert_eq!(kind, "TypeError");
    assert_eq!(
        message,
        "a TeacherData's firstname is a string, and 3 is not one"
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<TeacherData>(&globals, "foreign_subject");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A person who shared nothing, and a set that runs from empty to whole
///
/// The example gives everybody at least one contact detail, every teacher a
/// subject, and no student a period to sit out, so the shapes at the two ends
/// need the fixture `a_person_who_shared_nothing_reads_as_none` already builds.
/// What this pins is that the sets carry what the model holds — the round trip
/// through `from_py` says so field by field — and that an empty one is a set
/// rather than a `None`.
#[test]
fn a_value_carries_the_empty_sets_and_the_missing_contacts() {
    let dir = workspace("people-values-contacts");
    let source = dir.join("contacts.collomatique");
    contact_document(&source);

    let globals = run(include_str!("scripts/people_data_contacts.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

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

    assert_eq!(
        extracted_all::<TeacherData>(&globals, "teacher_values"),
        teachers
    );
    assert_eq!(
        extracted_all::<StudentData>(&globals, "student_values"),
        students
    );

    assert_eq!(
        global::<Vec<usize>>(&globals, "subject_counts"),
        teachers
            .iter()
            .map(|teacher| teacher.subjects.len())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "excluded_counts"),
        students
            .iter()
            .map(|student| student.excluded_periods.len())
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed subject stales the values that name it, and a removed teacher
/// stales `to_data()`
///
/// Two different failures, so two different removals. A value holds an id, and
/// an id whose entity is gone names nothing: extracting such a value raises
/// `StaleHandleError`, whether the script wrote the handle or the id, and it is
/// the same refusal every method of this api already makes. `to_data()` is a
/// read, so a dead handle refuses it like any other read.
///
/// The removals come from rust: the write surface does not exist yet, and this
/// is what `run_stages` is for.
#[test]
fn a_removed_entity_stales_the_values_that_name_it() {
    let dir = workspace("people-values-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let subject_ids: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    let teachers: Vec<_> = params.teachers.teacher_map.iter().collect();

    // A subject at least one teacher interrogates in, and a teacher who does
    // not — so the survivor the script looks at is not the one being removed.
    let (doomed_subject_index, doomed_subject) = subject_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (index, *id))
        .find(|(_index, id)| {
            teachers
                .iter()
                .any(|(_teacher_id, teacher)| teacher.subjects.contains(id))
        })
        .expect("some teacher of the example interrogates in some subject");
    let (doomed_teacher_index, doomed_teacher) = teachers
        .iter()
        .enumerate()
        .find(|(_index, (_id, teacher))| !teacher.subjects.contains(&doomed_subject))
        .map(|(index, (id, _teacher))| (index, *id))
        .expect("some teacher of the example does not interrogate in that subject");

    let globals = run_stages(
        &[
            include_str!("scripts/people_data_stale_before.py"),
            include_str!("scripts/people_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("doomed_subject_index", doomed_subject_index)?;
            globals.set_item("doomed_teacher_index", doomed_teacher_index)?;
            Ok(())
        },
        // Two scripts, so this runs once — and it makes both removals, because
        // the two failures they cause are the two halves of one test.
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Subjects(
                        collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed_subject),
                    ),
                )
                .expect("a subject of the example is removable");
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Teachers(
                        collomatique_ops::TeachersUpdateOp::DeleteTeacher(doomed_teacher),
                    ),
                )
                .expect("a teacher of the example is removable");
        },
    );

    // The two values naming the dead subject no longer name anything, and it
    // makes no difference whether the script wrote the handle or the id.
    for name in ["naming_the_dead_by_handle", "naming_the_dead_by_id"] {
        let (kind, _message) = refused::<TeacherData>(&globals, name);
        assert_eq!(kind, "StaleHandleError", "`{name}` should be refused");
    }

    // And the one naming a subject that survived still extracts.
    let still_good = extracted::<TeacherData>(&globals, "naming_the_living");
    assert_eq!(still_good.subjects.len(), 1);

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
///.
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

/// The pattern and slot values go out through `to_data()` and come back in
/// unchanged
///
/// The headline is the round trip: what each handle handed the script,
/// extracted again, is the pattern and the slot the document holds — every
/// field of each, in one comparison per collection. The same fields as python
/// saw them ride beside it, so that a conversion wrong in both directions at
/// once cannot cancel itself out; the entities among them are named by their
/// place in the walk they belong to, since an id means nothing written down.
///
/// The rest is the other kinds this milestone's tests are made of: values
/// written out by hand, entity fields taking a handle and an id alike, and the
/// refusals with the sentence each one raises. Neither model type has a
/// `Default`, so there is no default to pin here — what §2.5 of the design
/// asks for is a pin per class whose model has one.
#[test]
fn the_pattern_and_slot_values_carry_the_start_time_out_and_back() {
    let dir = workspace("slot-values");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/slot_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // In id order for the patterns, and in the subjects-then-position walk for
    // the slots: the two orders `doc.week_patterns` and `doc.slots` promise,
    // which are the orders the script says it saw.
    let patterns: Vec<_> = params
        .week_patterns
        .week_pattern_map
        .iter()
        .map(|(id, pattern)| (id, pattern.clone()))
        .collect();
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
        .map(|(_slot_id, slot)| slot.clone())
        .collect();
    let week_walk: Vec<_> = params.week_ids().collect();

    // The example is only worth reading if it has something to say: several
    // patterns, every one of them switching something off, and slots of both
    // shapes — some carrying a pattern and some not — with costs that are not
    // all the same.
    assert!(patterns.len() > 1);
    assert!(
        patterns
            .iter()
            .all(|(_id, pattern)| !pattern.excluded_weeks.is_empty())
    );
    assert!(walk.iter().any(|slot| slot.week_pattern.is_some()));
    assert!(walk.iter().any(|slot| slot.week_pattern.is_none()));
    assert!(walk.iter().any(|slot| slot.cost != walk[0].cost));

    // Out and back, whole.
    assert_eq!(
        extracted_all::<WeekPatternData>(&globals, "pattern_values"),
        patterns
            .iter()
            .map(|(_id, pattern)| pattern.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(extracted_all::<SlotData>(&globals, "slot_values"), walk);

    // And the same fields as python saw them.
    let week_position = |week: &collomatique_state_colloscopes::WeekId| {
        week_walk
            .iter()
            .position(|id| id == week)
            .expect("an excluded week is a live one")
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "value_pattern_names"),
        patterns
            .iter()
            .map(|(_id, pattern)| pattern.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "value_excluded_week_indices"),
        patterns
            .iter()
            .map(|(_id, pattern)| {
                let mut indices: Vec<_> =
                    pattern.excluded_weeks.iter().map(week_position).collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "value_subject_indices"),
        walk.iter()
            .map(|slot| params
                .subjects
                .find_subject_position(slot.subject_id)
                .expect("a slot names a live subject"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "value_teacher_surnames"),
        walk.iter()
            .map(|slot| params
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
        global::<Vec<String>>(&globals, "value_weekdays"),
        walk.iter()
            .map(|slot| weekday_name(slot.start_time.weekday).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<chrono::NaiveTime>>(&globals, "value_start_times"),
        walk.iter()
            .map(|slot| *slot.start_time.start_time.inner())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "value_extra_info"),
        walk.iter()
            .map(|slot| slot.extra_info.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<usize>>>(&globals, "value_pattern_indices"),
        walk.iter()
            .map(|slot| slot.week_pattern.map(|pattern_id| patterns
                .iter()
                .position(|(id, _pattern)| *id == pattern_id)
                .expect("a slot names a live pattern")))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<i32>>(&globals, "value_costs"),
        walk.iter().map(|slot| slot.cost).collect::<Vec<_>>()
    );

    // Built by hand: the value the script wrote out is the pattern expected of
    // it, whether the week was named by handle, by id, or in a list.
    let first_week = *week_walk.first().expect("the example has weeks");
    let even_weeks = collomatique_state_colloscopes::week_patterns::WeekPattern {
        name: "Semaines paires".to_owned(),
        excluded_weeks: BTreeSet::from([first_week]),
    };
    for name in ["pattern_by_handle", "pattern_by_id", "pattern_by_list"] {
        assert_eq!(extracted::<WeekPatternData>(&globals, name), even_weeks);
    }

    // A pattern that excludes nothing and was never named: both ends of what a
    // pattern can be, and neither of them a value the boundary refuses.
    assert_eq!(
        extracted::<WeekPatternData>(&globals, "bare_pattern"),
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: String::new(),
            excluded_weeks: BTreeSet::new(),
        }
    );

    // A slot written out from end to end, so that no field can pass by being
    // left at the value the model would have put there anyway — and written
    // twice, once naming its three entities by handle and once by id.
    let first_subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .next()
        .expect("the example has subjects");
    let first_teacher = params
        .teachers
        .teacher_map
        .keys()
        .next()
        .expect("the example has teachers");
    let first_pattern = patterns.first().expect("the example has patterns").0;
    let at = |hour: u32, minute: u32| {
        collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("a real time of day"),
        )
        .expect("a whole minute")
    };
    let written_out = collomatique_state_colloscopes::slots::Slot {
        subject_id: first_subject,
        teacher_id: first_teacher,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Thu),
            start_time: at(14, 0),
        },
        extra_info: "Salle 12".to_owned(),
        week_pattern: Some(first_pattern),
        cost: -3,
    };
    for name in ["slot_by_handle", "slot_by_id"] {
        assert_eq!(extracted::<SlotData>(&globals, name), written_out);
    }

    // And one carrying nothing but the four fields a slot cannot do without:
    // no extra info, every week, and no cost.
    assert_eq!(
        extracted::<SlotData>(&globals, "bare_slot"),
        collomatique_state_colloscopes::slots::Slot {
            subject_id: first_subject,
            teacher_id: first_teacher,
            start_time: collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
                start_time: at(8, 0),
            },
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<WeekPatternData>(&globals, "not_a_pattern_name"),
        (
            "TypeError".to_owned(),
            "a WeekPatternData's name is a string, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotData>(&globals, "not_a_weekday"),
        (
            "TypeError".to_owned(),
            "a SlotData's weekday is a Weekday, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotData>(&globals, "not_a_time"),
        (
            "TypeError".to_owned(),
            "a SlotData's start_time is a time of day, and '8h00' is not one".to_owned(),
        )
    );

    // The model's own precision, refused in the model's own words: a
    // `datetime.time` counts microseconds and the document does not, so a time
    // carrying either is not one this document can hold.
    for name in ["seconds_in_the_time", "microseconds_in_the_time"] {
        assert_eq!(
            refused::<SlotData>(&globals, name),
            (
                "ValueError".to_owned(),
                "a SlotData's start_time is a whole minute, with no seconds or microseconds"
                    .to_owned(),
            )
        );
    }

    assert_eq!(
        refused::<SlotData>(&globals, "not_an_extra_info"),
        (
            "TypeError".to_owned(),
            "a SlotData's extra_info is a string, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotData>(&globals, "not_a_cost"),
        (
            "TypeError".to_owned(),
            "a SlotData's cost is a whole number, and 'cher' is not one".to_owned(),
        )
    );

    // A field naming an entity refuses with `argument`'s own sentence, so a
    // script meets the same words here as anywhere else it passes something
    // that was never a reference to this document.
    assert_eq!(
        refused::<SlotData>(&globals, "not_a_subject"),
        (
            "TypeError".to_owned(),
            "a subject argument takes a Subject or a SubjectId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotData>(&globals, "not_a_pattern"),
        (
            "TypeError".to_owned(),
            "a week pattern argument takes a WeekPattern or a WeekPatternId, \
             and 'Semaines paires' is neither"
                .to_owned(),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<SlotData>(&globals, "foreign_teacher");
    assert_eq!(kind, "StaleHandleError");
    let (kind, _message) = refused::<WeekPatternData>(&globals, "foreign_week");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A pattern value carries the empty set, the whole one, and the missing name
///
/// The example cannot say this: all of its patterns carry a name and all of
/// them switch something off. So this is
/// [a_pattern_excludes_no_week_every_week_or_the_ones_it_names]'s document
/// again — the one built for the three ends of what a pattern can be — read
/// through `to_data()` this time, and written back in from python.
#[test]
fn the_pattern_values_carry_the_empty_set_and_the_whole_one() {
    let dir = workspace("pattern-values");
    let source = dir.join("exclusions.collomatique");
    week_pattern_document(&source);

    let globals = run(include_str!("scripts/pattern_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let patterns: Vec<_> = params
        .week_patterns
        .week_pattern_map
        .iter()
        .map(|(_id, pattern)| pattern.clone())
        .collect();
    let week_walk: Vec<_> = params.week_ids().collect();

    // The fixture is worth reading here for exactly the three shapes the
    // example has not got.
    assert!(
        patterns
            .iter()
            .any(|pattern| pattern.excluded_weeks.is_empty())
    );
    assert!(
        patterns
            .iter()
            .any(|pattern| pattern.excluded_weeks.len() == week_walk.len())
    );
    assert!(patterns.iter().any(|pattern| pattern.name.is_empty()));

    assert_eq!(
        extracted_all::<WeekPatternData>(&globals, "pattern_values"),
        patterns
    );
    assert_eq!(
        global::<Vec<String>>(&globals, "value_names"),
        patterns
            .iter()
            .map(|pattern| pattern.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<usize>>>(&globals, "value_excluded_week_indices"),
        patterns
            .iter()
            .map(|pattern| {
                let mut indices: Vec<_> = pattern
                    .excluded_weeks
                    .iter()
                    .map(|week| {
                        week_walk
                            .iter()
                            .position(|id| id == week)
                            .expect("an excluded week is a live one")
                    })
                    .collect();
                indices.sort();
                indices
            })
            .collect::<Vec<_>>()
    );

    // The same two ends, written out in python rather than read out of the
    // document.
    assert_eq!(
        extracted::<WeekPatternData>(&globals, "excluding_nothing"),
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: "Toutes les semaines".to_owned(),
            excluded_weeks: BTreeSet::new(),
        }
    );
    assert_eq!(
        extracted::<WeekPatternData>(&globals, "excluding_everything"),
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: "Aucune semaine".to_owned(),
            excluded_weeks: week_walk.iter().copied().collect(),
        }
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed slot, pattern and week stale the values that name them
///
/// Three removals between the two stages, because there are three failures to
/// tell apart: a `Slot` handle whose slot is gone refuses the read `to_data()`
/// is, a `WeekPattern` handle does the same, and a value holding a dead
/// reference — a pattern in a `SlotData`, a week in a `WeekPatternData` — names
/// nothing and is refused on extraction. The slot that survives keeps working
/// and its value still names its own subject.
///
/// The mutations come from rust: the write surface does not exist yet, and this
/// is what `run_stages` is for.
#[test]
fn a_removed_pattern_or_week_stales_the_slot_values_that_name_it() {
    let dir = workspace("slot-values-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The same three ends the script picks: the last slot of the walk, the last
    // pattern in id order, and the last period with every week in it.
    let doomed_slot = params
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
        .map(|(slot_id, _slot)| *slot_id)
        .last()
        .expect("the example has slots");
    let doomed_pattern = params
        .week_patterns
        .week_pattern_map
        .keys()
        .last()
        .expect("the example has week patterns");
    let doomed_period = params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");

    let globals = run_stages(
        &[
            include_str!("scripts/slot_data_stale_before.py"),
            include_str!("scripts/slot_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        // Two scripts, so this runs once — and it makes all three changes,
        // because the failures they cause are the halves of one test.
        |py, globals| {
            let doc = document_of(globals);
            let apply = |op| {
                doc.borrow_mut(py)
                    .update(py, op)
                    .expect("the example takes these three changes");
            };

            apply(collomatique_ops::UpdateOp::Slots(
                collomatique_ops::SlotsUpdateOp::DeleteSlot(doomed_slot),
            ));
            apply(collomatique_ops::UpdateOp::WeekPatterns(
                collomatique_ops::WeekPatternsUpdateOp::DeleteWeekPattern(doomed_pattern),
            ));
            apply(collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::DeletePeriodAndWeeks(doomed_period),
            ));
        },
    );

    // The values naming the dead pattern no longer name anything, and it makes
    // no difference whether the script wrote the handle or the id.
    for name in [
        "naming_the_dead_pattern_by_handle",
        "naming_the_dead_pattern_by_id",
    ] {
        let (kind, _message) = refused::<SlotData>(&globals, name);
        assert_eq!(kind, "StaleHandleError", "`{name}` should be refused");
    }
    let (kind, _message) = refused::<WeekPatternData>(&globals, "naming_the_dead_week");
    assert_eq!(kind, "StaleHandleError");

    // And the ones naming only what survived still extract — the slot that
    // carries no pattern at all had nothing to lose here.
    assert_eq!(
        extracted::<SlotData>(&globals, "naming_no_pattern").week_pattern,
        None
    );
    assert_eq!(
        extracted::<WeekPatternData>(&globals, "naming_the_living_week")
            .excluded_weeks
            .len(),
        1
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A week value round-trips, and names its own period
///
/// The example is worth reading here for the four shapes the weeks of a real
/// colloscope have: a week that runs colles and one that does not, an
/// annotation and a week without one. `to_data()` must carry all four, and
/// the derived `.index` and `.monday` must stay the handle's — a value that
/// stored them could contradict itself, since both are computed from the
/// week's place and the document's start date.
#[test]
fn the_week_values_round_trip_and_name_their_period() {
    let dir = workspace("week-values");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/week_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let walk: Vec<_> = params
        .walk_weeks()
        .map(|(period, id, week)| (period, id, week.clone()))
        .collect();
    let period_ids: Vec<_> = params.periods.period_ids().collect();

    // The example is only worth reading if it has something to say: weeks of
    // both flags, and annotations that are sometimes absent.
    assert!(walk.iter().any(|(_p, _id, week)| week.interrogations));
    assert!(walk.iter().any(|(_p, _id, week)| !week.interrogations));
    assert!(walk.iter().any(|(_p, _id, week)| week.annotation.is_some()));
    assert!(walk.iter().any(|(_p, _id, week)| week.annotation.is_none()));

    // Out and back, whole: the owning period, the flag, the annotation.
    assert_eq!(
        extracted_all::<WeekData>(&globals, "week_values"),
        walk.iter()
            .map(|(_period, _id, week)| week.clone())
            .collect::<Vec<_>>()
    );

    // And the same fields as python saw them.
    assert_eq!(
        global::<Vec<usize>>(&globals, "value_period_indices"),
        walk.iter()
            .map(|(period, _id, _week)| period_ids
                .iter()
                .position(|id| id == period)
                .expect("a walked week names a live period"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<bool>>(&globals, "value_interrogations"),
        walk.iter()
            .map(|(_period, _id, week)| week.interrogations)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Option<String>>>(&globals, "value_annotations"),
        walk.iter()
            .map(|(_period, _id, week)| week.annotation.as_ref().map(|text| text.to_string()))
            .collect::<Vec<_>>()
    );

    // The handle-or-id pair extracts to the same week, though the two python
    // objects do not compare equal — the wart §2.3 records.
    assert_eq!(
        extracted::<WeekData>(&globals, "week_by_handle"),
        extracted::<WeekData>(&globals, "week_by_id")
    );

    // Nothing but the period, so the defaulted two come out as the model's
    // own: a week that runs colles, and no annotation.
    assert_eq!(
        extracted::<WeekData>(&globals, "bare_week"),
        collomatique_state_colloscopes::weeks::Week {
            period_id: walk[0].0,
            interrogations: true,
            annotation: None,
        }
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<WeekData>(&globals, "not_a_period"),
        (
            "TypeError".to_owned(),
            "a period argument takes a Period or a PeriodId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<WeekData>(&globals, "not_a_flag"),
        (
            "TypeError".to_owned(),
            "a WeekData's interrogations is True or False, and 1 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<WeekData>(&globals, "not_an_annotation"),
        (
            "TypeError".to_owned(),
            "a WeekData's annotation is a string or None, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<WeekData>(&globals, "empty_annotation"),
        (
            "ValueError".to_owned(),
            "a WeekData's annotation is a non-empty string or None, and '' is neither".to_owned(),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<WeekData>(&globals, "foreign_period");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed period stales the week values that name it
///
/// The mutation between the two stages is the one the pattern and slot tests
/// already use, because there are two failures to tell apart: a `Week` handle
/// whose week is gone refuses the read `to_data()` is, and a value holding a
/// dead reference — a period in a `WeekData` — names nothing and is refused
/// on extraction. The week that survives keeps working and its value still
/// names its own period.
///
/// The mutation comes from rust: the write surface does not exist yet, and
/// this is what `run_stages` is for.
#[test]
fn a_removed_week_stales_the_week_values_that_name_it() {
    let dir = workspace("week-values-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entities the script is
    // holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The same end the script picks: the last period, whose weeks are all
    // about to go. The week the script doomed is among them.
    let doomed_period = params
        .periods
        .period_ids()
        .last()
        .expect("the example has periods");

    let globals = run_stages(
        &[
            include_str!("scripts/week_data_stale_before.py"),
            include_str!("scripts/week_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        // Two scripts, so this runs once — and it removes the whole last
        // period, because the week the script doomed is in it.
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::GeneralPlanning(
                        collomatique_ops::GeneralPlanningUpdateOp::DeletePeriodAndWeeks(
                            doomed_period,
                        ),
                    ),
                )
                .expect("the example takes this change");
        },
    );

    // The value naming the dead period no longer names anything, and the
    // values naming what survived still extract.
    let (kind, _message) = refused::<WeekData>(&globals, "naming_the_dead_period");
    assert_eq!(kind, "StaleHandleError");
    assert_eq!(
        extracted::<WeekData>(&globals, "naming_the_living_period").period_id,
        params.walk_weeks().next().expect("the example has weeks").0
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}
///
/// The example's table is complete — every subject holds a row on every
/// period — so the absent-address shape, the empty frozenset of a valid pair
/// no row is stored for, needs a document of its own: three rows on six
/// possible pairs, one subject with no row at all, and a second period that
/// does not repeat the first's rows. It is built as an `InnerData` through the
/// sealed types' own constructors and passed through `Data::from_inner_data`,
/// so a fixture that breaks an invariant fails here rather than halfway
/// through the script.
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
/// the dangling rows away (`colloscopes/ops/src/subjects.rs`), which is what makes the
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
    // the ops crate's tests (`colloscopes/ops/src/subjects.rs`).

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

/// The incompatibility values carry the busy windows out and back
///
/// The script walks `doc.incompats` and detaches each incompatibility with
/// `to_data()`; rust compares the values it left in the globals against the
/// same document read straight from the model — the names, the subjects, every
/// window of every list, the minimums and the patterns. The `TimeSlot` leaf
/// value makes the trip back *in* as a list element, which is the half of its
/// travel the read surface never exercised.
///
/// The example holds six incompatibilities across two subjects, one with more
/// than a single busy window, all bound to no week pattern — enough to pin the
/// walk and the `None` shape of `week_pattern`. The `Some` shape comes in only
/// through the hand-built value: the fixture has no incompatibility carrying a
/// pattern, so nothing here claims to read one out.
#[test]
fn the_incompat_values_carry_the_busy_windows_out_and_back() {
    let dir = workspace("incompat-values");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/incompat_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // In id order, which is the order `doc.incompats` promises.
    let incompats: Vec<_> = params
        .incompats
        .incompat_map
        .iter()
        .map(|(id, incompat)| (id, incompat.clone()))
        .collect();
    let patterns: Vec<_> = params.week_patterns.week_pattern_map.keys().collect();

    // The example is only worth reading if it has something to say: six
    // incompatibilities, one of them with more than a single busy window, and
    // all of them without a week pattern — the shape the script's `None`
    // assertions stand on.
    assert_eq!(
        incompats.len(),
        6,
        "the example holds six incompatibilities"
    );
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

    // Out and back, whole.
    assert_eq!(
        extracted_all::<IncompatData>(&globals, "incompat_values"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat.clone())
            .collect::<Vec<_>>()
    );

    // And the same fields as python saw them.
    let subject_position = |subject_id: &collomatique_state_colloscopes::SubjectId| {
        params
            .subjects
            .find_subject_position(*subject_id)
            .expect("an incompatibility names a live subject")
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "incompat_names"),
        incompats
            .iter()
            .map(|(_id, incompat)| incompat.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<usize>>(&globals, "incompat_subject_indices"),
        incompats
            .iter()
            .map(|(_id, incompat)| subject_position(&incompat.subject_id))
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
        global::<Vec<Option<usize>>>(&globals, "week_pattern_positions"),
        incompats
            .iter()
            .map(
                |(_id, incompat)| incompat.week_pattern_id.map(|pattern_id| patterns
                    .iter()
                    .position(|id| *id == pattern_id)
                    .expect("an incompatibility names a live pattern"))
            )
            .collect::<Vec<_>>()
    );

    // Built by hand: the value the script wrote out is the incompatibility
    // expected of it, whether the subject and the pattern were named by handle
    // or by id. The `Some` shape of `week_pattern` is inbound only, and it is
    // pinned here, since no fixture carries it.
    let first_subject = params
        .subjects
        .ordered_subject_list
        .keys()
        .next()
        .expect("the example has subjects");
    let first_pattern = *patterns.first().expect("the example has week patterns");
    let at = |hour: u32, minute: u32| {
        collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("a real time of day"),
        )
        .expect("a whole minute")
    };
    let noon = collomatique_time::SlotWithDuration::new(
        collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: at(12, 0),
        },
        collomatique_time::NonZeroMinutes::new(60).expect("an hour is a while"),
    )
    .expect("noon to one o'clock is a window");
    let written_out = collomatique_state_colloscopes::incompats::Incompatibility {
        subject_id: first_subject,
        name: "Mercredi après-midi".to_owned(),
        slots: vec![noon],
        minimum_free_slots: NonZeroU32::new(2).expect("two is not zero"),
        week_pattern_id: Some(first_pattern),
    };
    for name in ["incompat_by_handle", "incompat_by_id"] {
        assert_eq!(extracted::<IncompatData>(&globals, name), written_out);
    }

    // And one carrying nothing but the two fields an incompatibility cannot do
    // without: no windows, one window free at least, and every week.
    assert_eq!(
        extracted::<IncompatData>(&globals, "bare_incompat"),
        collomatique_state_colloscopes::incompats::Incompatibility {
            subject_id: first_subject,
            name: String::new(),
            slots: vec![],
            minimum_free_slots: NonZeroU32::new(1).expect("one is not zero"),
            week_pattern_id: None,
        }
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_name"),
        (
            "TypeError".to_owned(),
            "an IncompatData's name is a string, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_subject"),
        (
            "TypeError".to_owned(),
            "a subject argument takes a Subject or a SubjectId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_slots"),
        (
            "TypeError".to_owned(),
            "an IncompatData's slots is a list of TimeSlot values, and 3 \
             cannot be iterated over"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_time_slot"),
        (
            "TypeError".to_owned(),
            "an IncompatData's slots holds TimeSlot values, and 'jeudi' is not one".to_owned(),
        )
    );

    // The one field the model counts non-zero, refused in `values.rs`'s own
    // words — the boundary's only new refusal, since a `TimeSlot` was born
    // whole.
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_minimum"),
        (
            "ValueError".to_owned(),
            "an IncompatData's minimum_free_slots is at least 1, and 0 was given".to_owned(),
        )
    );
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_minimum_count"),
        (
            "TypeError".to_owned(),
            "an IncompatData's minimum_free_slots is a number of slots, and \
             'beaucoup' is not one"
                .to_owned(),
        )
    );

    // A field naming an entity refuses with `argument`'s own sentence, so a
    // script meets the same words here as anywhere else it passes something
    // that was never a reference to this document.
    assert_eq!(
        refused::<IncompatData>(&globals, "not_a_pattern"),
        (
            "TypeError".to_owned(),
            "a week pattern argument takes a WeekPattern or a WeekPatternId, \
             and 'Lundi Midi' is neither"
                .to_owned(),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<IncompatData>(&globals, "foreign_subject");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed incompatibility or pattern stales the values that name it
///
/// Stale is loud on the value surface like everywhere else: `to_data()`
/// through a dead handle raises, and a hand-built value naming an entity that
/// no longer exists refuses to extract. The read surface ships no removes, so
/// the two `UpdateOp`s land between two stages — the last incompatibility in
/// id order and the last week pattern, neither of which anything in the script
/// but the value it built names.
#[test]
fn a_removed_incompat_or_pattern_stales_the_incompat_values_that_name_it() {
    let dir = workspace("incompat-values-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entities the script is
    // holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let doomed_incompat = params
        .incompats
        .incompat_map
        .keys()
        .last()
        .expect("the example has incompatibilities");
    let doomed_pattern = params
        .week_patterns
        .week_pattern_map
        .keys()
        .last()
        .expect("the example has week patterns");

    let globals = run_stages(
        &[
            include_str!("scripts/incompat_data_stale_before.py"),
            include_str!("scripts/incompat_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        // Two scripts, so this runs once — and it makes both changes, because
        // the failures they cause are the halves of one test.
        |py, globals| {
            let doc = document_of(globals);
            let apply = |op| {
                doc.borrow_mut(py)
                    .update(py, op)
                    .expect("the example takes these two changes");
            };

            apply(collomatique_ops::UpdateOp::Incompatibilities(
                collomatique_ops::IncompatibilitiesUpdateOp::DeleteIncompat(doomed_incompat),
            ));
            apply(collomatique_ops::UpdateOp::WeekPatterns(
                collomatique_ops::WeekPatternsUpdateOp::DeleteWeekPattern(doomed_pattern),
            ));
        },
    );

    // The values naming the dead pattern no longer name anything, and it makes
    // no difference whether the script wrote the handle or the id.
    for name in [
        "naming_the_dead_pattern_by_handle",
        "naming_the_dead_pattern_by_id",
    ] {
        let (kind, _message) = refused::<IncompatData>(&globals, name);
        assert_eq!(kind, "StaleHandleError", "`{name}` should be refused");
    }

    // And the ones naming only what survived still extract — the value with no
    // pattern at all had nothing to lose here.
    assert_eq!(
        extracted::<IncompatData>(&globals, "naming_no_pattern").week_pattern_id,
        None
    );
    assert!(
        extracted::<IncompatData>(&globals, "naming_the_living_pattern")
            .week_pattern_id
            .is_some()
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
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
///.
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
    // group, the number always showing (`colloscopes/gtk4/src/editor/colloscope.rs`) —
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

/// The group lists come back detached, out and back
///
/// The script walks `doc.group_lists` and leaves what it saw; rust compares it
/// with the same document read straight from the model. The fixture the read
/// surface built for the two filling shapes carries this test — the automatic
/// list and the prefilled one side by side, which the example (all prefilled,
/// all unnamed) cannot show.
#[test]
fn the_group_lists_come_back_detached() {
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, PrefilledGroup,
    };

    let dir = workspace("group-list-data");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    let globals = run(include_str!("scripts/group_list_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The fixture is only worth reading if it has something to say: one list
    // of each filling shape, both named.
    let lists: Vec<_> = params
        .group_lists
        .group_list_map
        .iter()
        .map(|(id, gl)| (id, gl.clone()))
        .collect();
    assert_eq!(lists.len(), 2);
    assert!(lists.iter().any(|(_id, gl)| gl.is_prefilled()));
    assert!(lists.iter().any(|(_id, gl)| !gl.is_prefilled()));

    // Out and back, whole.
    assert_eq!(
        extracted_all::<GroupListData>(&globals, "gl_values"),
        lists.iter().map(|(_id, gl)| gl.clone()).collect::<Vec<_>>()
    );

    // And the same fields as python saw them.
    let bounds = |range: &collomatique_state_colloscopes::NonEmptyRangeInclusive<NonZeroU32>| {
        (range.start().get(), range.end().get())
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "gl_names"),
        lists
            .iter()
            .map(|(_id, gl)| gl.params().name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(u32, u32)>>(&globals, "gl_ranges"),
        lists
            .iter()
            .map(|(_id, gl)| bounds(&gl.params().students_per_group))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<Vec<Option<String>>>>(&globals, "group_names_lists"),
        lists
            .iter()
            .map(|(_id, gl)| gl
                .params()
                .group_names
                .iter()
                .map(|name| name.as_ref().map(|name| name.to_string()))
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );

    // The fillings, as python saw them: the exclusions of the automatic list
    // and the members of the prefilled groups, both by surname.
    let automatic = lists
        .iter()
        .find(|(_id, gl)| !gl.is_prefilled())
        .expect("the fixture holds an automatic list")
        .1
        .clone();
    let prefilled = lists
        .iter()
        .find(|(_id, gl)| gl.is_prefilled())
        .expect("the fixture holds a prefilled list")
        .1
        .clone();

    let excluded: BTreeSet<_> = match automatic.filling() {
        GroupListFilling::Automatic { excluded_students } => excluded_students.clone(),
        GroupListFilling::Prefilled { .. } => unreachable!("the list was picked as automatic"),
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

    let expected_members: Vec<Vec<String>> = match prefilled.filling() {
        GroupListFilling::Prefilled { groups } => groups
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
        GroupListFilling::Automatic { .. } => unreachable!("the list was picked as prefilled"),
    };
    assert_eq!(
        global::<Vec<Vec<String>>>(&globals, "prefilled_members"),
        expected_members
    );

    // Built by hand: the value the script wrote out is the group list expected
    // of it, whether the students were named by handle or by id — the prefilled
    // one reproduces the fixture's own list, group for group.
    let students: Vec<_> = params.students.student_map.keys().collect();
    let named = |text: &str| {
        text.to_owned()
            .try_into()
            .expect("the fixture's group names are not empty")
    };
    let expected_by_hand = GroupList::new(
        GroupListParameters {
            name: "Maisons".to_owned(),
            students_per_group: nonzero_range((2, 3)),
            group_names: vec![Some(named("Aurore")), None, Some(named("Serdaigle"))],
        },
        GroupListFilling::Prefilled {
            groups: vec![
                PrefilledGroup {
                    students: BTreeSet::from([students[0], students[1]]),
                },
                PrefilledGroup {
                    students: BTreeSet::from([students[2]]),
                },
                PrefilledGroup {
                    students: BTreeSet::from([students[3]]),
                },
            ],
        },
    )
    .expect("the hand-built list is internally consistent");
    for name in ["by_handle", "by_id"] {
        assert_eq!(extracted::<GroupListData>(&globals, name), expected_by_hand);
    }

    // The automatic shape, named entirely by id, with the fixture's exclusion.
    assert_eq!(
        extracted::<GroupListData>(&globals, "automatic_by_id"),
        GroupList::new(
            GroupListParameters {
                name: "Automatique".to_owned(),
                students_per_group: nonzero_range((1, 2)),
                group_names: vec![None; 4],
            },
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([students[2]]),
            },
        )
        .expect("the hand-built list is internally consistent")
    );

    // The default pin: `clm.GroupListData()` is the model's own default — a
    // list named « Liste », two to three students per group, sixteen unnamed
    // groups, and the solver filling them.
    assert_eq!(
        extracted::<GroupListData>(&globals, "bare"),
        GroupList::default()
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<GroupListData>(&globals, "not_a_name"),
        (
            "TypeError".to_owned(),
            "a GroupListData's name is a string, and 3 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<GroupListData>(&globals, "not_a_range"),
        (
            "ValueError".to_owned(),
            "a GroupListData's students_per_group is a (min, max) range, and \
             5 is above 2"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<GroupListData>(&globals, "not_a_names_list"),
        (
            "TypeError".to_owned(),
            "a GroupListData's group_names is a list of names, and 3 cannot be \
             iterated over"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<GroupListData>(&globals, "not_an_entry"),
        (
            "ValueError".to_owned(),
            "a GroupListData's group_names holds non-empty strings or None, \
             and '' is neither"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<GroupListData>(&globals, "not_a_filling"),
        (
            "TypeError".to_owned(),
            "a GroupListData's filling is a Filling, and 'Aurore' is not one".to_owned(),
        )
    );

    // The two sealed-constructor violations, in the model's own words.
    assert_eq!(
        refused::<GroupListData>(&globals, "mismatched_count"),
        (
            "ValueError".to_owned(),
            "prefilled group count (2) does not match the group name count (3)".to_owned(),
        )
    );
    assert_eq!(
        refused::<GroupListData>(&globals, "duplicated_student"),
        (
            "ValueError".to_owned(),
            format!("student {:?} appears in two prefilled groups", students[0]),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<GroupListData>(&globals, "foreign_student");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed group list stales the handle that names it
///
/// Stale is loud on the value surface like everywhere else: `to_data()`
/// through a dead handle raises, and it does not hand back a value describing
/// a group list that is gone. The value itself is untouched — nothing in it
/// names the list — so the same value still extracts after the removal. The
/// read surface ships no removes, so the `UpdateOp` lands between the two
/// stages.
#[test]
fn a_removed_group_list_stales_the_values_that_name_it() {
    let dir = workspace("group-list-data-stale");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entity the script is
    // holding.
    let data = reload(&source);
    let doomed = data
        .get_inner_data()
        .params
        .group_lists
        .group_list_map
        .keys()
        .last()
        .expect("the fixture has group lists");
    let expected = data
        .get_inner_data()
        .params
        .group_lists
        .group_list_map
        .get(&doomed)
        .cloned()
        .expect("the doomed list exists before the stage");

    let globals = run_stages(
        &[
            include_str!("scripts/group_list_data_stale_before.py"),
            include_str!("scripts/group_list_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        // One stage in the middle, so this runs once — and it makes the one
        // change the two halves of this test stand on.
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::GroupLists(
                        collomatique_ops::GroupListsUpdateOp::DeleteGroupList(doomed),
                    ),
                )
                .expect("the fixture takes the removal");
        },
    );

    // The value built before the removal is untouched, and still extracts to
    // the list the file used to hold — the students it names all survived.
    assert_eq!(
        extracted::<GroupListData>(&globals, "doomed_value"),
        expected
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The group lists are added, rewritten, associated and removed from python
///
/// The twelfth family of the ops mirror: `doc.group_lists` gains `add`,
/// `update`, `remove` and `remove_all` for the lists themselves, and
/// `set_association`, `duplicate_previous_period` and `clear_associations` for
/// the table beside them. An `update` carries
/// the whole list, parameters and filling together, because that is what the op
/// carries — so a filling that changes shape is an ordinary rewrite here.
///
/// Every cascade of this family runs through the colloscope, which the write
/// surface cannot reach yet: a colle names a group *number* measured against
/// the list its coordinate uses, and a placement names the group a student
/// landed in. So this runs in two stages, with rust writing that material in
/// between — the served list turned automatic, a placement row, and one colle
/// at a coordinate the list bounds — the way
/// [a_cascade_reports_every_repair_and_what_needed_it] does. The fourth write
/// in between belongs to the subjects, also a later piece: it stops the served
/// subject from running on one period, which is the only way the second of the
/// family's model refusals is reachable at all.
///
/// The first stage needs no fixture of its own: it finds the list one single
/// subject uses, the subject that runs no colles, and its own students off the
/// document. What rust asserts here is that the example really holds those
/// shapes.
///
/// Rust reads back the file the script saved after its last accepted write of
/// the first stage: the list the example did not have is the one the script
/// asked for, field by field, and the association table is exactly the one it
/// opened with — the script cleared rows and put them back through both
/// `set_association` and `duplicate_previous_period`.
#[test]
fn group_lists_are_added_rewritten_associated_and_removed() {
    use collomatique_ops::{ColloscopeUpdateOp, GroupListsUpdateOp, SubjectsUpdateOp, UpdateOp};
    use collomatique_state_colloscopes::group_lists::{GroupList, GroupListFilling};

    let dir = workspace("group-lists-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let period_ids: Vec<_> = params.periods.period_ids().collect();
    assert!(
        period_ids.len() >= 3,
        "the script copies onto the second period and leaves a third for the \
         second stage's exclusion"
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .all(|subject| subject.excluded_periods.is_empty()),
        "the script's copy assertion covers every subject that runs colles"
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .values()
            .any(|subject| subject.parameters.interrogation_parameters.is_none()),
        "the script needs a subject that runs no interrogations"
    );

    // The rows one list serves, in the association table's own key order.
    let rows_of = |list| {
        params
            .group_lists
            .subjects_associations
            .iter()
            .filter(move |(_key, group_list)| **group_list == list)
            .map(|(key, _group_list)| key)
            .collect::<Vec<_>>()
    };

    // The list the whole test is about: the one exactly one subject uses, and
    // on every period. One subject, so that the removal's unassignments are
    // countable; every period, so that the first is free for the script's
    // association writes and the second for its copy.
    let (served_index, served) = params
        .group_lists
        .group_list_map
        .keys()
        .enumerate()
        .find(|(_index, list)| {
            let rows = rows_of(*list);
            let subjects: BTreeSet<_> = rows.iter().map(|(_period, subject)| *subject).collect();
            subjects.len() == 1 && rows.len() == period_ids.len()
        })
        .expect("the example has a list one single subject uses on every period");
    let served_rows = rows_of(served);
    let served_subject = served_rows[0].1;
    assert!(
        params
            .group_lists
            .group_list_map
            .get(&served)
            .expect("the list was just read off the table")
            .params()
            .group_names
            .len()
            > 2,
        "the second stage writes a colle naming group 2 in that list"
    );

    // The two students the second stage's placement row holds, and the cell it
    // is measured against: the first week one of the served subject's slots is
    // really active on, at a coordinate that subject's list serves.
    let placed: Vec<_> = params.students.student_map.keys().take(2).collect();
    assert_eq!(placed.len(), 2, "the placement row holds two students");

    let (cell_period, cell_slot, cell_week) = params
        .slots
        .slots_for_subject(served_subject)
        .into_iter()
        .flatten()
        .flat_map(|(slot_id, _slot)| {
            params
                .walk_weeks()
                .map(move |(period, week, _week)| (period, *slot_id, week))
        })
        .find(|(period, slot, week)| {
            params.is_interrogation_possible(*slot, *week)
                && params
                    .group_lists
                    .subjects_associations
                    .get(&(*period, served_subject))
                    == Some(&served)
        })
        .expect("the served subject has a slot active on a week its list serves");

    // The period the second stage's refusal is asserted on: not the cell's, so
    // that the colle the script leans on survives the exclusion.
    let gone_period = *period_ids
        .iter()
        .find(|period| **period != cell_period)
        .expect("the example has more than one period");

    // The french labels the eight operations carry, so that the script's undo
    // assertions pin the operations' own names and not merely some strings.
    // Only the variant — and, for the association, whether it names a list — is
    // read, so the payloads below are the nearest ones to hand.
    let label = |op: GroupListsUpdateOp| op.get_desc().1;
    let blank = params
        .group_lists
        .group_list_map
        .get(&served)
        .expect("the list was just read off the table")
        .clone();
    let add_label = label(GroupListsUpdateOp::AddNewGroupList(blank.clone()));
    let update_label = label(GroupListsUpdateOp::UpdateGroupList(served, blank));
    let remove_label = label(GroupListsUpdateOp::DeleteGroupList(served));
    let assign_label = label(GroupListsUpdateOp::AssignGroupListToSubject(
        cell_period,
        served_subject,
        Some(served),
    ));
    let unassign_label = label(GroupListsUpdateOp::AssignGroupListToSubject(
        cell_period,
        served_subject,
        None,
    ));
    let duplicate_label = label(GroupListsUpdateOp::DuplicatePreviousPeriod(cell_period));
    let remove_all_label = label(GroupListsUpdateOp::DeleteAllGroupLists);
    let clear_label = label(GroupListsUpdateOp::ClearPeriodAssociations(cell_period));

    // The list the second stage reads: the served one, automatic and otherwise
    // untouched, so that it can hold a placement row at all.
    let automatic = GroupList::new(
        params
            .group_lists
            .group_list_map
            .get(&served)
            .expect("the list was just read off the table")
            .params()
            .clone(),
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::new(),
        },
    )
    .expect("an automatic filling asks nothing of the group names");

    run_stages(
        &[
            include_str!("scripts/group_lists_write_before.py"),
            include_str!("scripts/group_lists_write_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("target", &target)?;
            globals.set_item("served_index", served_index)?;
            globals.set_item("add_label", &add_label)?;
            globals.set_item("update_label", &update_label)?;
            globals.set_item("remove_label", &remove_label)?;
            globals.set_item("assign_label", &assign_label)?;
            globals.set_item("unassign_label", &unassign_label)?;
            globals.set_item("duplicate_label", &duplicate_label)?;
            globals.set_item("remove_all_label", &remove_all_label)?;
            globals.set_item("clear_label", &clear_label)?;
            Ok(())
        },
        |py, globals| {
            applied_write(
                py,
                globals,
                "prepared_filling",
                UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                    served,
                    automatic.clone(),
                )),
            );
            applied_write(
                py,
                globals,
                "prepared_placements",
                UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeGroupList(
                    served,
                    BTreeMap::from([(placed[0], 0), (placed[1], 2)]),
                )),
            );
            applied_write(
                py,
                globals,
                "prepared_cell",
                UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                    cell_slot,
                    cell_week,
                    BTreeSet::from([0, 2]),
                )),
            );
            // The subjects are a later piece of the mirror, so the one thing
            // the second stage cannot say for itself is said here.
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                        served_subject,
                        gone_period,
                        false,
                    )),
                )
                .expect("a subject of the example can stop running on a period");
        },
    );

    // The document the script saved holds everything it opened with, plus the
    // one list it wrote — and the association table it opened with, since every
    // row it cleared it put back.
    let written = reload(&target);
    let after = &written.get_inner_data().params.group_lists;
    let added: Vec<_> = after
        .group_list_map
        .iter()
        .filter(|(id, _list)| !params.group_lists.group_list_map.contains(id))
        .map(|(_id, list)| list.clone())
        .collect();

    assert_eq!(added.len(), 1);
    let added = &added[0];
    assert_eq!(added.params().name, "Maisons");
    assert_eq!(
        added.params().group_names.len(),
        3,
        "the script's last accepted rewrite gave it three unnamed groups"
    );
    assert!(added.params().group_names.iter().all(|name| name.is_none()));
    assert_eq!(
        added.filling(),
        &GroupListFilling::Prefilled {
            groups: vec![
                collomatique_state_colloscopes::group_lists::PrefilledGroup {
                    students: BTreeSet::from([placed[0]]),
                },
                collomatique_state_colloscopes::group_lists::PrefilledGroup {
                    students: BTreeSet::from([placed[1]]),
                },
                collomatique_state_colloscopes::group_lists::PrefilledGroup {
                    students: BTreeSet::new(),
                },
            ],
        }
    );
    assert_eq!(
        after.group_list_map.len(),
        params.group_lists.group_list_map.len() + 1
    );
    assert_eq!(
        after.subjects_associations.iter().collect::<Vec<_>>(),
        params
            .group_lists
            .subjects_associations
            .iter()
            .collect::<Vec<_>>(),
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The colloscope is written row by row, erased, and refuses what cannot sit
/// in it
///
/// The thirteenth family of the ops mirror: `doc.colloscope` gains
/// `set_interrogation` and `set_group_list` for its two sparse tables, and
/// `erase` and `erase_group_lists` for emptying them. The family's fifth op,
/// `install`, stays gated out — it is the solver's landing door.
///
/// Nothing in the document points at a colloscope row, so no write here can
/// break anything: every result the script reads carries an empty `warnings`,
/// and there is no cascade to assert. What there is instead is the sparse
/// shape on the write — an empty iterable and an empty mapping are the absent
/// row, which is what clears one — and five of the six refusals the model
/// keeps for this family.
///
/// The sixth needs a subject that skips a period, which is the subjects family
/// and a later piece, so it runs in a second stage with rust doing that write
/// in between — the way
/// [group_lists_are_added_rewritten_associated_and_removed] does for the same
/// reason.
///
/// The example carries no colloscope at all, so everything the script reads
/// back is its own doing. The coordinate it writes on is the first cell a
/// colle can really sit on whose subject holds a group list there, in the
/// collections' own orders; rust finds it by the same rule and checks the two
/// sides agree through the indices the script leaves behind.
///
/// Rust reads back the file the script saved once both tables held a row: the
/// one cell, and the placement row of the automatic list the example did not
/// have and the script added.
#[test]
fn the_colloscope_is_written_row_by_row_and_erased() {
    use collomatique_ops::{ColloscopeUpdateOp, GroupListsUpdateOp, SubjectsUpdateOp, UpdateOp};
    use collomatique_state_colloscopes::group_lists::GroupListFilling;

    let dir = workspace("colloscope-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let inner = data.get_inner_data();
    let params = &inner.params;

    assert_eq!(
        inner.colloscope.iter().count(),
        0,
        "the script opens on a document with no colloscope at all"
    );
    assert_eq!(
        inner.colloscope.group_lists_iter().count(),
        0,
        "the script opens on a document with no colloscope at all"
    );
    assert!(
        params.students.student_map.len() >= 3,
        "the script places two students and needs a third to be excluded"
    );
    assert!(
        params
            .walk_weeks()
            .any(|(_period, _id, week)| !week.interrogations),
        "the script needs a week no slot runs on"
    );

    // The coordinate the script writes on, found by the rule the script uses:
    // the slots in `doc.slots` order, the weeks in theirs, and the first pair a
    // colle can sit on whose subject holds a group list on that week's period —
    // the list being what the group numbers are measured against.
    let (cell_subject, cell_slot, cell_week) = params
        .subjects
        .ordered_subject_list
        .keys()
        .flat_map(|subject_id| {
            params
                .slots
                .slots_for_subject(subject_id)
                .into_iter()
                .flatten()
                .map(move |(slot_id, _slot)| (subject_id, *slot_id))
        })
        .flat_map(|(subject_id, slot_id)| {
            params
                .week_ids()
                .map(move |week_id| (subject_id, slot_id, week_id))
        })
        .find(|(subject_id, slot_id, week_id)| {
            params.is_interrogation_possible(*slot_id, *week_id)
                && params
                    .weeks
                    .week_position(*week_id)
                    .is_some_and(|(period_id, _pos)| {
                        params
                            .group_lists
                            .subjects_associations
                            .get(&(period_id, *subject_id))
                            .is_some()
                    })
        })
        .expect("the example has a slot active on a week its subject holds a list on");
    let (cell_period, _pos) = params
        .weeks
        .week_position(cell_week)
        .expect("the week was just walked");
    let cell_list = *params
        .group_lists
        .subjects_associations
        .get(&(cell_period, cell_subject))
        .expect("the coordinate was chosen for holding an association");
    assert!(
        params
            .group_lists
            .group_list_map
            .get(&cell_list)
            .expect("an association names a list the document holds")
            .params()
            .group_names
            .len()
            > 2,
        "the script writes a cell naming group 2 in that list"
    );

    // The two students the placement row holds, and the one the added list
    // excludes: the first three, in the students' own order.
    let students: Vec<_> = params.students.student_map.keys().take(3).collect();

    // The french labels the five operations carry, so that the script's undo
    // assertions pin the operations' own names and not merely some strings.
    // Only the variant is read, so the payloads below are the nearest ones to
    // hand.
    let label = |op: ColloscopeUpdateOp| op.get_desc().1;
    let set_interrogation_label = label(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
        cell_slot,
        cell_week,
        BTreeSet::new(),
    ));
    let set_group_list_label = label(ColloscopeUpdateOp::UpdateColloscopeGroupList(
        cell_list,
        BTreeMap::new(),
    ));
    let erase_label = label(ColloscopeUpdateOp::EraseColloscope);
    let erase_group_lists_label = label(ColloscopeUpdateOp::EraseGroupLists);
    let add_group_list_label = GroupListsUpdateOp::AddNewGroupList(
        params
            .group_lists
            .group_list_map
            .get(&cell_list)
            .expect("the list was just read off the table")
            .clone(),
    )
    .get_desc()
    .1;
    // The one write rust makes itself, between the two stages: the second stage
    // reads its label to say that the refusal it asserts cost no undo slot of
    // its own.
    let exclusion_label = SubjectsUpdateOp::UpdatePeriodStatus(cell_subject, cell_period, false)
        .get_desc()
        .1;

    let globals = run_stages(
        &[
            include_str!("scripts/colloscope_write_before.py"),
            include_str!("scripts/colloscope_write_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            globals.set_item("target", &target)?;
            globals.set_item("set_interrogation_label", &set_interrogation_label)?;
            globals.set_item("set_group_list_label", &set_group_list_label)?;
            globals.set_item("erase_label", &erase_label)?;
            globals.set_item("erase_group_lists_label", &erase_group_lists_label)?;
            globals.set_item("add_group_list_label", &add_group_list_label)?;
            globals.set_item("exclusion_label", &exclusion_label)?;
            Ok(())
        },
        |py, globals| {
            // The subjects are a later piece of the mirror, so the one thing
            // the second stage cannot say for itself is said here.
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                        cell_subject,
                        cell_period,
                        false,
                    )),
                )
                .expect("a subject of the example can stop running on a period");
        },
    );

    // The two sides really found the same coordinate: the script says which
    // subject and which period it wrote on, in the collections' own order.
    assert_eq!(
        params
            .subjects
            .ordered_subject_list
            .keys()
            .nth(global::<usize>(&globals, "cell_subject_index")),
        Some(cell_subject),
    );
    assert_eq!(
        params
            .periods
            .period_ids()
            .nth(global::<usize>(&globals, "cell_period_index")),
        Some(cell_period),
    );

    // The document the script saved holds the two rows it had written by then,
    // and nothing else: the example had no colloscope, so this is the whole of
    // it.
    let written = reload(&target);
    let after = written.get_inner_data();

    assert_eq!(
        after
            .colloscope
            .iter()
            .map(|(coord, groups)| (coord, groups.clone()))
            .collect::<Vec<_>>(),
        vec![((cell_slot, cell_week), BTreeSet::from([0, 2]))],
    );

    // The automatic list the script added — the example has none, and a
    // prefilled list holds no placement row.
    let added: Vec<_> = after
        .params
        .group_lists
        .group_list_map
        .iter()
        .filter(|(id, _list)| !params.group_lists.group_list_map.contains(id))
        .collect();
    assert_eq!(added.len(), 1);
    let (added_id, added_list) = added[0];
    assert_eq!(added_list.params().name, "Colles automatiques");
    assert_eq!(
        added_list.filling(),
        &GroupListFilling::Automatic {
            excluded_students: BTreeSet::from([students[2]]),
        }
    );
    assert_eq!(
        after
            .colloscope
            .group_lists_iter()
            .map(|(id, placements)| (id, placements.clone()))
            .collect::<Vec<_>>(),
        vec![(
            added_id,
            BTreeMap::from([(students[0], 0), (students[1], 2)])
        )],
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The subjects are added, rewritten, moved and removed from python
///
/// The fourteenth family of the ops mirror, and the most referenced entity the
/// document has: `doc.subjects` gains `add`, `update`, `remove`, `move_up`,
/// `move_down` and `set_period_status`. Eight kinds of place name a subject, so
/// the removal is the heaviest ordinary one there is — and the two other
/// cascades, switching the interrogations off and taking the subject off a
/// period, are what the family is really about.
///
/// It is the second family whose value is larger than what its ops carry: a
/// `SubjectData` holds the excluded periods and no subject op does, so `add`
/// refuses a value that excludes anything and `update` refuses one whose
/// exclusions differ from the document's, both with the `ValueError` the script
/// pins beside the model's own two refusals.
///
/// Everything the cascades need beyond what the example carries — an
/// incompatibility on the subject, a pairing rule naming it, and one colle
/// standing in one of its slots — the script writes for itself through the
/// surface the earlier pieces published, and undoes again. So this needs no
/// second stage.
///
/// What rust asserts on its own side is that the example really holds the shapes
/// the script leans on: two subjects that run colles, the first of them holding
/// every reference site the removal repairs — a teacher, slots, balancing
/// options of its own, an enrolment row and a group-list association on each of
/// the three periods — and a week of the first period one of its slots is really
/// active on.
///
/// Rust reads back the file the script saved after its last accepted write of
/// the first half: the two subjects the example did not have, field by field,
/// last in the list and in the order the script added them.
#[test]
fn subjects_are_added_rewritten_moved_and_removed() {
    use collomatique_ops::SubjectsUpdateOp;
    use collomatique_state_colloscopes::subjects::Subject;
    use collomatique_state_colloscopes::{
        SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
    };

    let dir = workspace("subjects-write");
    let source = example_copy(&dir, "source.collomatique");
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are stored,
    // so the copy rust reads names the same entities the script is holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let order: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    let with_colles: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
        .map(|(id, _subject)| id)
        .collect();
    assert!(
        with_colles.len() >= 2,
        "the script names two subjects that run colles"
    );

    // The subject the three cascades are about. Every reference site the
    // removal has to repair is asserted here, since a fixture that quietly lost
    // one would make the script's warning list shorter and still green.
    let rich = with_colles[0];
    assert_eq!(
        params
            .teachers
            .teacher_map
            .values()
            .filter(|teacher| teacher.subjects.contains(&rich))
            .count(),
        1,
        "exactly one teacher holds that subject's colles",
    );
    assert!(
        !params
            .slots
            .slots_for_subject(rich)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .is_empty(),
        "the script asserts the slots that go with the subject",
    );
    assert!(
        params.balancing.subjects.contains(&rich),
        "the script asserts the balancing override that goes with the subject",
    );
    for period in params.periods.period_ids() {
        assert!(
            params.assignments.students(period, rich).is_some(),
            "the subject holds an enrolment row on every period",
        );
        assert!(
            params
                .group_lists
                .subjects_associations
                .get(&(period, rich))
                .is_some(),
            "the subject uses a group list on every period",
        );
    }

    let first_period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");
    assert!(
        params
            .slots
            .slots_for_subject(rich)
            .into_iter()
            .flatten()
            .any(|(slot_id, _slot)| params.week_ids().any(|week| {
                params.weeks.week_position(week).map(|(p, _pos)| p) == Some(first_period)
                    && params.is_interrogation_possible(*slot_id, week)
            })),
        "one of the subject's slots is really active on a week of the first period",
    );

    // The french labels this family's operations carry, so that the script's
    // undo assertions pin the operations' own names and not merely some
    // strings. Only the variant is read, so the payloads below are the nearest
    // ones to hand — and `set_period_status` gets two, since the op names its
    // own direction.
    let label = |op: SubjectsUpdateOp| op.get_desc().1;
    let blank = params
        .subjects
        .find_subject(rich)
        .expect("the list names a live subject")
        .parameters
        .clone();
    let add_label = label(SubjectsUpdateOp::AddNewSubject(blank.clone()));
    let update_label = label(SubjectsUpdateOp::UpdateSubject(rich, blank));
    let remove_label = label(SubjectsUpdateOp::DeleteSubject(rich));
    let move_up_label = label(SubjectsUpdateOp::MoveSubjectUp(rich));
    let move_down_label = label(SubjectsUpdateOp::MoveSubjectDown(rich));
    let exclude_label = label(SubjectsUpdateOp::UpdatePeriodStatus(
        rich,
        first_period,
        false,
    ));
    let include_label = label(SubjectsUpdateOp::UpdatePeriodStatus(
        rich,
        first_period,
        true,
    ));

    run(include_str!("scripts/subjects_write.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("add_label", &add_label)?;
        globals.set_item("update_label", &update_label)?;
        globals.set_item("remove_label", &remove_label)?;
        globals.set_item("move_up_label", &move_up_label)?;
        globals.set_item("move_down_label", &move_down_label)?;
        globals.set_item("exclude_label", &exclude_label)?;
        globals.set_item("include_label", &include_label)?;
        Ok(())
    });

    // What the script's two adds asked for, as they stood when it saved. The
    // first was created bare, rewritten three times and taken off the first
    // period; the second is the subject that never holds a colle.
    let written_out = vec![
        Subject {
            parameters: SubjectParameters {
                name: "Sortilèges".into(),
                interrogation_parameters: Some(SubjectInterrogationParameters {
                    students_per_group: nonzero_range((2, 3)),
                    groups_per_interrogation: nonzero_range((1, 1)),
                    duration: collomatique_time::NonZeroMinutes::new(60)
                        .expect("an hour is a while"),
                    take_duration_into_account: true,
                    periodicity: SubjectPeriodicity::ExactlyPeriodic {
                        periodicity_in_weeks: NonZeroU32::new(2).expect("two is not zero"),
                    },
                }),
            },
            excluded_periods: BTreeSet::from([first_period]),
        },
        Subject {
            parameters: SubjectParameters {
                name: "Club de Bavboules".into(),
                interrogation_parameters: None,
            },
            excluded_periods: BTreeSet::new(),
        },
    ];

    // The document the script saved holds everything it opened with, in the
    // order it opened with, plus the two subjects it added, last and in that
    // order.
    let written = reload(&target);
    let after: Vec<_> = written
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .map(|(id, subject)| (id, subject.clone()))
        .collect();

    assert_eq!(after.len(), order.len() + 2);
    assert_eq!(
        after
            .iter()
            .take(order.len())
            .map(|(id, _subject)| *id)
            .collect::<Vec<_>>(),
        order,
        "the two moves cancelled out, so the example's own subjects are where they were",
    );
    assert!(
        after
            .iter()
            .skip(order.len())
            .all(|(id, _subject)| !order.contains(id)),
        "the two subjects at the end are the ones the script added",
    );
    assert_eq!(
        after
            .into_iter()
            .skip(order.len())
            .map(|(_id, subject)| subject)
            .collect::<Vec<_>>(),
        written_out,
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
///.
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

/// The pairing rules come back detached, out and back
///
/// The script walks `doc.pairings` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the whole rule, and
/// the same fields as python saw them. The fixture the read surface built for
/// the two rules carries this test: both `should_have` polarities on each
/// side, soft both ways, one rule excluding a period and one excluding none,
/// which the example (no subject pairing rules at all) cannot show.
#[test]
fn the_pairing_rules_come_back_detached() {
    use collomatique_state_colloscopes::pairings::{PairingRule, RulePart};

    let dir = workspace("pairing-rule-data");
    let source = dir.join("pairings.collomatique");
    pairings_document(&source);

    let globals = run(include_str!("scripts/pairing_rule_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The fixture is only worth reading if it has something to say: two rules,
    // both polarities on each side, soft both ways, one exclusion set empty
    // and one not.
    let rules: Vec<_> = params
        .pairings
        .pairing_rule_map
        .iter()
        .map(|(id, rule)| (id, rule.clone()))
        .collect();
    assert_eq!(rules.len(), 2);
    assert!(rules.iter().any(|(_id, rule)| rule.soft()));
    assert!(rules.iter().any(|(_id, rule)| !rule.soft()));

    // Out and back, whole — the rules, and the ends of them on their own.
    assert_eq!(
        extracted_all::<PairingRuleData>(&globals, "rule_values"),
        rules
            .iter()
            .map(|(_id, rule)| rule.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        extracted_all::<PairingRuleSideData>(&globals, "side_values"),
        rules
            .iter()
            .flat_map(|(_id, rule)| [rule.antecedent().clone(), rule.consequent().clone()])
            .collect::<Vec<_>>()
    );

    // And the same fields as python saw them.
    assert_eq!(
        global::<Vec<bool>>(&globals, "rule_softs"),
        rules
            .iter()
            .map(|(_id, rule)| rule.soft())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(bool, bool)>>(&globals, "side_should_haves"),
        rules
            .iter()
            .map(|(_id, rule)| (rule.antecedent().should_have, rule.consequent().should_have,))
            .collect::<Vec<_>>()
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

    // Built by hand: the value the script wrote out is the rule expected of
    // it, whether the subjects were named by handle or by id.
    use collomatique_state::ids::Id as _;
    let subject = |n: u64| unsafe { collomatique_state_colloscopes::SubjectId::new(n) };
    let period = |n: u64| unsafe { collomatique_state_colloscopes::PeriodId::new(n) };
    let expected_strict = PairingRule::new(
        RulePart {
            subject_id: subject(11),
            should_have: true,
        },
        RulePart {
            subject_id: subject(12),
            should_have: false,
        },
        BTreeSet::new(),
        false,
    )
    .expect("the hand-built rule is internally consistent");
    for name in ["by_handle", "by_id"] {
        assert_eq!(
            extracted::<PairingRuleData>(&globals, name),
            expected_strict
        );
    }

    // The soft rule with a period excluded, named entirely by id — reproducing
    // the fixture's own rule, so that rust can compare it whole.
    assert_eq!(
        extracted::<PairingRuleData>(&globals, "soft_by_id"),
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
        .expect("the hand-built rule is internally consistent")
    );

    // The defaults: `should_have` True on each side, no exclusion, strict —
    // the spellings the application itself starts a new rule with. The model
    // has no `Default` for the rule, so these are written out and re-read
    // rather than pinned.
    assert_eq!(
        extracted::<PairingRuleData>(&globals, "defaults"),
        PairingRule::new(
            RulePart {
                subject_id: subject(11),
                should_have: true,
            },
            RulePart {
                subject_id: subject(12),
                should_have: true,
            },
            BTreeSet::new(),
            false,
        )
        .expect("the hand-built rule is internally consistent")
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<PairingRuleData>(&globals, "not_a_subject"),
        (
            "TypeError".to_owned(),
            "a subject argument takes a Subject or a SubjectId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<PairingRuleData>(&globals, "not_a_side"),
        (
            "TypeError".to_owned(),
            "a PairingRuleData is expected here, and 'Aurore' has no antecedent.subject".to_owned(),
        )
    );
    assert_eq!(
        refused::<PairingRuleData>(&globals, "not_a_side_flag"),
        (
            "TypeError".to_owned(),
            "a PairingRuleData's antecedent.should_have is True or False, and 1 is neither"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<PairingRuleData>(&globals, "not_a_rule_flag"),
        (
            "TypeError".to_owned(),
            "a PairingRuleData's soft is True or False, and 1 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<PairingRuleData>(&globals, "not_a_periods_set"),
        (
            "TypeError".to_owned(),
            "a PairingRuleData's excluded_periods is a set of entities, and 3 cannot be \
             iterated over"
                .to_owned(),
        )
    );

    // The sealed-constructor violation, in the model's own words.
    assert_eq!(
        refused::<PairingRuleData>(&globals, "same_subject_twice"),
        (
            "ValueError".to_owned(),
            format!(
                "antecedent and consequent subjects are the same ({:?})",
                subject(11),
            ),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<PairingRuleData>(&globals, "foreign_rule");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The slot pairing rules come back detached, out and back
///
/// The script walks `doc.slot_pairings` and leaves what it saw; rust compares
/// it with the same document read straight from the model. The example
/// carries two slot pairing rules, both strict, both excluding no period,
/// with a used antecedent and an unused consequent; the soft-with-exclusion
/// shape it does not carry is built by hand.
#[test]
fn the_slot_pairing_rules_come_back_detached() {
    use collomatique_state_colloscopes::slot_pairings::{SlotPairingRule, SlotRulePart};

    let dir = workspace("slot-pairing-rule-data");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(
        include_str!("scripts/slot_pairing_rule_data.py"),
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
    );

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // The example is only worth reading if it has something to say: two rules,
    // both strict, both excluding no period, a used antecedent against an
    // unused consequent.
    let rules: Vec<_> = params
        .slot_pairings
        .slot_pairing_rule_map
        .iter()
        .map(|(id, rule)| (id, rule.clone()))
        .collect();
    assert_eq!(rules.len(), 2);
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

    // Out and back, whole — the rules, and the ends of them on their own.
    assert_eq!(
        extracted_all::<SlotPairingRuleData>(&globals, "rule_values"),
        rules
            .iter()
            .map(|(_id, rule)| rule.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        extracted_all::<SlotPairingRuleSideData>(&globals, "side_values"),
        rules
            .iter()
            .flat_map(|(_id, rule)| [rule.antecedent().clone(), rule.consequent().clone()])
            .collect::<Vec<_>>()
    );

    // And the same fields as python saw them.
    assert_eq!(
        global::<Vec<bool>>(&globals, "rule_softs"),
        rules
            .iter()
            .map(|(_id, rule)| rule.soft())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        global::<Vec<(bool, bool)>>(&globals, "side_should_haves"),
        rules
            .iter()
            .map(|(_id, rule)| (rule.antecedent().should_have, rule.consequent().should_have,))
            .collect::<Vec<_>>()
    );

    // The slots, named by their start time — that is what pins the part to a
    // slot of the document rather than to a number that happens to be there.
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
    assert_eq!(
        global::<Vec<chrono::NaiveTime>>(&globals, "consequent_slot_start_times"),
        rules
            .iter()
            .map(|(_id, rule)| start_time(rule.consequent().slot_id))
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

    // Built by hand: the value the script wrote out is the rule expected of
    // it, whether the slots were named by handle or by id — reproducing the
    // example's first rule.
    let first = &rules[0].1;
    let expected_first = first.clone();
    for name in ["by_handle", "by_id"] {
        assert_eq!(
            extracted::<SlotPairingRuleData>(&globals, name),
            expected_first
        );
    }

    // The shape the example does not carry — a soft rule with a period
    // excluded — written out and re-read whole.
    let first_period = params
        .periods
        .period_ids()
        .next()
        .expect("the example has periods");
    assert_eq!(
        extracted::<SlotPairingRuleData>(&globals, "soft_with_exclusion"),
        SlotPairingRule::new(
            SlotRulePart {
                slot_id: first.antecedent().slot_id,
                should_have: true,
            },
            SlotRulePart {
                slot_id: first.consequent().slot_id,
                should_have: false,
            },
            BTreeSet::from([first_period]),
            true,
        )
        .expect("the hand-built rule is internally consistent")
    );

    // The defaults: `should_have` True on each side, no exclusion, strict —
    // the spellings the application itself starts a new rule with.
    assert_eq!(
        extracted::<SlotPairingRuleData>(&globals, "defaults"),
        SlotPairingRule::new(
            SlotRulePart {
                slot_id: first.antecedent().slot_id,
                should_have: true,
            },
            SlotRulePart {
                slot_id: first.consequent().slot_id,
                should_have: true,
            },
            BTreeSet::new(),
            false,
        )
        .expect("the hand-built rule is internally consistent")
    );

    // The refusals, each with the sentence it raises.
    assert_eq!(
        refused::<SlotPairingRuleData>(&globals, "not_a_slot"),
        (
            "TypeError".to_owned(),
            "a slot argument takes a Slot or a SlotId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotPairingRuleData>(&globals, "not_a_side"),
        (
            "TypeError".to_owned(),
            "a SlotPairingRuleData is expected here, and 'Aurore' has no antecedent.slot"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<SlotPairingRuleData>(&globals, "not_a_side_flag"),
        (
            "TypeError".to_owned(),
            "a SlotPairingRuleData's antecedent.should_have is True or False, and 1 is neither"
                .to_owned(),
        )
    );

    // The sealed-constructor violation, in the model's own words.
    assert_eq!(
        refused::<SlotPairingRuleData>(&globals, "same_slot_twice"),
        (
            "ValueError".to_owned(),
            format!(
                "antecedent and consequent slots are the same ({:?})",
                first.antecedent().slot_id,
            ),
        )
    );

    // A handle of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<SlotPairingRuleData>(&globals, "foreign_rule");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed pairing rule stales the values that name it
///
/// Stale is loud on the value surface like everywhere else: `to_data()`
/// through a dead handle raises, and through a dead side view too — the ends
/// go with their rule. The value itself is untouched: nothing in it names the
/// rule, so the same value still extracts after the removal. The read surface
/// ships no removes, so the `UpdateOp` lands between the two stages.
#[test]
fn a_removed_pairing_rule_stales_the_values_that_name_it() {
    let dir = workspace("pairing-rule-data-stale");
    let source = dir.join("pairings.collomatique");
    pairings_document(&source);

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entity the script is
    // holding.
    let data = reload(&source);
    let doomed = data
        .get_inner_data()
        .params
        .pairings
        .pairing_rule_map
        .keys()
        .last()
        .expect("the fixture has pairing rules");
    let expected = data
        .get_inner_data()
        .params
        .pairings
        .pairing_rule_map
        .get(&doomed)
        .cloned()
        .expect("the doomed rule exists before the stage");

    let globals = run_stages(
        &[
            include_str!("scripts/pairing_rule_data_stale_before.py"),
            include_str!("scripts/pairing_rule_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        // One stage in the middle, so this runs once — and it makes the one
        // change the two halves of this test stand on.
        |py, globals| {
            let doc = document_of(globals);
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Pairings(
                        collomatique_ops::PairingsUpdateOp::DeletePairingRule(doomed),
                    ),
                )
                .expect("the fixture takes the removal");
        },
    );

    // The value built before the removal is untouched, and still extracts to
    // the rule the file used to hold — the subjects and periods it names all
    // survived.
    assert_eq!(
        extracted::<PairingRuleData>(&globals, "doomed_value"),
        expected
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed slot pairing rule stales the values that name it
///
/// The slots' twin of [a_removed_pairing_rule_stales_the_values_that_name_it]:
/// `to_data()` through a dead handle raises, and through a dead side view
/// too, while the value built before the removal still extracts — the slots
/// it names all survive.
#[test]
fn a_removed_slot_pairing_rule_stales_the_values_that_name_it() {
    let dir = workspace("slot-pairing-rule-data-stale");
    let source = example_copy(&dir, "source.collomatique");

    // The second of the example's two slot pairing rules, read from the file:
    // ids are stored, so this copy names the same rule the script is holding.
    let data = reload(&source);
    let doomed = data
        .get_inner_data()
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .keys()
        .nth(1)
        .expect("the example holds two slot pairing rules");
    let expected = data
        .get_inner_data()
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .get(&doomed)
        .cloned()
        .expect("the doomed rule exists before the stage");

    let globals = run_stages(
        &[
            include_str!("scripts/slot_pairing_rule_data_stale_before.py"),
            include_str!("scripts/slot_pairing_rule_data_stale_after.py"),
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

    // The value built before the removal is untouched, and still extracts to
    // the rule the file used to hold — the slots it names all survived.
    assert_eq!(
        extracted::<SlotPairingRuleData>(&globals, "doomed_value"),
        expected
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

/// The limits come back detached, out and back
///
/// The script walks `doc.settings` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the global entry,
/// Hermione's override (whose every field is set), Harry's resolved entry
/// (the global one), a partial entry built by hand with its `None` fields
/// intact, a zero per-week limit, and the model-default pin.
#[test]
fn the_limits_come_back_detached() {
    use collomatique_state_colloscopes::settings::Limits;
    use collomatique_state_colloscopes::settings::SoftParam;

    let dir = workspace("limits-data");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/limits_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let settings = &data.get_inner_data().params.settings;

    // The global entry and Hermione's override, whole — the same records the
    // document holds.
    assert_eq!(
        extracted::<LimitsData>(&globals, "global_value"),
        settings.global.clone()
    );
    let hermione = data
        .get_inner_data()
        .params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Granger")
        .expect("the example has Hermione")
        .0;
    assert_eq!(
        extracted::<LimitsData>(&globals, "hermione_value"),
        settings
            .students
            .get(&hermione)
            .expect("Hermione has an override")
            .clone()
    );

    // A student without an override resolves to the global entry, and the
    // value is that resolved entry.
    let harry = data
        .get_inner_data()
        .params
        .students
        .student_map
        .iter()
        .find(|(_id, student)| student.desc.surname == "Potter")
        .expect("the example has Harry")
        .0;
    assert_eq!(
        extracted::<LimitsData>(&globals, "harry_value"),
        settings.limits_for(harry).clone()
    );

    // The defaults: every field `None` — the model's own default, pinned so
    // the python side cannot drift.
    assert_eq!(
        extracted::<LimitsData>(&globals, "defaults"),
        Limits::default()
    );

    // The partial entry built by hand: one limit set, the two other fields
    // `None` — which is the whole-entry point, a `None` field disables the
    // inherited limit rather than inheriting it, and the value must carry the
    // `None`s across exactly as the model stores them.
    let partial = Limits {
        interrogations_per_week_min: Some(SoftParam {
            soft: true,
            value: 4,
        }),
        interrogations_per_week_max: None,
        max_interrogations_per_day: None,
    };
    assert_eq!(extracted::<LimitsData>(&globals, "partial"), partial);

    // A zero per-week limit is a value the model holds — a week with no
    // interrogation at all is a thing to say.
    assert_eq!(
        extracted::<LimitsData>(&globals, "week_min_zero"),
        Limits {
            interrogations_per_week_min: Some(SoftParam {
                soft: true,
                value: 0,
            }),
            interrogations_per_week_max: None,
            max_interrogations_per_day: None,
        }
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<LimitsData>(&globals, "day_zero"),
        (
            "ValueError".to_owned(),
            "a LimitsData's max_interrogations_per_day is at least 1, and 0 was given".to_owned(),
        )
    );
    assert_eq!(
        refused::<LimitsData>(&globals, "not_a_limit"),
        (
            "TypeError".to_owned(),
            "a LimitsData's interrogations_per_week_min is a Limit or None, and 3 is neither"
                .to_owned(),
        )
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// An override appearing and vanishing stales the value that names it
///
/// The mutation comes from rust, between the three halves, exactly as the
/// read surface's own staleness test stages it: a partial override for Harry
/// is installed, its raw view and its detached value are taken, and the
/// override is removed again. `to_data()` through the held raw view must then
/// die loudly — the message saying which entry is gone — while the value
/// built while the entry stood still extracts to the very entry that stood.
#[test]
fn an_override_appearing_and_vanishing_stales_the_value_that_names_it() {
    use collomatique_state_colloscopes::settings::Limits;
    use collomatique_state_colloscopes::settings::SoftParam;

    let dir = workspace("limits-data-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are
    // stored, so this copy names the same student the script is holding.
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
    let partial = Limits {
        interrogations_per_week_min: Some(SoftParam {
            soft: true,
            value: 4,
        }),
        interrogations_per_week_max: None,
        max_interrogations_per_day: None,
    };

    let mut stage = 0;
    let globals = run_stages(
        &[
            include_str!("scripts/limits_data_stale_before.py"),
            include_str!("scripts/limits_data_stale_override.py"),
            include_str!("scripts/limits_data_stale_after.py"),
        ],
        |globals| {
            globals.set_item("source", &source)?;
            Ok(())
        },
        |py, globals| {
            let op = match stage {
                0 => {
                    collomatique_ops::SettingsUpdateOp::UpdateStudentLimits(harry, partial.clone())
                }
                _ => collomatique_ops::SettingsUpdateOp::RemoveStudentLimits(harry),
            };
            stage += 1;

            document_of(globals)
                .borrow_mut(py)
                .update(py, collomatique_ops::UpdateOp::Settings(op))
                .expect("Harry's override is settable and removable");
        },
    );

    // The value written down while the entry stood is untouched, and still
    // extracts to the very entry that stood — the `None` fields included.
    assert_eq!(extracted::<LimitsData>(&globals, "partial_value"), partial);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The balancing comes back detached, out and back
///
/// The script walks `doc.balancing` and leaves what it saw; rust compares it
/// with the same document read straight from the model — the global entry
/// (whose `avoid_twice_in_a_row` is the `None` field, kept `None`),
/// Métamorphose's override (which hardens a rotation the global entry does
/// not pursue at all), a subject inheriting the global entry, a partial entry
/// built by hand with a goal left un-pursued, and the model-default pin.
#[test]
fn the_balancing_comes_back_detached() {
    use collomatique_state_colloscopes::balancing::BalancingOptions;
    use collomatique_state_colloscopes::settings::SoftParam;

    let dir = workspace("balancing-data");
    let source = example_copy(&dir, "source.collomatique");

    let globals = run(include_str!("scripts/balancing_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let balancing = &params.balancing;
    let subjects = &params.subjects;

    // The global entry, whole — with the goal it does not pursue kept `None`.
    assert_eq!(
        extracted::<BalancingData>(&globals, "global_value"),
        balancing.global.clone()
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
    assert_eq!(
        extracted::<BalancingData>(&globals, "metamorphose_value"),
        balancing
            .subjects
            .get(&metamorphose)
            .expect("Métamorphose has an override")
            .clone()
    );

    // A subject without an override resolves to the global entry, and the
    // value is that resolved entry.
    let arithmancie = subjects
        .ordered_subject_list
        .iter()
        .find(|(_id, subject)| subject.parameters.name == "Arithmancie")
        .expect("the example has Arithmancie")
        .0;
    assert_eq!(
        extracted::<BalancingData>(&globals, "arithmancie_value"),
        balancing.options_for(arithmancie).clone()
    );

    // The defaults: teacher rotation pursued as an objective, and nothing
    // else — the model's own default, pinned so the python side cannot drift.
    assert_eq!(
        extracted::<BalancingData>(&globals, "defaults"),
        BalancingOptions::default()
    );

    // The partial entry built by hand: one goal pursued, one hardened, one
    // left un-pursued — which is the whole-entry point, a `None` goal is
    // *not pursued*, never inherited — and the year switch on.
    assert_eq!(
        extracted::<BalancingData>(&globals, "hand_built"),
        BalancingOptions {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: Some(SoftParam {
                soft: false,
                value: (),
            }),
            year_teacher_rotation: true,
            period_teacher_rotation: false,
        }
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<BalancingData>(&globals, "not_an_enforcement"),
        (
            "TypeError".to_owned(),
            "a BalancingData's teacher_rotation is an Enforcement or None, and 3 is neither"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<BalancingData>(&globals, "not_a_goal"),
        (
            "TypeError".to_owned(),
            "a BalancingData's slot_rotation is an Enforcement or None, and 'OBJECTIVE' is neither"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<BalancingData>(&globals, "not_a_switch"),
        (
            "TypeError".to_owned(),
            "a BalancingData's year_teacher_rotation is True or False, and 1 is neither".to_owned(),
        )
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed balancing override stales the value that names it
///
/// The mutation comes from rust, between the two halves: Métamorphose's
/// balancing override goes. `to_data()` through the raw view held from the
/// first half must then die loudly, while the resolved view re-resolves to
/// the global entry — and the value built before the removal still extracts
/// to the entry the file held.
#[test]
fn a_removed_balancing_override_stales_the_value_that_names_it() {
    let dir = workspace("balancing-data-stale");
    let source = example_copy(&dir, "source.collomatique");

    // Read from the file rather than from the running document: ids are
    // stored, so this copy names the same subject the script is holding.
    let data = reload(&source);
    let metamorphose = data
        .get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .find(|(_id, subject)| subject.parameters.name == "Métamorphose")
        .expect("the example has Métamorphose")
        .0;
    let expected = data
        .get_inner_data()
        .params
        .balancing
        .subjects
        .get(&metamorphose)
        .expect("Métamorphose has an override")
        .clone();

    let globals = run_stages(
        &[
            include_str!("scripts/balancing_data_stale_before.py"),
            include_str!("scripts/balancing_data_stale_after.py"),
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

    // The value built before the removal is untouched, and still extracts to
    // the entry the file held.
    assert_eq!(
        extracted::<BalancingData>(&globals, "doomed_value"),
        expected
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
/// rather than halfway through the script.
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

/// The colloscope comes back detached, out and back
///
/// The script walks `doc.colloscope.to_data()` and leaves the value and the
/// readings it took through the handle surface; rust compares the value with
/// the same colloscope read straight from the model — the whole tree in one
/// extraction — the handle- and id-keyed spellings of one row both extracting
/// to the very row the model holds, the empty row accepted as "no row", and
/// the model's own default pinned so the python side cannot drift.
#[test]
fn the_colloscope_comes_back_detached() {
    use collomatique_ops::ColloscopeContents;

    let dir = workspace("colloscope-data");
    let source = dir.join("colloscope.collomatique");
    colloscope_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let globals = run(include_str!("scripts/colloscope_data.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let colloscope = &data.get_inner_data().colloscope;

    // The whole colloscope, out and back: the value extracts to the very
    // contents the model holds, rows and all.
    assert_eq!(
        extracted::<ColloscopeData>(&globals, "tree"),
        ColloscopeContents::from(colloscope)
    );

    // The readings the script took through the handles agree, position by
    // position: the slot's place within its subject, the week's global
    // index, and the sorted group numbers.
    let cells: Vec<_> = colloscope.iter().collect();
    let placements: Vec<_> = colloscope.group_lists_iter().collect();
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

    // And the placements rows: the list's position in `doc.group_lists`, and
    // the placements by surname.
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
        global::<Vec<(usize, Vec<(String, u32)>)>>(&globals, "row_reads"),
        expected_rows
    );

    // A handle and an id name the same row: both spellings of the first
    // stored cell and the first placed student extract to the same payload.
    let first_cell = *cells.first().expect("the fixture has cells");
    let (first_list, first_placements) = placements.first().expect("the fixture fills one list");
    let mut placed = BTreeMap::new();
    let (first_student, first_group) = first_placements
        .iter()
        .next()
        .expect("the fixture places a student");
    placed.insert(*first_student, *first_group);
    let expected_row = ColloscopeContents {
        interrogations: [(first_cell.0, first_cell.1.clone())].into_iter().collect(),
        group_lists: [(*first_list, placed)].into_iter().collect(),
    };
    assert_eq!(
        extracted::<ColloscopeData>(&globals, "by_handles"),
        expected_row
    );
    assert_eq!(
        extracted::<ColloscopeData>(&globals, "by_ids"),
        expected_row
    );

    // The empty row is "no row": an empty group set and an empty placement
    // map extract as the rows the payload promises its callers, without a
    // word of canonicalization.
    let mut empty = ColloscopeContents::default();
    empty.interrogations.insert(first_cell.0, BTreeSet::new());
    empty.group_lists.insert(*first_list, BTreeMap::new());
    assert_eq!(
        extracted::<ColloscopeData>(&globals, "with_empty_rows"),
        empty
    );

    // The default: the empty colloscope, what `clm.new_document()` holds —
    // pinned so the python side cannot drift.
    assert_eq!(
        extracted::<ColloscopeData>(&globals, "defaults"),
        ColloscopeContents::default()
    );

    // The refusals, each with the sentence it raises: the class a script
    // wrote down, the field, and what was given. The key refusals are the
    // argument convention's own — the same sentences the read surface's
    // coordinates raise.
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_table"),
        (
            "TypeError".to_owned(),
            "a ColloscopeData's interrogations is a mapping of (slot, week) pairs to sets of \
             group numbers, and [0] is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_cell_key"),
        (
            "TypeError".to_owned(),
            "a ColloscopeData's interrogations holds (slot, week) pairs, and 3 is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_week_key"),
        (
            "TypeError".to_owned(),
            "a week argument takes a Week or a WeekId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_groups"),
        (
            "TypeError".to_owned(),
            "a ColloscopeData's interrogations holds sets of group numbers, and 'x' is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_list_key"),
        (
            "TypeError".to_owned(),
            "a group list argument takes a GroupList or a GroupListId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_student_key"),
        (
            "TypeError".to_owned(),
            "a student argument takes a Student or a StudentId, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeData>(&globals, "bad_group_number"),
        (
            "TypeError".to_owned(),
            "a ColloscopeData's group_lists holds group numbers, and 'x' is not one".to_owned(),
        )
    );

    // A mapping naming one entity twice — a handle key and an id key, which
    // python cannot merge — is refused rather than keeping the last entry: a
    // cell, a placements row, and a student inside one placement.
    {
        use collomatique_state_colloscopes::group_lists::GroupListFilling;
        use collomatique_state_colloscopes::ids::Id as _;

        let (slot, week) = first_cell.0;
        assert_eq!(
            refused::<ColloscopeData>(&globals, "with_a_doubled_cell"),
            (
                "ValueError".to_owned(),
                format!(
                    "a ColloscopeData's interrogations names the (<SlotId {}>, <WeekId {}>) cell \
                     twice",
                    slot.inner(),
                    week.inner()
                ),
            )
        );

        let automatic = params
            .group_lists
            .group_list_map
            .iter()
            .find(|(_, list)| matches!(list.filling(), GroupListFilling::Automatic { .. }))
            .expect("the fixture has an automatic list")
            .0;
        assert_eq!(
            refused::<ColloscopeData>(&globals, "with_a_doubled_list"),
            (
                "ValueError".to_owned(),
                format!(
                    "a ColloscopeData's group_lists names <GroupListId {}> twice",
                    automatic.inner()
                ),
            )
        );

        let harry = params
            .students
            .student_map
            .iter()
            .find(|(_, student)| student.desc.surname == "Potter")
            .expect("the fixture holds Potter")
            .0;
        assert_eq!(
            refused::<ColloscopeData>(&globals, "with_a_doubled_student"),
            (
                "ValueError".to_owned(),
                format!(
                    "a ColloscopeData's group_lists names <StudentId {}> twice in one placement",
                    harry.inner()
                ),
            )
        );
    }

    // A reference that belongs to another document is stale, whatever its id
    // says — the same refusal every method of this api already makes.
    for name in ["foreign_slot", "foreign_group_list"] {
        let (kind, _message) = refused::<ColloscopeData>(&globals, name);
        assert_eq!(kind, "StaleHandleError");
    }

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A colloscope lands whole, through `install`
///
/// The family's fifth op, and the door a solve's outcome comes back through:
/// the script reads the colloscope out with `to_data()`, edits it the way
/// something that produced one would — one cell rewritten, one dropped, one
/// student moved — and puts it back with `install`.
///
/// Rust reads back the file the script saved: the document holds exactly the
/// value's rows and no others, which is what a dropped row makes visible —
/// nothing removed it, the value simply did not name it. What the script says
/// for itself is the rest: the empty `warnings` of a family nothing points at,
/// the single undo slot however much changed, the model's own refusal for a
/// group number past the bound, and the argument convention catching a value
/// that names a dead slot before the write ever runs.
///
/// The example was never resolved, so it has no colloscope to install a
/// changed copy of: this needs [colloscope_document], the fixture with cells
/// on several coordinates and one filled automatic list.
#[test]
fn a_colloscope_lands_whole_through_install() {
    use collomatique_ops::{ColloscopeContents, ColloscopeUpdateOp};

    let dir = workspace("colloscope-install");
    let source = dir.join("colloscope.collomatique");
    colloscope_document(&source);
    let target = dir.join("written.collomatique");

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entities the script is
    // holding.
    let data = reload(&source);
    let params = &data.get_inner_data().params;
    let original = ColloscopeContents::from(&data.get_inner_data().colloscope);

    assert!(
        original.interrogations.len() >= 2,
        "the script rewrites one cell and drops another"
    );
    assert_eq!(
        original.group_lists.len(),
        1,
        "the script names the single placements row"
    );

    // The rows the edit is about, found by the rule the script uses: the
    // value's key order is the model's, so the first and the last cell are
    // the same two on both sides.
    let cells: Vec<_> = original.interrogations.keys().copied().collect();
    let first_cell = *cells.first().expect("the fixture has cells");
    let dropped_cell = *cells.last().expect("the fixture has cells");
    assert_ne!(first_cell, dropped_cell);

    let (automatic, placed) = original
        .group_lists
        .iter()
        .next()
        .expect("the fixture fills one list");
    let group_count = params
        .group_lists
        .group_list_map
        .get(automatic)
        .expect("a placements row names a live group list")
        .params()
        .group_names
        .len();
    assert!(
        group_count >= 3,
        "the script rewrites a cell naming group 1 and refuses one naming group {group_count}"
    );

    // The colloscope the script installs: the first cell becomes `{0, 1}`,
    // the last one is gone, and the first placed student moves to the next
    // group round the list.
    let mut expected = original.clone();
    expected
        .interrogations
        .insert(first_cell, BTreeSet::from([0, 1]));
    expected.interrogations.remove(&dropped_cell);
    let mut moved = placed.clone();
    let (first_student, first_group) = moved
        .iter()
        .next()
        .map(|(student, group)| (*student, *group))
        .expect("the fixture places a student");
    moved.insert(
        first_student,
        (first_group + 1) % u32::try_from(group_count).expect("a group list holds few groups"),
    );
    assert_ne!(&moved, placed, "the script really moves the student");
    expected.group_lists.insert(*automatic, moved);

    // The french label the operation carries, so that the script's undo
    // assertion pins the operation's own name and not merely some string.
    // Only the variant is read, so the payload is the nearest one to hand.
    let install_label = ColloscopeUpdateOp::InstallColloscope(ColloscopeContents::default())
        .get_desc()
        .1;

    run(include_str!("scripts/colloscope_install.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("target", &target)?;
        globals.set_item("install_label", &install_label)?;
        Ok(())
    });

    // The document the script saved holds exactly the rows the value named:
    // the rewritten cell, the untouched ones, the moved placement — and not
    // the dropped cell, which no write removed.
    let written = reload(&target);
    let after = ColloscopeContents::from(&written.get_inner_data().colloscope);
    assert_eq!(after, expected);
    assert_eq!(
        after.interrogations.len() + 1,
        original.interrogations.len()
    );
    assert!(!after.interrogations.contains_key(&dropped_cell));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document written here rather than copied, holding a non-default export
/// configuration
///
/// The example's export configuration is essentially the default one, so the
/// shapes worth reading — every field away from the default, an extra color,
/// an auto-detected orientation — need a document of their own. The export
/// configuration is pure value data with no invariants linking it to the rest
/// of the document, so the fixture is a default `InnerData` with only that
/// section replaced, passed through `Data::from_inner_data` and written with
/// `serialize_data` like the colloscope fixture is.
fn export_config_document(path: &Path) {
    use collomatique_state_colloscopes::InnerData;
    use collomatique_state_colloscopes::export_config::{
        ColloscopeConfig, Color, ExportConfig, GlobalConfig, PageOrientation, PerGroupListConfig,
        PerStudentGroupsConfig,
    };

    let inner_data = InnerData {
        export_config: ExportConfig {
            global: GlobalConfig {
                background_color: Color {
                    red: 1,
                    green: 2,
                    blue: 3,
                },
                stripes_color_enabled: false,
                stripes_color: Color {
                    red: 4,
                    green: 5,
                    blue: 6,
                },
            },
            colloscope_enabled: false,
            all_groups_enabled: false,
            automatic_groups_enabled: true,
            prefilled_groups_enabled: true,
            per_group_list_enabled: false,
            colloscope_config: ColloscopeConfig {
                sheet_name: "Feuille".into(),
                extra_info_column_enabled: false,
                extra_info_column_name: "Notes".into(),
                teacher_email_enabled: false,
                teacher_email: "email".into(),
                teacher_tel_enabled: true,
                teacher_tel: "tel".into(),
                orientation: PageOrientation::Portrait,
                display_week_dates: false,
                display_annotations: false,
                no_interrogation_color: Color {
                    red: 7,
                    green: 8,
                    blue: 9,
                },
                annotation_color_enabled: false,
                annotation_color: Color {
                    red: 10,
                    green: 11,
                    blue: 12,
                },
                extra_colors: BTreeMap::from([(
                    "Vacances".to_owned(),
                    Color {
                        red: 13,
                        green: 14,
                        blue: 15,
                    },
                )]),
            },
            // The all-groups sheet reads as `orientation=None` — the
            // auto-detect case, the shape the script must see as `None` and
            // nothing else. The two other sheets hold a concrete orientation
            // each, so both spellings are read back.
            all_groups_config: PerStudentGroupsConfig {
                sheet_name: "Tous".into(),
                orientation: None,
                show_emails: false,
                show_tel: true,
            },
            automatic_groups_config: PerStudentGroupsConfig {
                sheet_name: "Auto".into(),
                orientation: Some(PageOrientation::Portrait),
                show_emails: false,
                show_tel: true,
            },
            prefilled_groups_config: PerStudentGroupsConfig {
                sheet_name: "Prérempli".into(),
                orientation: Some(PageOrientation::Landscape),
                show_emails: false,
                show_tel: true,
            },
            per_group_list_config: PerGroupListConfig {
                orientation: PageOrientation::Landscape,
                show_emails: false,
                show_tel: true,
                center_vertically: true,
            },
        },
        ..InnerData::default()
    };

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The export configuration reads back, field by field
///
/// The script walks `doc.export_config` and leaves what it saw; rust compares
/// it with the same document read straight from the model — the five flags,
/// every field of the four sections, and the extra colors.
///
/// The example's export configuration is essentially the default one, so the
/// non-default shapes need a document of their own: [export_config_document].
/// The script does the rest on its own, because it is about what python sees:
/// the `mappingproxy` that refuses assignment, the `orientation=None`
/// auto-detect case, and the views that compare equal by what they read.
#[test]
fn the_export_config_reads_back_field_by_field() {
    use collomatique_state_colloscopes::export_config::PageOrientation;

    let dir = workspace("export_config");
    let source = dir.join("export_config.collomatique");
    export_config_document(&source);

    let globals = run(include_str!("scripts/export_config.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let config = &data.get_inner_data().export_config;

    // The fixture is only worth reading if it has something to say: away from
    // the default on every field, so a partial read cannot pass by accident.
    assert_ne!(
        config,
        &collomatique_state_colloscopes::export_config::ExportConfig::default()
    );

    let color = |color: &collomatique_state_colloscopes::export_config::Color| {
        (color.red, color.green, color.blue)
    };
    let orientation = |orientation: &PageOrientation| match orientation {
        PageOrientation::Portrait => "Orientation.PORTRAIT",
        PageOrientation::Landscape => "Orientation.LANDSCAPE",
    };

    assert_eq!(
        global::<(bool, bool, bool, bool, bool)>(&globals, "flags"),
        (
            config.colloscope_enabled,
            config.all_groups_enabled,
            config.automatic_groups_enabled,
            config.prefilled_groups_enabled,
            config.per_group_list_enabled,
        )
    );

    assert_eq!(
        global::<((u8, u8, u8), bool, (u8, u8, u8))>(&globals, "global_reading"),
        (
            color(&config.global.background_color),
            config.global.stripes_color_enabled,
            color(&config.global.stripes_color),
        )
    );

    let colloscope = &config.colloscope_config;
    assert_eq!(
        global::<(
            (String, bool, String),
            (bool, String, bool, String),
            (String, bool, bool),
            (u8, u8, u8),
            (bool, u8, u8, u8),
        )>(&globals, "colloscope_reading"),
        (
            (
                colloscope.sheet_name.clone(),
                colloscope.extra_info_column_enabled,
                colloscope.extra_info_column_name.clone(),
            ),
            (
                colloscope.teacher_email_enabled,
                colloscope.teacher_email.clone(),
                colloscope.teacher_tel_enabled,
                colloscope.teacher_tel.clone(),
            ),
            (
                orientation(&colloscope.orientation).to_owned(),
                colloscope.display_week_dates,
                colloscope.display_annotations,
            ),
            color(&colloscope.no_interrogation_color),
            (
                colloscope.annotation_color_enabled,
                colloscope.annotation_color.red,
                colloscope.annotation_color.green,
                colloscope.annotation_color.blue,
            ),
        )
    );

    assert_eq!(
        global::<Vec<(String, (u8, u8, u8))>>(&globals, "extra_colors_items"),
        colloscope
            .extra_colors
            .iter()
            .map(|(name, c)| (name.clone(), color(c)))
            .collect::<Vec<_>>()
    );

    // The three per-student-groups sections, each read the way the script
    // reads them: sheet name, the orientation's repr (or `None` for the
    // auto-detected one), and the two booleans.
    let student_groups = [
        &config.all_groups_config,
        &config.automatic_groups_config,
        &config.prefilled_groups_config,
    ];
    let expected_groups = student_groups.map(|section| {
        (
            section.sheet_name.clone(),
            section
                .orientation
                .as_ref()
                .map(orientation)
                .map(str::to_owned),
            section.show_emails,
            section.show_tel,
        )
    });
    assert_eq!(
        global::<(
            (String, Option<String>, bool, bool),
            (String, Option<String>, bool, bool),
            (String, Option<String>, bool, bool),
        )>(&globals, "student_groups_readings"),
        (
            expected_groups[0].clone(),
            expected_groups[1].clone(),
            expected_groups[2].clone(),
        )
    );

    let per_group_list = &config.per_group_list_config;
    assert_eq!(
        global::<(String, bool, bool, bool)>(&globals, "group_list_reading"),
        (
            orientation(&per_group_list.orientation).to_owned(),
            per_group_list.show_emails,
            per_group_list.show_tel,
            per_group_list.center_vertically,
        )
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The export configuration comes back detached, out and back
///
/// The script walks `doc.export_config` and leaves what it saw; rust compares
/// it with the same document read straight from the model — the whole tree and
/// each of its sections, the six model defaults pinned against the model's own
/// builders (the three per-student-groups constructors included), the
/// auto-detected orientation round-tripping as `None`, and a detached tree
/// whose stripes were repainted extracting to the repainted configuration
/// rather than to the document's own.
#[test]
fn the_export_configuration_comes_back_detached() {
    use collomatique_state_colloscopes::export_config::{
        ColloscopeConfig, ExportConfig, GlobalConfig, PerGroupListConfig, PerStudentGroupsConfig,
    };

    let dir = workspace("export-config-data");
    let source = dir.join("export-config.collomatique");
    export_config_document(&source);

    let globals = run(include_str!("scripts/export_config_data.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    let data = reload(&source);
    let config = &data.get_inner_data().export_config;

    // The whole tree, and each of its sections on its own. The all-groups
    // sheet's auto-detected orientation is `None` on both sides of the trip.
    assert_eq!(
        extracted::<ExportConfigData>(&globals, "tree"),
        config.clone()
    );
    assert_eq!(
        extracted::<ExportGlobalConfigData>(&globals, "global_value"),
        config.global
    );
    assert_eq!(
        extracted::<ExportColloscopeConfigData>(&globals, "colloscope_value"),
        config.colloscope_config
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "all_groups_value"),
        config.all_groups_config
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "automatic_value"),
        config.automatic_groups_config
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "prefilled_value"),
        config.prefilled_groups_config
    );
    assert_eq!(
        extracted::<ExportGroupListConfigData>(&globals, "group_list_value"),
        config.per_group_list_config
    );

    // The repainted tree extracts to the repainted configuration: mutating a
    // detached value is a real mutation, and the value that comes back is the
    // one the script built, not the one the document holds.
    let mut repainted = config.clone();
    repainted.global.stripes_color = collomatique_state_colloscopes::export_config::Color {
        red: 9,
        green: 9,
        blue: 9,
    };
    assert_eq!(
        extracted::<ExportConfigData>(&globals, "mutated"),
        repainted
    );

    // The defaults: every section the model's own — the six section-level
    // builders of §3.9, the three per-student-groups constructors included,
    // and the whole tree — pinned so the python side cannot drift.
    assert_eq!(
        extracted::<ExportGlobalConfigData>(&globals, "defaults_global"),
        GlobalConfig::default()
    );
    assert_eq!(
        extracted::<ExportColloscopeConfigData>(&globals, "defaults_colloscope"),
        ColloscopeConfig::default()
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "defaults_student_all"),
        PerStudentGroupsConfig::default_all_groups()
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "defaults_student_automatic"),
        PerStudentGroupsConfig::default_automatic_groups()
    );
    assert_eq!(
        extracted::<ExportStudentGroupsConfigData>(&globals, "defaults_student_prefilled"),
        PerStudentGroupsConfig::default_prefilled_groups()
    );
    assert_eq!(
        extracted::<ExportGroupListConfigData>(&globals, "defaults_group_list"),
        PerGroupListConfig::default()
    );
    assert_eq!(
        extracted::<ExportConfigData>(&globals, "defaults_tree"),
        ExportConfig::default()
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the field, and what was given.
    assert_eq!(
        refused::<ExportGlobalConfigData>(&globals, "bad_global"),
        (
            "TypeError".to_owned(),
            "an ExportGlobalConfigData's background_color is a Color, and 'blanc' is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ExportColloscopeConfigData>(&globals, "bad_orientation"),
        (
            "TypeError".to_owned(),
            "an ExportColloscopeConfigData's orientation is an Orientation, and 'auto' is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ExportColloscopeConfigData>(&globals, "bad_colors"),
        (
            "TypeError".to_owned(),
            "an ExportColloscopeConfigData's extra_colors holds pairs of a name and a Color, \
             and ('Vacances', 'jaune') is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ExportColloscopeConfigData>(&globals, "bad_map"),
        (
            "TypeError".to_owned(),
            "an ExportColloscopeConfigData's extra_colors is a mapping of names to colors, \
             and ['Vacances'] is not one"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ExportStudentGroupsConfigData>(&globals, "bad_student_orientation"),
        (
            "TypeError".to_owned(),
            "an ExportStudentGroupsConfigData's orientation is an Orientation or None, \
             and 'auto' is neither"
                .to_owned(),
        )
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

/// A document written here rather than copied, holding at least one edge of
/// every site class
///
/// `referenced_by` needs a document where every reference the registry walks
/// appears at least once: exclusions on every kind that has them, an
/// assignments row and an association, teachers with subjects, slots with a
/// subject, a teacher and a week pattern, an incompatibility, pairing rules on
/// both levels, settings and balancing overrides, both group-list fillings,
/// and a filled colloscope. The example covers some of these and not the
/// others — subject pairing rules, overrides and a colloscope are its known
/// holes — so the fixture is built as an `InnerData` through the sealed types'
/// own constructors and passed through `Data::from_inner_data`, so a fixture
/// that breaks an invariant fails here rather than halfway through the script
///.
fn refs_document(path: &Path) {
    use collomatique_state_colloscopes::assignments::Assignments;
    use collomatique_state_colloscopes::balancing::{Balancing, BalancingOptions};
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
    };
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::incompats::{Incompatibility, Incompats};
    use collomatique_state_colloscopes::pairings::{PairingRule, Pairings, RulePart};
    use collomatique_state_colloscopes::settings::{Limits, Settings, SoftParam};
    use collomatique_state_colloscopes::slot_pairings::{
        SlotPairingRule, SlotPairings, SlotRulePart,
    };
    use collomatique_state_colloscopes::slots::{Slot, Slots};
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::teachers::{Teacher, Teachers};
    use collomatique_state_colloscopes::week_patterns::{WeekPattern, WeekPatterns};
    use collomatique_state_colloscopes::weeks::{WeekDesc, Weeks};
    use collomatique_state_colloscopes::{
        Data, GroupListId, IncompatId, InnerData, PairingRuleId, PeriodId, SlotId,
        SlotPairingRuleId, StudentId, Subject, SubjectId, SubjectInterrogationParameters,
        SubjectParameters, SubjectPeriodicity, TeacherId, WeekId, WeekPatternId,
    };

    // Ids nothing else in this document issues: it is written by hand from end
    // to end, so there is no issuer to keep in step with. The weeks are the
    // decoder's own synthesis in walk order on the other side, so their
    // numbers have nothing to be disjoint from — and the script only ever asks
    // about weeks with handles, which name their document.
    let period = |n: u64| unsafe { PeriodId::new(n) };
    let week = |n: u64| unsafe { WeekId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let teacher = |n: u64| unsafe { TeacherId::new(n) };
    let slot = |n: u64| unsafe { SlotId::new(n) };
    let student = |n: u64| unsafe { StudentId::new(n) };
    let group_list = |n: u64| unsafe { GroupListId::new(n) };
    let week_pattern = |n: u64| unsafe { WeekPatternId::new(n) };
    let incompat = |n: u64| unsafe { IncompatId::new(n) };
    let pairing_rule = |n: u64| unsafe { PairingRuleId::new(n) };
    let slot_pairing_rule = |n: u64| unsafe { SlotPairingRuleId::new(n) };

    let periods = vec![period(1), period(2)];

    // Both periods hold colles, except the fixture's last week, which is
    // switched off entirely. The week pattern's single exclusion names that
    // switched-off week, so it cannot trip any stored cell.
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

    // The first subject excludes the second period — the exclusion site — and
    // runs nothing there, so every row, association and cell the fixture
    // stores keeps off that pair.
    let subject_with = |name: &str, excluded_periods: BTreeSet<PeriodId>| Subject {
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
        excluded_periods,
    };
    let subjects = vec![
        (
            subject(11),
            subject_with("Sortilèges", BTreeSet::from([period(2)])),
        ),
        (subject(12), subject_with("Métamorphose", BTreeSet::new())),
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

    // The second slot follows the week pattern — the slot sides of the two
    // pattern sites.
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
                        week_pattern: Some(week_pattern(41)),
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

    // Ron sits the second period out — the student exclusion site. He is
    // neither assigned nor placed anywhere, so the exclusion trips nothing.
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
                excluded_periods: BTreeSet::from([period(2)]),
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

    let week_patterns = vec![(
        week_pattern(41),
        WeekPattern {
            name: "Semaine B".to_owned(),
            excluded_weeks: BTreeSet::from([week(84)]),
        },
    )];

    // The incompatibility's busy window runs the hour before the first slot of
    // its subject, on the same day — an edge both ends of the pattern sites.
    let window = |day: chrono::Weekday, hour, minute| {
        collomatique_time::SlotWithDuration::new(
            slot_start(collomatique_time::Weekday(day), hour, minute),
            collomatique_time::NonZeroMinutes::from(
                NonZeroU32::new(60).expect("an hour is a while"),
            ),
        )
        .expect("the window stays inside the day")
    };
    let incompats = vec![(
        incompat(91),
        Incompatibility {
            subject_id: subject(11),
            name: "Cours de potions".to_owned(),
            slots: vec![window(chrono::Weekday::Mon, 8, 0)],
            minimum_free_slots: NonZeroU32::new(1).expect("at least one"),
            week_pattern_id: Some(week_pattern(41)),
        },
    )];

    // Both rule families exclude the first period, and a rule whose antecedent
    // and consequent name different entities — the two value-internal
    // invariants the sealed constructors enforce.
    let rules = vec![(
        pairing_rule(101),
        PairingRule::new(
            RulePart {
                subject_id: subject(11),
                should_have: true,
            },
            RulePart {
                subject_id: subject(12),
                should_have: false,
            },
            BTreeSet::from([period(1)]),
            true,
        )
        .expect("the antecedent and the consequent name different subjects"),
    )];

    let slot_rules = vec![(
        slot_pairing_rule(111),
        SlotPairingRule::new(
            SlotRulePart {
                slot_id: slot(71),
                should_have: true,
            },
            SlotRulePart {
                slot_id: slot(72),
                should_have: false,
            },
            BTreeSet::from([period(1)]),
            false,
        )
        .expect("the antecedent and the consequent name different slots"),
    )];

    // The automatic list the solver filled, with one excluded student — the
    // student the exclusion site comes from — and the prefilled list whose
    // groups hold students.
    let named = |text: &str| {
        text.to_owned()
            .try_into()
            .expect("the fixture's group names are not empty")
    };
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
    // hand: a fixture that named an entity twice would otherwise quietly ship
    // one fewer than the script is about to read.
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
    inner_data.params.week_patterns = WeekPatterns {
        week_pattern_map: week_patterns.into_iter().collect(),
    };
    inner_data.params.incompats = Incompats {
        incompat_map: incompats.into_iter().collect(),
    };
    let (rule_count, slot_rule_count) = (rules.len(), slot_rules.len());
    inner_data.params.pairings = Pairings {
        pairing_rule_map: rules.into_iter().collect(),
    };
    inner_data.params.slot_pairings = SlotPairings {
        slot_pairing_rule_map: slot_rules.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.pairings.pairing_rule_map.len(),
        rule_count,
        "the fixture names each pairing rule once"
    );
    assert_eq!(
        inner_data.params.slot_pairings.slot_pairing_rule_map.len(),
        slot_rule_count,
        "the fixture names each slot pairing rule once"
    );
    // The automatic list serves the association pair on the first subject,
    // and the pair on the second subject's period — the three sides of the
    // association site.
    inner_data.params.group_lists = GroupLists {
        group_list_map: [(group_list(51), automatic), (group_list(52), prefilled)]
            .into_iter()
            .collect(),
        subjects_associations: [
            ((period(1), subject(11)), group_list(51)),
            ((period(2), subject(12)), group_list(51)),
        ]
        .into_iter()
        .collect(),
    };
    inner_data.params.assignments = Assignments {
        map: [
            (
                (period(1), subject(11)),
                BTreeSet::from([student(31), student(32)]),
            ),
            ((period(2), subject(12)), BTreeSet::from([student(34)])),
        ]
        .into_iter()
        .collect(),
    };
    // Hermione carries the settings override; Métamorphose the balancing one.
    inner_data.params.settings = Settings {
        global: Limits::default(),
        students: [(
            student(32),
            Limits {
                interrogations_per_week_min: Some(SoftParam {
                    soft: true,
                    value: 1,
                }),
                interrogations_per_week_max: Some(SoftParam {
                    soft: false,
                    value: 4,
                }),
                max_interrogations_per_day: Some(SoftParam {
                    soft: false,
                    value: NonZeroU32::new(2).expect("two interrogations"),
                }),
            },
        )]
        .into_iter()
        .collect(),
    };
    inner_data.params.balancing = Balancing {
        global: BalancingOptions::default(),
        subjects: [(
            subject(12),
            BalancingOptions {
                teacher_rotation: Some(SoftParam {
                    soft: true,
                    value: (),
                }),
                slot_rotation: None,
                avoid_twice_in_a_row: Some(SoftParam {
                    soft: false,
                    value: (),
                }),
                year_teacher_rotation: true,
                period_teacher_rotation: false,
            },
        )]
        .into_iter()
        .collect(),
    };

    // The colloscope itself, written through the canonical sparse writers:
    // three cells on three slots and three weeks — one of them carrying two
    // groups — and the automatic list filled. Every stored cell is possible:
    // the subject owning it runs on the week's period, the week holds colles,
    // and the pattern's exclusion is the switched-off week.
    inner_data
        .colloscope
        .set_interrogation(slot(71), week(81), BTreeSet::from([0, 2]));
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

/// What points at an entity reads back, place by place
///
/// The script walks `referenced_by()` on every entity of every referencable
/// kind and leaves what it saw; rust compares it with `references_to_*` mapped
/// through the same conversion — each site named by its class and its
/// coordinates, in the positions the script reads them in. The three kinds the
/// registry never targets are answered `()` by the script itself.
///
/// The example has no subject pairing rules, no overrides and no colloscope,
/// so the fixture — [refs_document] — is the document under test.
#[test]
fn what_points_at_an_entity() {
    use collomatique_state_colloscopes::refs::{
        GroupListRefSite, PeriodRefSite, SlotRefSite, StudentRefSite, SubjectRefSite,
        TeacherRefSite, WeekPatternRefSite, WeekRefSite,
    };

    let dir = workspace("refs");
    let source = dir.join("refs.collomatique");
    refs_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let globals = run(include_str!("scripts/refs.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    let data = reload(&source);
    let params = &data.get_inner_data().params;

    // A coordinate's place in its own collection, the number the script reads
    // as `.index` or as the position of `enumerate` — the only way an opaque
    // id crosses the boundary.
    fn position<T: PartialEq>(ids: &[T], id: &T) -> usize {
        ids.iter()
            .position(|x| x == id)
            .expect("a site names a live entity")
    }

    let period_ids: Vec<_> = params.periods.period_ids().collect();
    let week_ids: Vec<_> = params.week_ids().collect();
    let subject_ids: Vec<_> = params.subjects.ordered_subject_list.keys().collect();
    let teacher_ids: Vec<_> = params.teachers.teacher_map.keys().collect();
    let student_ids: Vec<_> = params.students.student_map.keys().collect();
    let week_pattern_ids: Vec<_> = params.week_patterns.week_pattern_map.keys().collect();
    let incompat_ids: Vec<_> = params.incompats.incompat_map.keys().collect();
    let group_list_ids: Vec<_> = params.group_lists.group_list_map.keys().collect();
    let pairing_rule_ids: Vec<_> = params.pairings.pairing_rule_map.keys().collect();
    let slot_pairing_rule_ids: Vec<_> = params.slot_pairings.slot_pairing_rule_map.keys().collect();
    // The model keeps no single slot table to read ids from, so the walk the
    // `doc.slots` view makes is composed here too: each subject, then its own
    // slots.
    let slot_ids: Vec<_> = subject_ids
        .iter()
        .flat_map(|subject| {
            params
                .slots
                .slots_for_subject(*subject)
                .into_iter()
                .flatten()
                .map(|(slot, _desc)| *slot)
        })
        .collect();

    // The fixture is only worth reading if every site class has at least one
    // edge to read; the twenty-four names collected below are that check.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut compare = |name: &str, expected: Vec<Vec<(String, Vec<usize>)>>| {
        assert_eq!(
            global::<Vec<Vec<(String, Vec<usize>)>>>(&globals, name),
            expected,
            "{name}"
        );
        for row in &expected {
            for (class, _coords) in row {
                seen.insert(class.clone());
            }
        }
    };

    let expected_periods: Vec<Vec<(String, Vec<usize>)>> = period_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_period(id)
                .iter()
                .map(|site| match site {
                    PeriodRefSite::WeekPeriodFk(week) => {
                        ("WeekPeriod", vec![position(&week_ids, week)])
                    }
                    PeriodRefSite::SubjectExcludedPeriods(subject) => (
                        "SubjectExcludedPeriod",
                        vec![position(&subject_ids, subject)],
                    ),
                    PeriodRefSite::StudentExcludedPeriods(student) => (
                        "StudentExcludedPeriod",
                        vec![position(&student_ids, student)],
                    ),
                    PeriodRefSite::PairingRuleExcludedPeriods(rule) => (
                        "PairingRuleExcludedPeriod",
                        vec![position(&pairing_rule_ids, rule)],
                    ),
                    PeriodRefSite::SlotPairingRuleExcludedPeriods(rule) => (
                        "SlotPairingRuleExcludedPeriod",
                        vec![position(&slot_pairing_rule_ids, rule)],
                    ),
                    PeriodRefSite::AssignmentsKey { subject } => (
                        "AssignmentRow",
                        vec![position(&period_ids, &id), position(&subject_ids, subject)],
                    ),
                    PeriodRefSite::AssociationEntry { subject } => (
                        "GroupListAssociation",
                        vec![position(&period_ids, &id), position(&subject_ids, subject)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_weeks: Vec<Vec<(String, Vec<usize>)>> = week_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_week(id)
                .iter()
                .map(|site| match site {
                    WeekRefSite::WeekPatternExcludedWeek(week_pattern) => (
                        "WeekPatternExcludedWeek",
                        vec![position(&week_pattern_ids, week_pattern)],
                    ),
                    WeekRefSite::ColloscopeInterrogation { slot } => (
                        "ColloscopeInterrogation",
                        vec![position(&slot_ids, slot), position(&week_ids, &id)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_subjects: Vec<Vec<(String, Vec<usize>)>> = subject_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_subject(id)
                .iter()
                .map(|site| match site {
                    SubjectRefSite::TeacherSubjects(teacher) => {
                        ("TeacherSubject", vec![position(&teacher_ids, teacher)])
                    }
                    SubjectRefSite::SlotSubject(slot) => {
                        ("SlotSubject", vec![position(&slot_ids, slot)])
                    }
                    SubjectRefSite::IncompatSubject(incompat) => {
                        ("IncompatSubject", vec![position(&incompat_ids, incompat)])
                    }
                    SubjectRefSite::PairingRuleAntecedent(rule) => (
                        "PairingRuleAntecedent",
                        vec![position(&pairing_rule_ids, rule)],
                    ),
                    SubjectRefSite::PairingRuleConsequent(rule) => (
                        "PairingRuleConsequent",
                        vec![position(&pairing_rule_ids, rule)],
                    ),
                    SubjectRefSite::BalancingSubjectKey => {
                        ("BalancingOverride", vec![position(&subject_ids, &id)])
                    }
                    SubjectRefSite::AssignmentsKey { period } => (
                        "AssignmentRow",
                        vec![position(&period_ids, period), position(&subject_ids, &id)],
                    ),
                    SubjectRefSite::AssociationEntry { period } => (
                        "GroupListAssociation",
                        vec![position(&period_ids, period), position(&subject_ids, &id)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_teachers: Vec<Vec<(String, Vec<usize>)>> = teacher_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_teacher(id)
                .iter()
                .map(|site| match site {
                    TeacherRefSite::SlotTeacher(slot) => {
                        ("SlotTeacher", vec![position(&slot_ids, slot)])
                    }
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_students: Vec<Vec<(String, Vec<usize>)>> = student_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_student(id)
                .iter()
                .map(|site| match site {
                    StudentRefSite::GroupListPrefilledStudent(group_list) => (
                        "GroupListPrefilledStudent",
                        vec![position(&group_list_ids, group_list)],
                    ),
                    StudentRefSite::GroupListExcludedStudent(group_list) => (
                        "GroupListExcludedStudent",
                        vec![position(&group_list_ids, group_list)],
                    ),
                    StudentRefSite::SettingsStudentKey => {
                        ("SettingsOverride", vec![position(&student_ids, &id)])
                    }
                    StudentRefSite::AssignmentsStudent { period, subject } => (
                        "AssignmentRow",
                        vec![
                            position(&period_ids, period),
                            position(&subject_ids, subject),
                        ],
                    ),
                    StudentRefSite::ColloscopeGroupListStudent(group_list) => (
                        "ColloscopeGroupListRow",
                        vec![position(&group_list_ids, group_list)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_week_patterns: Vec<Vec<(String, Vec<usize>)>> = week_pattern_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_week_pattern(id)
                .iter()
                .map(|site| match site {
                    WeekPatternRefSite::SlotWeekPattern(slot) => {
                        ("SlotWeekPattern", vec![position(&slot_ids, slot)])
                    }
                    WeekPatternRefSite::IncompatWeekPattern(incompat) => (
                        "IncompatWeekPattern",
                        vec![position(&incompat_ids, incompat)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_slots: Vec<Vec<(String, Vec<usize>)>> = slot_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_slot(id)
                .iter()
                .map(|site| match site {
                    SlotRefSite::SlotPairingRuleAntecedent(rule) => (
                        "SlotPairingRuleAntecedent",
                        vec![position(&slot_pairing_rule_ids, rule)],
                    ),
                    SlotRefSite::SlotPairingRuleConsequent(rule) => (
                        "SlotPairingRuleConsequent",
                        vec![position(&slot_pairing_rule_ids, rule)],
                    ),
                    SlotRefSite::ColloscopeInterrogation { week } => (
                        "ColloscopeInterrogation",
                        vec![position(&slot_ids, &id), position(&week_ids, week)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    let expected_group_lists: Vec<Vec<(String, Vec<usize>)>> = group_list_ids
        .iter()
        .map(|&id| {
            data.get_inner_data()
                .references_to_group_list(id)
                .iter()
                .map(|site| match site {
                    GroupListRefSite::AssociationEntry { period, subject } => (
                        "GroupListAssociation",
                        vec![
                            position(&period_ids, period),
                            position(&subject_ids, subject),
                        ],
                    ),
                    GroupListRefSite::ColloscopeGroupListKey => (
                        "ColloscopeGroupListRow",
                        vec![position(&group_list_ids, &id)],
                    ),
                })
                .map(|(class, coords)| (class.to_owned(), coords))
                .collect()
        })
        .collect();

    compare("period_refs", expected_periods);
    compare("week_refs", expected_weeks);
    compare("subject_refs", expected_subjects);
    compare("teacher_refs", expected_teachers);
    compare("student_refs", expected_students);
    compare("week_pattern_refs", expected_week_patterns);
    compare("slot_refs", expected_slots);
    compare("group_list_refs", expected_group_lists);

    assert_eq!(
        seen.len(),
        24,
        "the fixture holds at least one edge of every site class"
    );
    assert!(global::<bool>(&globals, "never_referenced"));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A removed entity makes its `referenced_by` raise
///
/// Stale is loud on the reverse door like everywhere else: once the entity is
/// gone, `referenced_by()` raises `StaleHandleError` — including for the three
/// kinds whose alive answer is always `()`. The read surface ships no removes,
/// so the four `UpdateOp`s land between two stages, on the [refs_document]
/// fixture, whose last subject carries the colloscope cells, the association
/// and the balancing override the cascade has to take with it.
#[test]
fn a_removed_entity_makes_its_referenced_by_raise() {
    let dir = workspace("refs-stale");
    let source = dir.join("refs.collomatique");
    refs_document(&source);

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the same entities the script is
    // holding. The doomed subject is removed last: the pairing rule and the
    // slot pairing rule that reference it (and that the cascade would
    // otherwise take down with it) are gone first.
    let fixture = reload(&source);
    let inner = fixture.get_inner_data();
    let doomed_subject = inner
        .params
        .subjects
        .ordered_subject_list
        .keys()
        .last()
        .expect("the fixture has subjects");
    let doomed_incompat = inner
        .params
        .incompats
        .incompat_map
        .keys()
        .next()
        .expect("the fixture has an incompatibility");
    let doomed_rule = inner
        .params
        .pairings
        .pairing_rule_map
        .keys()
        .next()
        .expect("the fixture has a pairing rule");
    let doomed_slot_rule = inner
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .keys()
        .next()
        .expect("the fixture has a slot pairing rule");

    run_stages(
        &[
            include_str!("scripts/refs_stale_before.py"),
            include_str!("scripts/refs_stale_after.py"),
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
                    collomatique_ops::UpdateOp::Pairings(
                        collomatique_ops::PairingsUpdateOp::DeletePairingRule(doomed_rule),
                    ),
                )
                .expect("the pairing rule is removable");
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::SlotPairings(
                        collomatique_ops::SlotPairingsUpdateOp::DeleteSlotPairingRule(
                            doomed_slot_rule,
                        ),
                    ),
                )
                .expect("the slot pairing rule is removable");
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Incompatibilities(
                        collomatique_ops::IncompatibilitiesUpdateOp::DeleteIncompat(
                            doomed_incompat,
                        ),
                    ),
                )
                .expect("the incompatibility is removable");
            doc.borrow_mut(py)
                .update(
                    py,
                    collomatique_ops::UpdateOp::Subjects(
                        collomatique_ops::SubjectsUpdateOp::DeleteSubject(doomed_subject),
                    ),
                )
                .expect("the subject is removable");
        },
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A document exercising every section of the snapshot, written to `path`
///
/// The completeness check of the whole milestone needs a document that has
/// something in every section: both shapes of every optional field, stored
/// rows in both junction tables, a filled colloscope, and non-default
/// settings, balancing and export configuration. The ids are picked by hand
/// the way `refs_document` does, every reference stays live, and every stored
/// row keeps off the pairs its subject or its students exclude — so the
/// document passes `Data::from_inner_data` on the way in.
fn snapshot_document(path: &Path) {
    use collomatique_state_colloscopes::assignments::Assignments;
    use collomatique_state_colloscopes::balancing::{Balancing, BalancingOptions};
    use collomatique_state_colloscopes::export_config::{
        ColloscopeConfig, Color, ExportConfig, GlobalConfig, PageOrientation, PerGroupListConfig,
        PerStudentGroupsConfig,
    };
    use collomatique_state_colloscopes::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
    };
    use collomatique_state_colloscopes::ids::Id as _;
    use collomatique_state_colloscopes::incompats::{Incompatibility, Incompats};
    use collomatique_state_colloscopes::pairings::{PairingRule, Pairings, RulePart};
    use collomatique_state_colloscopes::settings::{Limits, Settings, SoftParam};
    use collomatique_state_colloscopes::slot_pairings::{
        SlotPairingRule, SlotPairings, SlotRulePart,
    };
    use collomatique_state_colloscopes::slots::{Slot, Slots};
    use collomatique_state_colloscopes::students::{Student, Students};
    use collomatique_state_colloscopes::subjects::Subjects;
    use collomatique_state_colloscopes::teachers::{Teacher, Teachers};
    use collomatique_state_colloscopes::week_patterns::{WeekPattern, WeekPatterns};
    use collomatique_state_colloscopes::weeks::{WeekDesc, Weeks};
    use collomatique_state_colloscopes::{
        Data, GroupListId, IncompatId, InnerData, PairingRuleId, PeriodId, SlotId,
        SlotPairingRuleId, StudentId, Subject, SubjectId, SubjectInterrogationParameters,
        SubjectParameters, SubjectPeriodicity, TeacherId, WeekId, WeekPatternId,
    };

    let period = |n: u64| unsafe { PeriodId::new(n) };
    let week = |n: u64| unsafe { WeekId::new(n) };
    let subject = |n: u64| unsafe { SubjectId::new(n) };
    let teacher = |n: u64| unsafe { TeacherId::new(n) };
    let slot = |n: u64| unsafe { SlotId::new(n) };
    let student = |n: u64| unsafe { StudentId::new(n) };
    let group_list = |n: u64| unsafe { GroupListId::new(n) };
    let week_pattern = |n: u64| unsafe { WeekPatternId::new(n) };
    let incompat = |n: u64| unsafe { IncompatId::new(n) };
    let pairing_rule = |n: u64| unsafe { PairingRuleId::new(n) };
    let slot_pairing_rule = |n: u64| unsafe { SlotPairingRuleId::new(n) };

    // The model's optional non-empty string, reached without naming its crate:
    // the field says what the conversion lands in, so the fixture needs no
    // dependency of its own to build one.
    let named = |text: &str| {
        text.to_owned()
            .try_into()
            .expect("the fixture's names are not empty")
    };

    let periods = vec![period(1), period(2)];

    // The colles start on a Monday — the one date the model stores. One week
    // carries an annotation and one runs no colles, the two shapes of the
    // weeks' optional halves.
    let first_week = collomatique_time::WeekStart::new(
        chrono::NaiveDate::from_ymd_opt(2026, 8, 10).expect("a date"),
    )
    .expect("a Monday");
    let weeks = vec![
        (
            period(1),
            vec![
                (
                    week(81),
                    WeekDesc {
                        interrogations: true,
                        annotation: Some(named("Rentrée")),
                    },
                ),
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

    // The first subject excludes the second period, and the second student
    // does too — the two exclusion shapes, each kept off every row it would
    // trip.
    let subject_with = |name: &str, excluded_periods: BTreeSet<PeriodId>| Subject {
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
        excluded_periods,
    };
    let subjects = vec![
        (
            subject(11),
            subject_with("Sortilèges", BTreeSet::from([period(2)])),
        ),
        (subject(12), subject_with("Métamorphose", BTreeSet::new())),
    ];

    // One teacher with contact details and one without, so both shapes of the
    // optional halves of a person are in the tree.
    let teachers = vec![
        (
            teacher(21),
            Teacher {
                desc: person(
                    "Minerva",
                    "McGonagall",
                    Some("0601020304"),
                    Some("minerva@lycee.fr"),
                ),
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
                        extra_info: "Bâtiment B".to_owned(),
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
                        week_pattern: Some(week_pattern(41)),
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
                excluded_periods: BTreeSet::from([period(2)]),
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

    let week_patterns = vec![(
        week_pattern(41),
        WeekPattern {
            name: "Semaine B".to_owned(),
            excluded_weeks: BTreeSet::from([week(84)]),
        },
    )];

    let window = |day: chrono::Weekday, hour, minute| {
        collomatique_time::SlotWithDuration::new(
            slot_start(collomatique_time::Weekday(day), hour, minute),
            collomatique_time::NonZeroMinutes::from(
                NonZeroU32::new(60).expect("an hour is a while"),
            ),
        )
        .expect("the window stays inside the day")
    };

    let incompats = vec![(
        incompat(91),
        Incompatibility {
            subject_id: subject(11),
            name: "Cours de potions".to_owned(),
            slots: vec![window(chrono::Weekday::Mon, 8, 0)],
            minimum_free_slots: NonZeroU32::new(1).expect("at least one"),
            week_pattern_id: Some(week_pattern(41)),
        },
    )];

    // Both rule families exclude the first period, and a rule whose antecedent
    // and consequent name different entities — the two value-internal
    // invariants the sealed constructors enforce.
    let rules = vec![(
        pairing_rule(101),
        PairingRule::new(
            RulePart {
                subject_id: subject(11),
                should_have: true,
            },
            RulePart {
                subject_id: subject(12),
                should_have: false,
            },
            BTreeSet::from([period(1)]),
            true,
        )
        .expect("the antecedent and the consequent name different subjects"),
    )];

    let slot_rules = vec![(
        slot_pairing_rule(111),
        SlotPairingRule::new(
            SlotRulePart {
                slot_id: slot(71),
                should_have: true,
            },
            SlotRulePart {
                slot_id: slot(72),
                should_have: false,
            },
            BTreeSet::from([period(1)]),
            false,
        )
        .expect("the antecedent and the consequent name different slots"),
    )];

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
    inner_data.params.periods = collomatique_state_colloscopes::periods::Periods::from_ordered_ids(
        Some(first_week),
        periods,
    )
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
    // hand: a fixture that named an entity twice would otherwise quietly ship
    // one fewer than the script is about to read.
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
    inner_data.params.week_patterns = WeekPatterns {
        week_pattern_map: week_patterns.into_iter().collect(),
    };
    inner_data.params.incompats = Incompats {
        incompat_map: incompats.into_iter().collect(),
    };
    let (rule_count, slot_rule_count) = (rules.len(), slot_rules.len());
    inner_data.params.pairings = Pairings {
        pairing_rule_map: rules.into_iter().collect(),
    };
    inner_data.params.slot_pairings = SlotPairings {
        slot_pairing_rule_map: slot_rules.into_iter().collect(),
    };
    assert_eq!(
        inner_data.params.pairings.pairing_rule_map.len(),
        rule_count,
        "the fixture names each pairing rule once"
    );
    assert_eq!(
        inner_data.params.slot_pairings.slot_pairing_rule_map.len(),
        slot_rule_count,
        "the fixture names each slot pairing rule once"
    );
    // The automatic list serves every pair a stored cell stands on, and the
    // prefilled one serves nothing at all.
    inner_data.params.group_lists = GroupLists {
        group_list_map: [(group_list(51), automatic), (group_list(52), prefilled)]
            .into_iter()
            .collect(),
        subjects_associations: [
            ((period(1), subject(11)), group_list(51)),
            ((period(2), subject(12)), group_list(51)),
        ]
        .into_iter()
        .collect(),
    };
    inner_data.params.assignments = Assignments {
        map: [
            (
                (period(1), subject(11)),
                BTreeSet::from([student(31), student(32)]),
            ),
            ((period(2), subject(12)), BTreeSet::from([student(34)])),
        ]
        .into_iter()
        .collect(),
    };
    // The global limits entry and the per-student override, both whole and
    // non-default: Hermione's override disables the per-day limit, which the
    // whole-entry rule says stays a `None` field.
    inner_data.params.settings = Settings {
        global: Limits {
            interrogations_per_week_min: Some(SoftParam {
                soft: true,
                value: 2,
            }),
            interrogations_per_week_max: Some(SoftParam {
                soft: false,
                value: 4,
            }),
            max_interrogations_per_day: Some(SoftParam {
                soft: true,
                value: NonZeroU32::new(3).expect("three interrogations"),
            }),
        },
        students: [(
            student(32),
            Limits {
                interrogations_per_week_min: Some(SoftParam {
                    soft: true,
                    value: 1,
                }),
                interrogations_per_week_max: Some(SoftParam {
                    soft: false,
                    value: 4,
                }),
                max_interrogations_per_day: None,
            },
        )]
        .into_iter()
        .collect(),
    };
    inner_data.params.balancing = Balancing {
        global: BalancingOptions {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: Some(SoftParam {
                soft: false,
                value: (),
            }),
            year_teacher_rotation: true,
            period_teacher_rotation: false,
        },
        subjects: [(
            subject(12),
            BalancingOptions {
                teacher_rotation: Some(SoftParam {
                    soft: false,
                    value: (),
                }),
                slot_rotation: None,
                avoid_twice_in_a_row: None,
                year_teacher_rotation: false,
                period_teacher_rotation: true,
            },
        )]
        .into_iter()
        .collect(),
    };

    // The colloscope itself, written through the canonical sparse writers:
    // three cells on three slots and three weeks — one of them carrying two
    // groups — and the automatic list filled. Every stored cell is possible:
    // its subject runs on the week's period, the week holds colles, and the
    // pattern's exclusion is the switched-off week.
    inner_data
        .colloscope
        .set_interrogation(slot(71), week(81), BTreeSet::from([0, 2]));
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

    // The export configuration, deliberately far from the defaults: sheet
    // names and colors a user chose, the annotations hidden, the tel column
    // on and the emails off, and one extra cell color.
    inner_data.export_config = ExportConfig {
        global: GlobalConfig {
            background_color: Color {
                red: 240,
                green: 240,
                blue: 245,
            },
            stripes_color_enabled: false,
            stripes_color: Color {
                red: 220,
                green: 220,
                blue: 230,
            },
        },
        colloscope_enabled: true,
        all_groups_enabled: true,
        automatic_groups_enabled: true,
        prefilled_groups_enabled: false,
        per_group_list_enabled: true,
        colloscope_config: ColloscopeConfig {
            sheet_name: "Colles".to_owned(),
            extra_info_column_enabled: true,
            extra_info_column_name: "Salle".to_owned(),
            teacher_email_enabled: false,
            teacher_email: "Contact".to_owned(),
            teacher_tel_enabled: true,
            teacher_tel: "0601020304".to_owned(),
            orientation: PageOrientation::Landscape,
            display_week_dates: true,
            display_annotations: false,
            no_interrogation_color: Color {
                red: 200,
                green: 200,
                blue: 200,
            },
            annotation_color_enabled: true,
            annotation_color: Color {
                red: 255,
                green: 255,
                blue: 0,
            },
            extra_colors: [(
                "Vacances".to_owned(),
                Color {
                    red: 255,
                    green: 240,
                    blue: 200,
                },
            )]
            .into_iter()
            .collect(),
        },
        all_groups_config: PerStudentGroupsConfig::default_all_groups(),
        automatic_groups_config: {
            let mut config = PerStudentGroupsConfig::default_automatic_groups();
            config.orientation = Some(PageOrientation::Portrait);
            config.show_tel = true;
            config
        },
        prefilled_groups_config: PerStudentGroupsConfig::default_prefilled_groups(),
        per_group_list_config: PerGroupListConfig {
            orientation: PageOrientation::Portrait,
            show_emails: true,
            show_tel: false,
            center_vertically: true,
        },
    };

    let data = Data::from_inner_data(inner_data).expect("the fixture should be a valid document");
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .expect("the fixture's ids are far below the file-format ceiling");
    std::fs::write(path, content).expect("the fixture should be writable");
}

/// The snapshot holds the whole document, section by section
///
/// The tree `doc.snapshot()` hands out is the value milestone's payoff: the
/// same conversion `to_data()` is, run over everything at once. Rust extracts
/// the tree the script left behind and compares it with the document read
/// straight from the model — the completeness check of the whole milestone in
/// one assertion, since every section must come back exactly for the two to
/// be equal. A field this document's own design forgot would fail here even
/// if no test ever named it.
#[test]
fn the_snapshot_holds_the_whole_document() {
    let dir = workspace("snapshot");
    let source = dir.join("snapshot.collomatique");
    snapshot_document(&source);
    let other_source = example_copy(&dir, "other.collomatique");

    let globals = run(include_str!("scripts/document_data.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        Ok(())
    });

    let data = reload(&source);
    let inner = data.get_inner_data();

    // The whole tree, out and back — every section, every row, every order.
    assert_eq!(extracted::<DocumentData>(&globals, "tree"), *inner);

    // The handle- and id-keyed spellings of one tree extract to the same
    // document — the §2.3 rule at tree scale.
    assert_eq!(
        extracted::<DocumentData>(&globals, "by_handles"),
        extracted::<DocumentData>(&globals, "by_ids"),
    );

    // The empty tree is the model's own empty document, pinned the way every
    // default is, so the python side cannot drift.
    assert_eq!(
        extracted::<DocumentData>(&globals, "defaults"),
        collomatique_state_colloscopes::InnerData::default(),
    );

    // The refusals, each with the sentence it raises: the class the script
    // wrote down, the section, and what was given.
    assert_eq!(
        refused::<DocumentData>(&globals, "not_a_tree"),
        (
            "TypeError".to_owned(),
            "a DocumentData is expected here, and 3 has no first_week".to_owned(),
        )
    );
    assert_eq!(
        refused::<DocumentData>(&globals, "not_a_monday"),
        (
            "ValueError".to_owned(),
            "a DocumentData's first_week is a Monday, and 2026-08-12 is not one".to_owned(),
        )
    );
    assert_eq!(
        refused::<DocumentData>(&globals, "not_a_section"),
        (
            "TypeError".to_owned(),
            "a DocumentData's subjects is a mapping of entities to values, and 3 is not one"
                .to_owned(),
        )
    );

    // A section naming one entity twice — a handle key and an id key, which
    // python cannot merge — is refused rather than keeping the last entry, in
    // the section's own words: an entity section, and the two junction tables.
    {
        use collomatique_state_colloscopes::ids::Id as _;

        let first_teacher = inner
            .params
            .teachers
            .teacher_map
            .keys()
            .next()
            .expect("the fixture has teachers");
        assert_eq!(
            refused::<DocumentData>(&globals, "with_a_doubled_teacher"),
            (
                "ValueError".to_owned(),
                format!(
                    "a DocumentData's teachers names <TeacherId {}> twice",
                    first_teacher.inner()
                ),
            )
        );

        let first_period = inner
            .params
            .periods
            .period_ids()
            .next()
            .expect("the fixture has periods");
        let first_subject = inner
            .params
            .subjects
            .ordered_subject_list
            .keys()
            .next()
            .expect("the fixture has subjects");
        assert_eq!(
            refused::<DocumentData>(&globals, "with_a_doubled_assignment"),
            (
                "ValueError".to_owned(),
                format!(
                    "a DocumentData's assignments names the (<PeriodId {}>, <SubjectId {}>) row \
                     twice",
                    first_period.inner(),
                    first_subject.inner()
                ),
            )
        );
        assert_eq!(
            refused::<DocumentData>(&globals, "with_a_doubled_association"),
            (
                "ValueError".to_owned(),
                format!(
                    "a DocumentData's group_list_associations names the (<PeriodId {}>, \
                     <SubjectId {}>) row twice",
                    first_period.inner(),
                    first_subject.inner()
                ),
            )
        );
    }

    // A week of another document names nothing here — the same refusal every
    // method of this api already makes.
    let (kind, _message) = refused::<DocumentData>(&globals, "with_a_foreign_week");
    assert_eq!(kind, "StaleHandleError");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A whole tree goes back in, as one step
///
/// The coarse door of §8. The script's own assertions cover the shape of the
/// call — the undo slot, the empty warnings, the refusals — and rust holds the
/// two halves a script cannot see: that the document it saved really is the
/// tree it handed over, field for field, and that a refused tree names *every*
/// reference it left dangling rather than the first one it hit.
#[test]
fn a_whole_tree_goes_back_in_as_one_step() {
    let dir = workspace("replace_all");
    let source = example_copy(&dir, "source.collomatique");
    let other_source = example_copy(&dir, "other.collomatique");
    let target = dir.join("replaced.collomatique");

    let before = reload(&source);
    let before = before.get_inner_data();
    let fold_label = "Import Pronote";

    let globals = run(include_str!("scripts/replace_all.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("other_source", &other_source)?;
        globals.set_item("target", &target)?;
        globals.set_item("fold_label", fold_label)?;
        Ok(())
    });

    // The document the script saved *is* the tree it handed to `replace_all`,
    // read back through the same extraction the call itself used. Nothing is
    // sampled here: a global update that dropped a section, reordered one or
    // rewrote a field on the way through would fail this.
    let written = reload(&target);
    assert_eq!(
        extracted::<DocumentData>(&globals, "tree"),
        *written.get_inner_data()
    );

    // And the tree was not the document it started from, so the comparison
    // above has something to say: one subject renamed, one incompatibility
    // gone, everything else where it was.
    let after = written.get_inner_data();
    assert_ne!(*after, *before);
    assert_eq!(
        after.params.incompats.incompat_map.len(),
        before.params.incompats.incompat_map.len() - 1,
    );
    let renamed = after
        .params
        .subjects
        .ordered_subject_list
        .values()
        .next()
        .expect("the example has subjects");
    assert_eq!(
        renamed.parameters.name.as_str(),
        "Défense contre les forces du Mal"
    );

    // The refusal itemises the whole set. The tree dropped a teacher several
    // slots still name, and the message carries one dangling reference for each
    // of those slots — a message that stopped at the first would leave a script
    // fixing its tree one round trip at a time.
    let refusal = global::<String>(&globals, "refusal");
    let orphan_count = global::<usize>(&globals, "orphan_count");
    assert!(orphan_count >= 2);
    assert_eq!(refusal.matches("dangling reference").count(), orphan_count);

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The document goes out as a spreadsheet, the document's way or the caller's
///
/// The workbook itself is the xlsx crate's business and is tested there; what
/// this says is that python reaches that writer, with the right data and the
/// right configuration, and that a failure on the way arrives as an
/// `ExportError` naming the file.
#[test]
fn a_document_exports_to_a_spreadsheet() {
    let dir = workspace("export_xlsx");
    let source = example_copy(&dir, "source.collomatique");
    let own_target = dir.join("own.xlsx");
    let full_target = dir.join("full.xlsx");
    let bad_target = dir.join("refused.xlsx");

    let globals = run(include_str!("scripts/export_xlsx.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("own_target", &own_target)?;
        globals.set_item("full_target", &full_target)?;
        globals.set_item("bad_target", &bad_target)?;
        Ok(())
    });

    // An xlsx is a zip, so both files start with a local file header — they are
    // workbooks and not some bytes that happen to be there.
    for target in [&own_target, &full_target] {
        let bytes = std::fs::read(target).expect("the script wrote this file");
        assert_eq!(&bytes[..4], b"PK\x03\x04");
    }

    // The bare export used the document's own configuration, cut down to one
    // sheet, and the second one used the caller's, which asks for the group
    // sheets as well. An export that read the same configuration both times —
    // the document's for both calls, or the default for both — would write two
    // files of the same size.
    let size = |target: &Path| {
        std::fs::metadata(target)
            .expect("the script wrote this file")
            .len()
    };
    assert!(size(&own_target) < size(&full_target));

    // The refusals: nothing was written where the configuration was not one,
    // and the path that could not be written is what the message opens with.
    assert!(!bad_target.exists());
    let failure = global::<String>(&globals, "failure");
    assert!(failure.contains("no-such-directory"));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The solve configuration crosses the boundary whole, and refuses what means
/// nothing
///
/// The vocabulary of §10, extracted the way `build_colloscope_model` will
/// extract it. The document is the two-filling fixture because both shapes are
/// needed here: the automatic list is what a config speaks about, and naming
/// the prefilled one is one of the three refusals this class makes.
#[test]
fn the_solve_config_crosses_the_boundary() {
    use collomatique_constraints_colloscopes::{
        GroupListRecompute, GroupListSolveData, PeriodSolveData, SolveConfig,
    };
    use collomatique_python::data::{ColloscopeSolveConfig, Value};
    use collomatique_state_colloscopes::ids::Id as _;

    let dir = workspace("solve-config");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    let globals = run(include_str!("scripts/solve_config.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    // Read from the file rather than from the running document: ids are
    // stored, so the copy rust reads names the entities the script named.
    let data = reload(&source);
    let params = &data.get_inner_data().params;

    let periods: Vec<_> = params.periods.period_ids().collect();
    assert_eq!(periods.len(), 2);

    let mut automatic = None;
    let mut prefilled = None;
    for (id, group_list) in params.group_lists.group_list_map.iter() {
        if group_list.is_prefilled() {
            prefilled = Some(id);
        } else {
            automatic = Some(id);
        }
    }
    let automatic = automatic.expect("the fixture holds an automatic list");
    let prefilled = prefilled.expect("the fixture holds a prefilled list");

    // The default pin: `clm.ColloscopeSolveConfig()` is the model's own
    // default, weights included — recompute everything, freely.
    assert_eq!(
        extracted::<ColloscopeSolveConfig>(&globals, "bare"),
        SolveConfig::default()
    );

    // A config that says something about everything it can, out and back.
    let spelled_out = SolveConfig {
        periods: BTreeMap::from([
            (
                periods[0],
                PeriodSolveData {
                    recompute: false,
                    use_current_values: true,
                },
            ),
            (
                periods[1],
                PeriodSolveData {
                    recompute: true,
                    use_current_values: true,
                },
            ),
        ]),
        group_lists: BTreeMap::from([(
            automatic,
            GroupListSolveData {
                recompute: Some(GroupListRecompute {
                    previous_values_as_objective: true,
                }),
            },
        )]),
        objectify_cross_fixed_period: None,
        l1_anchor_weight: 0.0,
    };
    assert_eq!(
        extracted::<ColloscopeSolveConfig>(&globals, "spelled_out"),
        spelled_out
    );

    // And the other direction, which nothing else exercises: the value the
    // boundary writes is one it reads back unchanged, the flat two booleans of
    // a group list included — the shape the model nests.
    Python::attach(|py| {
        let doc = document_of(globals.bind(py));
        let value = ColloscopeSolveConfig::to_py(py, &spelled_out)
            .expect("a config should convert to python");
        assert_eq!(
            ColloscopeSolveConfig::from_py(&doc, &value).expect("and back out again"),
            spelled_out
        );
    });

    // A key names an entity, so a handle and an id are the same key.
    let pinned = SolveConfig {
        periods: BTreeMap::from([(
            periods[0],
            PeriodSolveData {
                recompute: false,
                use_current_values: false,
            },
        )]),
        group_lists: BTreeMap::from([(automatic, GroupListSolveData { recompute: None })]),
        ..SolveConfig::default()
    };
    for name in ["by_handle", "by_id"] {
        assert_eq!(extracted::<ColloscopeSolveConfig>(&globals, name), pinned);
    }

    // Zero is a weight: the term is written and prices nothing.
    assert_eq!(
        extracted::<ColloscopeSolveConfig>(&globals, "zero_weights"),
        SolveConfig {
            objectify_cross_fixed_period: Some(0.0),
            l1_anchor_weight: 0.0,
            ..SolveConfig::default()
        }
    );

    // The refusals, each with the sentence it raises. The first is the double
    // naming every mapping of entities refuses; the next two are this class's
    // own.
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "named_twice"),
        (
            "ValueError".to_owned(),
            format!(
                "a ColloscopeSolveConfig's periods names <PeriodId {}> twice",
                periods[0].inner()
            ),
        )
    );
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "prefilled_list"),
        (
            "ValueError".to_owned(),
            format!(
                "a ColloscopeSolveConfig's group_lists names <GroupListId {}>, and that group \
                 list is prefilled: a solve computes the automatic ones",
                prefilled.inner()
            ),
        )
    );
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "nothing_to_anchor"),
        (
            "ValueError".to_owned(),
            format!(
                "a ColloscopeSolveConfig's group_lists asks <GroupListId {}> for \
                 previous_values_as_objective without recompute, and a group list that is not \
                 recomputed keeps its groups",
                automatic.inner()
            ),
        )
    );

    // The weights, both ways of not being one.
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "negative_weight"),
        (
            "ValueError".to_owned(),
            "a ColloscopeSolveConfig's l1_anchor_weight is zero or more, and -1 is negative"
                .to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "infinite_weight"),
        (
            "ValueError".to_owned(),
            "a ColloscopeSolveConfig's objectify_cross_fixed_period is a finite weight, and inf \
             is not one"
                .to_owned(),
        )
    );

    // And the ordinary shapes of wrong: the flag is refused at the site of the
    // class the script wrote it in, and the mapping at its own.
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "not_a_flag"),
        (
            "TypeError".to_owned(),
            "a PeriodSolveConfig's recompute is True or False, and 3 is neither".to_owned(),
        )
    );
    assert_eq!(
        refused::<ColloscopeSolveConfig>(&globals, "not_a_mapping"),
        (
            "TypeError".to_owned(),
            "a ColloscopeSolveConfig's periods is a mapping of entities to values, and 3 is not \
             one"
            .to_owned(),
        )
    );

    // A key that names nothing here is refused the way every entity reference
    // is: a handle of another document, and a handle of something this one no
    // longer holds.
    for name in ["foreign_period", "dead_period"] {
        let (kind, _message) = refused::<ColloscopeSolveConfig>(&globals, name);
        assert_eq!(kind, "StaleHandleError");
    }

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// One strategy a script built, extracted the way a solve will extract it
///
/// Not [extracted]: the strategy family is not a [collomatique_python::data::Value]
/// — it names no entity, so it needs no document — and its door is an inherent
/// method rather than a trait one.
fn strategy(globals: &Py<PyDict>, name: &str) -> collomatique_strategies::ConductorStrategy {
    Python::attach(|py| {
        let value = globals
            .bind(py)
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"));

        collomatique_python::data::ConductorStrategy::from_py(&value).unwrap_or_else(|e| {
            e.print(py);
            panic!("`{name}` should extract")
        })
    })
}

/// How extracting one strategy the script built was refused
///
/// The mirror of [refused], for the same reason [strategy] is the mirror of
/// [extracted]: the class name and the message both, since a test that only
/// checked the class would not notice a message naming the wrong field.
fn refused_strategy(globals: &Py<PyDict>, name: &str) -> (String, String) {
    Python::attach(|py| {
        let value = globals
            .bind(py)
            .get_item(name)
            .expect("looking up a global should not fail")
            .unwrap_or_else(|| panic!("the script sets `{name}`"));

        let error = collomatique_python::data::ConductorStrategy::from_py(&value)
            .expect_err("this strategy is one the boundary must refuse");

        (
            error
                .get_type(py)
                .name()
                .expect("an exception class has a name")
                .to_string(),
            error.value(py).to_string(),
        )
    })
}

/// The conductor strategy crosses the boundary whole, and refuses what means
/// nothing
///
/// The vocabulary of §13, extracted the way `model.solve` will extract it. No
/// document is opened: a strategy says how a solve is run and names no entity,
/// which is exactly why this family is the one that is not a `Value`.
#[test]
fn the_conductor_strategy_crosses_the_boundary() {
    use collomatique_strategies::{
        ConductorStrategy as RawConductorStrategy, DefaultConfig, FuzzyConfig, IncrementalConfig,
        WarmStartConfig,
    };
    use collomatique_time::TimeLimit;

    let globals = run(include_str!("scripts/strategy.py"), |_| Ok(()));

    let count = |value: u32| NonZeroU32::new(value).expect("the test writes non-zero counts");
    let seconds = |value: u32| TimeLimit::seconds(count(value));

    // The default pin: `clm.ConductorStrategy()` is the application's own
    // « Recherche simple » — one worker, warm-start only.
    assert_eq!(strategy(&globals, "bare"), RawConductorStrategy::default());

    // And each bare sub-config is its own model default, which is what pins
    // the numbers written out in `data.py` — `epoch_incumbent_time_limit=60`
    // and the rest of them.
    assert_eq!(
        strategy(&globals, "all_bare"),
        RawConductorStrategy {
            worker_count: count(1),
            default_config: Some(DefaultConfig::default()),
            warm_start_config: Some(WarmStartConfig::default()),
            incremental_config: Some(IncrementalConfig::default()),
            fuzzy_config: Some(FuzzyConfig::default()),
            warm_start_incumbent: true,
        }
    );

    // The presets are the application's own structures, converted. `optimize`
    // asks this machine how many cores it has — and so does the rust side of
    // this comparison, in the same process, so the two agree by construction
    // rather than by luck.
    assert_eq!(
        strategy(&globals, "search"),
        RawConductorStrategy::default()
    );
    assert_eq!(
        strategy(&globals, "optimize"),
        RawConductorStrategy::with_parallelism_defaults()
    );

    // A strategy that says something about everything it can, out and back.
    let spelled_out = RawConductorStrategy {
        worker_count: count(3),
        default_config: Some(DefaultConfig {
            time_limit: seconds(600),
            incumbent_time_limit: seconds(120),
        }),
        warm_start_config: Some(WarmStartConfig {
            time_limit: seconds(30),
        }),
        incremental_config: Some(IncrementalConfig {
            l1_weight: 0.0,
            distance_tolerance: 0.0,
            epoch_time_limit: seconds(45),
            epoch_incumbent_time_limit: TimeLimit::none(),
        }),
        fuzzy_config: Some(FuzzyConfig {
            fuzzy_sigma: 0.0,
            find_closest_tolerance: 2.5,
            time_limit: TimeLimit::none(),
            incumbent_time_limit: seconds(7),
        }),
        // The one field the presets never exercise the other way round, so the
        // spelled-out strategy is where the non-default value is read.
        warm_start_incumbent: false,
    };
    assert_eq!(strategy(&globals, "spelled_out"), spelled_out);

    // And the other direction, which the presets exercise only for what they
    // happen to hold: the value the boundary writes is one it reads back
    // unchanged, the limits written as whole seconds included.
    Python::attach(|py| {
        let value = collomatique_python::data::ConductorStrategy::to_py(py, &spelled_out)
            .expect("a strategy should convert to python");
        assert_eq!(
            collomatique_python::data::ConductorStrategy::from_py(&value)
                .expect("and back out again"),
            spelled_out
        );
    });

    // The refusals, each with the sentence it raises. A solve runs on at least
    // one worker, and a worker count is a count.
    assert_eq!(
        refused_strategy(&globals, "no_worker"),
        (
            "ValueError".to_owned(),
            "a ConductorStrategy's worker_count is at least 1, and 0 was given".to_owned(),
        )
    );
    assert_eq!(
        refused_strategy(&globals, "not_a_count"),
        (
            "TypeError".to_owned(),
            "a ConductorStrategy's worker_count is a number of slots, and 'x' is not one"
                .to_owned(),
        )
    );

    // The time limits: zero is refused rather than read as no limit, and the
    // sentence says where no limit is said instead. The path names the field
    // the script wrote, not the sub-config's own class.
    assert_eq!(
        refused_strategy(&globals, "zero_limit"),
        (
            "ValueError".to_owned(),
            "a ConductorStrategy's warm_start_config.time_limit is at least one second, and None \
             is how no limit is said"
                .to_owned(),
        )
    );
    for name in ["negative_limit", "not_a_limit"] {
        let (kind, message) = refused_strategy(&globals, name);
        assert_eq!(kind, "TypeError");
        assert!(
            message.starts_with(
                "a ConductorStrategy's warm_start_config.time_limit is a number of seconds or \
                 None, and "
            ),
            "{message}"
        );
    }

    // A price the solver pays cannot be negative, and a measurement cannot be
    // infinite.
    assert_eq!(
        refused_strategy(&globals, "negative_weight"),
        (
            "ValueError".to_owned(),
            "a ConductorStrategy's incremental_config.l1_weight is zero or more, and -1 is \
             negative"
                .to_owned(),
        )
    );
    assert_eq!(
        refused_strategy(&globals, "infinite_sigma"),
        (
            "ValueError".to_owned(),
            "a ConductorStrategy's fuzzy_config.fuzzy_sigma is a finite number, and inf is not one"
                .to_owned(),
        )
    );

    // And the ordinary shapes of wrong: a sub-config is read by its fields, so
    // what is refused is an object without them — an object that is nothing of
    // the sort, and one carrying half of them.
    assert_eq!(
        refused_strategy(&globals, "not_a_config"),
        (
            "TypeError".to_owned(),
            "a ConductorStrategy is expected here, and 3 has no default_config.time_limit"
                .to_owned(),
        )
    );
    assert_eq!(
        refused_strategy(&globals, "half_a_config"),
        (
            "TypeError".to_owned(),
            "a ConductorStrategy is expected here, and a half-written config has no \
             default_config.time_limit"
                .to_owned(),
        )
    );
}

/// The conductor's preflight warnings, as a script reads them
///
/// The eight remarks of `ConductorStrategy.warnings()`: that they come out in
/// the model's own order, that they compare and hash the way a script expects
/// of a member, and that the sentence each one prints as is the very one the
/// application's dialog shows — asserted against `ui-text`'s own function, so
/// python cannot end up with a second set of words.
#[test]
fn the_conductor_warnings_are_preflight() {
    use collomatique_strategies::{ConductorStrategy as RawConductorStrategy, ConductorWarning};

    /// The eight warnings and the names python knows them by
    ///
    /// Written out rather than derived, since the point is to pin the two
    /// halves of the conversion against each other: a match in `solve.rs`
    /// would only agree with itself.
    const NAMES: [(ConductorWarning, &str); 8] = [
        (ConductorWarning::NoStrategyEnabled, "NO_STRATEGY_ENABLED"),
        (ConductorWarning::NoOptimizing, "NO_OPTIMIZING"),
        (ConductorWarning::NoSeed, "NO_SEED"),
        (ConductorWarning::StarvedFuzzy, "STARVED_FUZZY"),
        (ConductorWarning::WontFinish, "WONT_FINISH"),
        (ConductorWarning::ColdFuzzy, "COLD_FUZZY"),
        (ConductorWarning::RedundantWarmStart, "REDUNDANT_WARM_START"),
        (ConductorWarning::OverwhelmedCpu, "OVERWHELMED_CPU"),
    ];

    let globals = run(include_str!("scripts/strategy_warnings.py"), |_| Ok(()));

    // The script asserted the shapes it could decide for itself. What is left
    // is what only this side knows: the words, and the preset whose warnings
    // depend on the machine the test runs on.
    assert_eq!(
        global::<BTreeMap<String, String>>(&globals, "sentences"),
        NAMES
            .into_iter()
            .map(|(warning, name)| (
                name.to_owned(),
                collomatique_ui_text::solver::conductor_warning_text(warning).to_owned(),
            ))
            .collect::<BTreeMap<_, _>>(),
    );

    // « Optimisation complète » says nothing about its shape, but on a machine
    // with one or two cores its single slot is taken by the default worker and
    // the fuzzers never get one. Which of the two it is, is the application's
    // own answer, read here from the very structure the preset is built from.
    let optimize = RawConductorStrategy::with_parallelism_defaults();
    let named = |warning: ConductorWarning| {
        NAMES
            .into_iter()
            .find_map(|(candidate, name)| (candidate == warning).then_some(name.to_owned()))
            .expect("every warning has a python name")
    };
    assert_eq!(
        global::<Vec<String>>(&globals, "optimize_names"),
        optimize
            // As the script sees them: no solution is handed to a scripted solve.
            .warnings(false)
            .into_iter()
            .map(named)
            .collect::<Vec<_>>(),
    );

    // And a strategy that cannot be read at all is refused where it is read,
    // with the sentence `solve` will refuse it with.
    assert_eq!(
        global::<String>(&globals, "malformed"),
        "a ConductorStrategy's worker_count is at least 1, and 0 was given",
    );
}

/// A document builds its ILP model, and hands back a token for it
///
/// The problem itself is `constraints-colloscopes`' business and is tested
/// there; what this says is that python reaches that builder with the config
/// the script wrote, that what comes back carries the counts of the model that
/// was really built, that the build log arrives line by line, and that a
/// callback which raises is heard without the build being torn in half.
///
/// The document is the two-filling fixture: its automatic group list is what
/// gives the problem something to work out, so a student added to it is an
/// edit the model cannot fail to notice — which is how the script says the
/// model it holds is a snapshot and not a view.
#[test]
fn a_document_builds_its_colloscope_model() {
    use collomatique_constraints_colloscopes::{
        GroupListRecompute, GroupListSolveData, SolveConfig,
    };

    let dir = workspace("build-model");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    let globals = run(include_str!("scripts/build_model.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    // The same build, run here: the script's document was edited at the end and
    // never saved, so the file on disk is still the one it opened.
    let data = reload(&source);
    let built = SolveConfig::default()
        .build_model(data.get_inner_data(), &mut |_line| {})
        .expect("the fixture's model builds");
    let stats = built.stats();

    let automatic = data
        .get_inner_data()
        .params
        .group_lists
        .group_list_map
        .iter()
        .find(|(_id, group_list)| !group_list.is_prefilled())
        .map(|(id, _group_list)| id)
        .expect("the fixture holds an automatic list");

    // The repr's two numbers, each summed over the three kinds the modeler
    // distinguishes.
    let counts = |stats: &collomatique_constraints_colloscopes::ModelStats| {
        (
            stats.base_variable_count + stats.constraint_extra_count + stats.objective_extra_count,
            stats.user_constraint_count
                + stats.constraint_defining_constraint_count
                + stats.objective_defining_constraint_count,
        )
    };

    let (variables, constraints) = counts(&stats);

    // A problem with nothing in it would make the comparisons above and below
    // say nothing, so the fixture has to be worth building.
    assert!(variables > 0);
    assert!(constraints > 0);

    assert_eq!(
        global::<String>(&globals, "shown"),
        format!("<ColloscopeModel: {variables} variables, {constraints} constraints>")
    );

    // The same, for the model the anchored config builds. The plain build has
    // no objective at all, so it is this one that says the objective half of
    // both counts is in the repr — and that the group-list half of the config
    // reached the builder, since the anchor is what it asked for.
    let anchored_config = SolveConfig {
        group_lists: BTreeMap::from([(
            automatic,
            GroupListSolveData {
                recompute: Some(GroupListRecompute {
                    previous_values_as_objective: true,
                }),
            },
        )]),
        ..SolveConfig::default()
    };
    let anchored_stats = anchored_config
        .build_model(data.get_inner_data(), &mut |_line| {})
        .expect("the anchored model builds")
        .stats();
    assert!(anchored_stats.objective_extra_count > 0);
    assert!(anchored_stats.objective_defining_constraint_count > 0);

    let (anchored_variables, anchored_constraints) = counts(&anchored_stats);
    assert_eq!(
        global::<String>(&globals, "anchored_shown"),
        format!(
            "<ColloscopeModel: {anchored_variables} variables, \
             {anchored_constraints} constraints>"
        )
    );

    // The log the script collected is the builder's own, verbatim: the first
    // line rust's own build emits is the first line python was handed.
    let mut first = None;
    let _ = SolveConfig::default().build_model(data.get_inner_data(), &mut |line| {
        first.get_or_insert_with(|| line.to_owned());
    });
    let lines = global::<Vec<String>>(&globals, "lines");
    assert_eq!(lines.first(), first.as_ref());

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A built problem goes out as an MPS file, the whole one or the checker one
///
/// The file format is the mps crate's business and is tested there; what this
/// says is that python reaches that writer with the problem the config built,
/// that `checker=True` picks the other of the two problems one build carries,
/// and that a failure on the way arrives as an `ExportError` naming the file.
///
/// Each file is read back and asked how many rows it holds — an MPS file has
/// one row for the objective and one per constraint — and the answer is
/// compared with the problem rust builds here from the same document. A file
/// written from the wrong problem, or from no config at all, would not match.
#[test]
fn a_model_exports_to_mps() {
    use collomatique_constraints_colloscopes::{
        GroupListRecompute, GroupListSolveData, SolveConfig,
    };

    let dir = workspace("export-mps");
    let source = dir.join("filling.collomatique");
    group_lists_document(&source);

    let full_target = dir.join("problem.mps");
    let again_target = dir.join("again.mps");
    let anchored_target = dir.join("anchored.mps");
    let checker_target = dir.join("checker.mps");
    let pinned_target = dir.join("pinned.mps");
    let bad_target = dir.join("refused.mps");

    let globals = run(include_str!("scripts/export_mps.py"), |globals| {
        globals.set_item("source", &source)?;
        globals.set_item("full_target", &full_target)?;
        globals.set_item("again_target", &again_target)?;
        globals.set_item("anchored_target", &anchored_target)?;
        globals.set_item("checker_target", &checker_target)?;
        globals.set_item("pinned_target", &pinned_target)?;
        globals.set_item("bad_target", &bad_target)?;
        Ok(())
    });

    // The `ROWS` section runs from its header to the next one, and every line
    // in it is indented — so counting them is asking the file how many rows the
    // problem it holds has.
    let rows = |target: &Path| {
        let text = std::fs::read_to_string(target).expect("the script wrote this file");
        text.lines()
            .skip_while(|line| *line != "ROWS")
            .skip(1)
            .take_while(|line| line.starts_with(' '))
            .count()
    };

    // The script never saved, so the file on disk is still the one it opened.
    let data = reload(&source);

    let automatic = data
        .get_inner_data()
        .params
        .group_lists
        .group_list_map
        .iter()
        .find(|(_id, group_list)| !group_list.is_prefilled())
        .map(|(id, _group_list)| id)
        .expect("the fixture holds an automatic list");

    let build = |config: SolveConfig| {
        config
            .build_model(data.get_inner_data(), &mut |_line| {})
            .expect("the fixture's model builds")
    };

    let built = build(SolveConfig::default());
    let full_rows = built.problem().get_constraints().len() + 1;
    assert!(full_rows > 1);
    assert_eq!(rows(&full_target), full_rows);

    // The same model asked twice wrote the same file, which the script already
    // checked; what rust adds is that the second spelling of the path — a
    // `pathlib.Path` rather than a `str` — reached the same writer.
    assert_eq!(rows(&again_target), full_rows);

    // The anchored problem carries an objective, and its checker problem is the
    // constraints without what only that objective needed. Two problems out of
    // one build, and the flag chooses between them.
    let anchored = build(SolveConfig {
        group_lists: BTreeMap::from([(
            automatic,
            GroupListSolveData {
                recompute: Some(GroupListRecompute {
                    previous_values_as_objective: true,
                }),
            },
        )]),
        ..SolveConfig::default()
    });
    let checker_rows = anchored.checker_problem().get_constraints().len() + 1;
    assert!(checker_rows < anchored.problem().get_constraints().len() + 1);
    assert_eq!(
        rows(&anchored_target),
        anchored.problem().get_constraints().len() + 1
    );
    assert_eq!(rows(&checker_target), checker_rows);

    // And the pinned config's problem, which is a third one again: the group
    // list keeps the groups it has, so there is less to write down.
    let pinned = build(SolveConfig {
        group_lists: BTreeMap::from([(automatic, GroupListSolveData { recompute: None })]),
        ..SolveConfig::default()
    });
    let pinned_rows = pinned.problem().get_constraints().len() + 1;
    assert_ne!(pinned_rows, full_rows);
    assert_eq!(rows(&pinned_target), pinned_rows);

    // The refusals: nothing was written where the flag was handed over
    // positionally, and the path that could not be written is what the message
    // opens with.
    assert!(!bad_target.exists());
    let failure = global::<String>(&globals, "failure");
    assert!(failure.contains("no-such-directory"));

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A colloscope the model cannot read is refused before an engine is looked for
///
/// The happy path of `model.blame` needs an engine subprocess, which no test in
/// this crate spawns; what this says is everything that happens before one is
/// asked for — that the value crosses the boundary by id, that a colloscope of
/// another shape is refused there rather than panicked over, and that the
/// refusal is a `ValueError` naming the mismatch.
///
/// The refusal order is the point: the colloscope is read, then judged against
/// the model's own parameters, and only then is a machine looked for. So this
/// runs on a machine with no engine at all.
///
/// The document is the filled synthetic colloscope of the read surface's
/// commit ([colloscope_document]) — the example was never resolved, so it has
/// no placements to break.
#[test]
fn a_blame_refuses_a_colloscope_the_model_cannot_read() {
    let dir = workspace("blame-incompatible");
    let source = dir.join("colloscope.collomatique");
    colloscope_document(&source);

    let globals = run(include_str!("scripts/blame_incompatible.py"), |globals| {
        globals.set_item("source", &source)?;
        Ok(())
    });

    // One sentence for the one mistake, whichever of the two distances it was
    // found at: this colloscope and this model are not about the same document.
    let out_of_range = global::<String>(&globals, "out_of_range");
    assert!(
        out_of_range.contains("not compatible with the model"),
        "the refusal should say what does not fit: {out_of_range}"
    );

    // The detached refusal says what a key may be, and where to get one.
    let handle_refused = global::<String>(&globals, "handle_refused");
    assert!(
        handle_refused.contains("SlotId") && handle_refused.contains("to_data()"),
        "the refusal should name the id class and where ids come from: {handle_refused}"
    );

    // And a value that is not one at all is refused by the field it lacks, in
    // the voice every other value class is read with.
    let not_a_colloscope = global::<String>(&globals, "not_a_colloscope");
    assert!(
        not_a_colloscope.contains("ColloscopeData") && not_a_colloscope.contains("interrogations"),
        "the refusal should name the class expected: {not_a_colloscope}"
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// The six severity levels compare, sort worst first, and hash
///
/// A blame is a list a script sorts and filters, so the order of the levels is
/// part of the API and not an accident of how they are written down. `FIXED` is
/// the one that is not a tier of the model: a broken pin of the solve
/// configuration outranks anything the model itself says.
#[test]
fn the_severity_levels_are_ordered_worst_first() {
    let globals = run(include_str!("scripts/blame_severity.py"), |_globals| Ok(()));

    // The names python knows them by, in the order a sort puts them in.
    assert_eq!(
        global::<Vec<String>>(&globals, "names"),
        [
            "SeverityLevel.FIXED",
            "SeverityLevel.INFEASIBILITY",
            "SeverityLevel.STRUCTURAL",
            "SeverityLevel.QUALITY",
            "SeverityLevel.PROGRESSIVE",
            "SeverityLevel.PREFERENCE",
        ]
    );
}
