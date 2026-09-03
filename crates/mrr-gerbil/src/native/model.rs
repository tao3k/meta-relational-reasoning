//! Safe typed Rust bindings for the declaration-owned Gerbil AOT grammar ABI.

use std::sync::OnceLock;

use super::{
    ffi,
    runtime::{NativeRuntimeAccess, native_runtime_access, native_runtime_status_is_ready},
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
pub(crate) struct NumericLiteralSpec {
    pub(crate) form: String,
    pub(crate) notation: String,
    pub(crate) suffix: String,
    pub(crate) class: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CharacterStringLiteralSpec {
    pub(crate) form: String,
    pub(crate) lexeme: String,
    pub(crate) action: String,
    pub(crate) class: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParameterReferenceSpec {
    pub(crate) form: String,
    pub(crate) prefix: String,
    pub(crate) name: String,
    pub(crate) context: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PredicateTestSpec {
    pub(crate) kind: String,
    pub(crate) negation: String,
    pub(crate) value: String,
    pub(crate) operand: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AggregateFunctionSpec {
    pub(crate) name: String,
    pub(crate) keyword: String,
    pub(crate) kind: String,
    pub(crate) quantifier: String,
    pub(crate) arity: u8,
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
pub struct ReleaseSpec {
    pub id: String,
    pub normative_reference: String,
    pub kind: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSpec {
    pub id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSpec {
    pub id: String,
    pub release_id: String,
    pub claim: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileModuleSpec {
    pub profile_id: String,
    pub disposition: String,
    pub module_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSupplementSpec {
    pub profile_id: String,
    pub release_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureSpec {
    pub id: String,
    pub priority: u16,
    pub module_id: String,
    pub clause_status: String,
    pub layer_statuses: [String; 5],
    pub evidence_owner: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureDependencySpec {
    pub feature_id: String,
    pub dependency_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsoProfile {
    pub schema: String,
    pub releases: Vec<ReleaseSpec>,
    pub modules: Vec<ModuleSpec>,
    pub profiles: Vec<ProfileSpec>,
    pub profile_supplements: Vec<ProfileSupplementSpec>,
    pub profile_modules: Vec<ProfileModuleSpec>,
    pub features: Vec<FeatureSpec>,
    pub feature_dependencies: Vec<FeatureDependencySpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeGrammar {
    pub(crate) profile_schema: String,
    pub(crate) syntax_shapes: Vec<SyntaxShape>,
    pub(crate) keywords: Vec<KeywordSpec>,
    pub(crate) non_reserved_words: Vec<String>,
    pub(crate) numeric_literals: Vec<NumericLiteralSpec>,
    pub(crate) character_string_literals: Vec<CharacterStringLiteralSpec>,
    pub(crate) parameter_references: Vec<ParameterReferenceSpec>,
    pub(crate) predicate_tests: Vec<PredicateTestSpec>,
    pub(crate) aggregate_functions: Vec<AggregateFunctionSpec>,
    pub(crate) prefix_operators: Vec<OperatorSpec>,
    pub(crate) binary_operators: Vec<OperatorSpec>,
    pub(crate) parser_entrypoints: Vec<ParserEntrypointSpec>,
    pub(crate) recoveries: Vec<RecoverySpec>,
    pub(crate) releases: Vec<ReleaseSpec>,
    pub(crate) modules: Vec<ModuleSpec>,
    pub(crate) profiles: Vec<ProfileSpec>,
    pub(crate) profile_supplements: Vec<ProfileSupplementSpec>,
    pub(crate) profile_modules: Vec<ProfileModuleSpec>,
    pub(crate) features: Vec<FeatureSpec>,
    pub(crate) feature_dependencies: Vec<FeatureDependencySpec>,
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
            profile_schema: text(Table::ProfileSchema, "profile-schema", 0, 0)?,
            syntax_shapes,
            keywords: load_keywords()?,
            non_reserved_words: load_non_reserved_words()?,
            numeric_literals: load_numeric_literals()?,
            character_string_literals: load_character_string_literals()?,
            parameter_references: load_parameter_references()?,
            predicate_tests: load_predicate_tests()?,
            aggregate_functions: load_aggregate_functions()?,
            prefix_operators: load_operators(Table::PrefixOperators)?,
            binary_operators: load_operators(Table::BinaryOperators)?,
            parser_entrypoints: load_entrypoints()?,
            recoveries: load_recoveries()?,
            releases: load_releases()?,
            modules: load_modules()?,
            profiles: load_profiles()?,
            profile_supplements: load_profile_supplements()?,
            profile_modules: load_profile_modules()?,
            features: load_features()?,
            feature_dependencies: load_feature_dependencies()?,
        })
    }
}

pub fn load_iso_profile() -> Result<IsoProfile, IsoProfileLoadError> {
    let grammar = NativeGrammar::load().map_err(IsoProfileLoadError)?;
    Ok(IsoProfile {
        schema: grammar.profile_schema,
        releases: grammar.releases,
        modules: grammar.modules,
        profiles: grammar.profiles,
        profile_supplements: grammar.profile_supplements,
        profile_modules: grammar.profile_modules,
        features: grammar.features,
        feature_dependencies: grammar.feature_dependencies,
    })
}

#[derive(Debug)]
pub struct IsoProfileLoadError(NativeGrammarError);

impl std::fmt::Display for IsoProfileLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "ISO profile AOT load failed: {}", self.0)
    }
}

impl std::error::Error for IsoProfileLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
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
    Releases = 6,
    Modules = 7,
    Profiles = 8,
    ProfileModules = 9,
    Features = 10,
    FeatureDependencies = 11,
    ProfileSchema = 12,
    ProfileSupplements = 13,
    NonReservedWords = 14,
    NumericLiterals = 15,
    CharacterStringLiterals = 16,
    ParameterReferences = 17,
    PredicateTests = 18,
    AggregateFunctions = 19,
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
    InvalidPriority(String),
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
    if native_runtime_status_is_ready(status) {
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

fn load_non_reserved_words() -> Result<Vec<String>, NativeGrammarError> {
    (0..count(Table::NonReservedWords, "non-reserved-words")?)
        .map(|row| text(Table::NonReservedWords, "non-reserved-words", row, 0))
        .collect()
}

fn load_numeric_literals() -> Result<Vec<NumericLiteralSpec>, NativeGrammarError> {
    (0..count(Table::NumericLiterals, "numeric-literals")?)
        .map(|row| {
            Ok(NumericLiteralSpec {
                form: text(Table::NumericLiterals, "numeric-literals", row, 0)?,
                notation: text(Table::NumericLiterals, "numeric-literals", row, 1)?,
                suffix: text(Table::NumericLiterals, "numeric-literals", row, 2)?,
                class: text(Table::NumericLiterals, "numeric-literals", row, 3)?,
            })
        })
        .collect()
}

fn load_character_string_literals() -> Result<Vec<CharacterStringLiteralSpec>, NativeGrammarError> {
    (0..count(Table::CharacterStringLiterals, "character-string-literals")?)
        .map(|row| {
            Ok(CharacterStringLiteralSpec {
                form: text(
                    Table::CharacterStringLiterals,
                    "character-string-literals",
                    row,
                    0,
                )?,
                lexeme: text(
                    Table::CharacterStringLiterals,
                    "character-string-literals",
                    row,
                    1,
                )?,
                action: text(
                    Table::CharacterStringLiterals,
                    "character-string-literals",
                    row,
                    2,
                )?,
                class: text(
                    Table::CharacterStringLiterals,
                    "character-string-literals",
                    row,
                    3,
                )?,
            })
        })
        .collect()
}

fn load_parameter_references() -> Result<Vec<ParameterReferenceSpec>, NativeGrammarError> {
    (0..count(Table::ParameterReferences, "parameter-references")?)
        .map(|row| {
            Ok(ParameterReferenceSpec {
                form: text(Table::ParameterReferences, "parameter-references", row, 0)?,
                prefix: text(Table::ParameterReferences, "parameter-references", row, 1)?,
                name: text(Table::ParameterReferences, "parameter-references", row, 2)?,
                context: text(Table::ParameterReferences, "parameter-references", row, 3)?,
            })
        })
        .collect()
}

fn load_predicate_tests() -> Result<Vec<PredicateTestSpec>, NativeGrammarError> {
    (0..count(Table::PredicateTests, "predicate-tests")?)
        .map(|row| {
            Ok(PredicateTestSpec {
                kind: text(Table::PredicateTests, "predicate-tests", row, 0)?,
                negation: text(Table::PredicateTests, "predicate-tests", row, 1)?,
                value: text(Table::PredicateTests, "predicate-tests", row, 2)?,
                operand: text(Table::PredicateTests, "predicate-tests", row, 3)?,
            })
        })
        .collect()
}

fn load_aggregate_functions() -> Result<Vec<AggregateFunctionSpec>, NativeGrammarError> {
    (0..count(Table::AggregateFunctions, "aggregate-functions")?)
        .map(|row| {
            let arity = text(Table::AggregateFunctions, "aggregate-functions", row, 4)?
                .parse::<u8>()
                .map_err(|_| NativeGrammarError::InvalidText {
                    table: "aggregate-functions",
                    row,
                    column: 4,
                })?;
            Ok(AggregateFunctionSpec {
                name: text(Table::AggregateFunctions, "aggregate-functions", row, 0)?,
                keyword: text(Table::AggregateFunctions, "aggregate-functions", row, 1)?,
                kind: text(Table::AggregateFunctions, "aggregate-functions", row, 2)?,
                quantifier: text(Table::AggregateFunctions, "aggregate-functions", row, 3)?,
                arity,
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

fn load_releases() -> Result<Vec<ReleaseSpec>, NativeGrammarError> {
    (0..count(Table::Releases, "releases")?)
        .map(|row| {
            Ok(ReleaseSpec {
                id: text(Table::Releases, "releases", row, 0)?,
                normative_reference: text(Table::Releases, "releases", row, 1)?,
                kind: text(Table::Releases, "releases", row, 2)?,
                status: text(Table::Releases, "releases", row, 3)?,
            })
        })
        .collect()
}

fn load_modules() -> Result<Vec<ModuleSpec>, NativeGrammarError> {
    (0..count(Table::Modules, "modules")?)
        .map(|row| {
            Ok(ModuleSpec {
                id: text(Table::Modules, "modules", row, 0)?,
                kind: text(Table::Modules, "modules", row, 1)?,
            })
        })
        .collect()
}

fn load_profiles() -> Result<Vec<ProfileSpec>, NativeGrammarError> {
    (0..count(Table::Profiles, "profiles")?)
        .map(|row| {
            Ok(ProfileSpec {
                id: text(Table::Profiles, "profiles", row, 0)?,
                release_id: text(Table::Profiles, "profiles", row, 1)?,
                claim: text(Table::Profiles, "profiles", row, 2)?,
            })
        })
        .collect()
}

fn load_profile_modules() -> Result<Vec<ProfileModuleSpec>, NativeGrammarError> {
    (0..count(Table::ProfileModules, "profile-modules")?)
        .map(|row| {
            Ok(ProfileModuleSpec {
                profile_id: text(Table::ProfileModules, "profile-modules", row, 0)?,
                disposition: text(Table::ProfileModules, "profile-modules", row, 1)?,
                module_id: text(Table::ProfileModules, "profile-modules", row, 2)?,
            })
        })
        .collect()
}

fn load_profile_supplements() -> Result<Vec<ProfileSupplementSpec>, NativeGrammarError> {
    (0..count(Table::ProfileSupplements, "profile-supplements")?)
        .map(|row| {
            Ok(ProfileSupplementSpec {
                profile_id: text(Table::ProfileSupplements, "profile-supplements", row, 0)?,
                release_id: text(Table::ProfileSupplements, "profile-supplements", row, 1)?,
            })
        })
        .collect()
}

fn load_features() -> Result<Vec<FeatureSpec>, NativeGrammarError> {
    (0..count(Table::Features, "features")?)
        .map(|row| {
            let priority = text(Table::Features, "features", row, 1)?;
            let priority = priority
                .parse()
                .map_err(|_| NativeGrammarError::InvalidPriority(priority))?;
            Ok(FeatureSpec {
                id: text(Table::Features, "features", row, 0)?,
                priority,
                module_id: text(Table::Features, "features", row, 2)?,
                clause_status: text(Table::Features, "features", row, 3)?,
                layer_statuses: [
                    text(Table::Features, "features", row, 4)?,
                    text(Table::Features, "features", row, 5)?,
                    text(Table::Features, "features", row, 6)?,
                    text(Table::Features, "features", row, 7)?,
                    text(Table::Features, "features", row, 8)?,
                ],
                evidence_owner: text(Table::Features, "features", row, 9)?,
            })
        })
        .collect()
}

fn load_feature_dependencies() -> Result<Vec<FeatureDependencySpec>, NativeGrammarError> {
    (0..count(Table::FeatureDependencies, "feature-dependencies")?)
        .map(|row| {
            Ok(FeatureDependencySpec {
                feature_id: text(Table::FeatureDependencies, "feature-dependencies", row, 0)?,
                dependency_id: text(Table::FeatureDependencies, "feature-dependencies", row, 1)?,
            })
        })
        .collect()
}
