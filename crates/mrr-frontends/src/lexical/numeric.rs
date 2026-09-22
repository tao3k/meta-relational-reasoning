//! Canonical ISO numeric token decoding.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NumericLiteral {
    Integer(i64),
    Decimal(String),
    Approximate(String),
}

pub(crate) fn decode_numeric_literal(value: &str) -> Option<NumericLiteral> {
    if value.starts_with("0x") || value.starts_with("0o") || value.starts_with("0b") {
        return parse_integer_literal(value).map(NumericLiteral::Integer);
    }

    let suffix = value.as_bytes().last().copied();
    let exact_suffix = matches!(suffix, Some(b'm' | b'M'));
    let approximate_suffix = matches!(suffix, Some(b'f' | b'F' | b'd' | b'D'));
    let numeric_end = value.len() - usize::from(exact_suffix || approximate_suffix);
    let scientific = value[..numeric_end].contains(['e', 'E']);
    if !exact_suffix && !approximate_suffix && !scientific && !value[..numeric_end].contains('.') {
        return parse_integer_literal(value).map(NumericLiteral::Integer);
    }

    let mut canonical = String::with_capacity(value.len());
    canonical.extend(
        value[..numeric_end]
            .chars()
            .filter(|character| *character != '_')
            .map(|character| if character == 'e' { 'E' } else { character }),
    );
    if approximate_suffix || (scientific && !exact_suffix) {
        if approximate_suffix {
            canonical
                .push(char::from(suffix.expect("approximate suffix exists")).to_ascii_uppercase());
        }
        return Some(NumericLiteral::Approximate(canonical));
    }
    if exact_suffix || canonical.contains(['.', 'E']) {
        return Some(NumericLiteral::Decimal(canonical));
    }
    parse_integer_literal(&canonical).map(NumericLiteral::Integer)
}

fn parse_integer_literal(value: &str) -> Option<i64> {
    [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find_map(|(prefix, radix)| {
            value
                .strip_prefix(prefix)
                .map(|digits| parse_integer_digits(digits, radix))
        })
        .unwrap_or_else(|| parse_integer_digits(value, 10))
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
