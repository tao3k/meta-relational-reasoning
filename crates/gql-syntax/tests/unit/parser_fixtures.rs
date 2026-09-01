use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::parse;

const M5_POSITIVE_CORPUS: &str = include_str!("../../test-data/parser/m5-positive.tsv");
const M5_NEGATIVE_CORPUS: &str = include_str!("../../test-data/parser/m5-negative.tsv");

fn m5_corpus_cases(corpus: &'static str, prefix: &str) -> Vec<(&'static str, &'static str)> {
    let cases: Vec<_> = corpus
        .lines()
        .map(|line| {
            line.split_once('\t')
                .expect("M5 corpus rows must be id<TAB>query")
        })
        .collect();
    assert_eq!(
        cases.len(),
        100,
        "M5 {prefix} corpus must contain exactly 100 cases"
    );

    let ids: HashSet<_> = cases.iter().map(|(id, _)| *id).collect();
    let sources: HashSet<_> = cases.iter().map(|(_, source)| *source).collect();
    assert_eq!(
        ids.len(),
        cases.len(),
        "M5 {prefix} case ids must be unique"
    );
    assert_eq!(
        sources.len(),
        cases.len(),
        "M5 {prefix} queries must be unique"
    );
    assert!(
        cases
            .iter()
            .all(|(id, source)| id.starts_with(prefix) && !source.is_empty()),
        "M5 {prefix} rows must use the expected id prefix and non-empty query"
    );
    cases
}

#[test]
fn m5_positive_corpus_is_accepted_losslessly() {
    for (id, source) in m5_corpus_cases(M5_POSITIVE_CORPUS, "p") {
        let parsed = parse(id, source);
        assert!(
            parsed.diagnostics.is_empty(),
            "positive case {id} should parse cleanly: {:?}",
            parsed.diagnostics
        );
        assert_eq!(
            parsed.tree.source().text(),
            source,
            "source mismatch for {id}"
        );
        assert_eq!(
            parsed.tree.rowan_root().text().to_string(),
            source,
            "CST mismatch for {id}"
        );
    }
}

#[test]
fn m5_negative_corpus_is_rejected_losslessly() {
    for (id, source) in m5_corpus_cases(M5_NEGATIVE_CORPUS, "n") {
        let parsed = parse(id, source);
        assert!(
            !parsed.diagnostics.is_empty(),
            "negative case {id} must fail closed"
        );
        assert_eq!(
            parsed.tree.source().text(),
            source,
            "source mismatch for {id}"
        );
        assert_eq!(
            parsed.tree.rowan_root().text().to_string(),
            source,
            "CST mismatch for {id}"
        );
    }
}

#[test]
fn parser_ok_fixtures_have_no_diagnostics_and_roundtrip_text() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/parser/ok");

    for entry in fs::read_dir(&fixture_root).expect("parser ok fixtures directory should exist") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("gql") {
            continue;
        }

        let source = fs::read_to_string(&path).expect("read parser ok fixture");
        let parsed = parse(path.to_string_lossy().as_ref(), source.as_str());

        assert!(
            parsed.diagnostics.is_empty(),
            "fixture {} should parse cleanly, diagnostics={:?}",
            path.display(),
            parsed.diagnostics
        );
        assert_eq!(parsed.tree.source().text(), source.as_str());
        assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
    }
}

#[test]
fn parser_err_fixtures_emit_expected_diagnostics() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/parser/err");

    for entry in fs::read_dir(&fixture_root).expect("parser err fixtures directory should exist") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("gql") {
            continue;
        }

        let source = fs::read_to_string(&path).expect("read parser err fixture");
        let parsed = parse(path.to_string_lossy().as_ref(), source.as_str());
        let codes = expected_diagnostic_codes(path.file_name().and_then(|name| name.to_str()));

        assert!(
            !parsed.diagnostics.is_empty(),
            "fixture {} should emit diagnostics",
            path.display()
        );
        let emitted: Vec<_> = parsed
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();

        for code in codes {
            assert!(
                emitted.iter().any(|emitted| emitted == &code),
                "fixture {} missing diagnostic `{}`; got {:?}",
                path.display(),
                code,
                emitted
            );
        }
    }
}

#[test]
fn parser_err_fixtures_keep_source_and_recoverable_structure() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/parser/err");

    for entry in fs::read_dir(&fixture_root).expect("parser err fixtures directory should exist") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("gql") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read parser err fixture");
        let parsed = parse(path.to_string_lossy().as_ref(), source.as_str());

        assert_eq!(
            parsed.tree.source().text(),
            source,
            "fixture {} should preserve source text",
            path.display()
        );
        assert_eq!(
            parsed.tree.rowan_root().text().to_string(),
            source,
            "fixture {} should preserve rowan text",
            path.display()
        );
    }
}

fn expected_diagnostic_codes(filename: Option<&str>) -> Vec<&'static str> {
    match filename {
        Some("01-return-without-match.gql") => vec!["GQL-PARSE-MISSING-KEYWORD"],
        Some("04-malformed-delimiter.gql") => vec!["GQL-PARSE-MATCH-SYNTAX"],
        Some("05-unknown-token.gql") => vec!["GQL-SYNTAX-UNKNOWN-CHARACTER"],
        Some("06-unterminated-string.gql") => vec!["GQL-SYNTAX-UNTERMINATED-STRING"],
        Some("07-where-missing-expression.gql") => vec!["GQL-PARSE-WHERE-SYNTAX"],
        Some("08-edge-label-list-malformed.gql") => vec!["GQL-PARSE-MATCH-SYNTAX"],
        _ => Vec::new(),
    }
}

#[test]
fn parser_err_fixtures_include_all_known_cases() {
    let known = [
        "01-return-without-match.gql",
        "04-malformed-delimiter.gql",
        "05-unknown-token.gql",
        "06-unterminated-string.gql",
        "07-where-missing-expression.gql",
        "08-edge-label-list-malformed.gql",
    ];

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/parser/err");
    let mut fixtures = Vec::new();

    for entry in fs::read_dir(&fixture_root).expect("parser err fixtures directory should exist") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("gql")
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
        {
            fixtures.push(name.to_string());
        }
    }

    fixtures.sort();
    assert_eq!(fixtures, known);
}
