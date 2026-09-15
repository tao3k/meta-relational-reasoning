//! Raw declarations isolated behind the safe native grammar loader.

use std::ffi::{CStr, c_char};

#[repr(C)]
struct GerbilParserResultV1 {
    status: i32,
    payload: *mut c_char,
}

pub(super) struct ParserNativeResult {
    pub call_status: i32,
    pub result_status: i32,
    pub payload: Option<Vec<u8>>,
}

unsafe extern "C" {
    #[link_name = "___LNK_mrr__grammar__linker"]
    fn mrr_grammar_linker(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn mrr_grammar_native_abi_version() -> u32;
    fn mrr_grammar_native_table_count(table: i32) -> i64;
    fn mrr_grammar_native_row_text_length(table: i32, row: i64, column: i64) -> i64;
    fn mrr_grammar_native_row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32;
    fn mrr_grammar_native_syntax_field_count(row: i64) -> i64;
    fn mrr_grammar_native_syntax_field_length(row: i64, field: i64) -> i64;
    fn mrr_grammar_native_syntax_field_char(row: i64, field: i64, index: i64) -> i32;
    fn mrr_grammar_native_operator_precedence(table: i32, row: i64) -> i32;
    fn mrr_reasoning_native_table_count(table: i32) -> i64;
    fn mrr_reasoning_native_row_text_length(table: i32, row: i64, column: i64) -> i64;
    fn mrr_reasoning_native_row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32;
    fn mrr_reasoning_native_nested_count(table: i32, row: i64) -> i64;
    fn mrr_reasoning_native_nested_text_length(
        table: i32,
        row: i64,
        nested_row: i64,
        column: i64,
    ) -> i64;
    fn mrr_reasoning_native_nested_text_char(
        table: i32,
        row: i64,
        nested_row: i64,
        column: i64,
        index: i64,
    ) -> i32;
    fn mrr_reasoning_native_driver_request_resource(phase: i32) -> i32;
    fn mrr_reasoning_native_driver_transition(
        phase: i32,
        resource: i32,
        status: i32,
        cycle: i64,
        max_cycles: i64,
    ) -> i32;
    fn mrr_enhanced_query_table_count(table: i32) -> i64;
    fn mrr_enhanced_query_row_text_length(table: i32, row: i64, column: i64) -> i64;
    fn mrr_enhanced_query_row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32;
    fn mrr_enhanced_query_operand_count(table: i32, row: i64) -> i64;
    fn mrr_enhanced_query_operand_text_length(
        table: i32,
        row: i64,
        operand: i64,
        column: i64,
    ) -> i64;
    fn mrr_enhanced_query_operand_text_char(
        table: i32,
        row: i64,
        operand: i64,
        column: i64,
        index: i64,
    ) -> i32;
    fn gerbil_parser_result_v1_init(result: *mut GerbilParserResultV1);
    fn gerbil_parser_result_v1_release(result: *mut GerbilParserResultV1);
    fn gerbil_parser_native_abi_version() -> u32;
    fn gerbil_parser_native_descriptor(result: *mut GerbilParserResultV1) -> i32;
    fn gerbil_parser_native_parse(source: *const c_char, result: *mut GerbilParserResultV1) -> i32;
}

pub(super) fn runtime_init() -> i32 {
    unsafe { gerbil_scheme_sys::gerbil_scheme_rust_runtime_init_program(Some(mrr_grammar_linker)) }
}
pub(super) fn abi_version() -> u32 {
    unsafe { mrr_grammar_native_abi_version() }
}
pub(super) fn table_count(table: i32) -> i64 {
    unsafe { mrr_grammar_native_table_count(table) }
}
pub(super) fn row_text_length(table: i32, row: i64, column: i64) -> i64 {
    unsafe { mrr_grammar_native_row_text_length(table, row, column) }
}
pub(super) fn row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32 {
    unsafe { mrr_grammar_native_row_text_char(table, row, column, index) }
}
pub(super) fn syntax_field_count(row: i64) -> i64 {
    unsafe { mrr_grammar_native_syntax_field_count(row) }
}
pub(super) fn syntax_field_length(row: i64, field: i64) -> i64 {
    unsafe { mrr_grammar_native_syntax_field_length(row, field) }
}
pub(super) fn syntax_field_char(row: i64, field: i64, index: i64) -> i32 {
    unsafe { mrr_grammar_native_syntax_field_char(row, field, index) }
}
pub(super) fn operator_precedence(table: i32, row: i64) -> i32 {
    unsafe { mrr_grammar_native_operator_precedence(table, row) }
}
pub(super) fn reasoning_table_count(table: i32) -> i64 {
    unsafe { mrr_reasoning_native_table_count(table) }
}
pub(super) fn reasoning_row_text_length(table: i32, row: i64, column: i64) -> i64 {
    unsafe { mrr_reasoning_native_row_text_length(table, row, column) }
}
pub(super) fn reasoning_row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32 {
    unsafe { mrr_reasoning_native_row_text_char(table, row, column, index) }
}
pub(super) fn reasoning_nested_count(table: i32, row: i64) -> i64 {
    unsafe { mrr_reasoning_native_nested_count(table, row) }
}
pub(super) fn reasoning_nested_text_length(
    table: i32,
    row: i64,
    nested_row: i64,
    column: i64,
) -> i64 {
    unsafe { mrr_reasoning_native_nested_text_length(table, row, nested_row, column) }
}
pub(super) fn reasoning_nested_text_char(
    table: i32,
    row: i64,
    nested_row: i64,
    column: i64,
    index: i64,
) -> i32 {
    unsafe { mrr_reasoning_native_nested_text_char(table, row, nested_row, column, index) }
}
pub(super) fn reasoning_driver_request_resource(phase: i32) -> i32 {
    unsafe { mrr_reasoning_native_driver_request_resource(phase) }
}
pub(super) fn reasoning_driver_transition(
    phase: i32,
    resource: i32,
    status: i32,
    cycle: i64,
    max_cycles: i64,
) -> i32 {
    unsafe { mrr_reasoning_native_driver_transition(phase, resource, status, cycle, max_cycles) }
}
pub(super) fn enhanced_query_table_count(table: i32) -> i64 {
    unsafe { mrr_enhanced_query_table_count(table) }
}
pub(super) fn enhanced_query_row_text_length(table: i32, row: i64, column: i64) -> i64 {
    unsafe { mrr_enhanced_query_row_text_length(table, row, column) }
}
pub(super) fn enhanced_query_row_text_char(table: i32, row: i64, column: i64, index: i64) -> i32 {
    unsafe { mrr_enhanced_query_row_text_char(table, row, column, index) }
}
pub(super) fn enhanced_query_operand_count(table: i32, row: i64) -> i64 {
    unsafe { mrr_enhanced_query_operand_count(table, row) }
}
pub(super) fn enhanced_query_operand_text_length(
    table: i32,
    row: i64,
    operand: i64,
    column: i64,
) -> i64 {
    unsafe { mrr_enhanced_query_operand_text_length(table, row, operand, column) }
}
pub(super) fn enhanced_query_operand_text_char(
    table: i32,
    row: i64,
    operand: i64,
    column: i64,
    index: i64,
) -> i32 {
    unsafe { mrr_enhanced_query_operand_text_char(table, row, operand, column, index) }
}

pub(super) fn parser_native_abi_version() -> u32 {
    unsafe { gerbil_parser_native_abi_version() }
}

pub(super) fn parser_native_descriptor() -> ParserNativeResult {
    unsafe { parser_native_result(|result| gerbil_parser_native_descriptor(result)) }
}

pub(super) fn parser_native_parse(source: &CStr) -> ParserNativeResult {
    unsafe { parser_native_result(|result| gerbil_parser_native_parse(source.as_ptr(), result)) }
}

unsafe fn parser_native_result(
    call: impl FnOnce(*mut GerbilParserResultV1) -> i32,
) -> ParserNativeResult {
    let mut result = GerbilParserResultV1 {
        status: 0,
        payload: std::ptr::null_mut(),
    };
    unsafe { gerbil_parser_result_v1_init(&mut result) };
    let call_status = call(&mut result);
    let result_status = result.status;
    let payload = if result.payload.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(result.payload) }
                .to_bytes()
                .to_vec(),
        )
    };
    unsafe { gerbil_parser_result_v1_release(&mut result) };
    ParserNativeResult {
        call_status,
        result_status,
        payload,
    }
}
