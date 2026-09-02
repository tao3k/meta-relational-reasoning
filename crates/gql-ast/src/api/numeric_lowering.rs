//! Canonical lowering for ISO GQL exact and approximate numeric tokens.

use gql_source::Span;

use super::Expression;

pub(super) fn lower_numeric_literal(value: &str, span: Span) -> Option<Expression> {
    if value.starts_with("0x") || value.starts_with("0o") || value.starts_with("0b") {
        return parse_integer_literal(value).map(|value| Expression::Integer(value, span));
    }

    let suffix = value.as_bytes().last().copied();
    let is_exact_suffix = matches!(suffix, Some(b'm' | b'M'));
    let is_approximate_suffix = matches!(suffix, Some(b'f' | b'F' | b'd' | b'D'));
    let numeric_end = value.len() - usize::from(is_exact_suffix || is_approximate_suffix);
    let is_scientific = value[..numeric_end].contains('e') || value[..numeric_end].contains('E');
    if !is_exact_suffix
        && !is_approximate_suffix
        && !is_scientific
        && !value[..numeric_end].contains('.')
    {
        return parse_integer_literal(value).map(|value| Expression::Integer(value, span));
    }

    let mut canonical = String::with_capacity(value.len());
    canonical.extend(
        value[..numeric_end]
            .chars()
            .filter(|character| *character != '_')
            .map(|character| if character == 'e' { 'E' } else { character }),
    );
    if is_approximate_suffix || (is_scientific && !is_exact_suffix) {
        if is_approximate_suffix {
            canonical
                .push(char::from(suffix.expect("approximate suffix exists")).to_ascii_uppercase());
        }
        return Some(Expression::ApproximateNumeric(canonical, span));
    }
    if is_exact_suffix || canonical.contains('.') || canonical.contains('E') {
        return Some(Expression::Decimal(canonical, span));
    }
    parse_integer_literal(&canonical).map(|value| Expression::Integer(value, span))
}

fn parse_integer_literal(value: &str) -> Option<i64> {
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = value.strip_prefix(prefix) {
            return parse_integer_digits(digits, radix);
        }
    }
    parse_integer_digits(value, 10)
}

fn parse_integer_digits(digits: &str, radix: u32) -> Option<i64> {
    let mut digits = digits.bytes().filter(|byte| *byte != b'_');
    let first = i64::from(char::from(digits.next()?).to_digit(radix)?);
    digits.try_fold(first, |value, digit| {
        value
            .checked_mul(i64::from(radix))?
            .checked_add(i64::from(char::from(digit).to_digit(radix)?))
    })
}
