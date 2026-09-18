//! Recursive value and schema validation owned by the relation boundary.

use std::collections::BTreeSet;

use crate::api::{
    FloatWidth, RelationConstraint, RelationError, RelationField, TemporalUnit, TimezonePolicy,
    Value, ValueSchema,
};

pub(crate) fn validate_fields(fields: &[RelationField]) -> Result<(), RelationError> {
    if fields.is_empty() {
        return Err(RelationError::EmptyFields);
    }
    validate_property_fields(fields)
}

pub(crate) fn validate_property_fields(fields: &[RelationField]) -> Result<(), RelationError> {
    let mut names = BTreeSet::new();
    for field in fields {
        field.schema().validate()?;
        if !names.insert(field.name()) {
            return Err(RelationError::DuplicateFieldName(field.name().to_owned()));
        }
    }
    Ok(())
}

pub(crate) fn validate_constraints(
    fields: &[RelationField],
    constraints: &[RelationConstraint],
) -> Result<(), RelationError> {
    let names = fields
        .iter()
        .map(RelationField::name)
        .collect::<BTreeSet<_>>();
    let validate_names = |constraint: &str, selected: &[String]| {
        if selected.is_empty() {
            return Err(RelationError::InvalidConstraint(format!(
                "{constraint} has no fields"
            )));
        }
        let mut unique = BTreeSet::new();
        for name in selected {
            if !names.contains(name.as_str()) || !unique.insert(name) {
                return Err(RelationError::InvalidConstraint(format!(
                    "{constraint} references an unknown or duplicate field `{name}`"
                )));
            }
        }
        Ok(())
    };
    for constraint in constraints {
        match constraint {
            RelationConstraint::Key(selected) => validate_names("key", selected)?,
            RelationConstraint::Unique(selected) => validate_names("unique", selected)?,
            RelationConstraint::FunctionalDependency {
                determinant,
                dependent,
            } => {
                validate_names("functional determinant", determinant)?;
                validate_names("functional dependent", dependent)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_field_value(
    field: &RelationField,
    value: &Value,
    path: &str,
) -> Result<(), RelationError> {
    if matches!(value, Value::Null) {
        return if field.nullable() {
            Ok(())
        } else {
            Err(RelationError::NullNotAllowed(path.to_owned()))
        };
    }
    validate_value(field.schema(), value, path)
}

pub(crate) fn validate_value(
    schema: &ValueSchema,
    value: &Value,
    path: &str,
) -> Result<(), RelationError> {
    let matches_kind = matches!(
        (schema, value),
        (ValueSchema::Entity, Value::Entity(_))
            | (ValueSchema::Boolean, Value::Boolean(_))
            | (ValueSchema::Integer, Value::Integer(_))
            | (ValueSchema::Decimal { .. }, Value::Decimal(_))
            | (ValueSchema::Float { .. }, Value::Float(_))
            | (ValueSchema::String, Value::String(_))
            | (ValueSchema::ByteString, Value::ByteString(_))
            | (ValueSchema::Date, Value::Date(_))
            | (ValueSchema::Time { .. }, Value::Time(_))
            | (ValueSchema::Timestamp { .. }, Value::Timestamp(_))
            | (ValueSchema::Duration, Value::Duration(_))
            | (ValueSchema::List { .. }, Value::List(_))
            | (ValueSchema::Record { .. }, Value::Record(_))
    );
    if !matches_kind {
        return Err(RelationError::TypeMismatch {
            field: path.to_owned(),
            expected: schema.clone(),
            actual: value.kind(),
        });
    }
    match (schema, value) {
        (ValueSchema::Decimal { precision, scale }, Value::Decimal(text))
            if !valid_decimal(text, *precision, *scale) =>
        {
            return Err(RelationError::InvalidValue {
                field: path.to_owned(),
                reason: "decimal is not canonical for its precision and scale",
            });
        }
        (ValueSchema::Float { width }, Value::Float(text)) if !valid_float(text, *width) => {
            return Err(RelationError::InvalidValue {
                field: path.to_owned(),
                reason: "float is non-finite or not canonical for its width",
            });
        }
        (ValueSchema::Date, Value::Date(text)) if !valid_date(text) => {
            return Err(RelationError::InvalidValue {
                field: path.to_owned(),
                reason: "date must be a valid canonical YYYY-MM-DD value",
            });
        }
        (ValueSchema::Time { unit, timezone }, Value::Time(text))
            if !valid_time(text, *unit, *timezone) =>
        {
            return Err(RelationError::InvalidValue {
                field: path.to_owned(),
                reason: "time does not match its unit and timezone policy",
            });
        }
        (ValueSchema::Timestamp { unit, timezone }, Value::Timestamp(text)) => {
            let valid = text
                .split_once('T')
                .is_some_and(|(date, time)| valid_date(date) && valid_time(time, *unit, *timezone));
            if !valid {
                return Err(RelationError::InvalidValue {
                    field: path.to_owned(),
                    reason: "timestamp does not match its unit and timezone policy",
                });
            }
        }
        (ValueSchema::Duration, Value::Duration(text)) if !valid_duration(text) => {
            return Err(RelationError::InvalidValue {
                field: path.to_owned(),
                reason: "duration must be a canonical ISO 8601 duration",
            });
        }
        (
            ValueSchema::List {
                element,
                element_nullable,
            },
            Value::List(values),
        ) => {
            for (index, item) in values.iter().enumerate() {
                let item_path = format!("{path}[{index}]");
                if matches!(item, Value::Null) {
                    if !element_nullable {
                        return Err(RelationError::NullNotAllowed(item_path));
                    }
                } else {
                    validate_value(element, item, &item_path)?;
                }
            }
        }
        (ValueSchema::Record { fields }, Value::Record(values)) => {
            if fields.len() != values.len() {
                return Err(RelationError::ArityMismatch {
                    expected: fields.len(),
                    actual: values.len(),
                });
            }
            for (field, (name, item)) in fields.iter().zip(values) {
                if field.name() != name {
                    return Err(RelationError::InvalidValue {
                        field: path.to_owned(),
                        reason: "record fields must match the declared order and names",
                    });
                }
                validate_field_value(field, item, &format!("{path}.{name}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn valid_decimal(text: &str, precision: u8, scale: u8) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    if unsigned.is_empty() || text.starts_with('+') {
        return false;
    }
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() != usize::from(scale)
        || (whole.len() > 1 && whole.starts_with('0'))
        || (scale == 0 && unsigned.contains('.'))
    {
        return false;
    }
    let digits = whole.trim_start_matches('0').len() + fraction.len();
    digits.max(1) <= usize::from(precision)
        && !(text.starts_with('-') && unsigned.chars().all(|c| c == '0' || c == '.'))
}

fn valid_float(text: &str, width: FloatWidth) -> bool {
    match width {
        FloatWidth::Binary32 => text
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .is_some_and(|value| value.to_string() == text),
        FloatWidth::Binary64 => text
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .is_some_and(|value| value.to_string() == text),
    }
}

fn valid_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let Ok(year) = text[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = text[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = text[8..10].parse::<u8>() else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn valid_time(text: &str, unit: TemporalUnit, timezone: TimezonePolicy) -> bool {
    let body = match timezone {
        TimezonePolicy::Utc => match text.strip_suffix('Z') {
            Some(body) => body,
            None => return false,
        },
        TimezonePolicy::Naive if text.ends_with('Z') => return false,
        TimezonePolicy::Naive => text,
    };
    let (clock, fraction) = body.split_once('.').unwrap_or((body, ""));
    let clock_bytes = clock.as_bytes();
    if clock_bytes.len() != 8
        || clock_bytes[2] != b':'
        || clock_bytes[5] != b':'
        || !clock_bytes[..2].iter().all(u8::is_ascii_digit)
        || !clock_bytes[3..5].iter().all(u8::is_ascii_digit)
        || !clock_bytes[6..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let Ok(hour) = clock[0..2].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = clock[3..5].parse::<u8>() else {
        return false;
    };
    let Ok(second) = clock[6..8].parse::<u8>() else {
        return false;
    };
    hour < 24
        && minute < 60
        && second < 60
        && fraction.len() == unit.fractional_digits()
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
        && (unit != TemporalUnit::Second || !body.contains('.'))
}

fn valid_duration(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    let bytes = body.as_bytes();
    if bytes.first() != Some(&b'P') || bytes.len() == 1 {
        return false;
    }

    let mut index = 1;
    let mut in_time = false;
    let mut any_component = false;
    let mut time_component = false;
    let mut non_zero = false;
    let mut date_rank = 0;
    let mut time_rank = 0;
    let mut has_week = false;
    let mut has_non_week_date = false;

    while index < bytes.len() {
        if bytes[index] == b'T' {
            if in_time || index + 1 == bytes.len() {
                return false;
            }
            in_time = true;
            index += 1;
            continue;
        }

        let number_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            non_zero |= bytes[index] != b'0';
            index += 1;
        }
        if index == number_start || (index - number_start > 1 && bytes[number_start] == b'0') {
            return false;
        }
        let mut fractional = false;
        if index < bytes.len() && bytes[index] == b'.' {
            fractional = true;
            index += 1;
            let fraction_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                non_zero |= bytes[index] != b'0';
                index += 1;
            }
            if index == fraction_start || bytes[index - 1] == b'0' {
                return false;
            }
        }
        if index == bytes.len() {
            return false;
        }
        let designator = bytes[index];
        index += 1;

        if in_time {
            let rank = match designator {
                b'H' if !fractional => 1,
                b'M' if !fractional => 2,
                b'S' => 3,
                _ => return false,
            };
            if rank <= time_rank {
                return false;
            }
            time_rank = rank;
            time_component = true;
        } else {
            let rank = match designator {
                b'Y' if !fractional => 1,
                b'M' if !fractional => 2,
                b'W' if !fractional => 3,
                b'D' if !fractional => 4,
                _ => return false,
            };
            if rank <= date_rank {
                return false;
            }
            date_rank = rank;
            if designator == b'W' {
                has_week = true;
            } else {
                has_non_week_date = true;
            }
            if has_week && has_non_week_date {
                return false;
            }
        }
        any_component = true;
    }

    any_component && (!in_time || time_component) && (!text.starts_with('-') || non_zero)
}
