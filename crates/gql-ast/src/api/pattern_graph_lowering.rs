//! Graph-pattern lowering for the public, backend-neutral GQL AST.

use super::identifier_lowering::identifier_from_token;
use super::lowering::{descendant_tokens, syntax_node, syntax_tokens};
use super::pattern_lowering::{lower_inline_where_predicate, lower_pattern_properties};
use super::{
    DynamicParameterReference, EdgeDirection, EdgePattern, GraphPattern, NodePattern,
    NonNegativeIntegerSpecification, ParameterNameForm, PathMode, PathPattern, PathPrefix,
    PathQuantifier, PathSearch, PathTarget, PatternElement,
};
use gql_syntax::{
    ParameterNameForm as SyntaxParameterNameForm, SyntaxElement, SyntaxElementKind, SyntaxKind,
    SyntaxNode, TokenKind, decode_parameter_reference,
};

pub(super) fn lower_graph_pattern(node: &SyntaxNode, source: &str) -> Option<GraphPattern> {
    let elements_in = node.children();
    let mut elements = Vec::new();

    for element in elements_in.iter() {
        match &element.kind {
            SyntaxElementKind::Node(child_node) => match child_node.kind() {
                SyntaxKind::NodePattern => {
                    elements.push(PatternElement::Node(lower_node_pattern(child_node, source)))
                }
                SyntaxKind::EdgePattern => {
                    elements.push(PatternElement::Edge(lower_edge_pattern(child_node, source)));
                }
                SyntaxKind::PathPattern => {
                    if let Some(path) = lower_path_pattern(child_node, source) {
                        elements.push(PatternElement::Path(path));
                    }
                }
                _ => {}
            },
            SyntaxElementKind::Token(_) => {}
        }
    }

    (!elements.is_empty()).then_some(GraphPattern {
        elements,
        span: node.span(),
    })
}

pub(super) fn lower_path_pattern(node: &SyntaxNode, source: &str) -> Option<PathPattern> {
    let children = node.children();
    let has_binding_assignment = children.iter().any(|element| {
        matches!(
            &element.kind,
            SyntaxElementKind::Token(token) if token.kind == TokenKind::Punctuation('=')
        )
    });
    let binding = has_binding_assignment
        .then(|| {
            children.iter().find_map(|element| {
                let SyntaxElementKind::Token(token) = &element.kind else {
                    return None;
                };
                (token.kind == TokenKind::Identifier).then(|| identifier_from_token(token, source))
            })
        })
        .flatten();
    let prefix = children.iter().find_map(|element| {
        let child = syntax_node(element)?;
        (child.kind() == SyntaxKind::PathPrefix)
            .then(|| lower_path_prefix(child))
            .flatten()
    });
    let elements = children.into_iter().find_map(|element| {
        let child = syntax_node(&element)?;
        (child.kind() == SyntaxKind::GraphPattern)
            .then(|| lower_graph_pattern(child, source))
            .flatten()
            .map(|pattern| pattern.elements)
    })?;

    Some(PathPattern {
        binding,
        prefix,
        elements,
        span: node.span(),
    })
}

pub(super) fn lower_path_prefix(node: &SyntaxNode) -> Option<PathPrefix> {
    let tokens = descendant_tokens(node);
    let has = |word: &str| {
        tokens
            .iter()
            .any(|token| token.text().eq_ignore_ascii_case(word))
    };
    let count = tokens.iter().find_map(lower_path_count);
    let search = if has("ALL") && has("SHORTEST") {
        Some(PathSearch::AllShortest)
    } else if has("ANY") && has("SHORTEST") {
        Some(PathSearch::AnyShortest)
    } else if has("SHORTEST") && (has("GROUP") || has("GROUPS")) {
        Some(PathSearch::ShortestGroups { count })
    } else if has("SHORTEST") {
        Some(PathSearch::Shortest { count: count? })
    } else if has("ALL") {
        Some(PathSearch::All)
    } else if has("ANY") {
        Some(PathSearch::Any { count })
    } else {
        None
    };
    let mode = if has("TRAIL") {
        Some(PathMode::Trail)
    } else if has("ACYCLIC") {
        Some(PathMode::Acyclic)
    } else if has("SIMPLE") {
        Some(PathMode::Simple)
    } else if has("WALK") {
        Some(PathMode::Walk)
    } else {
        None
    };
    let target = if has("PATHS") {
        Some(PathTarget::Paths)
    } else if has("PATH") {
        Some(PathTarget::Path)
    } else {
        None
    };
    Some(PathPrefix {
        search,
        mode,
        target,
        span: node.span(),
    })
}

fn lower_path_count(token: &gql_syntax::Token) -> Option<NonNegativeIntegerSpecification> {
    match token.kind {
        TokenKind::Number => token
            .text()
            .parse()
            .ok()
            .map(NonNegativeIntegerSpecification::Literal),
        TokenKind::DynamicParameter => {
            let decoded = decode_parameter_reference(token.text())?;
            Some(NonNegativeIntegerSpecification::Parameter(
                DynamicParameterReference {
                    name: decoded.name.into_owned(),
                    form: match decoded.form {
                        SyntaxParameterNameForm::Extended => ParameterNameForm::Extended,
                        SyntaxParameterNameForm::Delimited => ParameterNameForm::Delimited,
                    },
                    span: token.span,
                },
            ))
        }
        _ => None,
    }
}

fn lower_path_quantifier(node: &SyntaxNode) -> Option<PathQuantifier> {
    let quantifier = node.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        (child.kind() == SyntaxKind::PathQuantifier).then_some(child)
    })?;
    let tokens = syntax_tokens(quantifier.children()).collect::<Vec<_>>();
    let shorthand = tokens.iter().find_map(|token| match token.kind {
        TokenKind::Punctuation('*') => Some((0, None)),
        TokenKind::Punctuation('+') => Some((1, None)),
        TokenKind::Punctuation('?') => Some((0, Some(1))),
        _ => None,
    });
    let (min, max) = if let Some(bounds) = shorthand {
        bounds
    } else {
        let comma = tokens
            .iter()
            .position(|token| token.kind == TokenKind::Punctuation(','));
        let numbers = tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| {
                (token.kind == TokenKind::Number)
                    .then(|| token.text().parse::<u32>().ok().map(|value| (index, value)))
                    .flatten()
            })
            .collect::<Vec<_>>();
        match comma {
            Some(comma) => (
                numbers
                    .iter()
                    .find_map(|(index, value)| (*index < comma).then_some(*value))
                    .unwrap_or(0),
                numbers
                    .iter()
                    .find_map(|(index, value)| (*index > comma).then_some(*value)),
            ),
            None => {
                let exact = numbers.first().map(|(_, value)| *value)?;
                (exact, Some(exact))
            }
        }
    };

    Some(PathQuantifier {
        min,
        max,
        span: quantifier.span(),
    })
}

fn edge_direction_from_pattern(edge: &SyntaxNode) -> EdgeDirection {
    let mut first_token = None;
    let mut last_token = None;

    for token in syntax_tokens(edge.children()) {
        if matches!(token.kind, TokenKind::Punctuation(_)) {
            if first_token.is_none() {
                first_token = Some(token.kind);
            }
            last_token = Some(token.kind);
        }
    }

    match (first_token, last_token) {
        (Some(TokenKind::Punctuation('<')), _) => EdgeDirection::In,
        (Some(TokenKind::Punctuation('-')), Some(TokenKind::Punctuation('>'))) => {
            EdgeDirection::Out
        }
        _ => EdgeDirection::Undirected,
    }
}

fn lower_edge_pattern(node: &SyntaxNode, source: &str) -> EdgePattern {
    let label_tokens = node
        .children()
        .into_iter()
        .find_map(|element| match element.kind {
            SyntaxElementKind::Node(child) if child.kind() == SyntaxKind::LabelList => {
                Some(syntax_tokens(child.children()).collect::<Vec<_>>())
            }
            _ => None,
        });
    let mut binding = None;
    let mut labels = Vec::new();
    if let Some(tokens) = label_tokens {
        let has_colon = tokens
            .iter()
            .any(|token| token.kind == TokenKind::Punctuation(':'));
        if has_colon {
            let mut in_labels = false;
            for token in tokens {
                match token.kind {
                    TokenKind::Punctuation(':') => in_labels = true,
                    TokenKind::Identifier | TokenKind::Keyword(_) => {
                        let identifier = identifier_from_token(&token, source);
                        if !in_labels && binding.is_none() {
                            binding = Some(identifier);
                        } else {
                            labels.push(identifier);
                        }
                    }
                    _ => {}
                }
            }
        } else {
            binding = tokens.iter().find_map(|token| {
                matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_))
                    .then(|| identifier_from_token(token, source))
            });
        }
    }

    EdgePattern {
        binding,
        labels,
        properties: lower_pattern_properties(node, source),
        predicate: lower_inline_where_predicate(node, source),
        direction: edge_direction_from_pattern(node),
        quantifier: lower_path_quantifier(node),
        span: node.span(),
    }
}

fn lower_node_pattern(node: &SyntaxNode, source: &str) -> NodePattern {
    let mut binding = None;
    let mut labels = Vec::new();
    let mut consumed_binding = false;
    let mut in_labels = false;

    let pattern_children = node
        .children()
        .iter()
        .filter(|element| {
            !matches!(
                element,
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if matches!(
                    child.kind(),
                    SyntaxKind::PropertyMap | SyntaxKind::InlineWhereClause
                )
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    for token in syntax_tokens(pattern_children) {
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::Punctuation(':') => in_labels = true,
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                let identifier = identifier_from_token(&token, source);
                if !consumed_binding && !in_labels {
                    binding = Some(identifier);
                    consumed_binding = true;
                } else if in_labels || consumed_binding {
                    labels.push(identifier);
                }
            }
            _ => {}
        }
    }

    NodePattern {
        binding,
        labels,
        properties: lower_pattern_properties(node, source),
        predicate: lower_inline_where_predicate(node, source),
        span: node.span(),
    }
}
