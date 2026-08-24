mod dump;
pub mod sys;

pub use dump::{read_mip_start, write_mip_start};

use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

static CBC_LOCK: Mutex<()> = Mutex::new(());

fn lock<T, F: FnOnce() -> T>(f: F) -> T {
    let _guard = CBC_LOCK.lock().unwrap();
    f()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}

impl From<sys::ColloCbcStatus> for Status {
    fn from(s: sys::ColloCbcStatus) -> Self {
        match s {
            sys::ColloCbcStatus::Optimal => Status::Optimal,
            sys::ColloCbcStatus::Infeasible => Status::Infeasible,
            sys::ColloCbcStatus::Stopped => Status::Stopped,
            sys::ColloCbcStatus::Error => Status::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// CBC reported a solution; [`Progress::incumbent`] says whether it could be
    /// handed over.
    Solution,
    /// A progress update from the model CBC is searching. `best_bound`,
    /// `node_count` and `solutions_found` are valid.
    TreeStatus,
    /// CBC is alive, and nothing more. It comes from a nested model — a
    /// heuristic sub-MIP — whose bound and incumbent live in its own reduced
    /// column space, so nothing about them is transmissible: `best_bound`,
    /// `node_count` and `solutions_found` are all zero and must not be read.
    /// It exists so a caller can still check its deadlines and still ask to stop
    /// while such a model holds the solve.
    Tick,
}

/// The incumbent-reconstruction outcome for a single progress event.
///
/// An objective only ever exists as a property of a successfully reconstructed,
/// original-space incumbent — there is no free-floating running objective (CBC's
/// `getObjValue()`, in preprocessed space, is never transmitted).
#[derive(Debug, Clone)]
pub enum IncumbentEvent {
    /// No fresh incumbent on this event: a tree-status update, or a duplicate
    /// solution id already reported.
    None,
    /// A fresh incumbent, reconstructed into the problem's original column space.
    Reconstructed { objective: f64, solution: Vec<f64> },
    /// CBC reported a fresh incumbent but it could not be reconstructed into
    /// original column space. No objective and no solution are available.
    ReconstructionFailed,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub event_type: EventType,
    pub best_bound: f64,
    pub node_count: i32,
    pub solutions_found: i32,
    pub incumbent: IncumbentEvent,
}

impl Progress {
    unsafe fn from_raw(raw: &sys::ColloCbcProgress) -> Self {
        let event_type = match raw.event_type {
            sys::ColloCbcEventType::Solution => EventType::Solution,
            sys::ColloCbcEventType::TreeStatus => EventType::TreeStatus,
            sys::ColloCbcEventType::Tick => EventType::Tick,
        };
        let incumbent = match raw.incumbent_status {
            sys::ColloCbcIncumbentStatus::None => IncumbentEvent::None,
            sys::ColloCbcIncumbentStatus::Ok => {
                let solution = if !raw.solution.is_null() && raw.num_cols > 0 {
                    unsafe { std::slice::from_raw_parts(raw.solution, raw.num_cols as usize) }
                        .to_vec()
                } else {
                    // OK status guarantees a valid solution pointer; treat a
                    // missing one defensively as a reconstruction failure.
                    return Progress {
                        event_type,
                        best_bound: raw.best_bound,
                        node_count: raw.node_count,
                        solutions_found: raw.solutions_found,
                        incumbent: IncumbentEvent::ReconstructionFailed,
                    };
                };
                IncumbentEvent::Reconstructed {
                    objective: raw.best_obj,
                    solution,
                }
            }
            sys::ColloCbcIncumbentStatus::Failed => IncumbentEvent::ReconstructionFailed,
        };
        Progress {
            event_type,
            best_bound: raw.best_bound,
            node_count: raw.node_count,
            solutions_found: raw.solutions_found,
            incumbent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SolveResult {
    pub status: Status,
    pub obj_value: f64,
    pub best_bound: f64,
    pub node_count: i32,
    pub solution: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProblemDesc {
    pub num_cols: i32,
    pub num_rows: i32,
    pub obj_sense: i32,
    pub col_lb: Vec<f64>,
    pub col_ub: Vec<f64>,
    pub obj_coeffs: Vec<f64>,
    pub is_integer: Vec<i32>,
    pub mat_start: Vec<i32>,
    pub mat_index: Vec<i32>,
    pub mat_value: Vec<f64>,
    pub row_lb: Vec<f64>,
    pub row_ub: Vec<f64>,
}

/// Opt-in model export, switched on with `COLLO_CBC_DUMP_MODEL=<prefix>` in the
/// environment. Every problem loaded into a `Model` is then written to
/// `<prefix>-<pid>-<NNN>.collomodel`, and a MIP start set on it afterwards to
/// `<prefix>-<pid>-<NNN>.collomipstart`.
///
/// `NNN` counts the problems loaded in this process, so an 11-epoch incremental
/// run yields 11 pairs. The pid is in the name because the conductor runs
/// several solver subprocesses at once, and they would otherwise overwrite each
/// other's dumps.
///
/// A dump is a complete reproducer — see `examples/replay.rs`, which reads one
/// back and solves it through the same event handler production uses.
fn dump_prefix() -> Option<&'static str> {
    static PREFIX: OnceLock<Option<String>> = OnceLock::new();
    PREFIX
        .get_or_init(|| match std::env::var("COLLO_CBC_DUMP_MODEL") {
            Ok(v) if !v.is_empty() => Some(v),
            _ => None,
        })
        .as_deref()
}

static DUMP_COUNTER: AtomicU32 = AtomicU32::new(0);

fn dump_path(prefix: &str, index: u32, extension: &str) -> PathBuf {
    PathBuf::from(format!(
        "{prefix}-{}-{index:03}.{extension}",
        std::process::id()
    ))
}

pub struct Model {
    ptr: *mut sys::ColloCbcModel,
    num_cols: i32,
    /// Index this model's dump files carry, assigned by `load_problem`. `None`
    /// when dumping is off or no problem has been loaded yet.
    dump_index: Option<u32>,
}

// Safety: Model owns a heap-allocated C++ object behind a raw pointer.
// Moving it to another thread is safe because CBC/CLP solvers don't use
// thread-local state or thread-affine resources.
unsafe impl Send for Model {}

/// Stop the C runtime from holding CBC's log in a buffer.
///
/// CBC prints through C stdout. Behind a pipe that is block-buffered, so a log
/// meant to be watched live arrives in lumps of several kilobytes — or only when
/// the solve ends. This makes it unbuffered, at the cost of one write per line.
///
/// Process-wide, so call it once at startup, before any solve.
pub fn unbuffer_output() {
    unsafe { sys::collo_cbc_unbuffer_output() }
}

impl Drop for Model {
    fn drop(&mut self) {
        lock(|| unsafe {
            sys::collo_cbc_free(self.ptr);
        });
    }
}

impl Model {
    pub fn new() -> Self {
        let ptr = lock(|| unsafe { sys::collo_cbc_new() });
        assert!(!ptr.is_null(), "collo_cbc_new returned null");
        Model {
            ptr,
            num_cols: 0,
            dump_index: None,
        }
    }

    pub fn load_problem(&mut self, desc: &ProblemDesc) {
        if let Some(prefix) = dump_prefix() {
            let index = DUMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            self.dump_index = Some(index);
            let path = dump_path(prefix, index, "collomodel");
            // A diagnostic must never break a solve: report and carry on.
            if let Err(e) = desc.write_to(&path) {
                eprintln!("collo-cbc: could not dump model to {}: {e}", path.display());
            }
        }

        self.num_cols = desc.num_cols;
        let nnz = desc.mat_value.len() as i32;
        lock(|| unsafe {
            sys::collo_cbc_load_problem(
                self.ptr,
                desc.num_cols,
                desc.num_rows,
                desc.obj_sense,
                desc.col_lb.as_ptr(),
                desc.col_ub.as_ptr(),
                desc.obj_coeffs.as_ptr(),
                desc.is_integer.as_ptr(),
                desc.mat_start.as_ptr(),
                desc.mat_index.as_ptr(),
                desc.mat_value.as_ptr(),
                nnz,
                desc.row_lb.as_ptr(),
                desc.row_ub.as_ptr(),
            );
        });
    }

    pub fn set_parameter(&mut self, key: &str, value: &str) {
        let key = CString::new(key).expect("key contains null byte");
        let value = CString::new(value).expect("value contains null byte");
        lock(|| unsafe {
            sys::collo_cbc_set_parameter(self.ptr, key.as_ptr(), value.as_ptr());
        });
    }

    /// Disable (or re-enable) CBC's heuristics. Useful for testing, so that
    /// incumbents come from branch-and-bound rather than a heuristic solving
    /// the whole problem up front.
    pub fn set_disable_heuristics(&mut self, disable: bool) {
        self.set_parameter("heuristicsOnOff", if disable { "off" } else { "on" });
    }

    /// Disable (or re-enable) CBC's cut generators. Useful for testing, to
    /// force more branching.
    pub fn set_disable_cuts(&mut self, disable: bool) {
        self.set_parameter("cuts", if disable { "off" } else { "on" });
    }

    pub fn set_log_level(&mut self, level: i32) {
        lock(|| unsafe {
            sys::collo_cbc_set_log_level(self.ptr, level);
        });
    }

    pub fn set_mip_start(&mut self, values: &[f64]) {
        if let (Some(prefix), Some(index)) = (dump_prefix(), self.dump_index) {
            let path = dump_path(prefix, index, "collomipstart");
            if let Err(e) = dump::write_mip_start(&path, values) {
                eprintln!(
                    "collo-cbc: could not dump MIP start to {}: {e}",
                    path.display()
                );
            }
        }

        lock(|| unsafe {
            sys::collo_cbc_set_mip_start(self.ptr, values.as_ptr(), values.len() as i32);
        });
    }

    pub fn solve(&mut self) -> SolveResult {
        self.solve_with_callback(|_| true)
    }

    pub fn solve_with_callback<F>(&mut self, mut callback: F) -> SolveResult
    where
        F: FnMut(&Progress) -> bool,
    {
        unsafe extern "C" fn trampoline<F: FnMut(&Progress) -> bool>(
            progress: *const sys::ColloCbcProgress,
            user_data: *mut c_void,
        ) -> c_int {
            let cb = unsafe { &mut *(user_data as *mut F) };
            let rust_progress = unsafe { Progress::from_raw(&*progress) };
            if cb(&rust_progress) { 0 } else { 1 }
        }

        let status = lock(|| unsafe {
            sys::collo_cbc_solve(
                self.ptr,
                Some(trampoline::<F>),
                &mut callback as *mut F as *mut c_void,
            )
        });

        self.build_result(status)
    }

    fn build_result(&self, status: sys::ColloCbcStatus) -> SolveResult {
        lock(|| {
            let solution = if self.num_cols > 0 {
                let mut buf = vec![0.0f64; self.num_cols as usize];
                let ret = unsafe {
                    sys::collo_cbc_get_solution(self.ptr, buf.as_mut_ptr(), self.num_cols)
                };
                if ret == 0 { Some(buf) } else { None }
            } else {
                Some(vec![])
            };

            SolveResult {
                status: Status::from(status),
                obj_value: unsafe { sys::collo_cbc_get_obj_value(self.ptr) },
                best_bound: unsafe { sys::collo_cbc_get_best_bound(self.ptr) },
                node_count: unsafe { sys::collo_cbc_get_node_count(self.ptr) },
                solution,
            }
        })
    }
}
