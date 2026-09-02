use crate::Compiler;
use crate::ast::{
    CatalogCreatePolicy as AstCreatePolicy, CatalogDropPolicy as AstDropPolicy, CatalogStatement,
    Expression as AstExpression, GraphTypeSource as AstGraphTypeSource,
    GraphTypeSpecification as AstGraphType, NodeTypeReference as AstNodeTypeReference,
    PatternElement as AstPatternElement, QueryClause, SessionCommand as AstSessionCommand,
    Statement, TransactionAccessMode as AstTransactionAccessMode,
    TransactionCommand as AstTransactionCommand,
};
use crate::catalog::{
    Catalog, CatalogName, Graph, GraphName, GraphType, GraphTypeName, Schema, SchemaName,
};
use crate::ir::{
    CatalogCommand, CatalogCreatePolicy as IrCreatePolicy, CatalogDropPolicy as IrDropPolicy,
    CatalogObjectName as IrCatalogObjectName, DeclaredValueTypeForm, Expression as IrExpression,
    GraphPatternElement as IrPatternElement, GraphTypeSource as IrGraphTypeSource,
    GraphTypeSpecification as IrGraphType, Mutation, NodeTypeReference as IrNodeTypeReference,
    ProcedureCommand, SessionCommand as IrSessionCommand,
    TransactionAccessMode as IrTransactionAccessMode, TransactionCommand as IrTransactionCommand,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("data-management-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

fn ir_catalog_name(parts: &[&str]) -> IrCatalogObjectName {
    IrCatalogObjectName {
        parts: parts.iter().map(|part| (*part).to_owned()).collect(),
    }
}

fn catalog_for(source: &str) -> Catalog {
    if source.starts_with("DROP SCHEMA") {
        Catalog::new(
            CatalogName("data-management-contract".into()),
            Vec::new(),
            vec![Schema::new(
                SchemaName("ANALYTICS".into()),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )],
        )
    } else if source.starts_with("DROP GRAPH") {
        Catalog::new(
            CatalogName("data-management-contract".into()),
            vec![Graph::new(GraphName("social".into()), None, None)],
            Vec::new(),
        )
    } else {
        empty_catalog()
    }
}

fn contains_node_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected
        || node
            .children()
            .into_iter()
            .any(|element| match element.kind {
                SyntaxElementKind::Node(child) => contains_node_kind(&child, expected),
                SyntaxElementKind::Token(_) => false,
            })
}

fn ast_endpoint_alias(endpoint: &AstNodeTypeReference) -> &str {
    let AstNodeTypeReference::Alias(alias) = endpoint else {
        panic!("expected an alias endpoint");
    };
    &alias.text
}

fn ir_endpoint_alias(endpoint: &IrNodeTypeReference) -> Option<&str> {
    let IrNodeTypeReference::Alias(alias) = endpoint else {
        return None;
    };
    Some(alias)
}

#[test]
fn qualified_catalog_paths_and_nested_graph_types_cross_frontend_admission() {
    let qualified_graph = "CREATE GRAPH analytics.social ANY";
    let graph = Compiler.compile(
        "qualified-create-graph.gql",
        qualified_graph,
        &empty_catalog(),
    );
    assert!(
        graph.analysis.diagnostics.is_empty(),
        "qualified graph identity must be admitted: {:?}",
        graph.analysis.diagnostics
    );
    assert_eq!(
        graph.parse.tree.rowan_root().text().to_string(),
        qualified_graph
    );
    assert!(graph.statement.is_some());
    assert!(graph.analysis.catalog_command.is_some());

    let Some(Statement::Catalog(CatalogStatement::CreateGraph { name, .. })) = &graph.statement
    else {
        panic!("qualified graph must lower to catalog AST");
    };
    assert_eq!(
        name.parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        ["analytics", "social"]
    );
    assert!(matches!(
        graph.analysis.catalog_command,
        Some(CatalogCommand::CreateGraph { ref name, .. })
            if name == &ir_catalog_name(&["ANALYTICS", "SOCIAL"])
    ));

    let nested = "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person {name STRING, age INT64}) }";
    let graph_type = Compiler.compile("nested-graph-type.gql", nested, &empty_catalog());
    assert!(
        graph_type.analysis.diagnostics.is_empty(),
        "nested graph type must be admitted: {:?}",
        graph_type.analysis.diagnostics
    );
    assert_eq!(
        graph_type.parse.tree.rowan_root().text().to_string(),
        nested
    );
    for kind in [
        SyntaxKind::CatalogObjectName,
        SyntaxKind::NestedGraphTypeSpecification,
        SyntaxKind::NodeTypeSpecification,
        SyntaxKind::PropertyTypeList,
        SyntaxKind::PropertyType,
        SyntaxKind::PropertyValueType,
    ] {
        assert!(contains_node_kind(&graph_type.parse.tree.root(), kind));
    }
    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        name,
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &graph_type.statement
    else {
        panic!("nested graph type must lower to catalog AST");
    };
    assert_eq!(
        name.parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        ["analytics", "social_type"]
    );
    let node = &specification.node_types[0];
    assert_eq!(
        node.name.as_ref().map(|name| name.text.as_str()),
        Some("Person")
    );
    assert_eq!(
        node.alias.as_ref().map(|name| name.text.as_str()),
        Some("person")
    );
    assert_eq!(
        node.properties
            .iter()
            .map(|property| property.name.text.as_str())
            .collect::<Vec<_>>(),
        ["name", "age"]
    );
    for property in &node.properties {
        assert_eq!(
            &nested[property.span.start as usize..property.span.end as usize],
            if property.name.text == "name" {
                "name STRING"
            } else {
                "age INT64"
            }
        );
    }
    assert!(matches!(
        graph_type.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            ref name,
            source: IrGraphTypeSource::Nested { ref node_types, .. },
            ..
        }) if name == &ir_catalog_name(&["ANALYTICS", "SOCIAL_TYPE"])
            && node_types[0].name.as_deref() == Some("PERSON")
            && node_types[0].alias.as_deref() == Some("PERSON")
            && node_types[0].properties[0].name == "NAME"
            && matches!(node_types[0].properties[0].value_type.form,
                DeclaredValueTypeForm::Named { ref name, .. } if name == "STRING")
            && node_types[0].properties[1].name == "AGE"
            && matches!(node_types[0].properties[1].value_type.form,
                DeclaredValueTypeForm::Named { ref name, .. } if name == "INT64")
    ));

    let malformed = Compiler.compile(
        "malformed-nested-graph-type.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { (person {name STRING,}) }",
        &empty_catalog(),
    );
    assert_eq!(
        malformed
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-NESTED-GRAPH-TYPE-SYNTAX"]
    );
    assert!(malformed.analysis.catalog_command.is_none());
    assert!(malformed.analysis.ir.is_none());

    let duplicate = Compiler.compile(
        "duplicate-nested-graph-type-property.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person {name STRING, name INT64}) }",
        &empty_catalog(),
    );
    assert_eq!(
        duplicate
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-GRAPH-TYPE-PROPERTY"]
    );
    assert!(duplicate.analysis.catalog_command.is_none());
    assert!(duplicate.analysis.ir.is_none());
}

#[test]
fn nested_edge_types_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person {name STRING}), NODE TYPE Company (company {name STRING}), EDGE TYPE WorksAt (person)-[{since INT64}]->(company), EDGE TYPE Employs (company)<-[{since INT64}]-(person), EDGE TYPE Partners (company)~[{since INT64}]~(person) }";
    let result = Compiler.compile("nested-edge-type.gql", source, &empty_catalog());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "nested edge type must be admitted: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    for kind in [
        SyntaxKind::EdgeTypeSpecification,
        SyntaxKind::NodeTypeReference,
        SyntaxKind::EdgeDirection,
        SyntaxKind::PropertyTypeList,
    ] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }
    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("nested edge types must lower to catalog AST");
    };
    assert_eq!(specification.edge_types.len(), 3);
    assert_eq!(
        specification
            .edge_types
            .iter()
            .map(|edge| { edge.name.as_ref().expect("named edge type").text.as_str() })
            .collect::<Vec<_>>(),
        ["WorksAt", "Employs", "Partners"]
    );
    assert_eq!(
        specification.edge_types[0]
            .endpoints
            .iter()
            .map(ast_endpoint_alias)
            .collect::<Vec<_>>(),
        ["person", "company"]
    );
    assert_eq!(
        specification.edge_types[1]
            .endpoints
            .iter()
            .map(ast_endpoint_alias)
            .collect::<Vec<_>>(),
        ["company", "person"]
    );
    assert_eq!(
        specification
            .edge_types
            .iter()
            .map(|edge| edge.direction)
            .collect::<Vec<_>>(),
        [
            crate::ast::EdgeDirection::Out,
            crate::ast::EdgeDirection::In,
            crate::ast::EdgeDirection::Undirected,
        ]
    );
    assert_eq!(
        &source[specification.edge_types[0].properties[0].span.start as usize
            ..specification.edge_types[0].properties[0].span.end as usize],
        "since INT64"
    );
    assert!(matches!(
        result.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::Nested { ref edge_types, .. },
            ..
        }) if edge_types.len() == 3
            && edge_types[0].name.as_deref() == Some("WORKSAT")
            && ir_endpoint_alias(&edge_types[0].source) == Some("PERSON")
            && ir_endpoint_alias(&edge_types[0].destination) == Some("COMPANY")
            && edge_types[0].direction == crate::ir::EdgeDirection::Out
            && edge_types[0].properties[0].name == "SINCE"
            && matches!(edge_types[0].properties[0].value_type.form,
                DeclaredValueTypeForm::Named { ref name, .. } if name == "INT64")
            && ir_endpoint_alias(&edge_types[1].source) == Some("PERSON")
            && ir_endpoint_alias(&edge_types[1].destination) == Some("COMPANY")
            && edge_types[1].direction == crate::ir::EdgeDirection::In
            && ir_endpoint_alias(&edge_types[2].source) == Some("COMPANY")
            && ir_endpoint_alias(&edge_types[2].destination) == Some("PERSON")
            && edge_types[2].direction == crate::ir::EdgeDirection::Undirected
    ));

    let missing_endpoint = Compiler.compile(
        "nested-edge-type-missing-endpoint.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person), EDGE TYPE Knows (person)-[{}]->(missing) }",
        &empty_catalog(),
    );
    assert_eq!(
        missing_endpoint
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-GRAPH-TYPE-ENDPOINT-NOT-FOUND"]
    );
    assert!(missing_endpoint.analysis.catalog_command.is_none());
    assert!(missing_endpoint.analysis.ir.is_none());

    let duplicate_property = Compiler.compile(
        "nested-edge-type-duplicate-property.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person), EDGE TYPE Knows (person)-[{since INT64, since STRING}]->(person) }",
        &empty_catalog(),
    );
    assert_eq!(
        duplicate_property
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-GRAPH-TYPE-PROPERTY"]
    );
    assert!(duplicate_property.analysis.catalog_command.is_none());
    assert!(duplicate_property.analysis.ir.is_none());

    let duplicate_alias = Compiler.compile(
        "nested-edge-type-duplicate-alias.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (entity), NODE TYPE Company (entity), EDGE TYPE Knows (entity)-[{}]->(entity) }",
        &empty_catalog(),
    );
    assert_eq!(
        duplicate_alias
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-GRAPH-TYPE-ALIAS"]
    );
    assert!(duplicate_alias.analysis.catalog_command.is_none());
    assert!(duplicate_alias.analysis.ir.is_none());

    let malformed_arc = Compiler.compile(
        "nested-edge-type-malformed-arc.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person), EDGE TYPE Knows (person)-[{}]-(person) }",
        &empty_catalog(),
    );
    assert_eq!(
        malformed_arc
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-NESTED-GRAPH-TYPE-SYNTAX"]
    );
    assert!(malformed_arc.statement.is_none());
    assert!(malformed_arc.analysis.catalog_command.is_none());
    assert!(malformed_arc.analysis.ir.is_none());
}

#[test]
fn insert_preserves_cst_ast_properties_and_canonical_mutation_order() {
    let source = "INSERT (:Person {name: 'Ada'})";
    let result = Compiler.compile("insert.gql", source, &empty_catalog());
    assert!(result.analysis.diagnostics.is_empty());
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    for kind in [
        SyntaxKind::InsertStatement,
        SyntaxKind::GraphPattern,
        SyntaxKind::PropertyMap,
        SyntaxKind::PropertyEntry,
    ] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("INSERT must lower to a data-modifying query statement");
    };
    let QueryClause::Insert { patterns, .. } = &query.clauses[0] else {
        panic!("INSERT clause was not preserved in AST order");
    };
    let AstPatternElement::Node(node) = &patterns[0].elements[0] else {
        panic!("INSERT must preserve its node pattern");
    };
    assert_eq!(node.labels[0].text, "Person");
    assert_eq!(node.properties[0].key.text, "name");
    let AstExpression::String(literal) = &node.properties[0].value else {
        panic!("INSERT property value must remain a string AST expression");
    };
    assert_eq!(literal.value, "Ada");
    assert_eq!(
        &source[literal.span.start as usize..literal.span.end as usize],
        "'Ada'"
    );

    let ir = result.analysis.ir.expect("canonical mutation IR");
    let Mutation::Insert { patterns } = &ir.mutations[0] else {
        panic!("first canonical mutation must be INSERT");
    };
    let IrPatternElement::Node(node) = &patterns[0].elements[0] else {
        panic!("canonical INSERT must retain its node");
    };
    assert_eq!(node.labels, ["PERSON"]);
    assert_eq!(node.properties[0].key, "NAME");
    assert_eq!(node.properties[0].value, IrExpression::String("Ada".into()));
}

#[test]
fn match_mutations_preserve_source_order_and_exact_targets() {
    let source = "MATCH (n:Person) SET n.age = 42 REMOVE n.legacy DELETE n";
    let result = Compiler.compile("mutation-pipeline.gql", source, &empty_catalog());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("mutation pipeline must lower to Query AST");
    };
    assert!(matches!(query.clauses[0], QueryClause::Match(_)));
    assert!(matches!(query.clauses[1], QueryClause::Set { .. }));
    assert!(matches!(query.clauses[2], QueryClause::Remove { .. }));
    assert!(matches!(query.clauses[3], QueryClause::Delete { .. }));

    let mutations = &result.analysis.ir.expect("mutation IR").mutations;
    assert_eq!(mutations.len(), 3);
    assert!(matches!(mutations[0], Mutation::SetProperty { .. }));
    assert!(matches!(mutations[1], Mutation::RemoveProperty { .. }));
    assert!(matches!(
        mutations[2],
        Mutation::Delete { detach: false, .. }
    ));
}

#[test]
fn catalog_procedure_and_transaction_intents_are_backend_neutral() {
    let drop_source = "DROP SCHEMA analytics";
    let drop = Compiler.compile("drop-schema.gql", drop_source, &catalog_for(drop_source));
    let Some(Statement::Catalog(CatalogStatement::DropSchema { name })) = &drop.statement else {
        panic!("DROP SCHEMA must lower to catalog AST");
    };
    assert_eq!(
        &drop_source[name.span.start as usize..name.span.end as usize],
        "analytics"
    );
    assert_eq!(
        drop.analysis.catalog_command,
        Some(CatalogCommand::DropSchema {
            name: ir_catalog_name(&["ANALYTICS"])
        })
    );

    let call = Compiler.compile("call.gql", "CALL db.refresh('now')", &empty_catalog());
    let Some(Statement::Procedure(procedure)) = &call.statement else {
        panic!("CALL must lower to procedure AST");
    };
    assert_eq!(
        procedure
            .name
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        ["db", "refresh"]
    );
    assert_eq!(procedure.arguments.len(), 1);
    assert_eq!(
        call.analysis.procedure_command,
        Some(ProcedureCommand {
            name: "DB.REFRESH".into(),
            arguments: vec![IrExpression::String("now".into())],
        })
    );

    let transaction = Compiler.compile(
        "transaction.gql",
        "START TRANSACTION READ WRITE",
        &empty_catalog(),
    );
    assert!(matches!(
        transaction.statement,
        Some(Statement::Transaction(AstTransactionCommand::Start {
            access_mode: Some(AstTransactionAccessMode::ReadWrite),
            ..
        }))
    ));
    assert_eq!(
        transaction.analysis.transaction_command,
        Some(IrTransactionCommand::Start {
            access_mode: Some(IrTransactionAccessMode::ReadWrite),
        })
    );
    for (source, ast, ir) in [
        (
            "COMMIT",
            AstTransactionCommand::Commit {
                span: crate::source::Span::new(0, 6),
            },
            IrTransactionCommand::Commit,
        ),
        (
            "ROLLBACK",
            AstTransactionCommand::Rollback {
                span: crate::source::Span::new(0, 8),
            },
            IrTransactionCommand::Rollback,
        ),
    ] {
        let result = Compiler.compile("transaction-terminal.gql", source, &empty_catalog());
        assert_eq!(result.statement, Some(Statement::Transaction(ast)));
        assert_eq!(result.analysis.transaction_command, Some(ir));
    }
}

#[test]
fn dropping_an_absent_schema_is_one_typed_terminal_without_command() {
    let result = Compiler.compile("drop-missing.gql", "DROP SCHEMA absent", &empty_catalog());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-SCHEMA-NOT-FOUND"]
    );
    assert!(result.analysis.catalog_command.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn malformed_data_management_statements_fail_once_without_ast_or_ir() {
    for (name, source, expected) in [
        (
            "insert-missing-pattern.gql",
            "INSERT",
            "GQL-PARSE-INSERT-SYNTAX",
        ),
        (
            "set-missing-value.gql",
            "MATCH (n) SET n.age =",
            "GQL-AST-SET-ASSIGNMENT",
        ),
        (
            "drop-schema-missing-name.gql",
            "DROP SCHEMA",
            "GQL-PARSE-DROP-SCHEMA-SYNTAX",
        ),
        (
            "call-missing-close.gql",
            "CALL refresh(",
            "GQL-PARSE-FUNCTION-CALL-SYNTAX",
        ),
        (
            "transaction-invalid-mode.gql",
            "START TRANSACTION WRITE READ",
            "GQL-PARSE-TRANSACTION-SYNTAX",
        ),
        (
            "create-graph-missing-type.gql",
            "CREATE GRAPH social",
            "GQL-PARSE-CREATE-GRAPH-SYNTAX",
        ),
        (
            "drop-graph-missing-name.gql",
            "DROP GRAPH",
            "GQL-PARSE-DROP-GRAPH-SYNTAX",
        ),
        (
            "session-set-missing-schema.gql",
            "SESSION SET SCHEMA",
            "GQL-PARSE-SESSION-COMMAND-SYNTAX",
        ),
        (
            "session-reset-unsupported-setting.gql",
            "SESSION RESET GRAPH",
            "GQL-PARSE-SESSION-COMMAND-SYNTAX",
        ),
        (
            "create-graph-invalid-conflict.gql",
            "CREATE GRAPH IF EXISTS social ANY",
            "GQL-PARSE-CREATE-GRAPH-SYNTAX",
        ),
        (
            "create-graph-type-copy-missing-of.gql",
            "CREATE GRAPH TYPE social_type COPY base_type",
            "GQL-PARSE-CREATE-GRAPH-TYPE-SYNTAX",
        ),
        (
            "create-graph-type-like-missing-source.gql",
            "CREATE GRAPH TYPE social_type LIKE",
            "GQL-PARSE-CREATE-GRAPH-TYPE-SYNTAX",
        ),
        (
            "drop-graph-type-missing-name.gql",
            "DROP GRAPH TYPE",
            "GQL-PARSE-DROP-GRAPH-TYPE-SYNTAX",
        ),
    ] {
        let result = Compiler.compile(name, source, &empty_catalog());
        assert_eq!(
            result.analysis.diagnostics.len(),
            1,
            "{source}: {:?}",
            result.analysis.diagnostics
        );
        assert_eq!(result.analysis.diagnostics[0].code, expected);
        assert!(result.statement.is_none(), "{source} published an AST");
        assert!(result.analysis.ir.is_none(), "{source} published query IR");
        assert!(
            result.analysis.catalog_command.is_none(),
            "{source} published a catalog command"
        );
        assert!(result.analysis.procedure_command.is_none());
        assert!(result.analysis.transaction_command.is_none());
        assert!(result.analysis.session_command.is_none());
    }
}

#[test]
fn catalog_and_session_administration_crosses_parser_admission() {
    let create_source = "CREATE GRAPH social ANY";
    let create = Compiler.compile("create-graph.gql", create_source, &empty_catalog());
    assert!(create.analysis.diagnostics.is_empty());
    assert_eq!(
        create.parse.tree.rowan_root().text().to_string(),
        create_source
    );
    assert!(contains_node_kind(
        &create.parse.tree.root(),
        SyntaxKind::CreateGraphStatement
    ));
    let Some(Statement::Catalog(CatalogStatement::CreateGraph {
        name,
        graph_type,
        policy,
    })) = &create.statement
    else {
        panic!("CREATE GRAPH must lower to catalog AST");
    };
    assert_eq!(
        &create_source[name.span.start as usize..name.span.end as usize],
        "social"
    );
    assert!(matches!(graph_type, AstGraphType::Any { typed: false, .. }));
    assert_eq!(*policy, AstCreatePolicy::Error);
    assert_eq!(
        create.analysis.catalog_command,
        Some(CatalogCommand::CreateGraph {
            name: ir_catalog_name(&["SOCIAL"]),
            graph_type: IrGraphType::Any { typed: false },
            policy: IrCreatePolicy::Error,
        })
    );

    let drop_source = "DROP GRAPH social";
    let drop = Compiler.compile("drop-graph.gql", drop_source, &catalog_for(drop_source));
    assert!(drop.analysis.diagnostics.is_empty());
    assert_eq!(
        drop.analysis.catalog_command,
        Some(CatalogCommand::DropGraph {
            name: ir_catalog_name(&["SOCIAL"]),
            policy: IrDropPolicy::Error,
        })
    );

    let schema_catalog = catalog_for("DROP SCHEMA analytics");
    let set_source = "SESSION SET SCHEMA analytics";
    let set = Compiler.compile("session-set-schema.gql", set_source, &schema_catalog);
    let Some(Statement::Session(AstSessionCommand::SetSchema { name, .. })) = &set.statement else {
        panic!("SESSION SET SCHEMA must lower to session AST");
    };
    assert_eq!(
        &set_source[name.span.start as usize..name.span.end as usize],
        "analytics"
    );
    assert_eq!(
        set.analysis.session_command,
        Some(IrSessionCommand::SetSchema {
            name: "ANALYTICS".into()
        })
    );

    for (source, ast, ir) in [
        (
            "SESSION RESET SCHEMA",
            AstSessionCommand::ResetSchema {
                span: crate::source::Span::new(0, 20),
            },
            IrSessionCommand::ResetSchema,
        ),
        (
            "SESSION CLOSE",
            AstSessionCommand::Close {
                span: crate::source::Span::new(0, 13),
            },
            IrSessionCommand::Close,
        ),
    ] {
        let result = Compiler.compile("session-terminal.gql", source, &empty_catalog());
        assert_eq!(result.statement, Some(Statement::Session(ast)));
        assert_eq!(result.analysis.session_command, Some(ir));
    }
}

#[test]
fn catalog_and_session_identity_fail_closed_without_active_intent() {
    for (name, source, catalog, expected) in [
        (
            "create-existing-graph.gql",
            "CREATE GRAPH social ANY",
            catalog_for("DROP GRAPH social"),
            "GQL-SEMA-GRAPH-ALREADY-EXISTS",
        ),
        (
            "drop-missing-graph.gql",
            "DROP GRAPH absent",
            empty_catalog(),
            "GQL-SEMA-GRAPH-NOT-FOUND",
        ),
        (
            "session-missing-schema.gql",
            "SESSION SET SCHEMA absent",
            empty_catalog(),
            "GQL-SEMA-SESSION-SCHEMA-NOT-FOUND",
        ),
    ] {
        let result = Compiler.compile(name, source, &catalog);
        assert_eq!(
            result
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected]
        );
        assert!(result.analysis.catalog_command.is_none());
        assert!(result.analysis.session_command.is_none());
        assert!(result.analysis.ir.is_none());
    }
}

#[test]
fn conditional_graph_and_graph_type_ddl_crosses_parser_admission() {
    let source = "CREATE GRAPH IF NOT EXISTS social ANY";
    let create = Compiler.compile("create-graph-if-not-exists.gql", source, &empty_catalog());
    assert!(create.analysis.diagnostics.is_empty());
    assert!(contains_node_kind(
        &create.parse.tree.root(),
        SyntaxKind::CatalogConflictClause
    ));
    let Some(Statement::Catalog(CatalogStatement::CreateGraph { policy, .. })) = &create.statement
    else {
        panic!("conditional CREATE GRAPH must lower to catalog AST");
    };
    assert_eq!(*policy, AstCreatePolicy::IfNotExists);
    assert!(matches!(
        create.analysis.catalog_command,
        Some(CatalogCommand::CreateGraph {
            policy: IrCreatePolicy::IfNotExists,
            ..
        })
    ));

    let existing_graph = catalog_for("DROP GRAPH social");
    let replace = Compiler.compile(
        "replace-graph.gql",
        "CREATE OR REPLACE GRAPH social ANY",
        &existing_graph,
    );
    assert!(replace.analysis.diagnostics.is_empty());
    assert!(matches!(
        replace.analysis.catalog_command,
        Some(CatalogCommand::CreateGraph {
            policy: IrCreatePolicy::OrReplace,
            ..
        })
    ));

    let drop = Compiler.compile(
        "drop-graph-if-exists.gql",
        "DROP GRAPH IF EXISTS absent",
        &empty_catalog(),
    );
    let Some(Statement::Catalog(CatalogStatement::DropGraph { policy, .. })) = &drop.statement
    else {
        panic!("conditional DROP GRAPH must lower to catalog AST");
    };
    assert_eq!(*policy, AstDropPolicy::IfExists);
    assert!(matches!(
        drop.analysis.catalog_command,
        Some(CatalogCommand::DropGraph {
            policy: IrDropPolicy::IfExists,
            ..
        })
    ));

    let source_catalog = Catalog::new(
        CatalogName("graph-type-source".into()),
        vec![Graph::new(GraphName("social".into()), None, None)],
        Vec::new(),
    )
    .with_graph_types(vec![GraphType::new(GraphTypeName("base_type".into()))]);
    let copy_source = "CREATE GRAPH TYPE social_type AS COPY OF base_type";
    let copy = Compiler.compile("create-graph-type-copy.gql", copy_source, &source_catalog);
    assert!(copy.analysis.diagnostics.is_empty());
    for kind in [
        SyntaxKind::CreateGraphTypeStatement,
        SyntaxKind::GraphTypeSource,
    ] {
        assert!(contains_node_kind(&copy.parse.tree.root(), kind));
    }
    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        name,
        source: AstGraphTypeSource::CopyOf { graph_type, .. },
        ..
    })) = &copy.statement
    else {
        panic!("COPY OF graph type must lower to catalog AST");
    };
    assert_eq!(
        &copy_source[name.span.start as usize..name.span.end as usize],
        "social_type"
    );
    assert_eq!(
        &copy_source[graph_type.span.start as usize..graph_type.span.end as usize],
        "base_type"
    );
    assert!(matches!(
        copy.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::CopyOf { ref graph_type },
            ..
        }) if graph_type == &ir_catalog_name(&["BASE_TYPE"])
    ));

    let like = Compiler.compile(
        "create-graph-type-like.gql",
        "CREATE GRAPH TYPE social_type LIKE social",
        &source_catalog,
    );
    assert!(matches!(
        like.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::LikeGraph { ref graph },
            ..
        }) if graph == &ir_catalog_name(&["SOCIAL"])
    ));

    let graph_type_catalog = Catalog::new(
        CatalogName("graph-type-drop".into()),
        Vec::new(),
        Vec::new(),
    )
    .with_graph_types(vec![GraphType::new(GraphTypeName("social_type".into()))]);
    let drop_type = Compiler.compile(
        "drop-graph-type.gql",
        "DROP GRAPH TYPE social_type",
        &graph_type_catalog,
    );
    assert_eq!(
        drop_type.analysis.catalog_command,
        Some(CatalogCommand::DropGraphType {
            name: ir_catalog_name(&["SOCIAL_TYPE"]),
            policy: IrDropPolicy::Error,
        })
    );

    let conditional_type = Compiler.compile(
        "create-graph-type-if-not-exists.gql",
        "CREATE GRAPH TYPE IF NOT EXISTS social_type AS COPY OF base_type",
        &graph_type_catalog.clone().with_graph_types(vec![
            GraphType::new(GraphTypeName("social_type".into())),
            GraphType::new(GraphTypeName("base_type".into())),
        ]),
    );
    assert!(conditional_type.analysis.diagnostics.is_empty());
    assert!(matches!(
        conditional_type.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            policy: IrCreatePolicy::IfNotExists,
            ..
        })
    ));

    let conditional_drop = Compiler.compile(
        "drop-graph-type-if-exists.gql",
        "DROP GRAPH TYPE IF EXISTS absent",
        &empty_catalog(),
    );
    assert!(matches!(
        conditional_drop.analysis.catalog_command,
        Some(CatalogCommand::DropGraphType {
            policy: IrDropPolicy::IfExists,
            ..
        })
    ));
}

#[test]
fn graph_type_catalog_identities_fail_closed_without_catalog_intent() {
    let sources = Catalog::new(
        CatalogName("graph-type-identities".into()),
        Vec::new(),
        Vec::new(),
    )
    .with_graph_types(vec![GraphType::new(GraphTypeName("base_type".into()))]);
    for (name, source, catalog, expected) in [
        (
            "missing-graph-type-source.gql",
            "CREATE GRAPH TYPE social_type COPY OF absent",
            empty_catalog(),
            "GQL-SEMA-GRAPH-TYPE-SOURCE-NOT-FOUND",
        ),
        (
            "existing-graph-type.gql",
            "CREATE GRAPH TYPE base_type COPY OF base_type",
            sources.clone(),
            "GQL-SEMA-GRAPH-TYPE-ALREADY-EXISTS",
        ),
        (
            "missing-drop-graph-type.gql",
            "DROP GRAPH TYPE absent",
            empty_catalog(),
            "GQL-SEMA-GRAPH-TYPE-NOT-FOUND",
        ),
    ] {
        let result = Compiler.compile(name, source, &catalog);
        assert_eq!(
            result
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected]
        );
        assert!(result.analysis.catalog_command.is_none());
        assert!(result.analysis.ir.is_none());
    }
}
