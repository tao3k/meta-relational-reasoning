//! Thin CLI around the native AOT grammar bindings and tracked Rust projection.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use crate::native::NativeGrammar;
use crate::{stamp_projection, validate_projection, workspace_input_fingerprint};

#[must_use]
/// Runs the native grammar projection check or update command.
pub fn run_cli() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("--emit-native-projection") {
        print!("{}", render_rust_projection(&NativeGrammar::load()?)?);
        return Ok(());
    }
    let mut workspace = PathBuf::from(".");
    let mut output = PathBuf::from("crates/gql-syntax/src/generated/projection.rs");
    let mut check = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--workspace" => {
                workspace = PathBuf::from(arguments.next().ok_or("--workspace requires a path")?)
            }
            "--output" => {
                output = PathBuf::from(arguments.next().ok_or("--output requires a path")?)
            }
            "--check" => check = true,
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }

    let fingerprint = workspace_input_fingerprint(&workspace)?;
    let rendered = render_native_projection_in_child()?;
    let (projection_body, lexical_body, aggregate_body, parser_body) = split_projection(&rendered)?;
    let projection = stamp_projection(&format_rust_projection(&projection_body)?, &fingerprint);
    let lexical = stamp_projection(&format_rust_projection(&lexical_body)?, &fingerprint);
    let aggregate = stamp_projection(&format_rust_projection(&aggregate_body)?, &fingerprint);
    let parser = stamp_projection(&format_rust_projection(&parser_body)?, &fingerprint);
    let output = workspace.join(output);
    let lexical_output = output
        .parent()
        .ok_or("generated projection has no parent directory")?
        .join("lexical_forms.rs");
    let aggregate_output = output
        .parent()
        .ok_or("generated projection has no parent directory")?
        .join("aggregate_forms.rs");
    let parser_output = output
        .parent()
        .ok_or("generated projection has no parent directory")?
        .join("parser_forms.rs");
    if check {
        let tracked = fs::read_to_string(&output)?;
        validate_projection(&tracked, &fingerprint)?;
        let tracked_lexical = fs::read_to_string(&lexical_output)?;
        validate_projection(&tracked_lexical, &fingerprint)?;
        let tracked_aggregate = fs::read_to_string(&aggregate_output)?;
        validate_projection(&tracked_aggregate, &fingerprint)?;
        let tracked_parser = fs::read_to_string(&parser_output)?;
        validate_projection(&tracked_parser, &fingerprint)?;
        if tracked != projection
            || tracked_lexical != lexical
            || tracked_aggregate != aggregate
            || tracked_parser != parser
        {
            return Err(
                "tracked Rust grammar projection differs from the native Gerbil ABI".into(),
            );
        }
        println!(
            "native grammar projection is current: {}, {}, {}, {}",
            output.display(),
            lexical_output.display(),
            aggregate_output.display(),
            parser_output.display()
        );
    } else {
        if let Some(parent) = lexical_output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, projection)?;
        fs::write(&lexical_output, lexical)?;
        fs::write(&aggregate_output, aggregate)?;
        fs::write(&parser_output, parser)?;
        println!(
            "wrote native grammar projection: {}, {}, {}, {}",
            output.display(),
            lexical_output.display(),
            aggregate_output.display(),
            parser_output.display()
        );
    }
    Ok(())
}

const LEXICAL_BEGIN: &str = "// @generated-part lexical-forms begin";
const LEXICAL_END: &str = "// @generated-part lexical-forms end";
const AGGREGATE_BEGIN: &str = "// @generated-part aggregate-forms begin";
const AGGREGATE_END: &str = "// @generated-part aggregate-forms end";
const PARSER_BEGIN: &str = "// @generated-part parser-forms begin";
const PARSER_END: &str = "// @generated-part parser-forms end";

fn split_projection(
    source: &str,
) -> Result<(String, String, String, String), Box<dyn std::error::Error>> {
    let begin = source
        .find(LEXICAL_BEGIN)
        .ok_or("native projection omitted lexical begin marker")?;
    let end = source
        .find(LEXICAL_END)
        .ok_or("native projection omitted lexical end marker")?;
    if end <= begin {
        return Err("native projection lexical markers are out of order".into());
    }
    let aggregate_begin = source
        .find(AGGREGATE_BEGIN)
        .ok_or("native projection omitted aggregate begin marker")?;
    let aggregate_end = source
        .find(AGGREGATE_END)
        .ok_or("native projection omitted aggregate end marker")?;
    if aggregate_end <= aggregate_begin || aggregate_begin <= end {
        return Err("native projection aggregate markers are out of order".into());
    }
    let parser_begin = source
        .find(PARSER_BEGIN)
        .ok_or("native projection omitted parser begin marker")?;
    let parser_end = source
        .find(PARSER_END)
        .ok_or("native projection omitted parser end marker")?;
    if parser_end <= parser_begin || parser_begin <= aggregate_end {
        return Err("native projection parser markers are out of order".into());
    }
    let lexical_start = begin + LEXICAL_BEGIN.len();
    let lexical = format!(
        "// Generated through the Gerbil native AOT bindings; do not edit.\n//! Lexical form tables projected from the Gerbil grammar authority.\n{}\n",
        source[lexical_start..end].trim()
    );
    let aggregate_start = aggregate_begin + AGGREGATE_BEGIN.len();
    let aggregate = format!(
        "// Generated through the Gerbil native AOT bindings; do not edit.\n//! Aggregate grammar forms projected from the Gerbil grammar authority.\n{}\n",
        source[aggregate_start..aggregate_end].trim()
    );
    let parser_start = parser_begin + PARSER_BEGIN.len();
    let parser = format!(
        "// Generated through the Gerbil native AOT bindings; do not edit.\n//! Parser grammar forms projected from the Gerbil grammar authority.\n{}\n",
        source[parser_start..parser_end].trim()
    );
    let mut projection = String::with_capacity(
        source.len()
            - (end - begin)
            - (aggregate_end - aggregate_begin)
            - (parser_end - parser_begin),
    );
    projection.push_str(&source[..begin]);
    projection.push_str(&source[end + LEXICAL_END.len()..aggregate_begin]);
    projection.push_str(&source[aggregate_end + AGGREGATE_END.len()..parser_begin]);
    projection.push_str(&source[parser_end + PARSER_END.len()..]);
    Ok((projection, lexical, aggregate, parser))
}

fn render_native_projection_in_child() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(env::current_exe()?)
        .arg("--emit-native-projection")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "native Gerbil projection renderer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn format_rust_projection(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let rustfmt = env::var_os("RUSTFMT").unwrap_or_else(|| "rustfmt".into());
    let mut child = Command::new(rustfmt)
        .args(["--emit", "stdout", "--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("rustfmt stdin is unavailable")?;
    let waiter = std::thread::spawn(move || child.wait_with_output());
    input.write_all(source.as_bytes())?;
    drop(input);
    let output = waiter
        .join()
        .map_err(|_| "rustfmt waiter thread panicked")??;
    if !output.status.success() {
        return Err(format!(
            "rustfmt rejected the native grammar projection: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn render_rust_projection(grammar: &NativeGrammar) -> Result<String, std::fmt::Error> {
    let mut rust = String::new();
    writeln!(
        rust,
        "// Generated through the Gerbil native AOT bindings; do not edit."
    )?;
    writeln!(
        rust,
        "//! Property-graph grammar projection consumed by the Rowan CST frontend."
    )?;
    writeln!(
        rust,
        "pub(crate) const GRAMMAR_PROJECTION_SCHEMA: &str = \"mrr.gerbil-grammar-projection.v1\";"
    )?;
    writeln!(
        rust,
        "pub(crate) const GERBIL_SCHEME_RUST_REVISION: &str = \"a83fb649ddbbeaabdb538a6eaf0ded10838f7fad\";"
    )?;
    writeln!(
        rust,
        "#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]"
    )?;
    writeln!(rust, "#[repr(u16)]")?;
    writeln!(rust, "/// Grammar-owned lossless Rowan syntax kinds.")?;
    writeln!(rust, "pub enum SyntaxKind {{")?;
    for shape in &grammar.syntax_shapes {
        writeln!(rust, "    {},", shape.name)?;
    }
    writeln!(rust, "}}")?;

    writeln!(rust, "{LEXICAL_BEGIN}")?;
    let non_reserved_words = grammar.non_reserved_words.join(" ");
    writeln!(
        rust,
        "/// ISO GQL non-reserved words admitted as regular identifiers."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_NON_RESERVED_WORDS: &str = {non_reserved_words:?};"
    )?;
    writeln!(
        rust,
        "/// Returns whether `word` is an ISO GQL non-reserved word."
    )?;
    writeln!(rust, "pub fn is_non_reserved_word(word: &str) -> bool {{")?;
    writeln!(
        rust,
        "    ISO_GQL_NON_RESERVED_WORDS.split_ascii_whitespace().any(|candidate| word.eq_ignore_ascii_case(candidate))"
    )?;
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "/// Gerbil-owned ISO GQL numeric literal forms: form, notation, suffix, semantic class."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_NUMERIC_LITERAL_FORMS: &[(&str, &str, &str, &str)] = &["
    )?;
    for literal in &grammar.numeric_literals {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}, {:?}),",
            literal.form, literal.notation, literal.suffix, literal.class
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(
        rust,
        "/// Gerbil-owned character-string forms and escape actions."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_CHARACTER_STRING_FORMS: &[(&str, &str, &str, &str)] = &["
    )?;
    for literal in &grammar.character_string_literals {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}, {:?}),",
            literal.form, literal.lexeme, literal.action, literal.class
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(
        rust,
        "/// Gerbil-owned parameter reference forms: form, prefix, name grammar, semantic context."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_PARAMETER_REFERENCE_FORMS: &[(&str, &str, &str, &str)] = &["
    )?;
    for parameter in &grammar.parameter_references {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}, {:?}),",
            parameter.form, parameter.prefix, parameter.name, parameter.context
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(
        rust,
        "/// Gerbil-owned postfix predicate tests: kind, negation, value, operand domain."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_PREDICATE_TEST_FORMS: &[(&str, &str, &str, &str)] = &["
    )?;
    for predicate in &grammar.predicate_tests {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}, {:?}),",
            predicate.kind, predicate.negation, predicate.value, predicate.operand
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(
        rust,
        "/// Gerbil-owned aggregate forms: semantic name, keyword, family, quantifier policy, arity."
    )?;
    writeln!(
        rust,
        "pub const ISO_GQL_AGGREGATE_FUNCTION_FORMS: &[(&str, &str, &str, &str, u8)] = &["
    )?;
    for aggregate in &grammar.aggregate_functions {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}, {:?}, {}),",
            aggregate.name,
            aggregate.keyword,
            aggregate.kind,
            aggregate.quantifier,
            aggregate.arity
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(rust, "{LEXICAL_END}")?;
    writeln!(rust, "impl SyntaxKind {{")?;
    writeln!(rust, "    pub const ALL: &'static [Self] = &[")?;
    for shape in &grammar.syntax_shapes {
        writeln!(rust, "        Self::{},", shape.name)?;
    }
    writeln!(rust, "    ];")?;
    writeln!(
        rust,
        "    pub(crate) fn to_rowan(self) -> rowan::SyntaxKind {{ rowan::SyntaxKind(self as u16) }}"
    )?;
    writeln!(
        rust,
        "    pub(crate) fn from_rowan(kind: rowan::SyntaxKind) -> Self {{ match kind.0 {{"
    )?;
    for (index, shape) in grammar.syntax_shapes.iter().enumerate() {
        writeln!(rust, "        {index} => Self::{},", shape.name)?;
    }
    writeln!(rust, "        _ => Self::Unknown,")?;
    writeln!(rust, "    }} }}")?;
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "pub(crate) const GRAMMAR_SYNTAX_SHAPES: &[(&str, &str, &[&str])] = &["
    )?;
    for shape in &grammar.syntax_shapes {
        let fields = shape
            .fields
            .iter()
            .map(|field| format!("{field:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            rust,
            "    ({:?}, {:?}, &[{fields}]),",
            shape.name, shape.category
        )?;
    }
    writeln!(rust, "];")?;

    writeln!(rust, "#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]")?;
    writeln!(rust, "/// Grammar-owned keyword identities.")?;
    writeln!(rust, "pub enum Keyword {{")?;
    for keyword in &grammar.keywords {
        writeln!(rust, "    {},", keyword.name)?;
    }
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "pub(crate) fn keyword(word: &str) -> Option<Keyword> {{"
    )?;
    writeln!(
        rust,
        "    if word.bytes().any(|byte| byte.is_ascii_lowercase()) {{ let uppercase = word.to_ascii_uppercase(); keyword_uppercase(&uppercase) }} else {{ keyword_uppercase(word) }}"
    )?;
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "fn keyword_uppercase(word: &str) -> Option<Keyword> {{"
    )?;
    writeln!(rust, "    match word {{")?;
    for keyword in &grammar.keywords {
        writeln!(
            rust,
            "        {:?} => Some(Keyword::{}),",
            keyword.text, keyword.name
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;

    writeln!(rust, "{AGGREGATE_BEGIN}")?;
    writeln!(rust, "use super::projection::Keyword;")?;
    writeln!(rust, "#[derive(Clone, Copy, Debug, Eq, PartialEq)]")?;
    writeln!(
        rust,
        "pub(crate) struct GrammarAggregateFunctionSpec {{ pub(crate) arity: u8, pub(crate) permits_star: bool, pub(crate) permits_quantifier: bool }}"
    )?;
    writeln!(
        rust,
        "pub(crate) fn aggregate_function_spec(keyword: Keyword) -> Option<GrammarAggregateFunctionSpec> {{"
    )?;
    writeln!(rust, "    match keyword {{")?;
    let mut aggregate_keywords = Vec::new();
    for aggregate in &grammar.aggregate_functions {
        if aggregate_keywords.contains(&aggregate.keyword) {
            continue;
        }
        aggregate_keywords.push(aggregate.keyword.clone());
        let rows = grammar
            .aggregate_functions
            .iter()
            .filter(|candidate| candidate.keyword == aggregate.keyword)
            .collect::<Vec<_>>();
        let permits_star = rows.iter().any(|row| row.kind == "star");
        let value_row = rows
            .iter()
            .find(|row| row.kind != "star")
            .copied()
            .unwrap_or(aggregate);
        let permits_quantifier = value_row.quantifier != "forbidden";
        writeln!(
            rust,
            "        Keyword::{} => Some(GrammarAggregateFunctionSpec {{ arity: {}, permits_star: {permits_star}, permits_quantifier: {permits_quantifier} }}),",
            aggregate.keyword, value_row.arity
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;
    writeln!(rust, "{AGGREGATE_END}")?;

    writeln!(rust, "{PARSER_BEGIN}")?;
    writeln!(rust, "use crate::syntax::TokenKind;")?;
    writeln!(rust, "use super::projection::Keyword;")?;

    let mut actions = Vec::new();
    for entry in &grammar.parser_entrypoints {
        if !actions.contains(&entry.action) {
            actions.push(entry.action.clone());
        }
    }
    writeln!(rust, "#[derive(Clone, Copy, Debug, Eq, PartialEq)]")?;
    writeln!(rust, "pub(crate) enum GrammarParserAction {{")?;
    for action in actions {
        writeln!(rust, "    {action},")?;
    }
    writeln!(rust, "}}")?;
    writeln!(rust, "#[derive(Clone, Copy, Debug, Eq, PartialEq)]")?;
    writeln!(
        rust,
        "pub(crate) struct GrammarParserEntrypoint {{ pub(crate) action: GrammarParserAction }}"
    )?;
    writeln!(
        rust,
        "pub(crate) fn top_level_parser_entrypoint(keyword: Keyword) -> Option<GrammarParserEntrypoint> {{"
    )?;
    writeln!(rust, "    match keyword {{")?;
    for entry in &grammar.parser_entrypoints {
        writeln!(
            rust,
            "        Keyword::{} => Some(GrammarParserEntrypoint {{ action: GrammarParserAction::{} }}),",
            entry.keyword, entry.action
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;

    writeln!(rust, "#[derive(Clone, Copy, Debug, Eq, PartialEq)]")?;
    writeln!(
        rust,
        "pub(crate) struct BinaryOperatorSpec {{ pub(crate) left_binding_power: u8, pub(crate) right_binding_power: u8, pub(crate) is_right_associative: bool, pub(crate) width: u8 }}"
    )?;
    writeln!(
        rust,
        "pub(crate) fn binary_operator_spec(first: TokenKind, second: Option<TokenKind>) -> Option<BinaryOperatorSpec> {{"
    )?;
    writeln!(rust, "    match (first, second) {{")?;
    let mut binary = grammar.binary_operators.iter().collect::<Vec<_>>();
    binary.sort_by_key(|operator| {
        !(operator.kind == "punctuation" && operator.lexeme.chars().count() > 1)
    });
    for operator in binary {
        let right = operator.associativity == "right";
        let (pattern, width) = if operator.kind == "keyword" {
            (
                format!("(TokenKind::Keyword(Keyword::{}), _)", operator.lexeme),
                1,
            )
        } else {
            let chars = operator.lexeme.chars().collect::<Vec<_>>();
            if chars.len() == 1 {
                (format!("(TokenKind::Punctuation({:?}), _)", chars[0]), 1)
            } else {
                (
                    format!(
                        "(TokenKind::Punctuation({:?}), Some(TokenKind::Punctuation({:?})))",
                        chars[0], chars[1]
                    ),
                    chars.len(),
                )
            }
        };
        writeln!(
            rust,
            "        {pattern} => Some(BinaryOperatorSpec {{ left_binding_power: {}, right_binding_power: {}, is_right_associative: {right}, width: {} }}),",
            operator.precedence,
            if right {
                operator.precedence
            } else {
                operator.precedence + 1
            },
            width
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "pub(crate) fn prefix_operator_precedence(kind: TokenKind) -> Option<u8> {{"
    )?;
    writeln!(rust, "    match kind {{")?;
    for operator in &grammar.prefix_operators {
        let pattern = if operator.kind == "keyword" {
            format!("TokenKind::Keyword(Keyword::{})", operator.lexeme)
        } else {
            let character = operator
                .lexeme
                .chars()
                .next()
                .expect("native grammar rejects empty operator lexemes");
            format!("TokenKind::Punctuation({character:?})")
        };
        writeln!(rust, "        {pattern} => Some({}),", operator.precedence)?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;

    writeln!(rust, "#[rustfmt::skip]")?;
    writeln!(
        rust,
        "pub(crate) const GRAMMAR_RECOVERIES: &[(&str, &str, &str)] = &["
    )?;
    for recovery in &grammar.recoveries {
        writeln!(
            rust,
            "    ({:?}, {:?}, {:?}),",
            recovery.site, recovery.code, recovery.strategy
        )?;
    }
    writeln!(rust, "];")?;
    writeln!(
        rust,
        "pub(crate) fn recovery_diagnostic(site: &str) -> Option<&'static str> {{ GRAMMAR_RECOVERIES.iter().find_map(|(candidate, code, _)| (*candidate == site).then_some(*code)) }}"
    )?;
    writeln!(rust, "{PARSER_END}")?;

    Ok(rust)
}
