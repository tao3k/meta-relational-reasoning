//! ISO GQL catalog identity and type contracts.

use gql_types::ValueType;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Catalog name identifier.
pub struct CatalogName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Graph name identifier.
pub struct GraphName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Schema name identifier.
pub struct SchemaName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Graph type name identifier.
pub struct GraphTypeName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Node type name identifier.
pub struct NodeTypeName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Edge type name identifier.
pub struct EdgeTypeName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Root catalog container used by ISO-facing clients.
pub struct Catalog {
    /// Catalog identifier.
    pub name: CatalogName,
    /// Registered graphs.
    pub graphs: Vec<Graph>,
    /// Registered schemas.
    pub schemas: Vec<Schema>,
}

impl Catalog {
    /// Construct a catalog definition.
    #[must_use]
    pub fn new(name: CatalogName, graphs: Vec<Graph>, schemas: Vec<Schema>) -> Self {
        Self {
            name,
            graphs,
            schemas,
        }
    }

    /// Resolve one graph declaration by name.
    #[must_use]
    pub fn graph(&self, name: &GraphName) -> Option<&Graph> {
        self.graphs.iter().find(|graph| &graph.name == name)
    }

    /// Resolve one schema declaration by name.
    #[must_use]
    pub fn schema(&self, name: &SchemaName) -> Option<&Schema> {
        self.schemas.iter().find(|schema| &schema.name == name)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Graph declaration in the catalog.
pub struct Graph {
    /// Graph identifier.
    pub name: GraphName,
    /// Optional schema name.
    pub schema: Option<SchemaName>,
    /// Optional graph-type name.
    pub graph_type: Option<GraphTypeName>,
    /// Node types in the graph.
    pub node_types: Vec<NodeTypeName>,
    /// Edge types in the graph.
    pub edge_types: Vec<EdgeTypeName>,
}

impl Graph {
    /// Construct a graph declaration.
    #[must_use]
    pub fn new(
        name: GraphName,
        schema: Option<SchemaName>,
        graph_type: Option<GraphTypeName>,
    ) -> Self {
        Self {
            name,
            schema,
            graph_type,
            node_types: Vec::new(),
            edge_types: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Schema declaration.
pub struct Schema {
    /// Schema identifier.
    pub name: SchemaName,
    /// Node types.
    pub node_types: Vec<NodeType>,
    /// Edge types.
    pub edge_types: Vec<EdgeType>,
    /// Procedure signatures.
    pub procedures: Vec<ProcedureSignature>,
}

impl Schema {
    /// Construct a schema declaration.
    #[must_use]
    pub fn new(
        name: SchemaName,
        node_types: Vec<NodeType>,
        edge_types: Vec<EdgeType>,
        procedures: Vec<ProcedureSignature>,
    ) -> Self {
        Self {
            name,
            node_types,
            edge_types,
            procedures,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Node type declaration.
pub struct NodeType {
    /// Node type identifier.
    pub name: NodeTypeName,
    /// Node properties.
    pub properties: Vec<Property>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Edge type declaration.
pub struct EdgeType {
    /// Edge type identifier.
    pub name: EdgeTypeName,
    /// Optional source node type.
    pub source_node_type: Option<NodeTypeName>,
    /// Optional destination node type.
    pub target_node_type: Option<NodeTypeName>,
    /// Edge properties.
    pub properties: Vec<Property>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Property declaration.
pub struct Property {
    /// Property identifier.
    pub name: String,
    /// Property value type.
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Procedure signature declaration.
pub struct ProcedureSignature {
    /// Procedure identifier.
    pub name: String,
    /// Input parameter types.
    pub parameters: Vec<ValueType>,
    /// Return value types.
    pub returns: Vec<ValueType>,
}

/// Language-owned catalog access used by semantic analysis.
pub trait GqlCatalog {
    /// Return the ISO catalog being used for name and type context.
    fn catalog(&self) -> &Catalog;
}

impl GqlCatalog for Catalog {
    fn catalog(&self) -> &Catalog {
        self
    }
}
