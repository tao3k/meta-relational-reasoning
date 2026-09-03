//! Data-management statement lowering owner.
#![forbid(unsafe_code)]

use super::identifier_lowering::identifier_from_token;
use super::lowering::{lower_expression, significant_node_span, syntax_node, syntax_tokens};
use super::lowering_support::is_expression_kind;
use super::pattern_graph_lowering::{lower_graph_pattern, lower_path_pattern};
use super::{
    BinaryOperator, CatalogCreatePolicy, CatalogDropPolicy, CatalogObjectName, CatalogStatement,
    ClosedReferenceTypeSpecification, EdgeDirection, EdgeKind, EdgeTypeSpecification, Expression,
    GraphPattern, GraphTypeSource, GraphTypeSpecification, NestedGraphTypeSpecification,
    NodeTypeReference, NodeTypeSpecification, ProcedureCall, PropertyType, PropertyValueType,
    PropertyValueTypeForm, QueryClause, ReferenceValueTypeKind, SessionCommand, SetItem,
    TransactionAccessMode, TransactionCommand, TypeParameter,
};
use gql_source::Diagnostic;
use gql_syntax::{Keyword, SyntaxKind, SyntaxNode, TokenKind};

pub(super) fn lower_catalog_statement(node: &SyntaxNode, source: &str) -> Option<CatalogStatement> {
    node.children().into_iter().find_map(|element| {
        let child = syntax_node(&element)?;
        let kind = child.kind();
        if !matches!(
            kind,
            SyntaxKind::CreateSchemaStatement
                | SyntaxKind::DropSchemaStatement
                | SyntaxKind::CreateGraphStatement
                | SyntaxKind::DropGraphStatement
                | SyntaxKind::CreateGraphTypeStatement
                | SyntaxKind::DropGraphTypeStatement
        ) {
            return None;
        }
        let name = lower_catalog_object_name(child, source)?;
        Some(match kind {
            SyntaxKind::CreateSchemaStatement => CatalogStatement::CreateSchema { name },
            SyntaxKind::DropSchemaStatement => CatalogStatement::DropSchema { name },
            SyntaxKind::CreateGraphStatement => CatalogStatement::CreateGraph {
                name,
                graph_type: GraphTypeSpecification::Any {
                    typed: syntax_tokens(child.children())
                        .any(|token| token.kind == TokenKind::Keyword(Keyword::Typed)),
                    span: significant_node_span(child),
                },
                policy: catalog_create_policy(child),
            },
            SyntaxKind::DropGraphStatement => CatalogStatement::DropGraph {
                name,
                policy: catalog_drop_policy(child),
            },
            SyntaxKind::CreateGraphTypeStatement => CatalogStatement::CreateGraphType {
                name,
                source: lower_graph_type_source(child, source)?,
                policy: catalog_create_policy(child),
            },
            SyntaxKind::DropGraphTypeStatement => CatalogStatement::DropGraphType {
                name,
                policy: catalog_drop_policy(child),
            },
            _ => unreachable!("catalog syntax kind was checked above"),
        })
    })
}

fn catalog_create_policy(node: &SyntaxNode) -> CatalogCreatePolicy {
    node.children()
        .iter()
        .filter_map(syntax_node)
        .find_map(|child| {
            (child.kind() == SyntaxKind::CatalogConflictClause).then(|| {
                if syntax_tokens(child.children())
                    .any(|token| token.kind == TokenKind::Keyword(Keyword::Replace))
                {
                    CatalogCreatePolicy::OrReplace
                } else {
                    CatalogCreatePolicy::IfNotExists
                }
            })
        })
        .unwrap_or(CatalogCreatePolicy::Error)
}

fn catalog_drop_policy(node: &SyntaxNode) -> CatalogDropPolicy {
    if node
        .children()
        .iter()
        .filter_map(syntax_node)
        .any(|child| child.kind() == SyntaxKind::CatalogConflictClause)
    {
        CatalogDropPolicy::IfExists
    } else {
        CatalogDropPolicy::Error
    }
}

fn lower_graph_type_source(node: &SyntaxNode, source: &str) -> Option<GraphTypeSource> {
    node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        if child.kind() != SyntaxKind::GraphTypeSource {
            return None;
        }
        let span = significant_node_span(child);
        let tokens = syntax_tokens(child.children()).collect::<Vec<_>>();
        if let Some(specification) = child.children().iter().find_map(|element| {
            let specification = syntax_node(element)?;
            (specification.kind() == SyntaxKind::NestedGraphTypeSpecification)
                .then(|| lower_nested_graph_type(specification, source))
                .flatten()
        }) {
            return Some(GraphTypeSource::Nested {
                specification,
                span,
            });
        }
        let target = lower_catalog_object_name(child, source)?;
        if tokens
            .iter()
            .any(|token| token.kind == TokenKind::Keyword(Keyword::Copy))
        {
            Some(GraphTypeSource::CopyOf {
                graph_type: target,
                span,
            })
        } else {
            Some(GraphTypeSource::LikeGraph {
                graph: target,
                span,
            })
        }
    })
}

fn lower_catalog_object_name(node: &SyntaxNode, source: &str) -> Option<CatalogObjectName> {
    if let Some(name) = node.children().iter().find_map(|element| {
        let name = syntax_node(element)?;
        (name.kind() == SyntaxKind::CatalogObjectName).then_some(name)
    }) {
        let parts = syntax_tokens(name.children())
            .filter(|token| token.kind == TokenKind::Identifier)
            .map(|token| identifier_from_token(&token, source))
            .collect::<Vec<_>>();
        return (!parts.is_empty()).then(|| CatalogObjectName {
            parts,
            span: significant_node_span(name),
        });
    }
    let identifier = syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier).then(|| identifier_from_token(&token, source))
    })?;
    Some(CatalogObjectName {
        span: identifier.span,
        parts: vec![identifier],
    })
}

fn lower_nested_graph_type(
    node: &SyntaxNode,
    source: &str,
) -> Option<NestedGraphTypeSpecification> {
    let node_types = node
        .children()
        .iter()
        .filter_map(|element| {
            let node_type = syntax_node(element)?;
            (node_type.kind() == SyntaxKind::NodeTypeSpecification)
                .then(|| lower_node_type(node_type, source))
                .flatten()
        })
        .collect::<Vec<_>>();
    let edge_types = node
        .children()
        .iter()
        .filter_map(|element| {
            let edge_type = syntax_node(element)?;
            (edge_type.kind() == SyntaxKind::EdgeTypeSpecification)
                .then(|| lower_edge_type(edge_type, source))
                .flatten()
        })
        .collect::<Vec<_>>();
    (!node_types.is_empty() || !edge_types.is_empty()).then(|| NestedGraphTypeSpecification {
        node_types,
        edge_types,
        span: significant_node_span(node),
    })
}

fn lower_node_type(node: &SyntaxNode, source: &str) -> Option<NodeTypeSpecification> {
    let tokens = syntax_tokens(node.children()).collect::<Vec<_>>();
    let named = tokens.iter().any(|token| {
        token.kind == TokenKind::Identifier
            && (token.text().eq_ignore_ascii_case("NODE")
                || token.text().eq_ignore_ascii_case("VERTEX"))
    });
    let identifiers = tokens
        .iter()
        .filter(|token| {
            token.kind == TokenKind::Identifier
                && !(named
                    && (token.text().eq_ignore_ascii_case("NODE")
                        || token.text().eq_ignore_ascii_case("VERTEX")))
        })
        .map(|token| identifier_from_token(token, source))
        .collect::<Vec<_>>();
    let boundary = tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Punctuation('(')
                || token.kind == TokenKind::Keyword(Keyword::As)
        })
        .map(|token| token.span.start);
    let (name, alias) = if let (true, Some(boundary)) = (named, boundary) {
        (
            identifiers
                .iter()
                .find(|identifier| identifier.span.start < boundary)
                .cloned(),
            identifiers
                .iter()
                .find(|identifier| identifier.span.start > boundary)
                .cloned(),
        )
    } else if named {
        (identifiers.first().cloned(), None)
    } else {
        (None, identifiers.first().cloned())
    };
    let properties = lower_property_type_list(node, source);
    Some(NodeTypeSpecification {
        name,
        alias,
        key_labels: lower_key_label_set(node, source),
        labels: lower_label_set(node, source),
        properties,
        span: significant_node_span(node),
    })
}

fn lower_edge_type(node: &SyntaxNode, source: &str) -> Option<EdgeTypeSpecification> {
    let kind = node.children().iter().find_map(|element| {
        let kind = syntax_node(element)?;
        (kind.kind() == SyntaxKind::EdgeKind).then(|| {
            syntax_tokens(kind.children()).find_map(|token| {
                token
                    .text()
                    .eq_ignore_ascii_case("DIRECTED")
                    .then_some(EdgeKind::Directed)
                    .or_else(|| {
                        token
                            .text()
                            .eq_ignore_ascii_case("UNDIRECTED")
                            .then_some(EdgeKind::Undirected)
                    })
            })
        })?
    });
    let name = syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier
            && !["EDGE", "RELATIONSHIP", "CONNECTING"]
                .iter()
                .any(|contextual| token.text().eq_ignore_ascii_case(contextual)))
        .then(|| identifier_from_token(&token, source))
    });
    let endpoints = node
        .children()
        .iter()
        .filter_map(syntax_node)
        .flat_map(|child| {
            if child.kind() == SyntaxKind::NodeTypeReference {
                lower_node_type_reference(child, source)
                    .into_iter()
                    .collect()
            } else if child.kind() == SyntaxKind::EndpointPair {
                child
                    .children()
                    .iter()
                    .filter_map(syntax_node)
                    .filter(|endpoint| endpoint.kind() == SyntaxKind::NodeTypeReference)
                    .filter_map(|endpoint| lower_node_type_reference(endpoint, source))
                    .collect()
            } else {
                Vec::new()
            }
        })
        .collect::<Vec<_>>();
    if endpoints.len() != 2 {
        return None;
    }
    let direction = node
        .children()
        .iter()
        .filter_map(syntax_node)
        .find_map(|child| {
            if child.kind() == SyntaxKind::EdgeDirection {
                lower_edge_direction(child, kind)
            } else if child.kind() == SyntaxKind::EndpointPair {
                child
                    .children()
                    .iter()
                    .filter_map(syntax_node)
                    .find_map(|nested| {
                        (nested.kind() == SyntaxKind::EdgeDirection)
                            .then(|| lower_edge_direction(nested, kind))
                            .flatten()
                    })
            } else {
                None
            }
        })?;
    Some(EdgeTypeSpecification {
        kind,
        name,
        endpoints,
        direction,
        key_labels: lower_key_label_set(node, source),
        labels: lower_label_set(node, source),
        properties: lower_property_type_list(node, source),
        span: significant_node_span(node),
    })
}

fn lower_edge_direction(node: &SyntaxNode, kind: Option<EdgeKind>) -> Option<EdgeDirection> {
    syntax_tokens(node.children()).find_map(|token| match token.kind {
        TokenKind::Punctuation('-') => Some(EdgeDirection::Out),
        TokenKind::Punctuation('<') => Some(EdgeDirection::In),
        TokenKind::Punctuation('~') => Some(EdgeDirection::Undirected),
        TokenKind::Identifier if token.text().eq_ignore_ascii_case("TO") => {
            Some(if kind == Some(EdgeKind::Undirected) {
                EdgeDirection::Undirected
            } else {
                EdgeDirection::Out
            })
        }
        _ => None,
    })
}

fn lower_node_type_reference(node: &SyntaxNode, source: &str) -> Option<NodeTypeReference> {
    if let Some(alias) = syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier).then(|| identifier_from_token(&token, source))
    }) {
        return Some(NodeTypeReference::Alias(alias));
    }
    Some(NodeTypeReference::Inline {
        key_labels: lower_key_label_set(node, source),
        labels: lower_label_set(node, source),
        properties: lower_property_type_list(node, source),
        span: significant_node_span(node),
    })
}

fn lower_key_label_set(node: &SyntaxNode, source: &str) -> Option<Vec<crate::Identifier>> {
    node.children()
        .iter()
        .filter_map(syntax_node)
        .find(|child| child.kind() == SyntaxKind::KeyLabelSet)
        .map(|key_set| lower_label_set(key_set, source))
}

fn lower_label_set(node: &SyntaxNode, source: &str) -> Vec<crate::Identifier> {
    let children = node.children();
    let Some(phrase) = children
        .iter()
        .filter_map(syntax_node)
        .find(|child| child.kind() == SyntaxKind::LabelSetPhrase)
    else {
        return Vec::new();
    };
    let identifiers = syntax_tokens(phrase.children())
        .filter(|token| token.kind == TokenKind::Identifier)
        .collect::<Vec<_>>();
    let skip_prefix = identifiers.first().is_some_and(|token| {
        token.text().eq_ignore_ascii_case("LABEL") || token.text().eq_ignore_ascii_case("LABELS")
    });
    identifiers
        .into_iter()
        .skip(usize::from(skip_prefix))
        .map(|token| identifier_from_token(&token, source))
        .collect()
}

fn lower_property_type_list(node: &SyntaxNode, source: &str) -> Vec<PropertyType> {
    node.children()
        .iter()
        .filter_map(syntax_node)
        .find(|child| child.kind() == SyntaxKind::PropertyTypeList)
        .map(|list| {
            list.children()
                .iter()
                .filter_map(|element| {
                    let property = syntax_node(element)?;
                    (property.kind() == SyntaxKind::PropertyType)
                        .then(|| lower_property_type(property, source))
                        .flatten()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn lower_property_type(node: &SyntaxNode, source: &str) -> Option<PropertyType> {
    let name = syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier).then(|| identifier_from_token(&token, source))
    })?;
    let value_type = node.children().iter().find_map(|element| {
        let value = syntax_node(element)?;
        (value.kind() == SyntaxKind::PropertyValueType)
            .then(|| lower_property_value_type(value, source))
            .flatten()
    })?;
    Some(PropertyType {
        name,
        value_type,
        span: significant_node_span(node),
    })
}

pub(super) fn lower_property_value_type(
    node: &SyntaxNode,
    source: &str,
) -> Option<PropertyValueType> {
    let atoms = node
        .children()
        .iter()
        .filter_map(|element| {
            let atom = syntax_node(element)?;
            (atom.kind() == SyntaxKind::ValueTypeAtom)
                .then(|| lower_value_type_atom(atom, source))
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut value_type = match atoms.as_slice() {
        [] => return None,
        [atom] => atom.clone(),
        _ => PropertyValueType {
            form: PropertyValueTypeForm::Union(atoms),
            non_null: false,
            span: significant_node_span(node),
        },
    };
    let postfix_list = syntax_tokens(node.children()).any(|token| {
        token.text().eq_ignore_ascii_case("LIST") || token.text().eq_ignore_ascii_case("ARRAY")
    });
    if postfix_list {
        value_type = PropertyValueType {
            form: PropertyValueTypeForm::List {
                element: Some(Box::new(value_type)),
                max_length: lower_type_bound(node),
            },
            non_null: has_not_null(node),
            span: significant_node_span(node),
        };
    }
    Some(value_type)
}

fn lower_value_type_atom(node: &SyntaxNode, source: &str) -> Option<PropertyValueType> {
    if let Some(reference) = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        (child.kind() == SyntaxKind::ReferenceValueType).then_some(child)
    }) {
        return lower_reference_value_type(reference, source);
    }
    let words = syntax_tokens(node.children())
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .filter(|token| !matches!(token.kind, TokenKind::Punctuation(_)))
        .map(|token| token.text().to_ascii_uppercase())
        .collect::<Vec<_>>();
    let nested = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        (child.kind() == SyntaxKind::PropertyValueType)
            .then(|| lower_property_value_type(child, source))
            .flatten()
    });
    let field_list = node.children().iter().find_map(|element| {
        let list = syntax_node(element)?;
        (list.kind() == SyntaxKind::FieldTypeList).then(|| {
            list.children()
                .iter()
                .filter_map(|field| lower_property_type(syntax_node(field)?, source))
                .collect::<Vec<_>>()
        })
    });
    if let Some(fields) = field_list {
        return Some(PropertyValueType {
            form: PropertyValueTypeForm::Record {
                open: false,
                fields,
            },
            non_null: has_not_null(node),
            span: significant_node_span(node),
        });
    }
    let first = words.first()?.as_str();
    let form = match first {
        "LIST" | "ARRAY" => PropertyValueTypeForm::List {
            element: nested.map(Box::new),
            max_length: lower_type_bound(node),
        },
        "RECORD" => PropertyValueTypeForm::Record {
            open: true,
            fields: Vec::new(),
        },
        "ANY" if words.get(1).is_some_and(|word| word == "RECORD") => {
            PropertyValueTypeForm::Record {
                open: true,
                fields: Vec::new(),
            }
        }
        "ANY" => {
            let members = nested.map(|value_type| match value_type.form {
                PropertyValueTypeForm::Union(members) => members,
                _ => vec![value_type],
            });
            PropertyValueTypeForm::DynamicUnion {
                property_values: words.iter().any(|word| word == "PROPERTY"),
                members,
            }
        }
        _ => PropertyValueTypeForm::Named {
            name: words.join(" "),
            parameters: lower_type_parameters(node),
        },
    };
    Some(PropertyValueType {
        form,
        non_null: has_not_null(node),
        span: significant_node_span(node),
    })
}

fn lower_reference_value_type(node: &SyntaxNode, source: &str) -> Option<PropertyValueType> {
    let words = syntax_tokens(node.children())
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .filter(|token| !matches!(token.kind, TokenKind::Punctuation(_)))
        .map(|token| token.text().to_ascii_uppercase())
        .collect::<Vec<_>>();
    let fields = node
        .children()
        .iter()
        .find_map(|element| {
            let list = syntax_node(element)?;
            (list.kind() == SyntaxKind::FieldTypeList).then(|| {
                list.children()
                    .iter()
                    .filter_map(|field| lower_property_type(syntax_node(field)?, source))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();
    let specification = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        match child.kind() {
            SyntaxKind::NestedGraphTypeSpecification => {
                lower_nested_graph_type(child, source).map(ClosedReferenceTypeSpecification::Graph)
            }
            SyntaxKind::NodeTypeSpecification => {
                lower_node_type(child, source).map(ClosedReferenceTypeSpecification::Node)
            }
            SyntaxKind::EdgeTypeSpecification => {
                lower_edge_type(child, source).map(ClosedReferenceTypeSpecification::Edge)
            }
            _ => None,
        }
    });
    let kind = if matches!(
        specification,
        Some(ClosedReferenceTypeSpecification::Graph(_))
    ) || words.iter().any(|word| word == "GRAPH")
    {
        ReferenceValueTypeKind::Graph
    } else if words.iter().any(|word| word == "TABLE") {
        ReferenceValueTypeKind::BindingTable
    } else if matches!(
        specification,
        Some(ClosedReferenceTypeSpecification::Node(_))
    ) || words
        .iter()
        .any(|word| matches!(word.as_str(), "NODE" | "VERTEX"))
    {
        ReferenceValueTypeKind::Node
    } else if matches!(
        specification,
        Some(ClosedReferenceTypeSpecification::Edge(_))
    ) || words
        .iter()
        .any(|word| matches!(word.as_str(), "EDGE" | "RELATIONSHIP"))
    {
        ReferenceValueTypeKind::Edge
    } else {
        return None;
    };
    Some(PropertyValueType {
        form: PropertyValueTypeForm::Reference {
            kind,
            open: words.iter().any(|word| word == "ANY"),
            property_graph: words.iter().any(|word| word == "PROPERTY"),
            specification: specification.map(Box::new),
            fields,
        },
        non_null: has_not_null(node),
        span: significant_node_span(node),
    })
}

fn lower_type_parameters(node: &SyntaxNode) -> Vec<TypeParameter> {
    node.children()
        .iter()
        .filter_map(syntax_node)
        .filter(|child| child.kind() == SyntaxKind::TypeParameterList)
        .flat_map(|parameters| {
            let tokens = syntax_tokens(parameters.children()).collect::<Vec<_>>();
            let numbers = tokens
                .iter()
                .filter(|token| token.kind == TokenKind::Number)
                .filter_map(|token| token.text().parse::<u64>().ok())
                .map(TypeParameter::Unsigned)
                .collect::<Vec<_>>();
            if !numbers.is_empty() {
                return numbers;
            }
            let words = tokens
                .iter()
                .filter(|token| token.kind == TokenKind::Identifier)
                .map(|token| token.text().to_ascii_uppercase())
                .filter(|word| word != "TO")
                .collect::<Vec<_>>();
            match words.as_slice() {
                [from, to] => vec![TypeParameter::DurationQualifier {
                    from: from.clone(),
                    to: to.clone(),
                }],
                _ => Vec::new(),
            }
        })
        .collect()
}

fn lower_type_bound(node: &SyntaxNode) -> Option<u64> {
    node.children()
        .iter()
        .filter_map(syntax_node)
        .filter(|child| child.kind() == SyntaxKind::TypeParameterList)
        .find_map(|bound| {
            syntax_tokens(bound.children())
                .find(|token| token.kind == TokenKind::Number)
                .and_then(|token| token.text().parse().ok())
        })
}

fn has_not_null(node: &SyntaxNode) -> bool {
    node.children().iter().any(|element| {
        syntax_node(element).is_some_and(|child| child.kind() == SyntaxKind::NotNullConstraint)
    })
}

pub(super) fn lower_session_statement(node: &SyntaxNode, source: &str) -> Option<SessionCommand> {
    node.children().into_iter().find_map(|element| {
        let child = syntax_node(&element)?;
        let span = significant_node_span(child);
        match child.kind() {
            SyntaxKind::SessionSetStatement => {
                let name = syntax_tokens(child.children()).find_map(|token| {
                    (token.kind == TokenKind::Identifier)
                        .then(|| identifier_from_token(&token, source))
                })?;
                Some(SessionCommand::SetSchema { name, span })
            }
            SyntaxKind::SessionResetStatement => Some(SessionCommand::ResetSchema { span }),
            SyntaxKind::SessionCloseStatement => Some(SessionCommand::Close { span }),
            _ => None,
        }
    })
}

pub(super) fn lower_procedure_statement(node: &SyntaxNode, source: &str) -> Option<ProcedureCall> {
    node.children().into_iter().find_map(|element| {
        let child = syntax_node(&element)?;
        if child.kind() != SyntaxKind::CallStatement {
            return None;
        }
        let name = child.children().iter().find_map(|element| {
            let name = syntax_node(element)?;
            (name.kind() == SyntaxKind::ProcedureName).then(|| {
                syntax_tokens(name.children())
                    .filter(|token| token.kind == TokenKind::Identifier)
                    .map(|token| identifier_from_token(&token, source))
                    .collect::<Vec<_>>()
            })
        })?;
        let arguments = child
            .children()
            .iter()
            .filter_map(|element| {
                let expression = syntax_node(element)?;
                is_expression_kind(expression.kind())
                    .then(|| lower_expression(expression, source))
                    .flatten()
            })
            .collect();
        Some(ProcedureCall {
            name,
            arguments,
            span: significant_node_span(child),
        })
    })
}

pub(super) fn lower_transaction_statement(node: &SyntaxNode) -> Option<TransactionCommand> {
    node.children().into_iter().find_map(|element| {
        let child = syntax_node(&element)?;
        match child.kind() {
            SyntaxKind::StartTransactionStatement => {
                let keywords = syntax_tokens(child.children())
                    .filter_map(|token| match token.kind {
                        TokenKind::Keyword(keyword) => Some(keyword),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let access_mode = if contains_keyword_pair(&keywords, Keyword::Read, Keyword::Only)
                {
                    Some(TransactionAccessMode::ReadOnly)
                } else if contains_keyword_pair(&keywords, Keyword::Read, Keyword::Write) {
                    Some(TransactionAccessMode::ReadWrite)
                } else {
                    None
                };
                Some(TransactionCommand::Start {
                    access_mode,
                    span: significant_node_span(child),
                })
            }
            SyntaxKind::CommitStatement => Some(TransactionCommand::Commit {
                span: significant_node_span(child),
            }),
            SyntaxKind::RollbackStatement => Some(TransactionCommand::Rollback {
                span: significant_node_span(child),
            }),
            _ => None,
        }
    })
}

pub(super) fn lower_data_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<QueryClause> {
    match node.kind() {
        SyntaxKind::InsertStatement => lower_insert(node, source, diagnostics),
        SyntaxKind::SetStatement => lower_set(node, source, diagnostics),
        SyntaxKind::RemoveStatement => Some(QueryClause::Remove {
            targets: lower_item_expressions(node, SyntaxKind::RemoveItem, source),
            span: significant_node_span(node),
        }),
        SyntaxKind::DeleteStatement => Some(QueryClause::Delete {
            targets: lower_item_expressions(node, SyntaxKind::DeleteItem, source),
            detach: syntax_tokens(node.children())
                .any(|token| token.kind == TokenKind::Keyword(Keyword::Detach)),
            span: significant_node_span(node),
        }),
        _ => None,
    }
}

fn lower_insert(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<QueryClause> {
    let patterns = lower_graph_pattern_list(node, source);
    if patterns.is_empty() {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-INSERT-MISSING-PATTERN",
            "INSERT statement is missing an insert graph pattern",
            node.span(),
        ));
        None
    } else {
        Some(QueryClause::Insert {
            patterns,
            span: significant_node_span(node),
        })
    }
}

fn lower_set(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<QueryClause> {
    let mut items = Vec::new();
    for element in node.children() {
        let Some(item) = syntax_node(&element) else {
            continue;
        };
        if item.kind() != SyntaxKind::SetItem {
            continue;
        }
        let Some(Expression::Binary {
            operator: BinaryOperator::Equals,
            left,
            right,
        }) = first_expression(item, source)
        else {
            diagnostics.push(Diagnostic::error(
                "GQL-AST-SET-ASSIGNMENT",
                "SET item must be a property assignment",
                item.span(),
            ));
            continue;
        };
        items.push(SetItem {
            target: *left,
            value: *right,
            span: significant_node_span(item),
        });
    }
    (!items.is_empty()).then(|| QueryClause::Set {
        items,
        span: significant_node_span(node),
    })
}

fn lower_graph_pattern_list(node: &SyntaxNode, source: &str) -> Vec<GraphPattern> {
    node.children()
        .iter()
        .filter_map(|element| {
            let list = syntax_node(element)?;
            (list.kind() == SyntaxKind::GraphPatternList).then_some(list)
        })
        .flat_map(SyntaxNode::children)
        .filter_map(|element| {
            let pattern = syntax_node(&element)?;
            match pattern.kind() {
                SyntaxKind::GraphPattern => lower_graph_pattern(pattern, source),
                SyntaxKind::PathPattern => {
                    lower_path_pattern(pattern, source).map(|path| GraphPattern {
                        elements: path.elements,
                        span: path.span,
                    })
                }
                _ => None,
            }
        })
        .collect()
}

fn lower_item_expressions(
    node: &SyntaxNode,
    item_kind: SyntaxKind,
    source: &str,
) -> Vec<Expression> {
    node.children()
        .iter()
        .filter_map(|element| {
            let item = syntax_node(element)?;
            (item.kind() == item_kind).then_some(item)
        })
        .filter_map(|item| first_expression(item, source))
        .collect()
}

fn first_expression(node: &SyntaxNode, source: &str) -> Option<Expression> {
    node.children().iter().find_map(|element| {
        let expression = syntax_node(element)?;
        is_expression_kind(expression.kind())
            .then(|| lower_expression(expression, source))
            .flatten()
    })
}

fn contains_keyword_pair(keywords: &[Keyword], left: Keyword, right: Keyword) -> bool {
    keywords.windows(2).any(|pair| pair == [left, right])
}
