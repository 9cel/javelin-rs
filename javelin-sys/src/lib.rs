//! Raw bindings to the vendored JavelinPattern C API.
//!
//! Prefer the safe `javelin-pattern` crate for ordinary matching. All functions
//! here are unsafe: handles must be live, buffers valid for their stated lengths,
//! and capture buffers large enough for twice `jp_get_number_of_captures` pointers.
//! Pattern strings must be NUL-terminated. Bytecode passed to `jp_pattern_create`
//! must come from the same engine version and target; arbitrary bytes are invalid.
//! A borrowed bytecode buffer must outlive its pattern.
//!
//! Callbacks run synchronously and must not unwind or invalidate the input or
//! pattern. Scan callbacks return zero to continue or a nonzero value to stop.
//! Global stack-handler configuration must not change while matching is active.
//! See `upstream/Javelin/JavelinPattern.h` for the full native contracts.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

/// An owned native pattern handle, released with `jp_pattern_free`.
pub type jp_pattern_t = *mut c_void;
/// An owned native bytecode handle, released with `jp_bytecode_free`.
pub type jp_bytecode_t = *mut c_void;

pub const JP_OPTION_IGNORE_CASE: c_int = 1;
pub const JP_OPTION_MULTILINE: c_int = 2;
pub const JP_OPTION_DOTALL: c_int = 4;
pub const JP_OPTION_UNICODE_CASE: c_int = 8;
pub const JP_OPTION_UNGREEDY: c_int = 0x10;
pub const JP_OPTION_UCP: c_int = 0x40;
pub const JP_OPTION_UTF8: c_int = 0x100;
pub const JP_OPTION_GLOB_SYNTAX: c_int = 0x400;
pub const JP_OPTION_ANCHORED: c_int = 0x800;
pub const JP_OPTION_NO_OPTIMIZE: c_int = 0x1000;
pub const JP_OPTION_PREFER_NFA: c_int = 0x2000;
pub const JP_OPTION_PREFER_BACK_TRACKING: c_int = 0x4000;
pub const JP_OPTION_PREFER_SCAN_AND_CAPTURE: c_int = 0x6000;
pub const JP_OPTION_PREFER_NO_SCAN: c_int = 0x8000;
pub const JP_RESULT_OK: c_int = 0;
pub const JP_RESULT_INTERNAL_ERROR: c_int = 1;
pub const JP_RESULT_EXPECTED_CLOSE_GROUP: c_int = 2;
pub const JP_RESULT_INVALID_BACK_REFERENCE: c_int = 3;
pub const JP_RESULT_INVALID_OPTIONS: c_int = 4;
pub const JP_RESULT_LOOK_BEHIND_NOT_CONSTANT_BYTE_LENGTH: c_int = 5;
pub const JP_RESULT_MALFORMED_CONDITIONAL: c_int = 6;
pub const JP_RESULT_MAXIMUM_REPETITION_COUNT_EXCEEDED: c_int = 7;
pub const JP_RESULT_MINIMUM_COUNT_EXCEEDS_MAXIMUM_COUNT: c_int = 8;
pub const JP_RESULT_TOO_MANY_BYTECODE_INSTRUCTIONS: c_int = 9;
pub const JP_RESULT_TOO_MANY_CAPTURES: c_int = 10;
pub const JP_RESULT_TOO_MANY_PROGRESS_CHECK_INSTRUCTIONS: c_int = 11;
pub const JP_RESULT_UNABLE_TO_PARSE_GROUP_TYPE: c_int = 12;
pub const JP_RESULT_UNABLE_TO_PARSE_REPETITION: c_int = 13;
pub const JP_RESULT_UNABLE_TO_RESOLVE_RECURSE_TARGET: c_int = 14;
pub const JP_RESULT_UNEXPECTED_CONTROL_CHARACTER: c_int = 15;
pub const JP_RESULT_UNEXPECTED_END_OF_PATTERN: c_int = 16;
pub const JP_RESULT_UNEXPECTED_GROUP_OPTIONS: c_int = 17;
pub const JP_RESULT_UNEXPECTED_HEX_CHARACTER: c_int = 18;
pub const JP_RESULT_UNEXPECTED_LOOK_BEHIND_TYPE: c_int = 19;
pub const JP_RESULT_UNEXPECTED_TOKEN: c_int = 20;
pub const JP_RESULT_UNKNOWN_ESCAPE: c_int = 21;
pub const JP_RESULT_UNKNOWN_POSIX_CHARACTER_CLASS: c_int = 22;
pub const JP_RESULT_MALFORMED_UNICODE_PROPERTY: c_int = 23;
pub const JP_RESULT_UNKNOWN_UNICODE_PROPERTY: c_int = 24;

unsafe extern "C" {
    pub fn jp_bytecode_compile(
        out_result: *mut jp_bytecode_t,
        pattern: *const c_char,
        options: c_int,
    ) -> c_int;
    pub fn jp_bytecode_get_data(bytecode: jp_bytecode_t) -> *const c_void;
    pub fn jp_bytecode_get_length(bytecode: jp_bytecode_t) -> usize;
    pub fn jp_bytecode_free(bytecode: jp_bytecode_t);

    pub fn jp_pattern_compile(
        out_result: *mut jp_pattern_t,
        pattern: *const c_char,
        options: c_int,
    ) -> c_int;
    pub fn jp_pattern_create(
        out_result: *mut jp_pattern_t,
        byte_code: *const c_void,
        byte_code_length: usize,
        make_copy_of_byte_code: bool,
    ) -> c_int;
    pub fn jp_pattern_free(pattern: jp_pattern_t);
    pub fn jp_get_number_of_captures(pattern: jp_pattern_t) -> c_int;
    pub fn jp_has_full_match(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
    ) -> bool;
    pub fn jp_has_partial_match(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
        data_offset: usize,
    ) -> bool;
    pub fn jp_full_match(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
        captures: *mut *const c_void,
    ) -> bool;
    pub fn jp_partial_match(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
        captures: *mut *const c_void,
        data_offset: usize,
    ) -> bool;
    pub fn jp_scan(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
        user: *mut c_void,
        on_match: Option<unsafe extern "C" fn(from: usize, to: usize, user: *mut c_void) -> c_int>,
        data_offset: usize,
    ) -> c_int;
    pub fn jp_scan_captures(
        pattern: jp_pattern_t,
        data: *const c_void,
        data_length: usize,
        user: *mut c_void,
        on_match: Option<
            unsafe extern "C" fn(
                captures: *const *const c_void,
                capture_count: usize,
                user: *mut c_void,
            ) -> c_int,
        >,
        data_offset: usize,
    ) -> c_int;

    pub fn jp_dfa_memory_manager_mode_set_unlimited();
    pub fn jp_dfa_memory_manager_mode_set_global_limit(number_of_bytes_limit: usize);
    pub fn jp_dfa_memory_manager_mode_set_per_pattern_limit(number_of_bytes_limit: usize);
    pub fn jp_stack_growth_handler_disable();
    pub fn jp_stack_growth_handler_set(
        allocate_function: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        free_function: Option<unsafe extern "C" fn(*mut c_void)>,
    );
}
