//! Thin Cargo adapter for the canonical Gerbil AOT program builder.

use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use gerbil_scheme_native_build::{
    NativeHeaderInput, ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramArchiveRequest, build_program_archive_with_contract,
    gerbil_command, observe_program_archive_operation, prepare_gsc_progress_launcher,
    run_native_process, source_workspace,
};

const REQUIRED_MODULES: &[&str] = &[
    "meta-relational-reasoning/scheme/grammar/native-program",
    "meta-relational-reasoning/scheme/grammar/native",
    "meta-relational-reasoning/scheme/reasoning/finite-native",
    "gerbil-ascent/table/expression",
    "gerbil-parser/src/ffi/parse-artifact-v1-native",
];

const FORBIDDEN_RUNTIME_MODULES: &[&str] = &[
    "asp-gerbil-scheme/src/build-api/package-build",
    "asp-gerbil-scheme/src/build-api/native-import-closure",
    "asp-gerbil-scheme/src/support/time",
    "poo-flow/src/module-system/observability/interface",
    "poo-flow/src/module-system/observability/source-admission",
    "poo-flow/src/module-system/observability/build-projection",
    "poo-flow/src/module-system/observability/config",
];

struct CargoObserver;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObservationChannel {
    Silent,
    Trace,
    Warning,
}

impl ProgramArchiveObserver for CargoObserver {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        match observation_channel(
            observation.phase,
            observation.state,
            observation.elapsed,
            gerbil_build_verbose_level(),
        ) {
            ObservationChannel::Silent => {}
            ObservationChannel::Trace => {
                let mut output = io::stderr().lock();
                writeln!(output, "[mrr-gerbil-native] {observation}")
                    .expect("write native build trace");
                output.flush().expect("flush native build trace");
            }
            ObservationChannel::Warning => {
                let mut output = io::stdout().lock();
                writeln!(output, "cargo:warning=[mrr-gerbil-native] {observation}")
                    .expect("write failed native build observation");
                output
                    .flush()
                    .expect("flush failed native build observation");
            }
        }
    }

    fn observe_source_input(&self, source: &Path) {
        println!("cargo:rerun-if-changed={}", source.display());
    }
}

/// Materializes the MRR program and delegates its complete AOT graph to the
/// upstream Gerbil-to-Rust builder.
pub fn build_native_archive(manifest: &Path) {
    // SDK selection changes the imported module closure and compiler inputs.
    // A cached archive from another prefix must not survive that selection.
    for name in ["GERBIL_PATH", "GERBIL_LOADPATH", "GERBIL_HOME", "GAMBOPT"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-env-changed=GERBIL_GSC");
    println!("cargo:rerun-if-env-changed=GERBIL_GXI");
    println!("cargo:rerun-if-env-changed=GERBIL_GXPKG");
    println!("cargo:rerun-if-env-changed=GERBIL_BUILD_VERBOSE");
    let build = NativeBuild::new(manifest);
    let observer = CargoObserver;
    build.prepare_package(&observer);
    build.stage_program(&observer);
    build.package_archive(&observer);
}

struct NativeBuild {
    workspace: PathBuf,
    program_source: PathBuf,
    program_stage: PathBuf,
    program_manifest: PathBuf,
    package_prefix: PathBuf,
    package_load_path: std::ffi::OsString,
    gxi: PathBuf,
    gsc: PathBuf,
    gxpkg: PathBuf,
}

impl NativeBuild {
    fn new(manifest: &Path) -> Self {
        let workspace = manifest
            .ancestors()
            .nth(2)
            .expect("workspace root")
            .to_path_buf();
        let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
        let program_stage = out.join("gerbil-program");
        let package_prefix = out.join("gerbil-package");
        let sdk_prefix =
            env::var_os("GERBIL_PATH").map_or_else(|| workspace.join(".gerbil"), PathBuf::from);
        let mut libraries = vec![package_prefix.join("lib"), sdk_prefix.join("lib")];
        if let Some(paths) = env::var_os("GERBIL_LOADPATH") {
            libraries.extend(env::split_paths(&paths));
        }
        let package_load_path = env::join_paths(libraries).expect("package library search path");
        Self {
            program_source: workspace.join("scheme/grammar/native-program.ss"),
            program_manifest: program_stage.join("program.json"),
            program_stage,
            package_prefix,
            package_load_path,
            gxi: resolve_program(env::var_os("GERBIL_GXI").unwrap_or_else(|| "gxi".into())),
            workspace,
            gsc: resolve_program(env::var_os("GERBIL_GSC").unwrap_or_else(|| "gsc".into())),
            gxpkg: resolve_program(env::var_os("GERBIL_GXPKG").unwrap_or_else(|| "gxpkg".into())),
        }
    }

    fn package_command(&self, program: &Path) -> Command {
        let mut command = gerbil_command(program);
        if program == self.gxi.as_path() && gerbil_build_verbose_level() > 0 {
            // Gerbil's driver appends GSC options after -target, where runtime
            // diagnostics are ignored. GAMBOPT reaches GXI and child GSC before
            // argv parsing while retaining the selected SDK directory mappings.
            let mut options = env::var_os("GAMBOPT").unwrap_or_default();
            if !options.is_empty() {
                options.push(",");
            }
            options.push("1n,2n,d5qQ");
            command.env("GAMBOPT", options);
        }
        command
            .current_dir(&self.workspace)
            .env("GERBIL_PATH", &self.package_prefix)
            .env("GERBIL_LOADPATH", &self.package_load_path);
        command
    }

    fn prepare_package(&self, observer: &CargoObserver) {
        let declaration = self.workspace.join("build.ss");
        observer.observe_source_input(&declaration);
        observer.observe_source_input(&self.workspace.join("scheme"));
        observe_program_archive_operation(
            observer,
            ProgramArchiveOperation {
                phase: "package-build",
                operation: "compile canonical MRR PackageSpec into isolated Cargo output",
                subject: Some("meta-relational-reasoning"),
            },
            || {
                let identity = self.package_identity()?;
                run_package_attempt(&self.package_prefix, identity.as_bytes(), || {
                    let compiler = prepare_gsc_progress_launcher(
                        &self.gsc,
                        &self.package_prefix.join("compiler"),
                        gerbil_build_verbose_level() > 0,
                    )?;
                    run(
                        self.package_command(&self.gxi)
                            .env("GERBIL_GSC", compiler)
                            .arg(&declaration)
                            .arg("compile"),
                        "compile canonical MRR PackageSpec",
                    )
                })
            },
        )
        .expect("prepare isolated MRR Scheme package");
    }

    fn package_identity(&self) -> Result<String, String> {
        let mut digest = Sha256::new();
        digest.update(b"mrr-package-v2\0");
        for tool in [&self.gsc, &self.gxi] {
            let path = fs::canonicalize(tool)
                .map_err(|error| format!("resolve package SDK tool: {error}"))?;
            let bytes =
                fs::read(&path).map_err(|error| format!("read package SDK tool: {error}"))?;
            digest.update(format!("{path:?}\0").as_bytes());
            digest.update(Sha256::digest(bytes));
        }
        let gsc =
            fs::canonicalize(&self.gsc).map_err(|error| format!("resolve package GSC: {error}"))?;
        let config = gsc.parent().expect("GSC directory").join("gambuild-C");
        match fs::read(config) {
            Ok(bytes) => digest.update(Sha256::digest(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                digest.update(b"no-gambuild-config")
            }
            Err(error) => return Err(format!("read package compiler configuration: {error}")),
        }
        for name in ["GERBIL_HOME", "GAMBOPT", "GERBIL_BUILD_VERBOSE"] {
            digest.update(format!("{name}={:?}\0", env::var_os(name)).as_bytes());
        }
        digest.update(format!("{:?}\0", self.package_load_path).as_bytes());
        Ok(format!("mrr-package-v2\n{:x}\n", digest.finalize()))
    }

    fn stage_program(&self, observer: &CargoObserver) {
        assert!(
            self.program_source.is_file(),
            "canonical Gerbil AOT root is missing: {}",
            self.program_source.display()
        );
        println!("cargo:rerun-if-changed={}", self.program_source.display());
        observe_program_archive_operation(
            observer,
            ProgramArchiveOperation {
                phase: "program-stage",
                operation: "stage compiler-owned Gerbil AOT program",
                subject: Some("meta-relational-reasoning/scheme/grammar/native-program"),
            },
            || {
                fs::create_dir_all(&self.program_stage).map_err(|error| error.to_string())?;
                let program_build = source_workspace().join("scheme/program-build.ss");
                if !program_build.is_file() {
                    return Err(format!(
                        "gerbil-scheme-rust program builder is missing: {}",
                        program_build.display()
                    ));
                }
                let expression = format!(
                    "(begin (import {}) (gerbil-rs-stage-program {} {}))",
                    scheme_string(&program_build),
                    scheme_string(&self.program_source),
                    scheme_string(&self.program_stage),
                );
                run(
                    self.package_command(&self.gxpkg)
                        .args(["env", "gxi", "-e"])
                        .arg(expression),
                    "stage compiler-owned Gerbil AOT program",
                )
            },
        )
        .expect("stage compiler-owned Gerbil AOT program");
    }

    fn package_archive(&self, observer: &CargoObserver) {
        let prefix = env::var_os("GERBIL_PATH")
            .map_or_else(|| self.workspace.join(".gerbil"), PathBuf::from);
        let parser_include = prefix.join("pkg/github.com/tao3k/gerbil-parser/include");
        let parser_headers = [parser_include.join("gerbil-parser/parse-artifact-v1.h")];
        let native_headers = [NativeHeaderInput {
            include_directory: &parser_include,
            header_files: &parser_headers,
        }];
        let receipt = build_program_archive_with_contract(
            ProgramArchiveRequest {
                manifest: &self.program_manifest,
                gsc: &self.gsc,
                archive_name: "mrr_grammar_native",
                linker_name: "mrr_grammar_linker",
                out_dir: &self.program_stage,
            },
            ProgramArchiveContract {
                required_modules: REQUIRED_MODULES,
                forbidden_modules: FORBIDDEN_RUNTIME_MODULES,
                linker_main_symbol: "mrr_grammar_gambit_main",
                additional_objects: &[],
                native_headers: &native_headers,
            },
            observer,
        )
        .expect("package MRR and parser Gerbil native archive");
        for directive in receipt.cargo_directives {
            println!("{}", directive.line());
        }
    }
}

pub(crate) fn observation_channel(
    phase: &str,
    state: &str,
    elapsed: Option<Duration>,
    verbose_level: u8,
) -> ObservationChannel {
    if state == "failed" {
        return ObservationChannel::Warning;
    }

    let phase_summary = matches!(
        phase,
        "package-build"
            | "program-stage"
            | "program-plan"
            | "module-c-batch"
            | "gsc-link"
            | "native-object-batch"
            | "static-archive"
    );
    let slow_terminal = elapsed.is_some_and(|elapsed| elapsed.as_millis() >= 5_000);
    if verbose_level >= 9 || (verbose_level > 0 && (phase_summary || slow_terminal)) {
        ObservationChannel::Trace
    } else {
        ObservationChannel::Silent
    }
}

fn gerbil_build_verbose_level() -> u8 {
    env::var("GERBIL_BUILD_VERBOSE")
        .ok()
        .map(|value| {
            value
                .parse::<u8>()
                .unwrap_or_else(|_| u8::from(!value.is_empty() && value != "0"))
        })
        .unwrap_or(0)
}

fn run(command: &mut Command, operation: &str) -> Result<(), String> {
    run_with_progress(command, operation, gerbil_build_verbose_level() > 0)
}

pub(crate) fn run_package_attempt(
    prefix: &Path,
    identity: &[u8],
    build: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let marker = prefix.join(".mrr-package-complete");
    let complete = match fs::read(&marker) {
        Ok(bytes) => bytes == identity,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("read package completion marker: {error}")),
    };
    // std/make can consider intermediate SSI/SCM files current after an
    // interrupted GSC call. Only this Cargo-owned directory is invalidated.
    if !complete && prefix.exists() {
        fs::remove_dir_all(prefix)
            .map_err(|error| format!("discard incomplete package: {error}"))?;
    }
    fs::create_dir_all(prefix).map_err(|error| format!("create package directory: {error}"))?;
    match fs::remove_file(&marker) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("invalidate package completion marker: {error}")),
    }
    build()?;
    fs::write(marker, identity)
        .map_err(|error| format!("publish package completion marker: {error}"))
}

pub(crate) fn run_with_progress(
    command: &mut Command,
    operation: &str,
    stream: bool,
) -> Result<(), String> {
    run_native_process(command, operation, stream)
}

fn resolve_program(program: impl AsRef<OsStr>) -> PathBuf {
    let program = PathBuf::from(program.as_ref());
    if program.components().count() > 1 {
        return program;
    }
    env::split_paths(&env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join(&program))
        .find(|path| path.is_file())
        .expect("locate Gerbil tool on PATH")
}

fn scheme_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}
