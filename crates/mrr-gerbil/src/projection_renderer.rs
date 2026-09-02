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
    writeln!(rust, "/// Grammar-owned lossless Rowan syntax kinds.")?;
    writeln!(rust, "pub enum SyntaxKind {{")?;
    for shape in &grammar.syntax_shapes {
        writeln!(rust, "    {},", shape.name)?;
    }
    writeln!(rust, "}}")?;

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

    Ok(rust)
}
