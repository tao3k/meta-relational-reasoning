//! Semantic owner for catalog, procedure, transaction, and mutation intents.
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use gql_ast::{
    CatalogCreatePolicy as AstCreatePolicy, CatalogDropPolicy as AstDropPolicy,
    CatalogObjectName as AstCatalogObjectName, CatalogStatement,
    ClosedReferenceTypeSpecification as AstClosedReferenceTypeSpecification, Expression,
    GraphTypeSource as AstGraphTypeSource, GraphTypeSpecification as AstGraphTypeSpecification,
    PropertyValueType as AstPropertyValueType, PropertyValueTypeForm as AstValueTypeForm,
    QueryClause, ReferenceValueTypeKind as AstReferenceValueTypeKind,
    SessionCommand as AstSessionCommand, Statement,
    TransactionAccessMode as AstTransactionAccessMode, TransactionCommand as AstTransactionCommand,
};
use gql_catalog::GqlCatalog;
use gql_ir::{
    CatalogCommand, CatalogCreatePolicy as IrCreatePolicy, CatalogDropPolicy as IrDropPolicy,
    CatalogObjectName as IrCatalogObjectName,
    ClosedReferenceTypeSpecification as IrClosedReferenceTypeSpecification, DeclaredTypeParameter,
    DeclaredValueType, DeclaredValueTypeForm, EdgeDirection as IrEdgeDirection,
    EdgeTypeSpecification as IrEdgeTypeSpecification, GraphTypeSource as IrGraphTypeSource,
    GraphTypeSpecification as IrGraphTypeSpecification, Mutation as IrMutation,
    NodeTypeReference as IrNodeTypeReference, NodeTypeSpecification as IrNodeTypeSpecification,
    ProcedureCommand, PropertyType as IrPropertyType, QueryBlock,
    ReferenceValueTypeKind as IrReferenceValueTypeKind, SessionCommand as IrSessionCommand,
    TransactionAccessMode as IrTransactionAccessMode, TransactionCommand as IrTransactionCommand,
};
use gql_source::Diagnostic;
use gql_types::ValueType;

use crate::api::{Analysis, build_graph_pattern, lower_expression};
use crate::binding_analysis::register_pattern_bindings;

pub(crate) fn analyze_non_query_statement(
    statement: &Statement,
    catalog: &dyn GqlCatalog,
) -> Option<Analysis> {
    match statement {
        Statement::Catalog(statement) => Some(analyze_catalog(statement, catalog)),
        Statement::Procedure(procedure) => Some(analyze_procedure(procedure)),
        Statement::Transaction(command) => Some(analyze_transaction(command)),
        Statement::Session(command) => Some(analyze_session(command, catalog)),
        Statement::Query(_) => None,
    }
}

pub(crate) fn analyze_data_clause(
    clause: &QueryClause,
    block: &mut QueryBlock,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match clause {
        QueryClause::Insert { patterns, .. } => {
            for pattern in patterns {
                register_pattern_bindings(pattern, bindings, diagnostics);
            }
            let patterns = patterns
                .iter()
                .map(|pattern| build_graph_pattern(pattern, bindings, diagnostics))
                .collect();
            block.mutations.push(IrMutation::Insert { patterns });
        }
        QueryClause::Set { items, .. } => analyze_set(items, block, bindings, diagnostics),
        QueryClause::Remove { targets, span } => {
            for target in targets {
                if !matches!(target, Expression::PropertyAccess { .. }) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-REMOVE-TARGET-NOT-PROPERTY",
                        "REMOVE target must be a property access",
                        *span,
                    ));
                    continue;
                }
                if let Some(target) = lower_expression(target, bindings, diagnostics) {
                    block.mutations.push(IrMutation::RemoveProperty { target });
                }
            }
        }
        QueryClause::Delete {
            targets, detach, ..
        } => {
            for target in targets {
                if let Some(target) = lower_expression(target, bindings, diagnostics) {
                    block.mutations.push(IrMutation::Delete {
                        target,
                        detach: *detach,
                    });
                }
            }
        }
        _ => return false,
    }
    true
}

fn analyze_catalog(statement: &CatalogStatement, catalog: &dyn GqlCatalog) -> Analysis {
    let (catalog_command, diagnostics) =
        match statement {
            CatalogStatement::CreateSchema { name } | CatalogStatement::DropSchema { name } => {
                let create = matches!(statement, CatalogStatement::CreateSchema { .. });
                let canonical_name = name.canonical_text();
                let exists = catalog
                    .catalog()
                    .schemas
                    .iter()
                    .any(|schema| schema.name.0.to_uppercase() == canonical_name);
                match (create, exists) {
                    (true, true) => (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-SCHEMA-ALREADY-EXISTS",
                            format!("schema `{canonical_name}` already exists"),
                            name.span,
                        )],
                    ),
                    (false, false) => (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-SCHEMA-NOT-FOUND",
                            format!("schema `{canonical_name}` does not exist"),
                            name.span,
                        )],
                    ),
                    (true, false) => (
                        Some(CatalogCommand::CreateSchema {
                            name: lower_catalog_name(name),
                        }),
                        Vec::new(),
                    ),
                    (false, true) => (
                        Some(CatalogCommand::DropSchema {
                            name: lower_catalog_name(name),
                        }),
                        Vec::new(),
                    ),
                }
            }
            CatalogStatement::CreateGraph {
                name,
                graph_type,
                policy,
            } => {
                let canonical_name = name.canonical_text();
                let exists = catalog
                    .catalog()
                    .graphs
                    .iter()
                    .any(|graph| graph.name.0.to_uppercase() == canonical_name);
                if exists && matches!(policy, AstCreatePolicy::Error) {
                    (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-GRAPH-ALREADY-EXISTS",
                            format!("graph `{canonical_name}` already exists"),
                            name.span,
                        )],
                    )
                } else {
                    let AstGraphTypeSpecification::Any { typed, .. } = graph_type;
                    (
                        Some(CatalogCommand::CreateGraph {
                            name: lower_catalog_name(name),
                            graph_type: IrGraphTypeSpecification::Any { typed: *typed },
                            policy: lower_create_policy(*policy),
                        }),
                        Vec::new(),
                    )
                }
            }
            CatalogStatement::DropGraph { name, policy } => {
                let canonical_name = name.canonical_text();
                let exists = catalog
                    .catalog()
                    .graphs
                    .iter()
                    .any(|graph| graph.name.0.to_uppercase() == canonical_name);
                if exists || matches!(policy, AstDropPolicy::IfExists) {
                    (
                        Some(CatalogCommand::DropGraph {
                            name: lower_catalog_name(name),
                            policy: lower_drop_policy(*policy),
                        }),
                        Vec::new(),
                    )
                } else {
                    (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-GRAPH-NOT-FOUND",
                            format!("graph `{canonical_name}` does not exist"),
                            name.span,
                        )],
                    )
                }
            }
            CatalogStatement::CreateGraphType {
                name,
                source,
                policy,
            } => {
                let canonical_name = name.canonical_text();
                let exists = catalog
                    .catalog()
                    .graph_types
                    .iter()
                    .any(|graph_type| graph_type.name.0.to_uppercase() == canonical_name);
                if exists && matches!(policy, AstCreatePolicy::Error) {
                    (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-GRAPH-TYPE-ALREADY-EXISTS",
                            format!("graph type `{canonical_name}` already exists"),
                            name.span,
                        )],
                    )
                } else {
                    let (source, source_exists, source_span, source_diagnostics) =
                        match source {
                            AstGraphTypeSource::CopyOf { graph_type, span } => {
                                let canonical = graph_type.canonical_text();
                                (
                                    IrGraphTypeSource::CopyOf {
                                        graph_type: lower_catalog_name(graph_type),
                                    },
                                    catalog.catalog().graph_types.iter().any(|candidate| {
                                        candidate.name.0.to_uppercase() == canonical
                                    }),
                                    *span,
                                    Vec::new(),
                                )
                            }
                            AstGraphTypeSource::LikeGraph { graph, span } => {
                                let canonical = graph.canonical_text();
                                (
                                    IrGraphTypeSource::LikeGraph {
                                        graph: lower_catalog_name(graph),
                                    },
                                    catalog.catalog().graphs.iter().any(|candidate| {
                                        candidate.name.0.to_uppercase() == canonical
                                    }),
                                    *span,
                                    Vec::new(),
                                )
                            }
                            AstGraphTypeSource::Nested {
                                specification,
                                span,
                            } => {
                                let (node_types, edge_types, diagnostics) =
                                    lower_nested_graph_type(specification);
                                (
                                    IrGraphTypeSource::Nested {
                                        node_types,
                                        edge_types,
                                    },
                                    diagnostics.is_empty(),
                                    *span,
                                    diagnostics,
                                )
                            }
                        };
                    if !source_diagnostics.is_empty() {
                        (None, source_diagnostics)
                    } else if source_exists {
                        (
                            Some(CatalogCommand::CreateGraphType {
                                name: lower_catalog_name(name),
                                source,
                                policy: lower_create_policy(*policy),
                            }),
                            Vec::new(),
                        )
                    } else {
                        (
                            None,
                            vec![Diagnostic::error(
                                "GQL-SEMA-GRAPH-TYPE-SOURCE-NOT-FOUND",
                                "graph type source does not exist",
                                source_span,
                            )],
                        )
                    }
                }
            }
            CatalogStatement::DropGraphType { name, policy } => {
                let canonical_name = name.canonical_text();
                let exists = catalog
                    .catalog()
                    .graph_types
                    .iter()
                    .any(|graph_type| graph_type.name.0.to_uppercase() == canonical_name);
                if exists || matches!(policy, AstDropPolicy::IfExists) {
                    (
                        Some(CatalogCommand::DropGraphType {
                            name: lower_catalog_name(name),
                            policy: lower_drop_policy(*policy),
                        }),
                        Vec::new(),
                    )
                } else {
                    (
                        None,
                        vec![Diagnostic::error(
                            "GQL-SEMA-GRAPH-TYPE-NOT-FOUND",
                            format!("graph type `{canonical_name}` does not exist"),
                            name.span,
                        )],
                    )
                }
            }
        };
    Analysis {
        catalog_command,
        diagnostics,
        ..Analysis::default()
    }
}

fn lower_catalog_name(name: &AstCatalogObjectName) -> IrCatalogObjectName {
    IrCatalogObjectName {
        parts: name
            .parts
            .iter()
            .map(gql_ast::Identifier::canonical_text)
            .collect(),
    }
}

fn lower_nested_graph_type(
    specification: &gql_ast::NestedGraphTypeSpecification,
) -> (
    Vec<IrNodeTypeSpecification>,
    Vec<IrEdgeTypeSpecification>,
    Vec<Diagnostic>,
) {
    let mut diagnostics = Vec::new();
    let mut aliases = HashSet::new();
    let node_types = specification
        .node_types
        .iter()
        .map(|node_type| {
            let alias = node_type
                .alias
                .as_ref()
                .map(gql_ast::Identifier::canonical_text);
            if let Some(alias) = &alias
                && !aliases.insert(alias.clone())
            {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-DUPLICATE-GRAPH-TYPE-ALIAS",
                    format!("graph type alias `{alias}` is declared more than once"),
                    node_type
                        .alias
                        .as_ref()
                        .expect("duplicate alias has an AST owner")
                        .span,
                ));
            }
            IrNodeTypeSpecification {
                name: node_type
                    .name
                    .as_ref()
                    .map(gql_ast::Identifier::canonical_text),
                alias,
                key_labels: node_type
                    .key_labels
                    .as_deref()
                    .map(|labels| lower_graph_type_labels(labels, &mut diagnostics)),
                labels: lower_graph_type_labels(&node_type.labels, &mut diagnostics),
                properties: lower_graph_type_properties(&node_type.properties, &mut diagnostics),
            }
        })
        .collect::<Vec<_>>();
    let edge_types = specification
        .edge_types
        .iter()
        .filter_map(|edge_type| {
            let kind_matches_direction = match edge_type.kind {
                None => true,
                Some(gql_ast::EdgeKind::Directed) => {
                    edge_type.direction != gql_ast::EdgeDirection::Undirected
                }
                Some(gql_ast::EdgeKind::Undirected) => {
                    edge_type.direction == gql_ast::EdgeDirection::Undirected
                }
            };
            if !kind_matches_direction {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-GRAPH-TYPE-EDGE-KIND-MISMATCH",
                    "edge kind does not match its endpoint connector",
                    edge_type.span,
                ));
                return None;
            }
            let endpoints = edge_type
                .endpoints
                .iter()
                .filter_map(|endpoint| {
                    lower_node_type_reference(endpoint, &aliases, &mut diagnostics)
                })
                .collect::<Vec<_>>();
            if endpoints.len() != 2 {
                return None;
            }
            let (source, destination, direction) = match edge_type.direction {
                gql_ast::EdgeDirection::Out => (
                    endpoints[0].clone(),
                    endpoints[1].clone(),
                    IrEdgeDirection::Out,
                ),
                gql_ast::EdgeDirection::In => (
                    endpoints[1].clone(),
                    endpoints[0].clone(),
                    IrEdgeDirection::In,
                ),
                gql_ast::EdgeDirection::Undirected => (
                    endpoints[0].clone(),
                    endpoints[1].clone(),
                    IrEdgeDirection::Undirected,
                ),
            };
            Some(IrEdgeTypeSpecification {
                name: edge_type
                    .name
                    .as_ref()
                    .map(gql_ast::Identifier::canonical_text),
                source,
                destination,
                direction,
                key_labels: edge_type
                    .key_labels
                    .as_deref()
                    .map(|labels| lower_graph_type_labels(labels, &mut diagnostics)),
                labels: lower_graph_type_labels(&edge_type.labels, &mut diagnostics),
                properties: lower_graph_type_properties(&edge_type.properties, &mut diagnostics),
            })
        })
        .collect();
    (node_types, edge_types, diagnostics)
}

fn lower_node_type_reference(
    endpoint: &gql_ast::NodeTypeReference,
    aliases: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<IrNodeTypeReference> {
    match endpoint {
        gql_ast::NodeTypeReference::Alias(alias) => {
            let canonical = alias.canonical_text();
            if !aliases.contains(&canonical) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-GRAPH-TYPE-ENDPOINT-NOT-FOUND",
                    format!("edge endpoint alias {canonical} is not declared"),
                    alias.span,
                ));
                return None;
            }
            Some(IrNodeTypeReference::Alias(canonical))
        }
        gql_ast::NodeTypeReference::Inline {
            key_labels,
            labels,
            properties,
            ..
        } => Some(IrNodeTypeReference::Inline {
            key_labels: key_labels
                .as_deref()
                .map(|labels| lower_graph_type_labels(labels, diagnostics)),
            labels: lower_graph_type_labels(labels, diagnostics),
            properties: lower_graph_type_properties(properties, diagnostics),
        }),
    }
}

fn lower_graph_type_labels(
    labels: &[gql_ast::Identifier],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let mut canonical_labels = HashSet::new();
    labels
        .iter()
        .filter_map(|label| {
            let canonical = label.canonical_text();
            if !canonical_labels.insert(canonical.clone()) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-DUPLICATE-GRAPH-TYPE-LABEL",
                    format!("graph type label `{canonical}` is declared more than once"),
                    label.span,
                ));
                return None;
            }
            Some(canonical)
        })
        .collect()
}

fn lower_graph_type_properties(
    properties: &[gql_ast::PropertyType],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<IrPropertyType> {
    let mut property_names = HashSet::new();
    properties
        .iter()
        .filter_map(|property| {
            let name = property.name.canonical_text();
            if !property_names.insert(name.clone()) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-DUPLICATE-GRAPH-TYPE-PROPERTY",
                    format!("graph type property `{name}` is declared more than once"),
                    property.name.span,
                ));
                return None;
            }
            Some(IrPropertyType {
                name,
                value_type: lower_declared_value_type(&property.value_type, diagnostics),
            })
        })
        .collect()
}

pub(crate) fn lower_declared_value_type(
    value_type: &AstPropertyValueType,
    diagnostics: &mut Vec<Diagnostic>,
) -> DeclaredValueType {
    let form = match &value_type.form {
        AstValueTypeForm::Named { name, parameters } => {
            if !declared_type_parameters_are_valid(name, parameters) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-VALUE-TYPE-ARITY",
                    format!("predefined type `{name}` does not admit the declared parameter shape"),
                    value_type.span,
                ));
            }
            if matches!(name.as_str(), "STRING" | "BYTES")
                && matches!(
                    parameters.as_slice(),
                    [gql_ast::TypeParameter::Unsigned(min), gql_ast::TypeParameter::Unsigned(max)]
                        if min > max
                )
            {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-VALUE-TYPE-LENGTH-RANGE",
                    "value-type minimum length cannot exceed its maximum length",
                    value_type.span,
                ));
            }
            DeclaredValueTypeForm::Named {
                name: canonical_declared_type_name(name),
                parameters: parameters
                    .iter()
                    .map(|parameter| match parameter {
                        gql_ast::TypeParameter::Unsigned(value) => {
                            DeclaredTypeParameter::Unsigned(*value)
                        }
                        gql_ast::TypeParameter::DurationQualifier { from, to } => {
                            DeclaredTypeParameter::DurationQualifier {
                                from: from.to_ascii_uppercase(),
                                to: to.to_ascii_uppercase(),
                            }
                        }
                    })
                    .collect(),
            }
        }
        AstValueTypeForm::List {
            element,
            max_length,
        } => DeclaredValueTypeForm::List {
            element: element
                .as_deref()
                .map(|element| Box::new(lower_declared_value_type(element, diagnostics))),
            max_length: *max_length,
        },
        AstValueTypeForm::Record { open, fields } => DeclaredValueTypeForm::Record {
            open: *open,
            fields: lower_graph_type_properties(fields, diagnostics),
        },
        AstValueTypeForm::DynamicUnion {
            property_values,
            members,
        } => DeclaredValueTypeForm::DynamicUnion {
            property_values: *property_values,
            members: members
                .as_ref()
                .map(|members| lower_union_members(members, diagnostics)),
        },
        AstValueTypeForm::Reference {
            kind,
            open,
            property_graph,
            specification,
            fields,
        } => DeclaredValueTypeForm::Reference {
            kind: match kind {
                AstReferenceValueTypeKind::Graph => IrReferenceValueTypeKind::Graph,
                AstReferenceValueTypeKind::BindingTable => IrReferenceValueTypeKind::BindingTable,
                AstReferenceValueTypeKind::Node => IrReferenceValueTypeKind::Node,
                AstReferenceValueTypeKind::Edge => IrReferenceValueTypeKind::Edge,
            },
            open: *open,
            property_graph: *property_graph,
            specification: specification
                .as_deref()
                .and_then(|specification| lower_closed_reference_type(specification, diagnostics))
                .map(Box::new),
            fields: lower_graph_type_properties(fields, diagnostics),
        },
        AstValueTypeForm::Union(members) => {
            DeclaredValueTypeForm::Union(lower_union_members(members, diagnostics))
        }
    };
    DeclaredValueType {
        form,
        non_null: value_type.non_null,
    }
}

fn lower_closed_reference_type(
    specification: &AstClosedReferenceTypeSpecification,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<IrClosedReferenceTypeSpecification> {
    let nested = match specification {
        AstClosedReferenceTypeSpecification::Graph(specification) => specification.clone(),
        AstClosedReferenceTypeSpecification::Node(node_type) => {
            gql_ast::NestedGraphTypeSpecification {
                node_types: vec![node_type.clone()],
                edge_types: Vec::new(),
                span: node_type.span,
            }
        }
        AstClosedReferenceTypeSpecification::Edge(edge_type) => {
            gql_ast::NestedGraphTypeSpecification {
                node_types: Vec::new(),
                edge_types: vec![edge_type.clone()],
                span: edge_type.span,
            }
        }
    };
    let (mut node_types, mut edge_types, nested_diagnostics) = lower_nested_graph_type(&nested);
    diagnostics.extend(nested_diagnostics);
    match specification {
        AstClosedReferenceTypeSpecification::Graph(_) => {
            Some(IrClosedReferenceTypeSpecification::Graph {
                node_types,
                edge_types,
            })
        }
        AstClosedReferenceTypeSpecification::Node(_) => node_types
            .pop()
            .map(IrClosedReferenceTypeSpecification::Node),
        AstClosedReferenceTypeSpecification::Edge(_) => edge_types
            .pop()
            .map(IrClosedReferenceTypeSpecification::Edge),
    }
}

fn lower_union_members(
    members: &[AstPropertyValueType],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<DeclaredValueType> {
    let mut lowered = Vec::with_capacity(members.len());
    for member in members {
        let canonical = lower_declared_value_type(member, diagnostics);
        if lowered.contains(&canonical) {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-DUPLICATE-VALUE-TYPE-UNION-MEMBER",
                "closed dynamic union members must be unique",
                member.span,
            ));
        }
        lowered.push(canonical);
    }
    lowered
}

fn canonical_declared_type_name(name: &str) -> String {
    match name.to_ascii_uppercase().as_str() {
        "BOOL" => "BOOLEAN".into(),
        "INTEGER8" | "SIGNED INTEGER8" => "INT8".into(),
        "INTEGER16" | "SIGNED INTEGER16" => "INT16".into(),
        "INTEGER32" | "SIGNED INTEGER32" => "INT32".into(),
        "INTEGER64" | "SIGNED INTEGER64" => "INT64".into(),
        "INTEGER128" | "SIGNED INTEGER128" => "INT128".into(),
        "INTEGER256" | "SIGNED INTEGER256" => "INT256".into(),
        "SMALL INTEGER" | "SIGNED SMALL INTEGER" => "SMALLINT".into(),
        "INTEGER" | "SIGNED INTEGER" => "INT".into(),
        "BIG INTEGER" | "SIGNED BIG INTEGER" => "BIGINT".into(),
        "UNSIGNED INTEGER8" => "UINT8".into(),
        "UNSIGNED INTEGER16" => "UINT16".into(),
        "UNSIGNED INTEGER32" => "UINT32".into(),
        "UNSIGNED INTEGER64" => "UINT64".into(),
        "UNSIGNED INTEGER128" => "UINT128".into(),
        "UNSIGNED INTEGER256" => "UINT256".into(),
        "UNSIGNED SMALL INTEGER" => "USMALLINT".into(),
        "UNSIGNED INTEGER" => "UINT".into(),
        "UNSIGNED BIG INTEGER" => "UBIGINT".into(),
        "DEC" => "DECIMAL".into(),
        "TIMESTAMP WITH TIME ZONE" => "ZONED DATETIME".into(),
        "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" => "LOCAL DATETIME".into(),
        "TIME WITH TIME ZONE" => "ZONED TIME".into(),
        "TIME WITHOUT TIME ZONE" => "LOCAL TIME".into(),
        other => other.to_string(),
    }
}

fn declared_type_parameters_are_valid(name: &str, parameters: &[gql_ast::TypeParameter]) -> bool {
    use gql_ast::TypeParameter::{DurationQualifier, Unsigned};

    let numeric = parameters
        .iter()
        .all(|parameter| matches!(parameter, Unsigned(_)));
    match name.to_ascii_uppercase().as_str() {
        "STRING" | "BYTES" => numeric && parameters.len() <= 2,
        "CHAR" | "VARCHAR" | "BINARY" | "VARBINARY" | "INT" | "UINT" | "INTEGER"
        | "SIGNED INTEGER" | "UNSIGNED INTEGER" => numeric && parameters.len() <= 1,
        "DECIMAL" | "DEC" | "FLOAT" => numeric && parameters.len() <= 2,
        "DURATION" => matches!(parameters, [DurationQualifier { .. }]),
        _ => parameters.is_empty(),
    }
}

fn lower_create_policy(policy: AstCreatePolicy) -> IrCreatePolicy {
    match policy {
        AstCreatePolicy::Error => IrCreatePolicy::Error,
        AstCreatePolicy::IfNotExists => IrCreatePolicy::IfNotExists,
        AstCreatePolicy::OrReplace => IrCreatePolicy::OrReplace,
    }
}

fn lower_drop_policy(policy: AstDropPolicy) -> IrDropPolicy {
    match policy {
        AstDropPolicy::Error => IrDropPolicy::Error,
        AstDropPolicy::IfExists => IrDropPolicy::IfExists,
    }
}

fn analyze_session(command: &AstSessionCommand, catalog: &dyn GqlCatalog) -> Analysis {
    let (session_command, diagnostics) = match command {
        AstSessionCommand::SetSchema { name, .. } => {
            let canonical_name = name.canonical_text();
            let exists = catalog
                .catalog()
                .schemas
                .iter()
                .any(|schema| schema.name.0.to_uppercase() == canonical_name);
            if exists {
                (
                    Some(IrSessionCommand::SetSchema {
                        name: canonical_name,
                    }),
                    Vec::new(),
                )
            } else {
                (
                    None,
                    vec![Diagnostic::error(
                        "GQL-SEMA-SESSION-SCHEMA-NOT-FOUND",
                        format!("session schema `{}` does not exist", name.text),
                        name.span,
                    )],
                )
            }
        }
        AstSessionCommand::ResetSchema { .. } => (Some(IrSessionCommand::ResetSchema), Vec::new()),
        AstSessionCommand::Close { .. } => (Some(IrSessionCommand::Close), Vec::new()),
    };
    Analysis {
        session_command,
        diagnostics,
        ..Analysis::default()
    }
}

fn analyze_procedure(procedure: &gql_ast::ProcedureCall) -> Analysis {
    let mut diagnostics = Vec::new();
    let bindings = HashMap::new();
    let arguments = procedure
        .arguments
        .iter()
        .filter_map(|argument| lower_expression(argument, &bindings, &mut diagnostics))
        .collect();
    Analysis {
        procedure_command: diagnostics.is_empty().then(|| ProcedureCommand {
            name: procedure
                .name
                .iter()
                .map(gql_ast::Identifier::canonical_text)
                .collect::<Vec<_>>()
                .join("."),
            arguments,
        }),
        diagnostics,
        ..Analysis::default()
    }
}

fn analyze_transaction(command: &AstTransactionCommand) -> Analysis {
    let transaction_command = match command {
        AstTransactionCommand::Start { access_mode, .. } => IrTransactionCommand::Start {
            access_mode: access_mode.map(|mode| match mode {
                AstTransactionAccessMode::ReadOnly => IrTransactionAccessMode::ReadOnly,
                AstTransactionAccessMode::ReadWrite => IrTransactionAccessMode::ReadWrite,
            }),
        },
        AstTransactionCommand::Commit { .. } => IrTransactionCommand::Commit,
        AstTransactionCommand::Rollback { .. } => IrTransactionCommand::Rollback,
    };
    Analysis {
        transaction_command: Some(transaction_command),
        ..Analysis::default()
    }
}

fn analyze_set(
    items: &[gql_ast::SetItem],
    block: &mut QueryBlock,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        if !matches!(item.target, Expression::PropertyAccess { .. }) {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-SET-TARGET-NOT-PROPERTY",
                "SET assignment target must be a property access",
                item.span,
            ));
            continue;
        }
        let Some(target) = lower_expression(&item.target, bindings, diagnostics) else {
            continue;
        };
        let Some(value) = lower_expression(&item.value, bindings, diagnostics) else {
            continue;
        };
        block
            .mutations
            .push(IrMutation::SetProperty { target, value });
    }
}
