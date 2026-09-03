use crate::Compiler;
use crate::ast::{
    CatalogStatement, ClosedReferenceTypeSpecification as AstClosedReferenceTypeSpecification,
    GraphTypeSource as AstGraphTypeSource, PropertyValueTypeForm,
    ReferenceValueTypeKind as AstReferenceValueTypeKind, Statement, TypeParameter,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    CatalogCommand, ClosedReferenceTypeSpecification as IrClosedReferenceTypeSpecification,
    DeclaredTypeParameter, DeclaredValueTypeForm, GraphTypeSource as IrGraphTypeSource,
    ReferenceValueTypeKind as IrReferenceValueTypeKind,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("value-type-specification-contract".into()),
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

#[test]
fn scalar_constructed_and_union_value_types_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.types AS { NODE TYPE Sample { active BOOLEAN NOT NULL, name STRING(1, 80), code CHAR(8), payload BYTES(0, 4096), `count` INT(32), unsigned UINT64, amount DECIMAL(10, 2), ratio DOUBLE PRECISION, observed ZONED DATETIME, lifetime DURATION(YEAR TO MONTH), bounded LIST<STRING NOT NULL>[16] NOT NULL, postfix INT64 ARRAY[8], open LIST[32], profile RECORD { display STRING, rank TYPED UINT32 NOT NULL }, scalar ANY VALUE NOT NULL, dynamic_property ANY PROPERTY VALUE, choice ANY VALUE<STRING | INT64>, shorthand STRING | INT64, route PATH NOT NULL, absent NULL, impossible NOTHING } }";

    let result = Compiler.compile("value-type-lattice.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "the official scalar, constructed, union and nullability lattice must parse losslessly: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "the declaration type lattice must reach backend-neutral analysis: {:?}",
        result.analysis.diagnostics
    );
    assert!(result.statement.is_some());
    assert!(result.analysis.catalog_command.is_some());
    for kind in [
        SyntaxKind::PropertyValueType,
        SyntaxKind::ValueTypeAtom,
        SyntaxKind::TypeParameterList,
        SyntaxKind::FieldTypeList,
        SyntaxKind::FieldType,
        SyntaxKind::NotNullConstraint,
    ] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }

    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("value types must remain attached to the graph-type AST");
    };
    let properties = &specification.node_types[0].properties;
    assert_eq!(properties.len(), 21, "no declared property may be dropped");
    assert!(matches!(
        properties[0].value_type.form,
        PropertyValueTypeForm::Named { ref name, ref parameters }
            if name == "BOOLEAN" && parameters.is_empty() && properties[0].value_type.non_null
    ));
    assert!(matches!(
        properties[1].value_type.form,
        PropertyValueTypeForm::Named { ref name, ref parameters }
            if name == "STRING"
                && parameters == &[TypeParameter::Unsigned(1), TypeParameter::Unsigned(80)]
    ));
    assert!(matches!(
        properties[9].value_type.form,
        PropertyValueTypeForm::Named { ref name, ref parameters }
            if name == "DURATION"
                && parameters == &[TypeParameter::DurationQualifier {
                    from: "YEAR".into(),
                    to: "MONTH".into(),
                }]
    ));
    assert!(matches!(
        properties[10].value_type.form,
        PropertyValueTypeForm::List {
            element: Some(ref element),
            max_length: Some(16),
        } if properties[10].value_type.non_null
            && element.non_null
            && matches!(element.form, PropertyValueTypeForm::Named { ref name, .. } if name == "STRING")
    ));
    assert!(matches!(
        properties[11].value_type.form,
        PropertyValueTypeForm::List {
            element: Some(ref element),
            max_length: Some(8),
        } if matches!(element.form, PropertyValueTypeForm::Named { ref name, .. } if name == "INT64")
    ));
    assert!(matches!(
        properties[12].value_type.form,
        PropertyValueTypeForm::List {
            element: None,
            max_length: Some(32),
        }
    ));
    assert!(matches!(
        properties[13].value_type.form,
        PropertyValueTypeForm::Record { ref fields, .. }
            if fields.len() == 2
                && fields[0].name.text == "display"
                && fields[1].name.text == "rank"
                && fields[1].value_type.non_null
    ));
    assert!(matches!(
        properties[16].value_type.form,
        PropertyValueTypeForm::DynamicUnion {
            property_values: false,
            members: Some(ref members),
        } if members.len() == 2
    ));
    assert!(matches!(
        properties[17].value_type.form,
        PropertyValueTypeForm::Union(ref members) if members.len() == 2
    ));
    let rank = match &properties[13].value_type.form {
        PropertyValueTypeForm::Record { fields, .. } => &fields[1],
        _ => unreachable!(),
    };
    assert_eq!(
        &source[rank.span.start as usize..rank.span.end as usize],
        "rank TYPED UINT32 NOT NULL"
    );

    let Some(CatalogCommand::CreateGraphType {
        source: IrGraphTypeSource::Nested { node_types, .. },
        ..
    }) = &result.analysis.catalog_command
    else {
        panic!("value types must reach one backend-neutral catalog command");
    };
    let canonical = &node_types[0].properties;
    assert_eq!(canonical.len(), 21);
    assert!(matches!(
        canonical[9].value_type.form,
        DeclaredValueTypeForm::Named { ref name, ref parameters }
            if name == "DURATION"
                && parameters == &[DeclaredTypeParameter::DurationQualifier {
                    from: "YEAR".into(),
                    to: "MONTH".into(),
                }]
    ));
    assert!(matches!(
        canonical[13].value_type.form,
        DeclaredValueTypeForm::Record { ref fields, .. }
            if fields.len() == 2 && fields[1].name == "RANK" && fields[1].value_type.non_null
    ));
    assert!(matches!(
        canonical[16].value_type.form,
        DeclaredValueTypeForm::DynamicUnion { members: Some(ref members), .. }
            if members.len() == 2
    ));
}

#[test]
fn invalid_value_type_bounds_and_duplicate_union_members_emit_no_command() {
    let source = "CREATE GRAPH TYPE analytics.invalid_types AS { NODE TYPE Broken { reversed STRING(80, 1), duplicate ANY VALUE<STRING | STRING> } }";
    let result = Compiler.compile("invalid-value-type-lattice.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        [
            "GQL-SEMA-VALUE-TYPE-LENGTH-RANGE",
            "GQL-SEMA-DUPLICATE-VALUE-TYPE-UNION-MEMBER",
        ]
    );
    assert!(result.analysis.catalog_command.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn reference_and_binding_table_value_types_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.references AS { NODE TYPE Sample { graph_ref ANY PROPERTY GRAPH NOT NULL, table_ref TABLE { key STRING, ordinal UINT64 }, node_ref ANY NODE, edge_ref ANY EDGE, path_ref PATH } }";
    let result = Compiler.compile("reference-value-types.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "official open reference and binding-table value types must parse losslessly: {:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.diagnostics.is_empty());
    assert!(result.analysis.catalog_command.is_some());
    assert!(contains_node_kind(
        &result.parse.tree.root(),
        SyntaxKind::ReferenceValueType
    ));

    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("reference types must remain attached to the graph-type AST");
    };
    let properties = &specification.node_types[0].properties;
    assert_eq!(properties.len(), 5);
    assert!(matches!(
        properties[0].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Graph,
            open: true,
            property_graph: true,
            specification: None,
            ref fields,
        } if fields.is_empty() && properties[0].value_type.non_null
    ));
    assert!(matches!(
        properties[1].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::BindingTable,
            open: false,
            ref fields,
            ..
        } if fields.len() == 2 && fields[0].name.text == "key" && fields[1].name.text == "ordinal"
    ));
    assert!(matches!(
        properties[2].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Node,
            open: true,
            ..
        }
    ));
    assert!(matches!(
        properties[3].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Edge,
            open: true,
            ..
        }
    ));

    let Some(CatalogCommand::CreateGraphType {
        source: IrGraphTypeSource::Nested { node_types, .. },
        ..
    }) = &result.analysis.catalog_command
    else {
        panic!("reference types must reach one backend-neutral catalog command");
    };
    let canonical = &node_types[0].properties;
    assert_eq!(canonical.len(), 5);
    assert!(matches!(
        canonical[0].value_type.form,
        DeclaredValueTypeForm::Reference {
            kind: IrReferenceValueTypeKind::Graph,
            open: true,
            property_graph: true,
            ..
        }
    ));
    assert!(matches!(
        canonical[1].value_type.form,
        DeclaredValueTypeForm::Reference {
            kind: IrReferenceValueTypeKind::BindingTable,
            ref fields,
            ..
        } if fields.len() == 2 && fields[0].name == "KEY" && fields[1].name == "ORDINAL"
    ));
}

#[test]
fn closed_graph_node_and_edge_reference_value_types_cross_frontend_admission() {
    let source = "CREATE GRAPH TYPE analytics.closed_references AS { NODE TYPE Sample { graph_ref PROPERTY GRAPH { NODE TYPE Person (person {name STRING}) }, node_ref NODE TYPE Person {id INT64}, edge_ref EDGE TYPE Knows ({id INT64})-[{since INT64}]->({id INT64}) } }";
    let result = Compiler.compile("closed-reference-value-types.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "official closed graph, node and edge reference types must parse losslessly: {:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.diagnostics.is_empty());
    assert!(result.analysis.catalog_command.is_some());
    for kind in [
        SyntaxKind::ReferenceValueType,
        SyntaxKind::NestedGraphTypeSpecification,
        SyntaxKind::NodeTypeSpecification,
        SyntaxKind::EdgeTypeSpecification,
    ] {
        assert!(contains_node_kind(&result.parse.tree.root(), kind));
    }

    let Some(Statement::Catalog(CatalogStatement::CreateGraphType {
        source: AstGraphTypeSource::Nested { specification, .. },
        ..
    })) = &result.statement
    else {
        panic!("closed reference descriptors must remain attached to the graph-type AST");
    };
    let properties = &specification.node_types[0].properties;
    assert_eq!(properties.len(), 3, "{specification:#?}");
    assert!(matches!(
        properties[0].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Graph,
            open: false,
            property_graph: true,
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), AstClosedReferenceTypeSpecification::Graph(graph)
            if graph.node_types.len() == 1
                && graph.node_types[0].alias.as_ref().is_some_and(|alias| alias.text == "person")
                && graph.node_types[0].properties[0].name.text == "name")
    ));
    assert!(matches!(
        properties[1].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Node,
            open: false,
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), AstClosedReferenceTypeSpecification::Node(node)
            if node.name.as_ref().is_some_and(|name| name.text == "Person")
                && node.properties[0].name.text == "id")
    ));
    assert!(matches!(
        properties[2].value_type.form,
        PropertyValueTypeForm::Reference {
            kind: AstReferenceValueTypeKind::Edge,
            open: false,
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), AstClosedReferenceTypeSpecification::Edge(edge)
            if edge.name.as_ref().is_some_and(|name| name.text == "Knows")
                && edge.endpoints.len() == 2
                && edge.properties[0].name.text == "since")
    ));

    let Some(CatalogCommand::CreateGraphType {
        source: IrGraphTypeSource::Nested { node_types, .. },
        ..
    }) = &result.analysis.catalog_command
    else {
        panic!("closed references must reach one backend-neutral catalog command");
    };
    let canonical = &node_types[0].properties;
    assert!(matches!(
        canonical[0].value_type.form,
        DeclaredValueTypeForm::Reference {
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), IrClosedReferenceTypeSpecification::Graph {
            node_types,
            edge_types,
        } if node_types.len() == 1 && edge_types.is_empty()
            && node_types[0].properties[0].name == "NAME")
    ));
    assert!(matches!(
        canonical[1].value_type.form,
        DeclaredValueTypeForm::Reference {
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), IrClosedReferenceTypeSpecification::Node(node)
            if node.name.as_deref() == Some("PERSON") && node.properties[0].name == "ID")
    ));
    assert!(matches!(
        canonical[2].value_type.form,
        DeclaredValueTypeForm::Reference {
            specification: Some(ref closed),
            ..
        } if matches!(closed.as_ref(), IrClosedReferenceTypeSpecification::Edge(edge)
            if edge.name.as_deref() == Some("KNOWS") && edge.properties[0].name == "SINCE")
    ));
}

#[test]
fn predefined_type_synonyms_and_temporal_forms_have_one_canonical_ir() {
    let source = "CREATE GRAPH TYPE analytics.predefined AS { NODE TYPE Sample { bool_alias BOOL, signed_verbose SIGNED INTEGER16, unsigned_verbose UNSIGNED BIG INTEGER, small_verbose SMALL INTEGER, decimal_alias DEC(10, 2), zoned_timestamp TIMESTAMP WITH TIME ZONE, local_timestamp TIMESTAMP WITHOUT TIME ZONE, default_timestamp TIMESTAMP, zoned_time TIME WITH TIME ZONE, local_time TIME WITHOUT TIME ZONE } }";
    let result = Compiler.compile("predefined-type-equivalence.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.diagnostics.is_empty());
    let Some(CatalogCommand::CreateGraphType {
        source: IrGraphTypeSource::Nested { node_types, .. },
        ..
    }) = &result.analysis.catalog_command
    else {
        panic!("valid predefined aliases must reach canonical IR");
    };
    let names = node_types[0]
        .properties
        .iter()
        .map(|property| match &property.value_type.form {
            DeclaredValueTypeForm::Named { name, .. } => name.as_str(),
            other => panic!("expected a named predefined type, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "BOOLEAN",
            "INT16",
            "UBIGINT",
            "SMALLINT",
            "DECIMAL",
            "ZONED DATETIME",
            "LOCAL DATETIME",
            "LOCAL DATETIME",
            "ZONED TIME",
            "LOCAL TIME",
        ]
    );
}

#[test]
fn invalid_predefined_type_parameter_arities_emit_no_command() {
    let source = "CREATE GRAPH TYPE analytics.invalid_arity AS { NODE TYPE Broken { boolean_with_parameter BOOLEAN(1), fixed_integer_with_parameter INT64(2), varchar_with_two_bounds VARCHAR(1, 2), duration_with_number DURATION(1) } }";
    let result = Compiler.compile(
        "invalid-predefined-type-arity.gql",
        source,
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        [
            "GQL-SEMA-VALUE-TYPE-ARITY",
            "GQL-SEMA-VALUE-TYPE-ARITY",
            "GQL-SEMA-VALUE-TYPE-ARITY",
            "GQL-SEMA-VALUE-TYPE-ARITY",
        ]
    );
    assert!(result.analysis.catalog_command.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn invalid_duration_qualifier_shape_emits_no_command() {
    let source = "CREATE GRAPH TYPE analytics.invalid_duration AS { NODE TYPE Broken { duration_with_incomplete_qualifier DURATION(YEAR) } }";
    let result = Compiler.compile("invalid-duration-qualifier.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-VALUE-TYPE-ARITY"]
    );
    assert!(result.analysis.catalog_command.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn every_predefined_type_production_is_source_admitted_and_canonicalized() {
    const CASES: &[(&str, &str)] = &[
        ("BOOL", "BOOLEAN"),
        ("BOOLEAN", "BOOLEAN"),
        ("STRING", "STRING"),
        ("STRING(8)", "STRING"),
        ("STRING(1, 8)", "STRING"),
        ("CHAR", "CHAR"),
        ("CHAR(8)", "CHAR"),
        ("VARCHAR", "VARCHAR"),
        ("VARCHAR(8)", "VARCHAR"),
        ("BYTES", "BYTES"),
        ("BYTES(8)", "BYTES"),
        ("BYTES(1, 8)", "BYTES"),
        ("BINARY", "BINARY"),
        ("BINARY(8)", "BINARY"),
        ("VARBINARY", "VARBINARY"),
        ("VARBINARY(8)", "VARBINARY"),
        ("INT8", "INT8"),
        ("INT16", "INT16"),
        ("INT32", "INT32"),
        ("INT64", "INT64"),
        ("INT128", "INT128"),
        ("INT256", "INT256"),
        ("SMALLINT", "SMALLINT"),
        ("INT", "INT"),
        ("INT(32)", "INT"),
        ("BIGINT", "BIGINT"),
        ("INTEGER8", "INT8"),
        ("INTEGER16", "INT16"),
        ("INTEGER32", "INT32"),
        ("INTEGER64", "INT64"),
        ("INTEGER128", "INT128"),
        ("INTEGER256", "INT256"),
        ("SMALL INTEGER", "SMALLINT"),
        ("INTEGER", "INT"),
        ("INTEGER(32)", "INT"),
        ("BIG INTEGER", "BIGINT"),
        ("SIGNED INTEGER8", "INT8"),
        ("SIGNED INTEGER16", "INT16"),
        ("SIGNED INTEGER32", "INT32"),
        ("SIGNED INTEGER64", "INT64"),
        ("SIGNED INTEGER128", "INT128"),
        ("SIGNED INTEGER256", "INT256"),
        ("SIGNED SMALL INTEGER", "SMALLINT"),
        ("SIGNED INTEGER", "INT"),
        ("SIGNED INTEGER(32)", "INT"),
        ("SIGNED BIG INTEGER", "BIGINT"),
        ("UINT8", "UINT8"),
        ("UINT16", "UINT16"),
        ("UINT32", "UINT32"),
        ("UINT64", "UINT64"),
        ("UINT128", "UINT128"),
        ("UINT256", "UINT256"),
        ("USMALLINT", "USMALLINT"),
        ("UINT", "UINT"),
        ("UINT(32)", "UINT"),
        ("UBIGINT", "UBIGINT"),
        ("UNSIGNED INTEGER8", "UINT8"),
        ("UNSIGNED INTEGER16", "UINT16"),
        ("UNSIGNED INTEGER32", "UINT32"),
        ("UNSIGNED INTEGER64", "UINT64"),
        ("UNSIGNED INTEGER128", "UINT128"),
        ("UNSIGNED INTEGER256", "UINT256"),
        ("UNSIGNED SMALL INTEGER", "USMALLINT"),
        ("UNSIGNED INTEGER", "UINT"),
        ("UNSIGNED INTEGER(32)", "UINT"),
        ("UNSIGNED BIG INTEGER", "UBIGINT"),
        ("DECIMAL", "DECIMAL"),
        ("DECIMAL(10)", "DECIMAL"),
        ("DECIMAL(10, 2)", "DECIMAL"),
        ("DEC", "DECIMAL"),
        ("DEC(10)", "DECIMAL"),
        ("DEC(10, 2)", "DECIMAL"),
        ("FLOAT16", "FLOAT16"),
        ("FLOAT32", "FLOAT32"),
        ("FLOAT64", "FLOAT64"),
        ("FLOAT128", "FLOAT128"),
        ("FLOAT256", "FLOAT256"),
        ("FLOAT", "FLOAT"),
        ("FLOAT(24)", "FLOAT"),
        ("FLOAT(24, 8)", "FLOAT"),
        ("REAL", "REAL"),
        ("DOUBLE", "DOUBLE"),
        ("DOUBLE PRECISION", "DOUBLE PRECISION"),
        ("ZONED DATETIME", "ZONED DATETIME"),
        ("TIMESTAMP WITH TIME ZONE", "ZONED DATETIME"),
        ("LOCAL DATETIME", "LOCAL DATETIME"),
        ("TIMESTAMP", "LOCAL DATETIME"),
        ("TIMESTAMP WITHOUT TIME ZONE", "LOCAL DATETIME"),
        ("DATE", "DATE"),
        ("ZONED TIME", "ZONED TIME"),
        ("TIME WITH TIME ZONE", "ZONED TIME"),
        ("LOCAL TIME", "LOCAL TIME"),
        ("TIME WITHOUT TIME ZONE", "LOCAL TIME"),
        ("DURATION(YEAR TO MONTH)", "DURATION"),
        ("DURATION(DAY TO SECOND)", "DURATION"),
        ("PATH", "PATH"),
        ("NULL", "NULL"),
        ("NULL NOT NULL", "NULL"),
        ("NOTHING", "NOTHING"),
    ];
    let properties = CASES
        .iter()
        .enumerate()
        .map(|(index, (spelling, _))| format!("p{index} {spelling}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "CREATE GRAPH TYPE analytics.predefined_matrix AS {{ NODE TYPE Sample {{ {properties} }} }}"
    );
    let result = Compiler.compile("predefined-type-matrix.gql", &source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let Some(CatalogCommand::CreateGraphType {
        source: IrGraphTypeSource::Nested { node_types, .. },
        ..
    }) = &result.analysis.catalog_command
    else {
        panic!("the predefined matrix must reach canonical IR");
    };
    assert_eq!(node_types[0].properties.len(), CASES.len());
    for (property, (_, expected)) in node_types[0].properties.iter().zip(CASES) {
        assert!(matches!(
            property.value_type.form,
            DeclaredValueTypeForm::Named { ref name, .. } if name == expected
        ));
    }
}
