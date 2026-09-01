//! Target-native archive/link adapter for the AOT artifact staged by `build.ss`.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gerbil_scheme_native_build::{
    NativeLinkLibrary, NativeStaticLinkPlan, build_static_archive_from_link_plan,
    discover_gambit_link_search_dir_from_gsc,
};

/// Materializes the staged Gerbil AOT artifact and emits Cargo link directives.
pub fn build_native_archive(manifest: &Path) {
    let workspace = manifest.ancestors().nth(2).expect("workspace root");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let generated = workspace
        .join("scheme/generated")
        .join("meta-relational-reasoning__scheme__grammar__native.scm");
    let scm = out.join("native.scm");
    fs::copy(&generated, &scm).expect("stage Gerbil native SCM");

    let gsc = resolve_program(env::var_os("GERBIL_GSC").unwrap_or_else(|| "gsc".into()));
    let linker_c = out.join("native_link.c");
    let linker_o = out.join("native_link.o");
    let native_c = out.join("native.c");
    let native_o = out.join("native.o");
    let runtime_o = out.join("runtime.o");

    run(
        clean_command(&gsc)
            .args(["-link", "-linker-name", "mrr_grammar_linker", "-o"])
            .arg(&linker_c)
            .arg(&scm),
        "generate Gambit link unit",
    );
    let expression = format!(
        "(compile-file-to-target {} output: {} module-name: \"meta-relational-reasoning/scheme/grammar/native\")",
        scheme_string(&scm),
        scheme_string(&native_c)
    );
    run(
        clean_command(&gsc).arg("-e").arg(expression),
        "generate Gerbil module C",
    );
    run(
        clean_command(&gsc)
            .args(["-obj", "-o"])
            .arg(&native_o)
            .arg(&native_c),
        "compile Gerbil module object",
    );
    run(
        clean_command(&gsc)
            .args([
                "-obj",
                "-cc-options",
                "-Dmain=mrr_grammar_gambit_main",
                "-o",
            ])
            .arg(&linker_o)
            .arg(&linker_c),
        "compile Gambit link object",
    );
    run(
        clean_command(&gsc)
            .args(["-obj", "-o"])
            .arg(&runtime_o)
            .arg(manifest.join("native/runtime.c")),
        "compile grammar runtime shim",
    );

    let gambit = discover_gambit_link_search_dir_from_gsc(&gsc)
        .expect("discover Gambit library for selected gsc");
    let mut libraries = vec![
        NativeLinkLibrary::new("static=gambit"),
        NativeLinkLibrary::new("dylib=m"),
    ];
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        libraries.push(NativeLinkLibrary::new("dylib=dl"));
    }
    let receipt = build_static_archive_from_link_plan(
        "mrr_grammar_native",
        &NativeStaticLinkPlan {
            module_objects: vec![runtime_o, native_o],
            link_object: linker_o,
            link_search_dirs: vec![gambit.search_dir],
            link_libraries: libraries,
        },
        &out,
    )
    .expect("package MRR Gerbil native archive");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
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
