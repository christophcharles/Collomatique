#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct ColloCbcModel {
    _opaque: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColloCbcStatus {
    Optimal = 0,
    Infeasible = 1,
    Stopped = 2,
    Error = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColloCbcEventType {
    Solution = 0,
    TreeStatus = 1,
}

#[repr(C)]
pub struct ColloCbcProgress {
    pub event_type: ColloCbcEventType,
    pub best_obj: f64,
    pub best_bound: f64,
    pub node_count: i32,
    pub solutions_found: i32,
    pub solution: *const f64,
    pub num_cols: i32,
}

pub type ColloCbcCallback = Option<
    unsafe extern "C" fn(progress: *const ColloCbcProgress, user_data: *mut c_void) -> c_int,
>;

unsafe extern "C" {
    pub fn collo_cbc_new() -> *mut ColloCbcModel;
    pub fn collo_cbc_free(model: *mut ColloCbcModel);

    pub fn collo_cbc_load_problem(
        model: *mut ColloCbcModel,
        num_cols: i32,
        num_rows: i32,
        obj_sense: i32,
        col_lb: *const f64,
        col_ub: *const f64,
        obj_coeffs: *const f64,
        is_integer: *const i32,
        mat_start: *const i32,
        mat_index: *const i32,
        mat_value: *const f64,
        nnz: i32,
        row_lb: *const f64,
        row_ub: *const f64,
    );

    pub fn collo_cbc_set_parameter(
        model: *mut ColloCbcModel,
        key: *const c_char,
        value: *const c_char,
    );

    pub fn collo_cbc_set_mip_start(model: *mut ColloCbcModel, values: *const f64, num_cols: i32);

    pub fn collo_cbc_solve(
        model: *mut ColloCbcModel,
        cb: ColloCbcCallback,
        user_data: *mut c_void,
    ) -> ColloCbcStatus;

    pub fn collo_cbc_get_obj_value(model: *const ColloCbcModel) -> f64;
    pub fn collo_cbc_get_best_bound(model: *const ColloCbcModel) -> f64;
    pub fn collo_cbc_get_node_count(model: *const ColloCbcModel) -> i32;
    pub fn collo_cbc_get_solution(model: *const ColloCbcModel, out: *mut f64, num_cols: i32)
    -> i32;
    pub fn collo_cbc_get_num_cols(model: *const ColloCbcModel) -> i32;
}
