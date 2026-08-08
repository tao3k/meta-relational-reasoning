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
    Match {
        relation: Identifier,
    },
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
