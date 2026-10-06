//! Pinned Lean kernel certification of the exact finite implementation tables.
//! The configured Elan installation and source owner remain explicit trust roots.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use mrr_relation::Value;
use sha2::{Digest, Sha256};

use super::transformation::{
    TransformationBinding, TransformationDefinition, TransformationEndpoint, TransformationError,
    TransformationEvidence, TransformationLimits, TransformationStep, TransformationVerifier,
    digest,
};
use super::transformation_execution::TransformationRuntime;
use super::transformation_finite::{FiniteTransformationCatalog, FiniteTransport};

static NEXT_CHECK: AtomicU64 = AtomicU64::new(0);
const TOOLCHAIN: &str = "leanprover/lean4:v4.31.0";

/// Certificate is created only after the exact generated source is kernel checked.
#[derive(Clone, Debug)]
pub struct FiniteKernelCertificate {
    source: [u8; 32],
    kernel: [u8; 32],
    output: [u8; 32],
}
impl FiniteKernelCertificate {
    #[must_use]
    pub fn source_digest(&self) -> &[u8; 32] {
        &self.source
    }
    #[must_use]
    pub fn kernel_digest(&self) -> &[u8; 32] {
        &self.kernel
    }
}
/// Kernel-certified catalog whose runtime was also exhaustively qualified.
#[derive(Clone, Debug)]
pub struct KernelCheckedFiniteCatalog {
    native: FiniteTransformationCatalog,
    certificate: FiniteKernelCertificate,
    policy: [u8; 32],
    limits: TransformationLimits,
}
impl KernelCheckedFiniteCatalog {
    /// Use an explicitly trusted absolute Elan path and a caller-owned temporary
    /// directory. Kernel checks have a fixed five-second deadline, bounded Lean
    /// memory, and bounded diagnostics. No certificate text is accepted as input.
    pub fn check(
        transports: Vec<FiniteTransport>,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        Self::check_with_contract(transports, binding, limits, elan, temporary, "", &[])
    }
    pub(super) fn check_with_contract(
        transports: Vec<FiniteTransport>,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
        additional_source: &str,
        additional_obligations: &[&str],
    ) -> Result<Self, TransformationError> {
        #[cfg(feature = "native-inference")]
        if !mrr_search::reserve_native_worker_host() {
            return Err(TransformationError::KernelUnavailable {
                diagnostics: "kernel checking requires a Rust process Host; embedded Gambit owns the child reaper. Configure an isolated native worker before embedding".to_owned(),
            });
        }
        if !elan.is_absolute() || !temporary.is_absolute() {
            return Err(TransformationError::Rejected);
        }
        let native = FiniteTransformationCatalog::new(transports, binding, limits)?;
        let source = format!("{}{}", kernel_source(native.tables()), additional_source);
        if source.len() > limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        let files = CheckFiles::new(temporary, &source)?;
        let located = run(elan, &files, &["which", "lean"], limits)?;
        let kernel = PathBuf::from(located.trim());
        if !kernel.is_absolute() || located.lines().count() != 1 {
            return Err(TransformationError::KernelUnavailable {
                diagnostics: format!("Elan returned an invalid kernel location: {located:?}"),
            });
        }
        let version = run(&kernel, &files, &["--version"], limits)?;
        if !version.starts_with("Lean (version 4.31.0,") {
            return Err(TransformationError::EvidenceMismatch);
        }
        let checked = run(
            &kernel,
            &files,
            &[
                "-M",
                "1024",
                "-j",
                "1",
                files.source.to_str().ok_or(TransformationError::Encoding)?,
            ],
            limits,
        )?;
        if checked.contains("sorry")
            || checked.lines().any(|line| {
                line.contains("depends on axioms:")
                    && !((line.starts_with("'MRRFiniteKernel.transport_")
                        && line.ends_with("depends on axioms: [propext]"))
                        || (line.starts_with("'MRRFiniteKernel.certified_")
                            && line.ends_with("depends on axioms: [propext, Quot.sound]"))
                        || (additional_obligations.iter().any(|name| line == format!("'{name}' depends on axioms: [propext]")))
                        || line == "'MRRTransformation.finiteCheck_sound' depends on axioms: [propext, Quot.sound]")
            })
        {
            return Err(TransformationError::KernelRejected {
                diagnostics: checked,
            });
        }
        for name in [
            "compose_sound",
            "compose_forward_assoc",
            "compose_extract_assoc",
            "identity_extract",
            "identity_left",
            "identity_right",
            "compose_assoc",
        ] {
            if !checked.lines().any(|line| {
                line == format!("'MRRTransformation.{name}' does not depend on any axioms")
            }) {
                return Err(TransformationError::KernelRejected {
                    diagnostics: checked,
                });
            }
        }
        if !checked.lines().any(|line| {
            line == "'MRRTransformation.finiteCheck_sound' depends on axioms: [propext, Quot.sound]"
        }) {
            return Err(TransformationError::KernelRejected {
                diagnostics: checked,
            });
        }
        for ordinal in 0..native.definitions().len() {
            if !checked.contains(&format!(
                "'MRRFiniteKernel.transport_{ordinal}' does not depend on any axioms"
            )) && !checked.contains(&format!(
                "'MRRFiniteKernel.transport_{ordinal}' depends on axioms: [propext]"
            )) {
                return Err(TransformationError::KernelRejected {
                    diagnostics: checked,
                });
            }
            if !checked.lines().any(|line| {
                line == format!(
                    "'MRRFiniteKernel.certified_{ordinal}' depends on axioms: [propext, Quot.sound]"
                )
            }) {
                return Err(TransformationError::KernelRejected {
                    diagnostics: checked,
                });
            }
        }
        for name in additional_obligations {
            if !checked.lines().any(|line| {
                line == format!("'{name}' does not depend on any axioms")
                    || line == format!("'{name}' depends on axioms: [propext]")
            }) {
                return Err(TransformationError::KernelRejected {
                    diagnostics: checked,
                });
            }
        }
        let certificate = FiniteKernelCertificate {
            source: Sha256::digest(source.as_bytes()).into(),
            kernel: Sha256::digest(version.as_bytes()).into(),
            output: Sha256::digest(checked.as_bytes()).into(),
        };
        let checker: [u8; 32] = Sha256::digest(include_bytes!("transformation_kernel.rs")).into();
        let policy = digest(
            &(
                "mrr.finite-kernel-native.v1",
                checker,
                TransformationVerifier::policy(&native),
                certificate.source,
                certificate.kernel,
                certificate.output,
            ),
            limits,
        )?;
        Ok(Self {
            native,
            certificate,
            policy,
            limits,
        })
    }
    /// Compose actual finite tables, kernel check the new artifact, and attach
    /// it to the same owner selection only after both checks complete.
    pub fn compose(
        &self,
        first: usize,
        second: usize,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        let native = self.native.compose(first, second)?;
        let tables = ciborium::de::from_reader(native.to_bytes()?.as_slice())
            .map_err(|_| TransformationError::Encoding)?;
        let binding = self.native.selected_binding().clone();
        let mut checked = Self::check(tables, binding, self.limits, elan, temporary)?;
        checked.native.share_authority(&self.native)?;
        Ok(checked)
    }
    #[must_use]
    pub fn native(&self) -> &FiniteTransformationCatalog {
        &self.native
    }
    #[must_use]
    pub fn certificate(&self) -> &FiniteKernelCertificate {
        &self.certificate
    }
    pub fn evidence(&self, ordinal: usize) -> Result<TransformationEvidence, TransformationError> {
        let mut evidence = self.native.evidence(ordinal)?;
        evidence.checker = self.policy;
        evidence.toolchain = self.certificate.kernel;
        evidence.environment = self.certificate.source;
        evidence.assumptions = digest(
            &"trusted-elan-owner-finite-spec-standard-propext-quot-sound.v1",
            self.limits,
        )?;
        Ok(evidence)
    }
}
struct CheckFiles {
    source: PathBuf,
    output: PathBuf,
}
impl CheckFiles {
    fn new(root: &Path, source: &str) -> Result<Self, TransformationError> {
        let stem = format!(
            "mrr-finite-kernel-{}-{}",
            std::process::id(),
            NEXT_CHECK.fetch_add(1, Ordering::Relaxed)
        );
        let source_path = root.join(format!("{stem}.lean"));
        let output_path = root.join(format!("{stem}.log"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&source_path)
            .map_err(|_| TransformationError::Encoding)?;
        if OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .is_err()
        {
            let _ = fs::remove_file(&source_path);
            return Err(TransformationError::Encoding);
        }
        let files = Self {
            source: source_path,
            output: output_path,
        };
        file.write_all(source.as_bytes())
            .map_err(|_| TransformationError::Encoding)?;
        Ok(files)
    }
}
impl Drop for CheckFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.source);
        let _ = fs::remove_file(&self.output);
    }
}
struct OwnedKernelProcess(std::process::Child);
impl Drop for OwnedKernelProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn run(
    elan: &Path,
    files: &CheckFiles,
    args: &[&str],
    limits: TransformationLimits,
) -> Result<String, TransformationError> {
    let output = File::create(&files.output).map_err(|_| TransformationError::Encoding)?;
    let mut child = OwnedKernelProcess(
        Command::new(elan)
            .env("ELAN_TOOLCHAIN", TOOLCHAIN)
            .args(args)
            .stdin(Stdio::null())
            .stdout(
                output
                    .try_clone()
                    .map_err(|_| TransformationError::Encoding)?,
            )
            .stderr(output)
            .spawn()
            .map_err(|error| TransformationError::KernelUnavailable {
                diagnostics: format!("cannot launch {}: {error}", elan.display()),
            })?,
    );
    let started = Instant::now();
    let status = loop {
        if fs::metadata(&files.output)
            .map_err(|_| TransformationError::Encoding)?
            .len()
            > limits.max_bytes.get() as u64
            || started.elapsed() > Duration::from_secs(5)
        {
            let _ = child.0.kill();
            let _ = child.0.wait();
            return Err(TransformationError::Budget);
        }
        if let Some(status) =
            child
                .0
                .try_wait()
                .map_err(|error| TransformationError::KernelUnavailable {
                    diagnostics: format!("cannot poll {}: {error}", elan.display()),
                })?
        {
            break status;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let mut text = String::new();
    File::open(&files.output)
        .map_err(|_| TransformationError::Encoding)?
        .take(limits.max_bytes.get() as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|_| TransformationError::Encoding)?;
    if text.len() > limits.max_bytes.get() {
        return Err(TransformationError::Budget);
    }
    if !status.success() {
        return Err(TransformationError::KernelRejected { diagnostics: text });
    }
    Ok(text)
}
// Balanced selection keeps kernel reduction bounded by logarithmic table depth.
// Equal adjacent subtrees collapse without changing any table cell.
pub(super) fn lookup(values: &[String], variable: &str, offset: usize) -> String {
    if values.iter().all(|value| value == &values[0]) {
        return values[0].clone();
    }
    let middle = values.len() / 2;
    format!(
        "(if {variable} < {} then {} else {})",
        offset + middle,
        lookup(&values[..middle], variable, offset),
        lookup(&values[middle..], variable, offset + middle)
    )
}
fn correctness(rows: &[Vec<usize>]) -> String {
    lookup(
        &rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|answer| format!("answer.val == {answer}"))
                    .collect::<Vec<_>>()
                    .join(" || ")
            })
            .collect::<Vec<_>>(),
        "input.val",
        0,
    )
}
fn numbers(values: &[usize], variable: &str) -> String {
    // Compress only progressions matched against every original table cell.
    // The finite input coordinate starts at zero for both forwards and extractors.
    let first = values[0];
    if values
        .iter()
        .enumerate()
        .all(|(i, value)| first.checked_add(i) == Some(*value))
    {
        return format!("({variable} + {first})");
    }
    if values
        .iter()
        .enumerate()
        .all(|(i, value)| first.checked_sub(i) == Some(*value))
    {
        return format!("({first} - {variable})");
    }
    lookup(
        &values.iter().map(usize::to_string).collect::<Vec<_>>(),
        variable,
        0,
    )
}
fn kernel_source(transports: &[FiniteTransport]) -> String {
    let mut source = include_str!("../../../proofs/MRRProof/Transformation.lean")
        .split("\nnamespace MRRTransformation\n\n-- Nontrivial")
        .next()
        .unwrap_or_default()
        .to_owned();
    source.push_str(include_str!(
        "../../../proofs/MRRProof/FiniteTransformation.lean"
    ));
    source.push_str(
        "\nset_option maxRecDepth 4096\nnamespace MRRFiniteKernel\nopen MRRTransformation\n",
    );
    for (i, t) in transports.iter().enumerate() {
        let (n, m, p, q) = (
            t.source.correct.len(),
            t.source.answers,
            t.target.correct.len(),
            t.target.answers,
        );
        source.push_str(&format!("def source_{i} : FiniteSpecification {n} {m} := \u{27e8}fun input answer => {source}\u{27e9}\ndef target_{i} : FiniteSpecification {p} {q} := \u{27e8}fun input answer => {target}\u{27e9}\ndef forward_{i} (input : Fin {n}) : Fin {p} := \u{27e8}({forward}) % {p}, Nat.mod_lt _ (by decide)\u{27e9}\ndef extract_{i} (input : Fin {n}) (answer : Fin {q}) : Fin {m} := \u{27e8}({extract}) % {m}, Nat.mod_lt _ (by decide)\u{27e9}\ntheorem transport_{i} : finiteCheck source_{i} target_{i} forward_{i} extract_{i} = true := by decide\n#print axioms transport_{i}\ndef certified_{i} : CertifiedTransformation source_{i}.problem target_{i}.problem := certifyFinite source_{i} target_{i} forward_{i} extract_{i} transport_{i}\n#print axioms certified_{i}\n", source=correctness(&t.source.correct), target=correctness(&t.target.correct), forward=numbers(&t.forward, "input.val"), extract=lookup(&t.extract.iter().map(|row| numbers(row, "answer.val")).collect::<Vec<_>>(), "input.val", 0)));
    }
    source.push_str("end MRRFiniteKernel\n");
    source
}
impl TransformationVerifier for KernelCheckedFiniteCatalog {
    fn policy(&self) -> [u8; 32] {
        self.policy
    }
    fn check_definition(
        &self,
        definition: &TransformationDefinition,
        evidence: &TransformationEvidence,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        let ordinal = self
            .native
            .definitions()
            .iter()
            .position(|d| d == definition)
            .ok_or(TransformationError::Unknown)?;
        if evidence != &self.evidence(ordinal)? {
            return Err(TransformationError::EvidenceMismatch);
        }
        let receipt =
            self.native
                .check_definition(definition, &self.native.evidence(ordinal)?, binding)?;
        digest(&(self.policy, receipt), self.limits)
    }
    fn check_step(
        &self,
        step: &TransformationStep,
        binding: &TransformationBinding,
        ordinal: usize,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        digest(
            &(
                self.policy,
                self.native.check_step(step, binding, ordinal, prior)?,
            ),
            self.limits,
        )
    }
    fn check_solver(
        &self,
        target: &TransformationEndpoint,
        solver: &[u8; 32],
        input: &[u8; 32],
        binding: &TransformationBinding,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        digest(
            &(
                self.policy,
                self.native
                    .check_solver(target, solver, input, binding, prior)?,
            ),
            self.limits,
        )
    }
}
impl TransformationRuntime for KernelCheckedFiniteCatalog {
    fn identity(&self) -> [u8; 32] {
        self.policy
    }
    fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.native.forward(step, input, binding)
    }
    fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.native.solve(solver, target, input, binding)
    }
    fn extract(
        &self,
        step: &TransformationStep,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.native.extract(step, input, answer, binding)
    }
    fn check_answer(
        &self,
        endpoint: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.native.check_answer(endpoint, input, answer, binding)
    }
}
