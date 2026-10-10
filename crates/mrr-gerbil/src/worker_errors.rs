//! Typed native error projection; codes and diagnostics remain inert.
#[cfg(feature = "embedded-runtime")]
use crate::native::datum::Value;
use crate::{
    FiniteInferenceError, NativeRuntimeStatus, NativeWorkerError, ParseArtifactLoadError,
    worker_projection::{decode, list, text},
};
// Preserve native error tags and codes; diagnostics stay inert strings.
#[cfg(feature = "embedded-runtime")]
pub(crate) fn parser_error(error: &ParseArtifactLoadError) -> String {
    let fields: Vec<String> = match error {
        ParseArtifactLoadError::InteriorNul => vec!["nul".into()],
        ParseArtifactLoadError::RuntimeUnavailable => vec!["unavailable".into()],
        ParseArtifactLoadError::RuntimeStatus(s) => vec!["runtime".into(), s.code().to_string()],
        ParseArtifactLoadError::ParserFailed {
            call_status,
            result_status,
            diagnostic,
        }
        | ParseArtifactLoadError::NativeDescriptorFailed {
            call_status,
            result_status,
            diagnostic,
        } => vec![
            if matches!(error, ParseArtifactLoadError::ParserFailed { .. }) {
                "parser"
            } else {
                "descriptor"
            }
            .into(),
            call_status.to_string(),
            result_status.to_string(),
            diagnostic.clone().unwrap_or_default(),
        ],
        _ => vec!["invalid-descriptor".into()],
    };
    Value::List(fields.into_iter().map(Value::String).collect()).encode()
}
pub(crate) fn decode_parser_error(
    bytes: &[u8],
) -> Result<ParseArtifactLoadError, NativeWorkerError> {
    let value = decode(bytes)?;
    let values = list(&value)?
        .iter()
        .map(text)
        .collect::<Result<Vec<_>, _>>()?;
    let number = |s: &str| s.parse::<i32>().map_err(|_| NativeWorkerError::Protocol);
    Ok(match values.as_slice() {
        ["nul"] => ParseArtifactLoadError::InteriorNul,
        ["unavailable"] => ParseArtifactLoadError::RuntimeUnavailable,
        ["runtime", code] => {
            ParseArtifactLoadError::RuntimeStatus(NativeRuntimeStatus::from_code(number(code)?))
        }
        ["parser", call, result, diagnostic] => ParseArtifactLoadError::ParserFailed {
            call_status: number(call)?,
            result_status: number(result)?,
            diagnostic: if diagnostic.is_empty() {
                None
            } else {
                Some((*diagnostic).into())
            },
        },
        ["descriptor", call, result, diagnostic] => {
            ParseArtifactLoadError::NativeDescriptorFailed {
                call_status: number(call)?,
                result_status: number(result)?,
                diagnostic: if diagnostic.is_empty() {
                    None
                } else {
                    Some((*diagnostic).into())
                },
            }
        }
        ["invalid-descriptor"] => ParseArtifactLoadError::InvalidHostDescriptor,
        _ => return Err(NativeWorkerError::Protocol),
    })
}
#[cfg(feature = "embedded-runtime")]
pub(crate) fn finite_error(error: FiniteInferenceError) -> String {
    let (tag, code) = match error {
        FiniteInferenceError::CoordinateOverflow => (0, 0),
        FiniteInferenceError::ForeignNode => (1, 0),
        FiniteInferenceError::RuntimeUnavailable => (2, 0),
        FiniteInferenceError::RuntimeInitialization(s) => (3, s.code()),
        FiniteInferenceError::UnsupportedAbi => (4, 0),
        FiniteInferenceError::NativeRejected(n) => (5, n),
        FiniteInferenceError::InvalidNativeCandidate => (6, 0),
        FiniteInferenceError::Worker(_) => (7, 0),
    };
    Value::List(vec![Value::Integer(tag), Value::String(code.to_string())]).encode()
}
pub(crate) fn decode_finite_error(bytes: &[u8]) -> Result<FiniteInferenceError, NativeWorkerError> {
    let value = decode(bytes)?;
    let fields = list(&value)?;
    if fields.len() != 2 {
        return Err(NativeWorkerError::Protocol);
    }
    let code = text(&fields[1])?
        .parse::<i32>()
        .map_err(|_| NativeWorkerError::Protocol)?;
    Ok(match fields[0].integer() {
        Some(0) => FiniteInferenceError::CoordinateOverflow,
        Some(1) => FiniteInferenceError::ForeignNode,
        Some(2) => FiniteInferenceError::RuntimeUnavailable,
        Some(3) => {
            FiniteInferenceError::RuntimeInitialization(NativeRuntimeStatus::from_code(code))
        }
        Some(4) => FiniteInferenceError::UnsupportedAbi,
        Some(5) => FiniteInferenceError::NativeRejected(code),
        Some(6) => FiniteInferenceError::InvalidNativeCandidate,
        _ => return Err(NativeWorkerError::Protocol),
    })
}
