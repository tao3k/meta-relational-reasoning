//! Safe typed Rust bindings for the declaration-owned Gerbil AOT grammar ABI.

use std::sync::OnceLock;

use super::{
    ffi,
    runtime::{NativeRuntimeAccess, native_runtime_access},
};

const ABI_VERSION: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SyntaxShape {
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) fields: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeywordSpec {
    pub(crate) name: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OperatorSpec {
    pub(crate) kind: String,
    pub(crate) lexeme: String,
    pub(crate) precedence: u8,
    pub(crate) associativity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParserEntrypointSpec {
    pub(crate) keyword: String,
    pub(crate) action: String,
    pub(crate) effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecoverySpec {
    pub(crate) site: String,
    pub(crate) code: String,
    pub(crate) strategy: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeGrammar {
    pub(crate) syntax_shapes: Vec<SyntaxShape>,
    pub(crate) keywords: Vec<KeywordSpec>,
    pub(crate) prefix_operators: Vec<OperatorSpec>,
    pub(crate) binary_operators: Vec<OperatorSpec>,
    pub(crate) parser_entrypoints: Vec<ParserEntrypointSpec>,
    pub(crate) recoveries: Vec<RecoverySpec>,
}

impl NativeGrammar {
    /// Loads the complete grammar through the native AOT bindings.
    pub(crate) fn load() -> Result<Self, NativeGrammarError> {
        let runtime =
            native_runtime_access().map_err(|()| NativeGrammarError::RuntimeLockPoisoned)?;
        Self::load_with_runtime(&runtime)
    }

    /// Loads while the caller holds the process-global Gambit runtime capability.
    pub(super) fn load_with_runtime(
        _runtime: &NativeRuntimeAccess,
    ) -> Result<Self, NativeGrammarError> {
        initialize()?;
        let actual = ffi::abi_version();
        if actual != ABI_VERSION {
            return Err(NativeGrammarError::AbiMismatch {
                expected: ABI_VERSION,
                actual,
            });
        }
        let syntax_shapes = load_syntax_shapes()?;
        Ok(Self {
            syntax_shapes,
            keywords: load_keywords()?,
            prefix_operators: load_operators(Table::PrefixOperators)?,
            binary_operators: load_operators(Table::BinaryOperators)?,
            parser_entrypoints: load_entrypoints()?,
            recoveries: load_recoveries()?,
        })
    }
}

#[derive(Clone, Copy)]
#[repr(i32)]
enum Table {
    SyntaxKinds = 0,
    Keywords = 1,
    PrefixOperators = 2,
    BinaryOperators = 3,
    ParserEntrypoints = 4,
    Recoveries = 5,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum NativeGrammarError {
    RuntimeLockPoisoned,
    RuntimeStatus(i32),
    AbiMismatch {
        expected: u32,
        actual: u32,
    },
    InvalidCount {
        table: &'static str,
        value: i64,
    },
    InvalidText {
        table: &'static str,
        row: i64,
        column: i64,
    },
    InvalidCodepoint(i32),
    InvalidPrecedence(i32),
}

impl std::fmt::Display for NativeGrammarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "native Gerbil grammar binding failed: {self:?}")
    }
}

impl std::error::Error for NativeGrammarError {}

fn initialize() -> Result<(), NativeGrammarError> {
    static STATUS: OnceLock<i32> = OnceLock::new();
    let status = *STATUS.get_or_init(ffi::runtime_init);
    if status == 0 || status == 6 {
        Ok(())
    } else {
        Err(NativeGrammarError::RuntimeStatus(status))
    }
}

fn count(table: Table, name: &'static str) -> Result<i64, NativeGrammarError> {
    let value = ffi::table_count(table as i32);
    if value < 0 {
        Err(NativeGrammarError::InvalidCount { table: name, value })
    } else {
        Ok(value)
    }
}

fn text(
    table: Table,
    table_name: &'static str,
    row: i64,
    column: i64,
) -> Result<String, NativeGrammarError> {
    let length = ffi::row_text_length(table as i32, row, column);
    if length < 0 {
        return Err(NativeGrammarError::InvalidText {
            table: table_name,
            row,
            column,
        });
    }
    (0..length)
        .map(|index| {
            let codepoint = ffi::row_text_char(table as i32, row, column, index);
            char::from_u32(codepoint as u32).ok_or(NativeGrammarError::InvalidCodepoint(codepoint))
        })
        .collect()
}

fn load_syntax_shapes() -> Result<Vec<SyntaxShape>, NativeGrammarError> {
    let mut rows = Vec::new();
    for row in 0..count(Table::SyntaxKinds, "syntax-kinds")? {
        let field_count = ffi::syntax_field_count(row);
        if field_count < 0 {
            return Err(NativeGrammarError::InvalidCount {
                table: "syntax-fields",
                value: field_count,
            });
        }
        let mut fields = Vec::new();
        for field in 0..field_count {
            let length = ffi::syntax_field_length(row, field);
            if length < 0 {
                return Err(NativeGrammarError::InvalidText {
                    table: "syntax-fields",
                    row,
                    column: field,
                });
            }
            let value = (0..length)
                .map(|index| {
                    let codepoint = ffi::syntax_field_char(row, field, index);
                    char::from_u32(codepoint as u32)
                        .ok_or(NativeGrammarError::InvalidCodepoint(codepoint))
                })
                .collect::<Result<String, _>>()?;
            fields.push(value);
        }
        rows.push(SyntaxShape {
            name: text(Table::SyntaxKinds, "syntax-kinds", row, 0)?,
            category: text(Table::SyntaxKinds, "syntax-kinds", row, 1)?,
            fields,
        });
    }
    Ok(rows)
}

fn load_keywords() -> Result<Vec<KeywordSpec>, NativeGrammarError> {
    (0..count(Table::Keywords, "keywords")?)
        .map(|row| {
            Ok(KeywordSpec {
                name: text(Table::Keywords, "keywords", row, 0)?,
                text: text(Table::Keywords, "keywords", row, 1)?,
            })
        })
        .collect()
}

fn load_operators(table: Table) -> Result<Vec<OperatorSpec>, NativeGrammarError> {
    let name = match table {
        Table::PrefixOperators => "prefix-operators",
        _ => "binary-operators",
    };
    (0..count(table, name)?)
        .map(|row| {
            let precedence = ffi::operator_precedence(table as i32, row);
            let precedence = u8::try_from(precedence)
                .map_err(|_| NativeGrammarError::InvalidPrecedence(precedence))?;
            Ok(OperatorSpec {
                kind: text(table, name, row, 0)?,
                lexeme: text(table, name, row, 1)?,
                precedence,
                associativity: text(table, name, row, 3)?,
            })
        })
        .collect()
}

fn load_entrypoints() -> Result<Vec<ParserEntrypointSpec>, NativeGrammarError> {
    (0..count(Table::ParserEntrypoints, "parser-entrypoints")?)
        .map(|row| {
            Ok(ParserEntrypointSpec {
                keyword: text(Table::ParserEntrypoints, "parser-entrypoints", row, 0)?,
                action: text(Table::ParserEntrypoints, "parser-entrypoints", row, 1)?,
                effect: text(Table::ParserEntrypoints, "parser-entrypoints", row, 2)?,
            })
        })
        .collect()
}

fn load_recoveries() -> Result<Vec<RecoverySpec>, NativeGrammarError> {
    (0..count(Table::Recoveries, "recoveries")?)
        .map(|row| {
            Ok(RecoverySpec {
                site: text(Table::Recoveries, "recoveries", row, 0)?,
                code: text(Table::Recoveries, "recoveries", row, 1)?,
                strategy: text(Table::Recoveries, "recoveries", row, 2)?,
            })
        })
        .collect()
}
