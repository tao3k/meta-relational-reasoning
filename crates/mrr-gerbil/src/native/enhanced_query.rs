//! Safe typed projection of the Scheme-owned enhanced Tree-sitter Query V1 table.

use std::fmt;

use super::{
    ffi,
    model::NativeGrammar,
    runtime::{NativeRuntimeError, with_native_runtime},
};

const ABI_VERSION: u32 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnhancedQueryOperandSpec {
    pub position: usize,
    pub kind: String,
    pub domain: String,
    pub cardinality: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnhancedQueryOperatorSpec {
    pub spelling: String,
    pub kind: String,
    pub minimum_arity: usize,
    pub maximum_arity: Option<usize>,
    pub operands: Vec<EnhancedQueryOperandSpec>,
    pub lowering: String,
    pub failure_code: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnhancedQueryRecoverySpec {
    pub site: String,
    pub code: String,
    pub strategy: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnhancedTreeSitterQueryOperatorTable {
    pub profile_id: String,
    pub owner: String,
    pub declaration_digest: String,
    pub operators: Vec<EnhancedQueryOperatorSpec>,
    pub recoveries: Vec<EnhancedQueryRecoverySpec>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum EnhancedQueryOperatorTableLoadError {
    RuntimeLockPoisoned,
    NativeGrammar(String),
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
    InvalidInteger {
        table: &'static str,
        row: i64,
        value: String,
    },
    InvalidOperandPosition {
        row: i64,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for EnhancedQueryOperatorTableLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "enhanced Tree-sitter Query AOT load failed: {self:?}"
        )
    }
}

impl std::error::Error for EnhancedQueryOperatorTableLoadError {}

/// Loads the MRR-owned namespaced extension table from Gerbil AOT.
///
/// Ordinary Tree-sitter Query syntax and provider node/field vocabularies are
/// deliberately absent. The digest is over a length-prefixed canonical
/// encoding of the typed AOT rows, in their declared order.
pub fn load_enhanced_tree_sitter_query_operator_table()
-> Result<EnhancedTreeSitterQueryOperatorTable, EnhancedQueryOperatorTableLoadError> {
    with_native_runtime(load_enhanced_tree_sitter_query_operator_table_on_owner).map_err(
        |error| match error {
            NativeRuntimeError::Unavailable => {
                EnhancedQueryOperatorTableLoadError::RuntimeLockPoisoned
            }
            NativeRuntimeError::Status(status) => {
                EnhancedQueryOperatorTableLoadError::NativeGrammar(format!(
                    "native Gerbil runtime initialization failed with {status}"
                ))
            }
        },
    )?
}

fn load_enhanced_tree_sitter_query_operator_table_on_owner()
-> Result<EnhancedTreeSitterQueryOperatorTable, EnhancedQueryOperatorTableLoadError> {
    NativeGrammar::load_on_owner()
        .map_err(|error| EnhancedQueryOperatorTableLoadError::NativeGrammar(error.to_string()))?;
    let actual = ffi::abi_version();
    if actual != ABI_VERSION {
        return Err(EnhancedQueryOperatorTableLoadError::AbiMismatch {
            expected: ABI_VERSION,
            actual,
        });
    }

    let profile_id = singleton(QueryTable::Profile, "profile")?;
    let owner = singleton(QueryTable::Owner, "owner")?;
    let operators = load_operators()?;
    let recoveries = load_recoveries()?;
    let declaration_digest = declaration_digest(&profile_id, &owner, &operators, &recoveries);

    Ok(EnhancedTreeSitterQueryOperatorTable {
        profile_id,
        owner,
        declaration_digest,
        operators,
        recoveries,
    })
}

#[derive(Clone, Copy)]
#[repr(i32)]
enum QueryTable {
    Profile = 0,
    Owner = 1,
    Operators = 2,
    Recoveries = 3,
}

fn count(
    table: QueryTable,
    name: &'static str,
) -> Result<i64, EnhancedQueryOperatorTableLoadError> {
    let value = ffi::enhanced_query_table_count(table as i32);
    if value < 0 {
        Err(EnhancedQueryOperatorTableLoadError::InvalidCount { table: name, value })
    } else {
        Ok(value)
    }
}

fn text(
    table: QueryTable,
    table_name: &'static str,
    row: i64,
    column: i64,
) -> Result<String, EnhancedQueryOperatorTableLoadError> {
    let length = ffi::enhanced_query_row_text_length(table as i32, row, column);
    if length < 0 {
        return Err(EnhancedQueryOperatorTableLoadError::InvalidText {
            table: table_name,
            row,
            column,
        });
    }
    (0..length)
        .map(|index| {
            let codepoint = ffi::enhanced_query_row_text_char(table as i32, row, column, index);
            char::from_u32(codepoint as u32).ok_or(
                EnhancedQueryOperatorTableLoadError::InvalidCodepoint(codepoint),
            )
        })
        .collect()
}

fn operand_text(
    row: i64,
    operand: i64,
    column: i64,
) -> Result<String, EnhancedQueryOperatorTableLoadError> {
    let length =
        ffi::enhanced_query_operand_text_length(QueryTable::Operators as i32, row, operand, column);
    if length < 0 {
        return Err(EnhancedQueryOperatorTableLoadError::InvalidText {
            table: "operator-operands",
            row,
            column,
        });
    }
    (0..length)
        .map(|index| {
            let codepoint = ffi::enhanced_query_operand_text_char(
                QueryTable::Operators as i32,
                row,
                operand,
                column,
                index,
            );
            char::from_u32(codepoint as u32).ok_or(
                EnhancedQueryOperatorTableLoadError::InvalidCodepoint(codepoint),
            )
        })
        .collect()
}

fn singleton(
    table: QueryTable,
    name: &'static str,
) -> Result<String, EnhancedQueryOperatorTableLoadError> {
    let actual = count(table, name)?;
    if actual != 1 {
        return Err(EnhancedQueryOperatorTableLoadError::InvalidCount {
            table: name,
            value: actual,
        });
    }
    text(table, name, 0, 0)
}

fn parse_usize(
    table: &'static str,
    row: i64,
    value: String,
) -> Result<usize, EnhancedQueryOperatorTableLoadError> {
    value
        .parse()
        .map_err(|_| EnhancedQueryOperatorTableLoadError::InvalidInteger { table, row, value })
}

fn load_operators() -> Result<Vec<EnhancedQueryOperatorSpec>, EnhancedQueryOperatorTableLoadError> {
    (0..count(QueryTable::Operators, "operators")?)
        .map(|row| {
            let minimum = parse_usize(
                "operators",
                row,
                text(QueryTable::Operators, "operators", row, 2)?,
            )?;
            let maximum_text = text(QueryTable::Operators, "operators", row, 3)?;
            let maximum = if maximum_text == "unbounded" {
                None
            } else {
                Some(parse_usize("operators", row, maximum_text)?)
            };
            let operand_count =
                ffi::enhanced_query_operand_count(QueryTable::Operators as i32, row);
            if operand_count < 0 {
                return Err(EnhancedQueryOperatorTableLoadError::InvalidCount {
                    table: "operator-operands",
                    value: operand_count,
                });
            }
            let operands = (0..operand_count)
                .map(|operand| {
                    let position =
                        parse_usize("operator-operands", row, operand_text(row, operand, 0)?)?;
                    let expected = operand as usize;
                    if position != expected {
                        return Err(
                            EnhancedQueryOperatorTableLoadError::InvalidOperandPosition {
                                row,
                                expected,
                                actual: position,
                            },
                        );
                    }
                    Ok(EnhancedQueryOperandSpec {
                        position,
                        kind: operand_text(row, operand, 1)?,
                        domain: operand_text(row, operand, 2)?,
                        cardinality: operand_text(row, operand, 3)?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnhancedQueryOperatorSpec {
                spelling: text(QueryTable::Operators, "operators", row, 0)?,
                kind: text(QueryTable::Operators, "operators", row, 1)?,
                minimum_arity: minimum,
                maximum_arity: maximum,
                lowering: text(QueryTable::Operators, "operators", row, 4)?,
                failure_code: text(QueryTable::Operators, "operators", row, 5)?,
                operands,
            })
        })
        .collect()
}

fn load_recoveries() -> Result<Vec<EnhancedQueryRecoverySpec>, EnhancedQueryOperatorTableLoadError>
{
    (0..count(QueryTable::Recoveries, "recoveries")?)
        .map(|row| {
            Ok(EnhancedQueryRecoverySpec {
                site: text(QueryTable::Recoveries, "recoveries", row, 0)?,
                code: text(QueryTable::Recoveries, "recoveries", row, 1)?,
                strategy: text(QueryTable::Recoveries, "recoveries", row, 2)?,
            })
        })
        .collect()
}

fn declaration_digest(
    profile_id: &str,
    owner: &str,
    operators: &[EnhancedQueryOperatorSpec],
    recoveries: &[EnhancedQueryRecoverySpec],
) -> String {
    let mut hasher = blake3::Hasher::new();
    digest_field(
        &mut hasher,
        "mrr-enhanced-tree-sitter-query-operator-table-v1",
    );
    digest_field(&mut hasher, profile_id);
    digest_field(&mut hasher, owner);
    digest_usize(&mut hasher, operators.len());
    for operator in operators {
        digest_field(&mut hasher, &operator.spelling);
        digest_field(&mut hasher, &operator.kind);
        digest_usize(&mut hasher, operator.minimum_arity);
        digest_optional_usize(&mut hasher, operator.maximum_arity);
        digest_field(&mut hasher, &operator.lowering);
        digest_field(&mut hasher, &operator.failure_code);
        digest_usize(&mut hasher, operator.operands.len());
        for operand in &operator.operands {
            digest_usize(&mut hasher, operand.position);
            digest_field(&mut hasher, &operand.kind);
            digest_field(&mut hasher, &operand.domain);
            digest_field(&mut hasher, &operand.cardinality);
        }
    }
    digest_usize(&mut hasher, recoveries.len());
    for recovery in recoveries {
        digest_field(&mut hasher, &recovery.site);
        digest_field(&mut hasher, &recovery.code);
        digest_field(&mut hasher, &recovery.strategy);
    }
    format!("blake3-256:{}", hasher.finalize().to_hex())
}

fn digest_field(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn digest_usize(hasher: &mut blake3::Hasher, value: usize) {
    hasher.update(&(value as u64).to_le_bytes());
}

fn digest_optional_usize(hasher: &mut blake3::Hasher, value: Option<usize>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            digest_usize(hasher, value);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}
