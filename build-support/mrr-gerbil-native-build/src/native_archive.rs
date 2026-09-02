use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gerbil_scheme_native_build::{
    NativeLinkLibrary, NativeStaticLinkPlan, build_static_archive_from_link_plan,
    discover_gambit_link_search_dir_from_gsc,
};

const NATIVE_MODULE_NAME: &str = "meta-relational-reasoning/scheme/grammar/native";

struct NativeArchiveBuild<'a> {
    manifest: &'a Path,
    out: PathBuf,
    gsc: PathBuf,
    scm: PathBuf,
    linker_c: PathBuf,
    linker_o: PathBuf,
    native_c: PathBuf,
    native_o: PathBuf,
    runtime_o: PathBuf,
}

impl<'a> NativeArchiveBuild<'a> {
    fn new(manifest: &'a Path) -> Self {
        let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
        Self {
            manifest,
            gsc: resolve_program(env::var_os("GERBIL_GSC").unwrap_or_else(|| "gsc".into())),
            scm: out.join("native.scm"),
            linker_c: out.join("native_link.c"),
            linker_o: out.join("native_link.o"),
            native_c: out.join("native.c"),
            native_o: out.join("native.o"),
            runtime_o: out.join("runtime.o"),
            out,
        }
    }

    fn stage_declared_aot_module(&self) {
        let generated = self
            .manifest
            .ancestors()
            .nth(2)
            .expect("workspace root")
            .join("scheme/generated")
            .join("meta-relational-reasoning__scheme__grammar__native.scm");
        fs::copy(generated, &self.scm).expect("stage Gerbil native SCM");
    }

    fn generate_native_sources(&self) {
        run(
            clean_command(&self.gsc)
                .args(["-link", "-linker-name", "mrr_grammar_linker", "-o"])
                .arg(&self.linker_c)
                .arg(&self.scm),
            "generate Gambit link unit",
        );
        let expression = format!(
            "(compile-file-to-target {} output: {} module-name: \"{NATIVE_MODULE_NAME}\")",
            scheme_string(&self.scm),
            scheme_string(&self.native_c)
        );
        run(
            clean_command(&self.gsc).arg("-e").arg(expression),
            "generate Gerbil module C",
        );
    }

    fn compile_objects(&self) {
        self.compile_object(
            &self.native_o,
            &self.native_c,
            &[],
            "compile Gerbil module object",
        );
        self.compile_object(
            &self.linker_o,
            &self.linker_c,
            &["-cc-options", "-Dmain=mrr_grammar_gambit_main"],
            "compile Gambit link object",
        );
        self.compile_object(
            &self.runtime_o,
            &self.manifest.join("native/runtime.c"),
            &[],
            "compile grammar runtime shim",
        );
    }

    fn compile_object(&self, output: &Path, input: &Path, options: &[&str], operation: &str) {
        let mut command = clean_command(&self.gsc);
        command
            .arg("-obj")
            .args(options)
            .arg("-o")
            .arg(output)
            .arg(input);
        run(&mut command, operation);
    }

    fn package_archive(&self) {
        let gambit = discover_gambit_link_search_dir_from_gsc(&self.gsc)
            .expect("discover Gambit library for selected gsc");
        let receipt = build_static_archive_from_link_plan(
            "mrr_grammar_native",
            &NativeStaticLinkPlan {
                module_objects: vec![self.runtime_o.clone(), self.native_o.clone()],
                link_object: self.linker_o.clone(),
                link_search_dirs: vec![gambit.search_dir],
                link_libraries: native_link_libraries(),
            },
            &self.out,
        )
        .expect("package MRR Gerbil native archive");
        for directive in receipt.cargo_directives {
            println!("{}", directive.line());
        }
    }
}

/// Materializes the staged Gerbil AOT artifact and emits Cargo link directives.
pub fn build_native_archive(manifest: &Path) {
    let build = NativeArchiveBuild::new(manifest);
    build.stage_declared_aot_module();
    build.generate_native_sources();
    build.compile_objects();
    build.package_archive();
}

fn native_link_libraries() -> Vec<NativeLinkLibrary> {
    let mut libraries = vec![
        NativeLinkLibrary::new("static=gambit"),
        NativeLinkLibrary::new("dylib=m"),
    ];
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        libraries.push(NativeLinkLibrary::new("dylib=dl"));
    }
    libraries
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

fn run(command: &mut Command, operation: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("{operation}: {error}"));
    assert!(status.success(), "{operation} failed with {status}");
}

fn resolve_program(program: impl AsRef<OsStr>) -> PathBuf {
    let program = PathBuf::from(program.as_ref());
    if program.components().count() > 1 {
        return program;
    }
    env::split_paths(&env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join(&program))
        .find(|path| path.is_file())
        .expect("locate gsc on PATH")
}

fn scheme_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}
