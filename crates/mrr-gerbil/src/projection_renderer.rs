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
    let mut output = PathBuf::from("crates/gql-syntax/src/generated.rs");
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
    let body = format_rust_projection(&render_native_projection_in_child()?)?;
    let stamped = stamp_projection(&body, &fingerprint);
    let output = workspace.join(output);
    if check {
        let tracked = fs::read_to_string(&output)?;
        validate_projection(&tracked, &fingerprint)?;
        if tracked != stamped {
            return Err(
                "tracked Rust grammar projection differs from the native Gerbil ABI".into(),
            );
        }
        println!("native grammar projection is current: {}", output.display());
    } else {
        fs::write(&output, stamped)?;
        println!("wrote native grammar projection: {}", output.display());
    }
    Ok(())
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
    writeln!(rust, "use crate::syntax::TokenKind;")?;
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
    writeln!(rust, "pub enum SyntaxKind {{")?;
    for shape in &grammar.syntax_shapes {
        writeln!(rust, "    {},", shape.name)?;
    }
    writeln!(rust, "}}")?;
    writeln!(rust, "impl SyntaxKind {{")?;
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
    writeln!(rust, "pub enum Keyword {{")?;
    for keyword in &grammar.keywords {
        writeln!(rust, "    {},", keyword.name)?;
    }
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "pub(crate) fn keyword(word: &str) -> Option<Keyword> {{"
    )?;
    writeln!(rust, "    match word.to_ascii_uppercase().as_str() {{")?;
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
        "pub(crate) struct GrammarParserEntrypoint {{ pub(crate) action: GrammarParserAction, pub(crate) marks_match: bool, pub(crate) marks_return: bool }}"
    )?;
    writeln!(
        rust,
        "pub(crate) fn top_level_parser_entrypoint(keyword: Keyword) -> Option<GrammarParserEntrypoint> {{"
    )?;
    writeln!(rust, "    match keyword {{")?;
    for entry in &grammar.parser_entrypoints {
        writeln!(
            rust,
            "        Keyword::{} => Some(GrammarParserEntrypoint {{ action: GrammarParserAction::{}, marks_match: {}, marks_return: {} }}),",
            entry.keyword,
            entry.action,
            entry.effect == "marks-match",
            entry.effect == "marks-return"
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
        let pattern = if operator.kind == "keyword" {
            format!("(TokenKind::Keyword(Keyword::{}), _)", operator.lexeme)
        } else {
            let chars = operator.lexeme.chars().collect::<Vec<_>>();
            if chars.len() == 1 {
                format!("(TokenKind::Punctuation({:?}), _)", chars[0])
            } else {
                format!(
                    "(TokenKind::Punctuation({:?}), Some(TokenKind::Punctuation({:?})))",
                    chars[0], chars[1]
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
            operator.lexeme.len()
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;
    writeln!(
        rust,
        "pub(crate) fn prefix_operator_precedence(keyword: Keyword) -> Option<u8> {{"
    )?;
    writeln!(rust, "    match keyword {{")?;
    for operator in &grammar.prefix_operators {
        writeln!(
            rust,
            "        Keyword::{} => Some({}),",
            operator.lexeme, operator.precedence
        )?;
    }
    writeln!(rust, "        _ => None,")?;
    writeln!(rust, "    }}")?;
    writeln!(rust, "}}")?;

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
    Ok(rust)
}
