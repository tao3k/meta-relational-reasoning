// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Thin Cargo adapter for the canonical Gerbil AOT program builder.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use gerbil_scheme_native_build::{
    ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramArchiveRequest, build_program_archive_with_contract,
    observe_program_archive_operation, source_workspace,
};

const REQUIRED_MODULES: &[&str] = &[
    "meta-relational-reasoning/scheme/grammar/native-program",
    "meta-relational-reasoning/scheme/grammar/native",
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
    println!("cargo:rerun-if-env-changed=GERBIL_GSC");
    println!("cargo:rerun-if-env-changed=GERBIL_GXPKG");
    println!("cargo:rerun-if-env-changed=GERBIL_BUILD_VERBOSE");
    let build = NativeBuild::new(manifest);
    let observer = CargoObserver;
    build.stage_program(&observer);
    build.package_archive(&observer);
}

struct NativeBuild {
    workspace: PathBuf,
    program_source: PathBuf,
    program_stage: PathBuf,
    program_manifest: PathBuf,
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
        Self {
            program_source: workspace.join("scheme/grammar/native-program.ss"),
            program_manifest: program_stage.join("program.json"),
            program_stage,
            workspace,
            gsc: resolve_program(env::var_os("GERBIL_GSC").unwrap_or_else(|| "gsc".into())),
            gxpkg: resolve_program(env::var_os("GERBIL_GXPKG").unwrap_or_else(|| "gxpkg".into())),
        }
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
                    clean_command(&self.gxpkg)
                        .current_dir(&self.workspace)
                        .args(["env", "gxi", "-e"])
                        .arg(expression),
                    "stage compiler-owned Gerbil AOT program",
                )
            },
        )
        .expect("stage compiler-owned Gerbil AOT program");
    }

    fn package_archive(&self, observer: &CargoObserver) {
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
        "program-stage"
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
    let output = command
        .output()
        .map_err(|error| format!("{operation}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{operation}: {}; {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn clean_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    for name in [
        "CC",
        "CFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "CPATH",
        "C_INCLUDE_PATH",
        "CPLUS_INCLUDE_PATH",
        "LIBRARY_PATH",
        "NIX_CFLAGS_COMPILE",
        "NIX_LDFLAGS",
        "SDKROOT",
    ] {
        command.env_remove(name);
    }
    command
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
