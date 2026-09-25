// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Catalog-bound expression, parameter, aggregation, and result typing.

mod catalog;
mod checker;
mod model;
mod rules;

pub(crate) use checker::type_query;
pub use model::{ExpressionType, ParameterType, QueryType, ResultField, StaticQueryTyping};
