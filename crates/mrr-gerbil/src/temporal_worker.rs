//! Persistent isolated native owner. Inert Scheme v1 frames carry no authority.
use crate::{NativeRuntimeStatus, TemporalRuntimeError, worker_wire as wire};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const IDLE: Duration = Duration::from_secs(5);
const WALL: Duration = Duration::from_secs(45);

#[derive(Debug)]
pub enum TemporalWorkerError {
    InvalidInput,
    ModeConflict,
    Closed,
    Protocol,
    Timeout,
    Io(io::Error),
    Native(TemporalRuntimeError),
}
impl std::fmt::Display for TemporalWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TemporalWorkerError {}
impl From<io::Error> for TemporalWorkerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// One serialized session, with no automatic restart or embedded fallback.
/// Closing destroys its process-local proof registry.
pub struct TemporalWorker {
    child: Child,
    commands: Option<mpsc::SyncSender<String>>,
    replies: mpsc::Receiver<io::Result<Vec<u8>>>,
    threads: Vec<JoinHandle<()>>,
    progress: Arc<Mutex<Instant>>,
    next: u64,
}
impl TemporalWorker {
    /// Launch an explicitly selected trusted absolute executable.
    pub fn start(executable: &Path) -> Result<Self, TemporalWorkerError> {
        if !executable.is_absolute() {
            return Err(TemporalWorkerError::InvalidInput);
        }
        if !crate::native::claim_worker_host() {
            return Err(TemporalWorkerError::ModeConflict);
        }
        let mut child = Command::new(executable)
            .env("MRR_NATIVE_PROGRESS", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (commands, requests) = mpsc::sync_channel::<String>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let progress = Arc::new(Mutex::new(Instant::now()));
        let clock = Arc::clone(&progress);
        let io_thread = thread::spawn(move || {
            let mut input = stdin;
            let mut output = BufReader::new(stdout);
            while let Ok(request) = requests.recv() {
                let result = (|| {
                    input.write_all(request.as_bytes())?;
                    input.flush()?;
                    wire::read_frame(&mut output)?.ok_or_else(|| {
                        io::Error::new(io::ErrorKind::UnexpectedEof, "worker closed")
                    })
                })();
                let failed = result.is_err();
                if responses.try_send(result).is_err() || failed {
                    break;
                }
            }
        });
        let diagnostics = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            loop {
                let mut line = Vec::new();
                match reader.by_ref().take(4097).read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        // Only completed/native boundaries reset the idle clock.
                        if line.len() <= 4096
                            && matches!(
                                line.as_slice(),
                                b"mrr-native: runtime initialization started\n"
                                    | b"mrr-native: finite projection initialized\n"
                                    | b"mrr-native: grammar projection initialized\n"
                                    | b"mrr-native: runtime initialization returned\n"
                                    | b"mrr-native: operation started on owner thread\n"
                                    | b"mrr-native: operation returned on owner thread\n"
                            )
                            && let Ok(mut time) = clock.lock()
                        {
                            *time = Instant::now();
                        }
                        let _ = io::stderr().write_all(&line);
                    }
                }
            }
        });
        Ok(Self {
            child,
            commands: Some(commands),
            replies,
            threads: vec![io_thread, diagnostics],
            progress,
            next: 1,
        })
    }
    fn invoke(&mut self, operation: i32, payload: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        if !wire::valid_payload(payload) {
            return Err(TemporalWorkerError::InvalidInput);
        }
        let sender = self.commands.as_ref().ok_or(TemporalWorkerError::Closed)?;
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(TemporalWorkerError::Protocol)?;
        let start = Instant::now();
        *self
            .progress
            .lock()
            .map_err(|_| TemporalWorkerError::Closed)? = start;
        let request = wire::encode(
            "request",
            id,
            operation,
            std::str::from_utf8(payload).expect("checked UTF8"),
        );
        let result = if sender.try_send(request).is_err() {
            Err(TemporalWorkerError::Closed)
        } else {
            self.await_reply(id, start)
        };
        if result.is_err()
            && !matches!(
                result,
                Err(TemporalWorkerError::Native(
                    TemporalRuntimeError::NativeRejected(_)
                ))
            )
        {
            self.cancel();
        }
        result
    }
    fn await_reply(&self, id: u64, start: Instant) -> Result<Vec<u8>, TemporalWorkerError> {
        loop {
            match self.replies.recv_timeout(Duration::from_millis(50)) {
                Ok(frame) => {
                    let (received, status, payload) =
                        wire::decode("response", &frame?).ok_or(TemporalWorkerError::Protocol)?;
                    if received != id {
                        return Err(TemporalWorkerError::Protocol);
                    }
                    return decode_result(status, payload);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(TemporalWorkerError::Closed);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let progress = *self
                        .progress
                        .lock()
                        .map_err(|_| TemporalWorkerError::Closed)?;
                    if start.elapsed() >= WALL || progress.elapsed() >= IDLE {
                        return Err(TemporalWorkerError::Timeout);
                    }
                }
            }
        }
    }
    pub fn refresh_policy(&mut self, p: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        self.invoke(0, p)
    }
    pub fn refresh_proof_state(&mut self, p: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        self.invoke(1, p)
    }
    pub fn register_proof(&mut self, p: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        self.invoke(2, p)
    }
    pub fn admit_derivation(&mut self, p: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        self.invoke(3, p)
    }
    pub fn current_proof(&mut self, p: &[u8]) -> Result<Vec<u8>, TemporalWorkerError> {
        self.invoke(4, p)
    }
    /// Terminal cancellation reaps the actual worker child.
    pub fn cancel(&mut self) {
        self.commands.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
    /// EOF closes a healthy session; a stuck or unsuccessful exit is rejected.
    pub fn close(mut self) -> Result<(), TemporalWorkerError> {
        self.commands.take();
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                for handle in self.threads.drain(..) {
                    let _ = handle.join();
                }
                return if status.success() {
                    Ok(())
                } else {
                    Err(TemporalWorkerError::Closed)
                };
            }
            if start.elapsed() >= IDLE {
                self.cancel();
                return Err(TemporalWorkerError::Timeout);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for TemporalWorker {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn decode_result(status: i32, payload: Vec<u8>) -> Result<Vec<u8>, TemporalWorkerError> {
    let native = match status {
        0 => return Ok(payload),
        -1 if payload.is_empty() => TemporalRuntimeError::InvalidInput,
        -2 if payload.is_empty() => TemporalRuntimeError::RuntimeUnavailable,
        -3 | -4 => {
            let code = std::str::from_utf8(&payload)
                .ok()
                .and_then(|s| s.parse::<i32>().ok())
                .ok_or(TemporalWorkerError::Protocol)?;
            if status == -3 {
                TemporalRuntimeError::RuntimeInitialization(NativeRuntimeStatus::from_code(code))
            } else {
                TemporalRuntimeError::NativeRejected(code)
            }
        }
        _ => return Err(TemporalWorkerError::Protocol),
    };
    Err(TemporalWorkerError::Native(native))
}
