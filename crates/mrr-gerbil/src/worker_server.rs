//! Isolated native process entry and Gambit standard descriptor adaptation.
use crate::{NativeWorkerError, TemporalHost, TemporalRuntimeError, worker_wire as wire};
use std::{
    io::{self, BufReader, Read, Write},
    thread,
    time::Duration,
};
/// Binary entry point. Never evaluates transport source or grants Data effects.
pub fn run_native_worker() -> io::Result<()> {
    let mut input = BufReader::new(NativeStdio(io::stdin().lock()));
    let mut output = NativeStdio(io::stdout().lock());
    let mut previous = 0;
    while let Some(frame) = wire::read_frame(&mut input)? {
        let (id, op, payload) = wire::decode("request", &frame)
            .filter(|(id, _, _)| *id > previous)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid worker request"))?;
        previous = id;
        let host = TemporalHost;
        let result = match op {
            0 => host
                .refresh_policy(&payload)
                .map_err(NativeWorkerError::Native),
            1 => host
                .refresh_proof_state(&payload)
                .map_err(NativeWorkerError::Native),
            2 => host
                .register_proof(&payload)
                .map_err(NativeWorkerError::Native),
            3 => host
                .admit_derivation(&payload)
                .map_err(NativeWorkerError::Native),
            4 => host
                .current_proof(&payload)
                .map_err(NativeWorkerError::Native),
            5 | 6 => crate::worker_projection::parser_request(op, &payload),
            7 => crate::worker_projection::finite_request(&payload),
            8 => host
                .compile_search_projection(&payload)
                .map_err(NativeWorkerError::Native),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unknown worker operation",
                ));
            }
        };
        let (status, bytes) = match result {
            Ok(bytes) => (0, bytes),
            Err(NativeWorkerError::Native(TemporalRuntimeError::InvalidInput)) => (-1, Vec::new()),
            Err(NativeWorkerError::Native(TemporalRuntimeError::RuntimeUnavailable)) => {
                (-2, Vec::new())
            }
            Err(NativeWorkerError::Native(TemporalRuntimeError::RuntimeInitialization(s))) => {
                (-3, s.code().to_string().into_bytes())
            }
            Err(NativeWorkerError::Native(TemporalRuntimeError::NativeRejected(code))) => {
                (-4, code.to_string().into_bytes())
            }
            Err(NativeWorkerError::Parser(error)) => {
                (-20, crate::worker_errors::parser_error(&error).into_bytes())
            }
            Err(NativeWorkerError::Finite(error)) => {
                (-30, crate::worker_errors::finite_error(error).into_bytes())
            }
            Err(error) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    error.to_string(),
                ));
            }
        };
        if !wire::valid_payload(&bytes) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid worker result",
            ));
        }
        output.write_all(
            wire::encode(
                "response",
                id,
                status,
                std::str::from_utf8(&bytes).expect("checked UTF8"),
            )
            .as_bytes(),
        )?;
        output.flush()?;
    }
    Ok(())
}

// Gambit changes standard descriptors to nonblocking mode during initialization.
// Retry actual readiness failures; the parent enforces operation deadlines.
struct NativeStdio<T>(T);
impl<T: Read> Read for NativeStdio<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.0.read(bytes) {
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    thread::sleep(Duration::from_millis(1))
                }
                result => return result,
            }
        }
    }
}
impl<T: Write> Write for NativeStdio<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        loop {
            match self.0.write(bytes) {
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    thread::sleep(Duration::from_millis(1))
                }
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
