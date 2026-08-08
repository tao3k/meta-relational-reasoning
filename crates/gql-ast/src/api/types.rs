//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

use gql_source::{Diagnostic, Span};

/// Named identifier token plus source location.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier {
    pub text: String,
    pub span: Span,
}

/// A lowered statement surface for all supported entry points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    Query(Query),
    Catalog(CatalogStatement),
    Data(DataStatement),
}

/// A parsed query with concrete clauses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Query {
    pub clauses: Vec<QueryClause>,
    pub span: Span,
}

/// The public clause variants supported by the current pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryClause {
    Match(MatchClause),
    Where {
        expression: Expression,
    },
    Let {
        binding: Identifier,
        value: Expression,
    },
    Return {
        expressions: Vec<Expression>,
    },
}

/// A `MATCH` clause with a compiled graph pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchClause {
    pub pattern: GraphPattern,
    pub span: Span,
}

/// A graph pattern made of nodes, edges, and path fragments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphPattern {
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

/// A graph pattern element in AST order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatternElement {
    Node(NodePattern),
    Edge(EdgePattern),
    Path(PathPattern),
}

/// A node pattern with optional binding and labels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePattern {
    pub binding: Option<Identifier>,
    pub labels: Vec<Identifier>,
    pub span: Span,
}

/// An edge pattern with relation labels and direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgePattern {
    pub labels: Vec<Identifier>,
    pub direction: EdgeDirection,
    pub span: Span,
}

/// A path pattern that stores nested pattern elements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPattern {
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

/// Relationship direction recovered from parser token neighborhoods.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeDirection {
    Out,
    In,
    Undirected,
}

impl MatchClause {
    #[must_use]
    pub fn relation_candidates(&self) -> Vec<Identifier> {
        self.pattern
            .elements
            .iter()
            .filter_map(PatternElement::relation_identifier)
            .collect()
    }
}

impl GraphPattern {
    #[must_use]
    pub fn relation_candidates(&self) -> Vec<Identifier> {
        self.elements
            .iter()
            .filter_map(PatternElement::relation_identifier)
            .collect()
    }
}

impl PatternElement {
    fn relation_identifier(&self) -> Option<Identifier> {
        match self {
            PatternElement::Node(node) => node.labels.first().cloned(),
            PatternElement::Edge(edge) => edge.labels.first().cloned(),
            PatternElement::Path(path) => path
                .elements
                .iter()
                .find_map(PatternElement::relation_identifier),
        }
    }
}

/// Lowered expression form used by analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Name(Identifier),
    String(String, Span),
    Integer(i64, Span),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

/// Unary expression operators in lowered form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    Not,
}

/// Binary expression operators in lowered form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
}

/// Catalog statement variants accepted by the broader parser surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogStatement {
    CreateGraph { name: Identifier },
    DropGraph { name: Identifier },
}

/// Data statement variants accepted by the parser surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataStatement {
    Insert,
    Delete,
    Set,
    Remove,
}

/// Result of syntax lowering and collected diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxParseOutput {
    pub statement: Statement,
    pub diagnostics: Vec<Diagnostic>,
}

