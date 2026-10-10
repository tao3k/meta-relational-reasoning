//! Restricted inert Scheme v1 envelopes; no evaluator or JSON codec.
use std::io::{self, BufRead, Read};

pub(crate) const PAYLOAD_LIMIT: usize = 1_048_576;
pub(crate) const FRAME_LIMIT: usize = PAYLOAD_LIMIT * 2 + 256;

pub(crate) fn valid_payload(value: &[u8]) -> bool {
    value.len() <= PAYLOAD_LIMIT
        && std::str::from_utf8(value).is_ok_and(|s| {
            s.chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        })
}

pub(crate) fn encode(kind: &str, id: u64, operation: i32, payload: &str) -> String {
    let mut frame = format!("(mrr.native-worker.{kind}.v1 {id} {operation} \"");
    for c in payload.chars() {
        match c {
            '\\' => frame.push_str("\\\\"),
            '"' => frame.push_str("\\\""),
            '\n' => frame.push_str("\\n"),
            '\r' => frame.push_str("\\r"),
            '\t' => frame.push_str("\\t"),
            _ => frame.push(c),
        }
    }
    frame.push_str("\")\n");
    frame
}

pub(crate) fn decode(kind: &str, frame: &[u8]) -> Option<(u64, i32, Vec<u8>)> {
    let text = std::str::from_utf8(frame).ok()?;
    let rest = text.strip_prefix(&format!("(mrr.native-worker.{kind}.v1 "))?;
    let (id, rest) = rest.split_once(' ')?;
    let (operation, rest) = rest.split_once(' ')?;
    let id = id.parse::<u64>().ok().filter(|n| *n != 0)?;
    let operation = operation.parse::<i32>().ok()?;
    let mut chars = rest.strip_prefix('"')?.chars();
    let mut payload = String::new();
    loop {
        match chars.next()? {
            '"' => break,
            '\\' => payload.push(match chars.next()? {
                '\\' => '\\',
                '"' => '"',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => return None,
            }),
            c if !c.is_control() => payload.push(c),
            _ => return None,
        }
        if payload.len() > PAYLOAD_LIMIT {
            return None;
        }
    }
    if chars.as_str() != ")\n" || !valid_payload(payload.as_bytes()) {
        return None;
    }
    Some((id, operation, payload.into_bytes()))
}

pub(crate) fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    let n = reader
        .take((FRAME_LIMIT + 1) as u64)
        .read_until(b'\n', &mut frame)?;
    if n == 0 {
        return Ok(None);
    }
    if n > FRAME_LIMIT || frame.last() != Some(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded Scheme frame required",
        ));
    }
    Ok(Some(frame))
}
