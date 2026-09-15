//! Parser-owned Rowan CST lowering for the admitted Graph-Relational slice.

use crate::projection::{
    AggregateFunction, BinaryOperator, DynamicParameterReference, EdgeDirection, EdgePattern,
    Expression, Identifier, MatchClause, NodePattern, NonNegativeIntegerSpecification,
    NullOrdering, PathPattern, PatternElement, PropertyConstraint, Query, QueryClause, RecordField,
    ReturnProjection, SetQuantifier, SortDirection, SortKey, TruthValue, UnaryOperator,
};
use mrr_gerbil::{ParserCst, ParserSyntax, parse_gql_artifact};
use rowan::SyntaxNode;

use crate::FrontendError;

type Node = SyntaxNode<ParserSyntax>;

pub(crate) struct ParserOwnedAst {
    pub query: Query,
    pub grammar_digest: String,
    pub source_digest: String,
}

pub(crate) fn lower_parser_owned_ast(source: &str) -> Result<ParserOwnedAst, FrontendError> {
    let artifact = parse_gql_artifact(source)
        .map_err(|error| FrontendError::ParserOwned(format!("ParseArtifact: {error:?}")))?;
    let cst = artifact
        .to_rowan_cst()
        .map_err(|error| FrontendError::ParserOwned(format!("ParserCst: {error:?}")))?;
    let root = cst.root();
    if root.text() != source {
        return Err(FrontendError::ParserOwned(
            "ParserCst: source is not losslessly covered by the parser artifact".into(),
        ));
    }
    require_name(&cst, &root, "GqlProgram")?;
    reject_unlowered_semantics(&cst, &root)?;

    let matches = descendants_named(&cst, &root, "SimpleMatchStatement");
    if matches.len() != 1 {
        return unsupported("parser-owned parity requires exactly one SimpleMatchStatement");
    }
    let returns = descendants_named(&cst, &root, "ReturnStatement");
    let finishes = descendants_named(&cst, &root, "PrimitiveResultStatement")
        .into_iter()
        .filter(|statement| {
            significant_tokens(&cst, statement)
                .iter()
                .any(|token| token.text().eq_ignore_ascii_case("FINISH"))
        })
        .collect::<Vec<_>>();
    if returns.len() + finishes.len() != 1 {
        return unsupported("parser-owned parity requires exactly one result statement");
    }

    let mut clauses = vec![QueryClause::Match(lower_match(&cst, &matches[0])?)];
    let where_clauses = descendants_named(&cst, &root, "GraphPatternWhereClause");
    if where_clauses.len() > 1 {
        return unsupported("multiple GraphPatternWhereClause nodes");
    }
    if let Some(where_clause) = where_clauses.first() {
        let condition = first_descendant(&cst, where_clause, "SearchCondition")
            .ok_or_else(|| unsupported_error("WHERE condition"))?;
        clauses.push(QueryClause::Where(lower_expression(&cst, &condition)?));
    }
    for filter in descendants_named(&cst, &root, "FilterStatement") {
        let condition = first_descendant(&cst, &filter, "SearchCondition")
            .ok_or_else(|| unsupported_error("FILTER condition"))?;
        clauses.push(QueryClause::Filter(lower_expression(&cst, &condition)?));
    }
    if let Some(result) = returns.first() {
        let result_tokens = significant_tokens(&cst, result);
        let all_bindings = first_descendant(&cst, result, "ReturnItemList").is_none()
            && matches!(
                result_tokens.as_slice(),
                [return_token, wildcard]
                    if return_token.text().eq_ignore_ascii_case("RETURN")
                        && wildcard.text() == "*"
            );
        clauses.push(QueryClause::Return {
            quantifier: lower_result_quantifier(&cst, result)?,
            all_bindings,
            projections: if all_bindings {
                Vec::new()
            } else {
                lower_return(&cst, result)?
            },
        });
    } else {
        clauses.push(QueryClause::Finish);
    }
    if let Some(group_by) = first_descendant(&cst, &root, "GroupByClause") {
        clauses.push(QueryClause::GroupBy(lower_group_by(&cst, &group_by)?));
    }
    if let Some(order_by) = first_descendant(&cst, &root, "OrderByClause") {
        clauses.push(QueryClause::OrderBy(lower_order_by(&cst, &order_by)?));
    }
    if let Some(offset) = first_descendant(&cst, &root, "OffsetClause") {
        clauses.push(QueryClause::Offset(lower_limit(&cst, &offset)?));
    }
    if let Some(limit) = first_descendant(&cst, &root, "LimitClause") {
        clauses.push(QueryClause::Limit(lower_limit(&cst, &limit)?));
    }
    Ok(ParserOwnedAst {
        query: Query { clauses },
        grammar_digest: artifact.grammar_digest,
        source_digest: artifact.source_digest,
    })
}

fn lower_result_quantifier(
    cst: &ParserCst,
    node: &Node,
) -> Result<Option<SetQuantifier>, FrontendError> {
    first_descendant(cst, node, "SetQuantifier")
        .map(|quantifier| match quantifier.text().to_string().trim() {
            "ALL" => Ok(SetQuantifier::All),
            "DISTINCT" => Ok(SetQuantifier::Distinct),
            _ => unsupported("result set quantifier"),
        })
        .transpose()
}

fn lower_group_by(cst: &ParserCst, node: &Node) -> Result<Vec<Expression>, FrontendError> {
    let keys = descendants_named(cst, node, "GroupingElement");
    if keys.is_empty() {
        return unsupported("GROUP BY key");
    }
    keys.iter()
        .map(|key| {
            let expression = first_descendant(cst, key, "AggregatingValueExpression")
                .or_else(|| first_descendant(cst, key, "ValueExpression"))
                .ok_or_else(|| unsupported_error("GROUP BY expression"))?;
            lower_expression(cst, &expression)
        })
        .collect()
}

fn lower_order_by(cst: &ParserCst, node: &Node) -> Result<Vec<SortKey>, FrontendError> {
    let list = first_descendant(cst, node, "SortSpecificationList")
        .ok_or_else(|| unsupported_error("SortSpecificationList"))?;
    direct_children_named(cst, &list, "SortSpecification")
        .iter()
        .map(|specification| {
            let expression = first_descendant(cst, specification, "AggregatingValueExpression")
                .ok_or_else(|| unsupported_error("sort key expression"))?;
            let direction = first_descendant(cst, specification, "OrderingSpecification")
                .map(|ordering| match ordering.text().to_string().trim() {
                    "ASC" | "ASCENDING" => Ok(SortDirection::Ascending),
                    "DESC" | "DESCENDING" => Ok(SortDirection::Descending),
                    _ => unsupported("ordering specification"),
                })
                .transpose()?;
            let null_ordering = first_descendant(cst, specification, "NullOrdering")
                .map(|ordering| match ordering.text().to_string().trim() {
                    "NULLS FIRST" => Ok(NullOrdering::First),
                    "NULLS LAST" => Ok(NullOrdering::Last),
                    _ => unsupported("null ordering"),
                })
                .transpose()?;
            Ok(SortKey {
                expression: lower_expression(cst, &expression)?,
                direction,
                null_ordering,
            })
        })
        .collect()
}

fn lower_limit(
    cst: &ParserCst,
    node: &Node,
) -> Result<NonNegativeIntegerSpecification, FrontendError> {
    if let Some(integer) = first_descendant(cst, node, "UnsignedInteger") {
        let value = significant_tokens(cst, &integer)
            .into_iter()
            .next()
            .ok_or_else(|| unsupported_error("pagination integer"))?
            .text()
            .parse::<u64>()
            .map_err(|_| unsupported_error("pagination integer"))?;
        return Ok(NonNegativeIntegerSpecification::Literal(value));
    }
    let parameter = first_descendant(cst, node, "DynamicParameterSpecification")
        .ok_or_else(|| unsupported_error("pagination value"))?;
    Ok(NonNegativeIntegerSpecification::Parameter(
        lower_dynamic_parameter(&parameter)?,
    ))
}

fn lower_dynamic_parameter(node: &Node) -> Result<DynamicParameterReference, FrontendError> {
    let text = node.text().to_string();
    let decoded = crate::lexical::decode_parameter_reference(text.trim())
        .ok_or_else(|| unsupported_error("dynamic parameter"))?;
    Ok(DynamicParameterReference {
        name: decoded.name.into_owned(),
    })
}

fn reject_unlowered_semantics(cst: &ParserCst, root: &Node) -> Result<(), FrontendError> {
    for (kind, semantic) in [
        ("OptionalMatchStatement", "OPTIONAL MATCH"),
        ("MatchMode", "graph match mode"),
        ("KeepClause", "KEEP path prefix"),
        ("PathPatternPrefix", "path search prefix"),
        ("PathVariableDeclaration", "path variable declaration"),
        ("GraphPatternQuantifier", "quantified path pattern"),
    ] {
        if first_descendant(cst, root, kind).is_some() {
            return unsupported_exact(semantic);
        }
    }
    for predicate in descendants_named(cst, root, "ElementPatternPredicate") {
        if first_descendant(cst, &predicate, "ElementPropertySpecification").is_none() {
            return unsupported("non-property ElementPatternPredicate");
        }
    }
    if descendants_named(cst, root, "OrderByClause").len() > 1 {
        return unsupported("multiple OrderByClause nodes");
    }
    if descendants_named(cst, root, "LimitClause").len() > 1 {
        return unsupported("multiple LimitClause nodes");
    }
    if descendants_named(cst, root, "OffsetClause").len() > 1 {
        return unsupported("multiple OffsetClause nodes");
    }
    if descendants_named(cst, root, "GroupByClause").len() > 1 {
        return unsupported("multiple GroupByClause nodes");
    }
    Ok(())
}

fn lower_match(cst: &ParserCst, node: &Node) -> Result<MatchClause, FrontendError> {
    let graph = first_descendant(cst, node, "GraphPattern")
        .ok_or_else(|| unsupported_error("GraphPattern"))?;
    let list = direct_child_named(cst, &graph, "PathPatternList")
        .ok_or_else(|| unsupported_error("PathPatternList"))?;
    let patterns = direct_children_named(cst, &list, "PathPattern")
        .iter()
        .map(|path| lower_path(cst, path))
        .collect::<Result<Vec<_>, _>>()?;
    if patterns.is_empty() {
        return unsupported("empty graph pattern");
    }
    Ok(MatchClause { patterns })
}

fn lower_path(cst: &ParserCst, node: &Node) -> Result<PathPattern, FrontendError> {
    let term =
        first_descendant(cst, node, "PathTerm").ok_or_else(|| unsupported_error("PathTerm"))?;
    let mut elements = Vec::new();
    for factor in direct_children_named(cst, &term, "PathFactor") {
        if let Some(found) = first_descendant(cst, &factor, "NodePattern") {
            elements.push(PatternElement::Node(lower_node(cst, &found)?));
        } else if let Some(found) = first_descendant(cst, &factor, "EdgePattern") {
            elements.push(PatternElement::Edge(lower_edge(cst, &found)?));
        } else {
            return unsupported("PathFactor outside node-edge parity slice");
        }
    }
    Ok(PathPattern { elements })
}

fn lower_node(cst: &ParserCst, node: &Node) -> Result<NodePattern, FrontendError> {
    Ok(NodePattern {
        binding: lower_binding(cst, node)?,
        labels: lower_labels(cst, node)?,
        properties: lower_properties(cst, node)?,
    })
}

fn lower_edge(cst: &ParserCst, node: &Node) -> Result<EdgePattern, FrontendError> {
    let direction = if first_descendant(cst, node, "FullEdgePointingRight").is_some() {
        EdgeDirection::Out
    } else if first_descendant(cst, node, "FullEdgePointingLeft").is_some() {
        EdgeDirection::In
    } else {
        EdgeDirection::Undirected
    };
    Ok(EdgePattern {
        binding: lower_binding(cst, node)?,
        labels: lower_labels(cst, node)?,
        properties: lower_properties(cst, node)?,
        direction,
    })
}

fn lower_binding(cst: &ParserCst, node: &Node) -> Result<Option<Identifier>, FrontendError> {
    first_descendant(cst, node, "ElementVariableDeclaration")
        .map(|declaration| lower_identifier(cst, &declaration))
        .transpose()
}

fn lower_labels(cst: &ParserCst, node: &Node) -> Result<Vec<Identifier>, FrontendError> {
    descendants_named(cst, node, "LabelName")
        .iter()
        .map(|label| lower_identifier(cst, label))
        .collect()
}

fn lower_properties(
    cst: &ParserCst,
    node: &Node,
) -> Result<Vec<PropertyConstraint>, FrontendError> {
    descendants_named(cst, node, "PropertyKeyValuePair")
        .iter()
        .map(|pair| {
            let name = direct_child_named(cst, pair, "PropertyName")
                .ok_or_else(|| unsupported_error("property name"))?;
            let value = direct_child_named(cst, pair, "ValueExpression")
                .ok_or_else(|| unsupported_error("property value"))?;
            Ok(PropertyConstraint {
                key: lower_identifier(cst, &name)?,
                value: lower_expression(cst, &value)?,
            })
        })
        .collect()
}

fn lower_return(cst: &ParserCst, node: &Node) -> Result<Vec<ReturnProjection>, FrontendError> {
    let list = first_descendant(cst, node, "ReturnItemList")
        .ok_or_else(|| unsupported_error("ReturnItemList"))?;
    direct_children_named(cst, &list, "ReturnItem")
        .iter()
        .map(|item| {
            let expression = direct_child_named(cst, item, "AggregatingValueExpression")
                .ok_or_else(|| unsupported_error("return expression"))?;
            let alias = direct_child_named(cst, item, "ReturnItemAlias")
                .map(|node| lower_identifier(cst, &node))
                .transpose()?;
            Ok(ReturnProjection {
                expression: lower_expression(cst, &expression)?,
                alias,
            })
        })
        .collect()
}

fn lower_expression(cst: &ParserCst, node: &Node) -> Result<Expression, FrontendError> {
    if first_descendant(cst, node, "ValueTypePredicate").is_some() {
        return unsupported_exact("value-type predicate expression");
    }
    if kind_name(cst, node) == Some("ValueExpression") {
        let mut levels = Vec::new();
        collect_unparenthesized_binary_levels(cst, node, &mut levels);
        levels.sort_unstable();
        levels.dedup();
        if levels.len() > 1 {
            return unsupported("mixed operator precedence not encoded by parser CST");
        }
    }
    if kind_name(cst, node) == Some("AggregateFunction") {
        return lower_aggregate(cst, node);
    }
    if kind_name(cst, node) == Some("NullPredicate") {
        let operand = direct_child_named(cst, node, "ValueExpressionPrimary")
            .ok_or_else(|| unsupported_error("NULL predicate operand"))?;
        let suffix = direct_child_named(cst, node, "NullPredicatePart2")
            .ok_or_else(|| unsupported_error("NULL predicate suffix"))?;
        let negated = significant_tokens(cst, &suffix)
            .iter()
            .any(|token| token.text().eq_ignore_ascii_case("NOT"));
        return Ok(Expression::NullPredicate {
            operand: Box::new(lower_expression(cst, &operand)?),
            negated,
        });
    }
    if let Some(truth) = direct_child_named(cst, node, "TruthValue") {
        let operands = direct_children_named(cst, node, "ValueExpression");
        if operands.len() != 1 {
            return unsupported("truth predicate without exactly one operand");
        }
        let value = match truth
            .text()
            .to_string()
            .trim()
            .to_ascii_uppercase()
            .as_str()
        {
            "TRUE" => TruthValue::True,
            "FALSE" => TruthValue::False,
            "UNKNOWN" => TruthValue::Unknown,
            _ => return unsupported("truth predicate value"),
        };
        let negated = node
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .any(|token| token.text().eq_ignore_ascii_case("NOT"));
        return Ok(Expression::TruthPredicate {
            operand: Box::new(lower_expression(cst, &operands[0])?),
            value,
            negated,
        });
    }
    let direct_expression_children = direct_children_named(cst, node, "ValueExpression");
    if direct_expression_children.len() == 1 {
        let leading = node
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|token| {
                !matches!(
                    cst.kind_name(token.kind()),
                    Some("WhitespaceTrivia" | "CommentTrivia")
                )
            });
        if let Some(operator) = leading {
            let operator = match operator.text() {
                "+" => Some(UnaryOperator::Plus),
                "-" => Some(UnaryOperator::Negate),
                text if text.eq_ignore_ascii_case("NOT") => Some(UnaryOperator::Not),
                _ => None,
            };
            if let Some(operator) = operator {
                return Ok(Expression::Unary {
                    operator,
                    operand: Box::new(lower_expression(cst, &direct_expression_children[0])?),
                });
            }
        }
    }
    if let Some(operator) = direct_child_named(cst, node, "CompOp") {
        let operands = direct_children_named(cst, node, "ValueExpression");
        if operands.len() != 2 {
            return unsupported("comparison without exactly two operands");
        }
        let operator_text = operator.text().to_string();
        return Ok(Expression::Binary {
            operator: comparison_operator(operator_text.trim())?,
            left: Box::new(lower_expression(cst, &operands[0])?),
            right: Box::new(lower_expression(cst, &operands[1])?),
        });
    }
    if direct_expression_children.len() == 2 {
        let operator = node
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find_map(|token| {
                let text = token.text().to_ascii_uppercase();
                matches!(
                    text.as_str(),
                    "+" | "-" | "*" | "/" | "AND" | "OR" | "XOR" | "||"
                )
                .then_some(text)
            })
            .ok_or_else(|| unsupported_error("binary value operator"))?;
        let operator = match operator.as_str() {
            "+" => BinaryOperator::Add,
            "-" => BinaryOperator::Subtract,
            "*" => BinaryOperator::Multiply,
            "/" => BinaryOperator::Divide,
            "AND" => BinaryOperator::And,
            "OR" => BinaryOperator::Or,
            "XOR" => return unsupported_exact("XOR expression"),
            "||" => return unsupported_exact("concatenation expression"),
            _ => return unsupported("binary value operator"),
        };
        return Ok(Expression::Binary {
            operator,
            left: Box::new(lower_expression(cst, &direct_expression_children[0])?),
            right: Box::new(lower_expression(cst, &direct_expression_children[1])?),
        });
    }
    if kind_name(cst, node) == Some("ValueExpressionPrimary")
        && let (Some(base), Some(property)) = (
            direct_child_named(cst, node, "ValueExpressionPrimary"),
            direct_child_named(cst, node, "PropertyName"),
        )
    {
        return Ok(Expression::PropertyAccess {
            base: Box::new(lower_expression(cst, &base)?),
            property: lower_identifier(cst, &property)?,
        });
    }
    if kind_name(cst, node) == Some("BindingVariableReference") {
        return Ok(Expression::Name(lower_identifier(cst, node)?));
    }
    if kind_name(cst, node) == Some("DynamicParameterSpecification") {
        return Ok(Expression::Parameter(lower_dynamic_parameter(node)?));
    }
    if kind_name(cst, node) == Some("GeneralLiteral") {
        for child_kind in [
            "TemporalLiteral",
            "DurationLiteral",
            "ListLiteral",
            "RecordLiteral",
        ] {
            if let Some(child) = direct_child_named(cst, node, child_kind) {
                return lower_expression(cst, &child);
            }
        }
    }
    if matches!(
        kind_name(cst, node),
        Some("DateLiteral" | "TimeLiteral" | "DatetimeLiteral" | "DurationLiteral")
    ) {
        let value = first_descendant(cst, node, "CharacterStringLiteral")
            .ok_or_else(|| unsupported_error("temporal character string"))?;
        let value = significant_tokens(cst, &value)
            .into_iter()
            .next()
            .and_then(|token| {
                crate::lexical::decode_character_string(token.text())
                    .map(|decoded| decoded.value.into_owned())
            })
            .ok_or_else(|| unsupported_error("temporal character sequence"))?;
        return Ok(match kind_name(cst, node) {
            Some("DateLiteral") => Expression::Date(value),
            Some("TimeLiteral") => Expression::Time(value),
            Some("DatetimeLiteral") => Expression::Timestamp(value),
            Some("DurationLiteral") => Expression::Duration(value),
            _ => unreachable!("temporal kind restricted above"),
        });
    }
    if kind_name(cst, node) == Some("ListLiteral") {
        let list = first_descendant(cst, node, "ListElementList")
            .ok_or_else(|| unsupported_error("list element list"))?;
        let values = direct_children_named(cst, &list, "ListElement")
            .iter()
            .map(|element| {
                let value = first_descendant(cst, element, "ValueExpression")
                    .ok_or_else(|| unsupported_error("list element value"))?;
                lower_expression(cst, &value)
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Expression::List(values));
    }
    if kind_name(cst, node) == Some("RecordLiteral") {
        let list = first_descendant(cst, node, "FieldList")
            .ok_or_else(|| unsupported_error("record field list"))?;
        let fields = direct_children_named(cst, &list, "Field")
            .iter()
            .map(|field| {
                let name = direct_child_named(cst, field, "FieldName")
                    .ok_or_else(|| unsupported_error("record field name"))?;
                let value = direct_child_named(cst, field, "ValueExpression")
                    .ok_or_else(|| unsupported_error("record field value"))?;
                Ok(RecordField {
                    name: lower_identifier(cst, &name)?,
                    value: lower_expression(cst, &value)?,
                })
            })
            .collect::<Result<Vec<_>, FrontendError>>()?;
        return Ok(Expression::Record(fields));
    }
    if kind_name(cst, node) == Some("GeneralLiteral") {
        let token = significant_tokens(cst, node)
            .into_iter()
            .next()
            .ok_or_else(|| unsupported_error("general literal token"))?;
        let text = token.text().to_string();
        match text.to_ascii_uppercase().as_str() {
            "TRUE" => return Ok(Expression::Boolean(true)),
            "FALSE" => return Ok(Expression::Boolean(false)),
            "NULL" => return Ok(Expression::Null),
            _ => {}
        }
        let value = crate::lexical::decode_character_string(&text)
            .ok_or_else(|| unsupported_error("character string literal"))?;
        match value.form {
            crate::lexical::CharacterStringForm::Single
            | crate::lexical::CharacterStringForm::Double => {}
            crate::lexical::CharacterStringForm::Grave => {
                return unsupported("grave-quoted character string");
            }
        }
        return Ok(Expression::String(value.value.into_owned()));
    }
    if kind_name(cst, node) == Some("ExactNumericLiteral") {
        let token = significant_tokens(cst, node)
            .into_iter()
            .next()
            .ok_or_else(|| unsupported_error("exact numeric literal"))?;
        let text = token.text().to_string();
        return match crate::lexical::decode_numeric_literal(&text) {
            Some(crate::lexical::NumericLiteral::Integer(value)) => Ok(Expression::Integer(value)),
            Some(crate::lexical::NumericLiteral::Decimal(value)) => Ok(Expression::Decimal(value)),
            Some(crate::lexical::NumericLiteral::Approximate(value)) => {
                Ok(Expression::ApproximateNumeric(value))
            }
            None => Err(unsupported_error("numeric literal")),
        };
    }

    let children = node
        .children()
        .filter(|child| is_expression_wrapper(kind_name(cst, child)))
        .collect::<Vec<_>>();
    if children.len() == 1 {
        return lower_expression(cst, &children[0]);
    }
    unsupported(&format!(
        "expression kind {}",
        kind_name(cst, node).unwrap_or("<unknown>")
    ))
}

fn collect_unparenthesized_binary_levels(cst: &ParserCst, node: &Node, levels: &mut Vec<u8>) {
    if kind_name(cst, node) == Some("ParenthesizedValueExpression") {
        return;
    }
    if kind_name(cst, node) == Some("ValueExpression")
        && let Some(level) = direct_binary_level(cst, node)
    {
        levels.push(level);
    }
    for child in node.children() {
        collect_unparenthesized_binary_levels(cst, &child, levels);
    }
}

fn direct_binary_level(cst: &ParserCst, node: &Node) -> Option<u8> {
    if direct_children_named(cst, node, "ValueExpression").len() != 2 {
        return None;
    }
    if direct_child_named(cst, node, "CompOp").is_some() {
        return Some(2);
    }
    node.children_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
        .find_map(|token| match token.text().to_ascii_uppercase().as_str() {
            "OR" => Some(0),
            "AND" => Some(1),
            "+" | "-" => Some(3),
            "*" | "/" => Some(4),
            _ => None,
        })
}

fn lower_identifier(cst: &ParserCst, node: &Node) -> Result<Identifier, FrontendError> {
    let regular = if kind_name(cst, node) == Some("RegularIdentifier") {
        node.clone()
    } else {
        first_descendant(cst, node, "RegularIdentifier")
            .ok_or_else(|| unsupported_error("regular identifier"))?
    };
    let token = significant_tokens(cst, &regular)
        .into_iter()
        .next()
        .ok_or_else(|| unsupported_error("identifier token"))?;
    Ok(Identifier {
        text: token.text().to_string(),
    })
}

fn comparison_operator(text: &str) -> Result<BinaryOperator, FrontendError> {
    match text {
        "=" => Ok(BinaryOperator::Equals),
        "<>" | "!=" => Ok(BinaryOperator::NotEquals),
        "<" => Ok(BinaryOperator::LessThan),
        "<=" => Ok(BinaryOperator::LessThanOrEqual),
        ">" => Ok(BinaryOperator::GreaterThan),
        ">=" => Ok(BinaryOperator::GreaterThanOrEqual),
        _ => unsupported(&format!("comparison operator {text}")),
    }
}

fn is_expression_wrapper(name: Option<&str>) -> bool {
    matches!(
        name,
        Some(
            "SearchCondition"
                | "BooleanValueExpression"
                | "ValueExpression"
                | "AggregatingValueExpression"
                | "ValueExpressionPrimary"
                | "ParenthesizedValueExpression"
                | "UnsignedValueSpecification"
                | "GeneralValueSpecification"
                | "UnsignedLiteral"
                | "UnsignedNumericLiteral"
                | "BindingVariableReference"
                | "DynamicParameterSpecification"
                | "AggregateFunction"
                | "DependentValueExpression"
                | "IndependentValueExpression"
                | "NumericValueExpression"
                | "TemporalLiteral"
                | "DateLiteral"
                | "TimeLiteral"
                | "DatetimeLiteral"
                | "DurationLiteral"
                | "ListLiteral"
                | "RecordLiteral"
                | "Predicate"
                | "NullPredicate"
                | "GeneralLiteral"
                | "ExactNumericLiteral"
        )
    )
}

fn lower_aggregate(cst: &ParserCst, node: &Node) -> Result<Expression, FrontendError> {
    let function_node = first_descendant(cst, node, "GeneralSetFunctionType")
        .or_else(|| first_descendant(cst, node, "BinarySetFunctionType"))
        .ok_or_else(|| unsupported_error("aggregate function type"))?;
    let function = significant_tokens(cst, &function_node)
        .into_iter()
        .next()
        .ok_or_else(|| unsupported_error("aggregate function token"))?;
    let function = match function.text().to_ascii_uppercase().as_str() {
        "AVG" => AggregateFunction::Average,
        "COUNT" => AggregateFunction::Count,
        "MAX" => AggregateFunction::Maximum,
        "MIN" => AggregateFunction::Minimum,
        "SUM" => AggregateFunction::Sum,
        "COLLECT_LIST" => AggregateFunction::CollectList,
        "STDDEV_SAMP" => AggregateFunction::StandardDeviationSample,
        "STDDEV_POP" => AggregateFunction::StandardDeviationPopulation,
        "PERCENTILE_CONT" => AggregateFunction::PercentileContinuous,
        "PERCENTILE_DISC" => AggregateFunction::PercentileDiscrete,
        _ => return unsupported("aggregate function identity"),
    };
    let quantifier = first_descendant(cst, node, "SetQuantifier")
        .map(|quantifier| match quantifier.text().to_string().trim() {
            "ALL" => Ok(SetQuantifier::All),
            "DISTINCT" => Ok(SetQuantifier::Distinct),
            _ => unsupported("aggregate set quantifier"),
        })
        .transpose()?;
    let arguments = if let Some(general) = direct_child_named(cst, node, "GeneralSetFunction") {
        direct_children_named(cst, &general, "ValueExpression")
    } else if let Some(binary) = direct_child_named(cst, node, "BinarySetFunction") {
        ["DependentValueExpression", "IndependentValueExpression"]
            .into_iter()
            .map(|kind| {
                direct_child_named(cst, &binary, kind)
                    .ok_or_else(|| unsupported_error("binary aggregate argument"))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return unsupported("aggregate function form");
    };
    if arguments.is_empty() {
        return unsupported("aggregate without value arguments");
    }
    Ok(Expression::AggregateCall {
        function,
        quantifier,
        arguments: arguments
            .iter()
            .map(|argument| lower_expression(cst, argument))
            .collect::<Result<Vec<_>, _>>()?,
        count_star: false,
    })
}

fn require_name(cst: &ParserCst, node: &Node, expected: &str) -> Result<(), FrontendError> {
    if kind_name(cst, node) == Some(expected) {
        Ok(())
    } else {
        unsupported(expected)
    }
}

fn kind_name<'a>(cst: &'a ParserCst, node: &Node) -> Option<&'a str> {
    cst.kind_name(node.kind())
}

fn direct_child_named(cst: &ParserCst, node: &Node, expected: &str) -> Option<Node> {
    node.children()
        .find(|child| kind_name(cst, child) == Some(expected))
}

fn direct_children_named(cst: &ParserCst, node: &Node, expected: &str) -> Vec<Node> {
    node.children()
        .filter(|child| kind_name(cst, child) == Some(expected))
        .collect()
}

fn first_descendant(cst: &ParserCst, node: &Node, expected: &str) -> Option<Node> {
    node.descendants()
        .find(|child| kind_name(cst, child) == Some(expected))
}

fn descendants_named(cst: &ParserCst, node: &Node, expected: &str) -> Vec<Node> {
    node.descendants()
        .filter(|child| kind_name(cst, child) == Some(expected))
        .collect()
}

fn significant_tokens(cst: &ParserCst, node: &Node) -> Vec<rowan::SyntaxToken<ParserSyntax>> {
    node.descendants_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
        .filter(|token| {
            !matches!(
                cst.kind_name(token.kind()),
                Some("WhitespaceTrivia" | "CommentTrivia")
            )
        })
        .collect()
}

fn unsupported<T>(name: &str) -> Result<T, FrontendError> {
    Err(unsupported_error(name))
}

fn unsupported_exact<T>(name: &str) -> Result<T, FrontendError> {
    Err(FrontendError::Unsupported(name.into()))
}

fn unsupported_error(name: &str) -> FrontendError {
    FrontendError::Unsupported(format!("parser-owned lowering does not admit {name}"))
}
