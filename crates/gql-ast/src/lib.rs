#![forbid(unsafe_code)]

use gql_source::Span;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    Query(Query),
    Catalog(CatalogStatement),
    Data(DataStatement),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Query {
    pub clauses: Vec<QueryClause>,
    pub span: Span,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchClause {
    pub pattern: GraphPattern,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphPattern {
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatternElement {
    Node(NodePattern),
    Edge(EdgePattern),
    Path(PathPattern),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePattern {
    pub binding: Option<Identifier>,
    pub labels: Vec<Identifier>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgePattern {
    pub labels: Vec<Identifier>,
    pub direction: EdgeDirection,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPattern {
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Name(Identifier),
    String(String, Span),
    Integer(i64, Span),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogStatement {
    CreateGraph { name: Identifier },
    DropGraph { name: Identifier },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataStatement {
    Insert,
    Delete,
    Set,
    Remove,
}
