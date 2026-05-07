pub mod sys;

use std::ffi::CString;
use std::os::raw::{c_int, c_void};

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
    Solution,
    TreeStatus,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub event_type: EventType,
    pub best_obj: f64,
    pub best_bound: f64,
    pub node_count: i32,
    pub solutions_found: i32,
    pub solution: Option<Vec<f64>>,
}

impl Progress {
    unsafe fn from_raw(raw: &sys::ColloCbcProgress) -> Self {
        let event_type = match raw.event_type {
            sys::ColloCbcEventType::Solution => EventType::Solution,
            sys::ColloCbcEventType::TreeStatus => EventType::TreeStatus,
        };
        let solution = if !raw.solution.is_null() && raw.num_cols > 0 {
            Some(
                unsafe { std::slice::from_raw_parts(raw.solution, raw.num_cols as usize) }.to_vec(),
            )
        } else {
            None
        };
        Progress {
            event_type,
            best_obj: raw.best_obj,
            best_bound: raw.best_bound,
            node_count: raw.node_count,
            solutions_found: raw.solutions_found,
            solution,
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

pub struct Model {
    ptr: *mut sys::ColloCbcModel,
    num_cols: i32,
}

unsafe impl Send for Model {}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            sys::collo_cbc_free(self.ptr);
        }
    }
}

impl Model {
    pub fn new() -> Self {
        let ptr = unsafe { sys::collo_cbc_new() };
        assert!(!ptr.is_null(), "collo_cbc_new returned null");
        Model { ptr, num_cols: 0 }
    }

    pub fn load_problem(&mut self, desc: &ProblemDesc) {
        self.num_cols = desc.num_cols;
        let nnz = desc.mat_value.len() as i32;
        unsafe {
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
        }
    }

    pub fn set_parameter(&mut self, key: &str, value: &str) {
        let key = CString::new(key).expect("key contains null byte");
        let value = CString::new(value).expect("value contains null byte");
        unsafe {
            sys::collo_cbc_set_parameter(self.ptr, key.as_ptr(), value.as_ptr());
        }
    }

    pub fn set_mip_start(&mut self, values: &[f64]) {
        unsafe {
            sys::collo_cbc_set_mip_start(self.ptr, values.as_ptr(), values.len() as i32);
        }
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

        let status = unsafe {
            sys::collo_cbc_solve(
                self.ptr,
                Some(trampoline::<F>),
                &mut callback as *mut F as *mut c_void,
            )
        };

        self.build_result(status)
    }

    fn build_result(&self, status: sys::ColloCbcStatus) -> SolveResult {
        let solution = if self.num_cols > 0 {
            let mut buf = vec![0.0f64; self.num_cols as usize];
            let ret =
                unsafe { sys::collo_cbc_get_solution(self.ptr, buf.as_mut_ptr(), self.num_cols) };
            if ret == 0 { Some(buf) } else { None }
        } else {
            None
        };

        SolveResult {
            status: Status::from(status),
            obj_value: unsafe { sys::collo_cbc_get_obj_value(self.ptr) },
            best_bound: unsafe { sys::collo_cbc_get_best_bound(self.ptr) },
            node_count: unsafe { sys::collo_cbc_get_node_count(self.ptr) },
            solution,
        }
    }
}
