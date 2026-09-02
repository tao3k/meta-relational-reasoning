use crate::Compiler;
use crate::ast::{
    CatalogStatement, GraphTypeSource as AstGraphTypeSource,
    NodeTypeReference as AstNodeTypeReference, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    CatalogCommand, DeclaredValueTypeForm, GraphTypeSource as IrGraphTypeSource,
    NodeTypeReference as IrNodeTypeReference,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("edge-type-specification-contract".into()),
        Vec::new(),
        Vec::new(),
    )
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

fn ir_endpoint_alias(endpoint: &IrNodeTypeReference) -> Option<&str> {
    let IrNodeTypeReference::Alias(alias) = endpoint else {
        return None;
    };
    Some(alias)
}

#[test]
fn edge_type_phrases_synonyms_and_explicit_kinds_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person), NODE TYPE Company (company), DIRECTED RELATIONSHIP TYPE WorksAt {since INT64} CONNECTING (person TO company), DIRECTED EDGE Employs {since INT64} CONNECTING (company <- person), UNDIRECTED RELATIONSHIP Partners {since INT64} CONNECTING (company ~ person), DIRECTED RELATIONSHIP Collaborates (person)-[{}]->(company), (person)-[{}]->(company) }";
    let result = Compiler.compile("edge-type-phrase.gql", source, &empty_catalog());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "the complete edge-type specification family must be admitted: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    for kind in [
        SyntaxKind::EdgeKind,
        SyntaxKind::EndpointPair,
        SyntaxKind::EdgeTypeSpecification,
        SyntaxKind::NodeTypeReference,
        SyntaxKind::EdgeDirection,
    ] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }
    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("edge-type phrases must lower through the catalog AST");
    };
    assert_eq!(specification.edge_types.len(), 5);
    assert_eq!(
        specification
            .edge_types
            .iter()
            .map(|edge| edge.name.as_ref().map(|name| name.text.as_str()))
            .collect::<Vec<_>>(),
        [
            Some("WorksAt"),
            Some("Employs"),
            Some("Partners"),
            Some("Collaborates"),
            None,
        ]
    );
    assert_eq!(
        specification
            .edge_types
            .iter()
            .map(|edge| edge.kind)
            .collect::<Vec<_>>(),
        [
            Some(crate::ast::EdgeKind::Directed),
            Some(crate::ast::EdgeKind::Directed),
            Some(crate::ast::EdgeKind::Undirected),
            Some(crate::ast::EdgeKind::Directed),
            None,
        ]
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
            crate::ast::EdgeDirection::Out,
            crate::ast::EdgeDirection::Out,
        ]
    );
    assert!(matches!(
        result.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::Nested { ref edge_types, .. },
            ..
        }) if edge_types.len() == 5
            && edge_types[0].name.as_deref() == Some("WORKSAT")
            && ir_endpoint_alias(&edge_types[0].source) == Some("PERSON")
            && ir_endpoint_alias(&edge_types[0].destination) == Some("COMPANY")
            && edge_types[0].direction == crate::ir::EdgeDirection::Out
            && edge_types[0].properties[0].name == "SINCE"
            && ir_endpoint_alias(&edge_types[1].source) == Some("PERSON")
            && ir_endpoint_alias(&edge_types[1].destination) == Some("COMPANY")
            && edge_types[1].direction == crate::ir::EdgeDirection::In
            && edge_types[2].direction == crate::ir::EdgeDirection::Undirected
            && edge_types[4].name.is_none()
            && edge_types[4].direction == crate::ir::EdgeDirection::Out
    ));

    let mismatched_kind = Compiler.compile(
        "edge-type-kind-mismatch.gql",
        "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person), NODE TYPE Company (company), UNDIRECTED EDGE Bad (person)-[{}]->(company) }",
        &empty_catalog(),
    );
    assert_eq!(
        mismatched_kind
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-GRAPH-TYPE-EDGE-KIND-MISMATCH"]
    );
    assert!(mismatched_kind.analysis.catalog_command.is_none());
    assert!(mismatched_kind.analysis.ir.is_none());
}

#[test]
fn inline_and_empty_edge_endpoint_references_remain_typed() {
    let source =
        "CREATE GRAPH TYPE analytics.inline_type AS { EDGE TYPE Transfer ({id INT64})-[{}]->() }";
    let result = Compiler.compile("inline-edge-endpoints.gql", source, &empty_catalog());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "inline endpoint fillers must be admitted: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(contains_node_kind(
        &result.parse.tree.root(),
        SyntaxKind::NodeTypeReference
    ));
    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("inline endpoint references must lower to the catalog AST");
    };
    let AstNodeTypeReference::Inline {
        properties: source_properties,
        ..
    } = &specification.edge_types[0].endpoints[0]
    else {
        panic!("source endpoint must remain an inline node type");
    };
    assert_eq!(source_properties[0].name.text, "id");
    assert_eq!(
        &source[source_properties[0].span.start as usize..source_properties[0].span.end as usize],
        "id INT64"
    );
    assert!(matches!(
        specification.edge_types[0].endpoints[1],
        AstNodeTypeReference::Inline {
            ref properties,
            ..
        } if properties.is_empty()
    ));
    assert!(matches!(
        result.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::Nested { ref edge_types, .. },
            ..
        }) if edge_types.len() == 1
            && matches!(
                edge_types[0].source,
                IrNodeTypeReference::Inline { ref properties, .. }
                    if properties.len() == 1
                        && properties[0].name == "ID"
                        && matches!(properties[0].value_type.form,
                            DeclaredValueTypeForm::Named { ref name, .. } if name == "INT64")
            )
            && matches!(
                edge_types[0].destination,
                IrNodeTypeReference::Inline { ref properties, .. } if properties.is_empty()
            )
    ));

    let duplicate = Compiler.compile(
        "inline-edge-endpoint-duplicate-property.gql",
        "CREATE GRAPH TYPE analytics.inline_type AS { EDGE TYPE Transfer ({id INT64, id STRING})-[{}]->() }",
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
fn label_set_key_label_and_filler_only_types_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.labels AS { NODE TYPE Person LABEL Person {id INT64} AS person, NODE TYPE LABEL PersonKey IMPLIES LABELS Person & Actor {id INT64} AS keyed_person, VERTEX TYPE PersonViaIs IS Person AS is_person, VERTEX TYPE PersonViaColon :Person & Actor AS colon_person, NODE TYPE PatternNode (pattern_node LABEL PatternLabel {id INT64}), DIRECTED EDGE TYPE WorksAt LABEL Employment {since INT64} CONNECTING (person TO keyed_person), DIRECTED RELATIONSHIP TYPE LABELS EmploymentKey & Temporal => LABELS Employment & Audited {since INT64} CONNECTING (keyed_person TO person), DIRECTED EDGE TYPE IMPLIES CONNECTING (person TO keyed_person), DIRECTED EDGE TYPE PatternEdge (pattern_node)-[LABEL PatternRel {since INT64}]->(person), DIRECTED EDGE TYPE InlineLabels (LABEL Endpoint {id INT64})-[{}]->(IMPLIES LABEL Empty) }";
    let result = Compiler.compile("label-set-key-label.gql", source, &empty_catalog());

    assert!(
        result.analysis.diagnostics.is_empty(),
        "official label-set and key-label fillers must cross the frontend: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    for kind in [SyntaxKind::KeyLabelSet, SyntaxKind::LabelSetPhrase] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }

    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("label-bearing type phrases must lower to the catalog AST");
    };
    assert_eq!(specification.node_types.len(), 5);
    assert_eq!(specification.edge_types.len(), 5);
    assert_eq!(
        specification.node_types[0]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Person"]
    );
    assert!(specification.node_types[0].key_labels.is_none());
    assert_eq!(
        specification.node_types[1]
            .key_labels
            .as_ref()
            .expect("filler-only key labels")
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["PersonKey"]
    );
    assert_eq!(
        specification.node_types[1]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Person", "Actor"]
    );
    assert_eq!(
        specification.node_types[1].name.as_ref(),
        None,
        "a filler-only phrase must not manufacture a type name"
    );
    assert_eq!(
        specification.node_types[1]
            .alias
            .as_ref()
            .map(|alias| alias.text.as_str()),
        Some("keyed_person")
    );
    let actor = &specification.node_types[1].labels[1];
    assert_eq!(
        &source[actor.span.start as usize..actor.span.end as usize],
        "Actor"
    );
    assert_eq!(
        specification.edge_types[1]
            .key_labels
            .as_ref()
            .expect("symbolic implication must retain key labels")
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["EmploymentKey", "Temporal"]
    );
    assert_eq!(
        specification.edge_types[1]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Employment", "Audited"]
    );
    assert_eq!(
        specification.node_types[2]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Person"]
    );
    assert_eq!(
        specification.node_types[3]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Person", "Actor"]
    );
    assert_eq!(
        specification.node_types[4]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["PatternLabel"]
    );
    assert!(
        specification.edge_types[2]
            .key_labels
            .as_ref()
            .is_some_and(Vec::is_empty)
    );
    assert_eq!(
        specification.edge_types[3]
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["PatternRel"]
    );
    let AstNodeTypeReference::Inline {
        labels: inline_source_labels,
        ..
    } = &specification.edge_types[4].endpoints[0]
    else {
        panic!("label-bearing source endpoint must remain inline");
    };
    assert_eq!(inline_source_labels[0].text, "Endpoint");
    let AstNodeTypeReference::Inline {
        key_labels: inline_destination_keys,
        labels: inline_destination_labels,
        ..
    } = &specification.edge_types[4].endpoints[1]
    else {
        panic!("key-label-bearing destination endpoint must remain inline");
    };
    assert!(inline_destination_keys.as_ref().is_some_and(Vec::is_empty));
    assert_eq!(inline_destination_labels[0].text, "Empty");

    assert!(matches!(
        result.analysis.catalog_command,
        Some(CatalogCommand::CreateGraphType {
            source: IrGraphTypeSource::Nested {
                ref node_types,
                ref edge_types,
            },
            ..
        }) if node_types[0].labels == ["PERSON"]
            && node_types[1].name.is_none()
            && node_types[1].alias.as_deref() == Some("KEYED_PERSON")
            && node_types[1].key_labels.as_ref().is_some_and(|labels|
                labels.iter().map(String::as_str).eq(["PERSONKEY"])
            )
            && node_types[1].labels == ["PERSON", "ACTOR"]
            && edge_types[0].labels == ["EMPLOYMENT"]
            && edge_types[1].name.is_none()
            && edge_types[1].key_labels.as_ref().is_some_and(|labels|
                labels.iter().map(String::as_str).eq(["EMPLOYMENTKEY", "TEMPORAL"])
            )
            && edge_types[1].labels == ["EMPLOYMENT", "AUDITED"]
            && edge_types[1].properties[0].name == "SINCE"
            && edge_types[2].key_labels.as_ref().is_some_and(Vec::is_empty)
            && edge_types[3].labels == ["PATTERNREL"]
            && matches!(
                edge_types[4].source,
                IrNodeTypeReference::Inline { ref labels, .. }
                    if labels.iter().map(String::as_str).eq(["ENDPOINT"])
            )
            && matches!(
                edge_types[4].destination,
                IrNodeTypeReference::Inline {
                    ref key_labels,
                    ref labels,
                    ..
                } if key_labels.as_ref().is_some_and(Vec::is_empty)
                    && labels.iter().map(String::as_str).eq(["EMPTY"])
            )
    ));

    for (file, invalid_source) in [
        (
            "duplicate-label-set.gql",
            "CREATE GRAPH TYPE analytics.labels AS { NODE TYPE Person LABELS Person & Person AS person }",
        ),
        (
            "duplicate-key-label-set.gql",
            "CREATE GRAPH TYPE analytics.labels AS { NODE TYPE Person (person), DIRECTED EDGE TYPE LABELS Key & Key IMPLIES CONNECTING (person TO person) }",
        ),
    ] {
        let invalid = Compiler.compile(file, invalid_source, &empty_catalog());
        assert_eq!(
            invalid
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["GQL-SEMA-DUPLICATE-GRAPH-TYPE-LABEL"]
        );
        assert!(invalid.analysis.catalog_command.is_none());
        assert!(invalid.analysis.ir.is_none());
    }
}
