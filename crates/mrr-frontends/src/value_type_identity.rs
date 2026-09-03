//! Canonical query-identity encoding for the full GQL value-type lattice.
#![forbid(unsafe_code)]

use gql_ast as ast;

pub(crate) fn append_value_type(key: &mut Vec<u8>, value_type: &ast::PropertyValueType) {
    append(
        key,
        if value_type.non_null {
            "non-null"
        } else {
            "nullable"
        },
    );
    match &value_type.form {
        ast::PropertyValueTypeForm::Named { name, parameters } => {
            append(key, "named");
            append(key, name);
            for parameter in parameters {
                match parameter {
                    ast::TypeParameter::Unsigned(value) => append(key, &value.to_string()),
                    ast::TypeParameter::DurationQualifier { from, to } => {
                        append(key, "duration-qualifier");
                        append(key, from);
                        append(key, to);
                    }
                }
            }
        }
        ast::PropertyValueTypeForm::List {
            element,
            max_length,
        } => {
            append(key, "list");
            if let Some(element) = element {
                append_value_type(key, element);
            }
            if let Some(max_length) = max_length {
                append(key, &max_length.to_string());
            }
        }
        ast::PropertyValueTypeForm::Record { open, fields } => {
            append(
                key,
                if *open {
                    "open-record"
                } else {
                    "closed-record"
                },
            );
            append_property_types(key, fields);
        }
        ast::PropertyValueTypeForm::DynamicUnion {
            property_values,
            members,
        } => {
            append(
                key,
                if *property_values {
                    "property-value-union"
                } else {
                    "value-union"
                },
            );
            if let Some(members) = members {
                for member in members {
                    append_value_type(key, member);
                }
            }
        }
        ast::PropertyValueTypeForm::Reference {
            kind,
            open,
            property_graph,
            specification,
            fields,
        } => {
            append(
                key,
                match kind {
                    ast::ReferenceValueTypeKind::Graph => "graph-reference",
                    ast::ReferenceValueTypeKind::BindingTable => "binding-table-reference",
                    ast::ReferenceValueTypeKind::Node => "node-reference",
                    ast::ReferenceValueTypeKind::Edge => "edge-reference",
                },
            );
            append(key, if *open { "open" } else { "closed" });
            append(
                key,
                if *property_graph {
                    "property-graph"
                } else {
                    "reference"
                },
            );
            if let Some(specification) = specification {
                append_closed_reference_type(key, specification);
            }
            append_property_types(key, fields);
        }
        ast::PropertyValueTypeForm::Union(members) => {
            append(key, "union");
            for member in members {
                append_value_type(key, member);
            }
        }
    }
}

fn append_property_types(key: &mut Vec<u8>, properties: &[ast::PropertyType]) {
    for property in properties {
        append(key, &property.name.canonical_text());
        append_value_type(key, &property.value_type);
    }
}

fn append_closed_reference_type(
    key: &mut Vec<u8>,
    specification: &ast::ClosedReferenceTypeSpecification,
) {
    match specification {
        ast::ClosedReferenceTypeSpecification::Graph(graph) => {
            append(key, "closed-graph");
            for node in &graph.node_types {
                append_node_type(key, node);
            }
            for edge in &graph.edge_types {
                append_edge_type(key, edge);
            }
        }
        ast::ClosedReferenceTypeSpecification::Node(node) => append_node_type(key, node),
        ast::ClosedReferenceTypeSpecification::Edge(edge) => append_edge_type(key, edge),
    }
}

fn append_node_type(key: &mut Vec<u8>, node: &ast::NodeTypeSpecification) {
    append(key, "node-type");
    append_optional_identifier(key, node.name.as_ref());
    append_optional_identifier(key, node.alias.as_ref());
    append_identifier_set(key, node.key_labels.as_deref());
    append_identifiers(key, &node.labels);
    append_property_types(key, &node.properties);
}

fn append_edge_type(key: &mut Vec<u8>, edge: &ast::EdgeTypeSpecification) {
    append(key, "edge-type");
    append(
        key,
        match edge.kind {
            Some(ast::EdgeKind::Directed) => "directed-kind",
            Some(ast::EdgeKind::Undirected) => "undirected-kind",
            None => "implicit-kind",
        },
    );
    append_optional_identifier(key, edge.name.as_ref());
    append(
        key,
        match edge.direction {
            ast::EdgeDirection::Out => "out",
            ast::EdgeDirection::In => "in",
            ast::EdgeDirection::Undirected => "undirected",
        },
    );
    for endpoint in &edge.endpoints {
        match endpoint {
            ast::NodeTypeReference::Alias(alias) => {
                append(key, "endpoint-alias");
                append(key, &alias.canonical_text());
            }
            ast::NodeTypeReference::Inline {
                key_labels,
                labels,
                properties,
                ..
            } => {
                append(key, "endpoint-inline");
                append_identifier_set(key, key_labels.as_deref());
                append_identifiers(key, labels);
                append_property_types(key, properties);
            }
        }
    }
    append_identifier_set(key, edge.key_labels.as_deref());
    append_identifiers(key, &edge.labels);
    append_property_types(key, &edge.properties);
}

fn append_optional_identifier(key: &mut Vec<u8>, identifier: Option<&ast::Identifier>) {
    if let Some(identifier) = identifier {
        append(key, &identifier.canonical_text());
    } else {
        append(key, "absent");
    }
}

fn append_identifier_set(key: &mut Vec<u8>, identifiers: Option<&[ast::Identifier]>) {
    append(
        key,
        if identifiers.is_some() {
            "present"
        } else {
            "absent"
        },
    );
    if let Some(identifiers) = identifiers {
        append_identifiers(key, identifiers);
    }
}

fn append_identifiers(key: &mut Vec<u8>, identifiers: &[ast::Identifier]) {
    for identifier in identifiers {
        append(key, &identifier.canonical_text());
    }
}

pub(crate) fn append(key: &mut Vec<u8>, value: &str) {
    key.extend_from_slice(value.len().to_string().as_bytes());
    key.push(b':');
    key.extend_from_slice(value.as_bytes());
    key.push(0);
}
