//! Raw declarations isolated behind the safe native grammar loader.

unsafe extern "C" {
    fn mrr_grammar_native_runtime_init() -> i32;
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
}

pub(super) fn runtime_init() -> i32 {
    unsafe { mrr_grammar_native_runtime_init() }
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
