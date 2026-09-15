// mrr.gerbil-grammar-projection.v1 input-sha256=7143973977b61d1f57ee49e2f136148e714997d48d61516c06550da59c695407 body-sha256=9b49ba005a0ab55e7180234c8a50ad701a0ad0375fe52a55d9f4472a646a0c0f gerbil-scheme-rust-rev=a83fb649ddbbeaabdb538a6eaf0ded10838f7fad
// Generated through the Gerbil native AOT bindings; do not edit.
//! Aggregate grammar forms projected from the Gerbil grammar authority.
use super::projection::Keyword;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GrammarAggregateFunctionSpec {
    pub(crate) arity: u8,
    pub(crate) permits_star: bool,
    pub(crate) permits_quantifier: bool,
}
pub(crate) fn aggregate_function_spec(keyword: Keyword) -> Option<GrammarAggregateFunctionSpec> {
    match keyword {
        Keyword::Count => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: true,
            permits_quantifier: true,
        }),
        Keyword::Avg => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::Max => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::Min => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::Sum => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::CollectList => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::StddevSamp => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::StddevPop => Some(GrammarAggregateFunctionSpec {
            arity: 1,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::PercentileCont => Some(GrammarAggregateFunctionSpec {
            arity: 2,
            permits_star: false,
            permits_quantifier: true,
        }),
        Keyword::PercentileDisc => Some(GrammarAggregateFunctionSpec {
            arity: 2,
            permits_star: false,
            permits_quantifier: true,
        }),
        _ => None,
    }
}
