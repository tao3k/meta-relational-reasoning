//! Direct upstream C ABI GxTest entry; the existing supervisor owns time/output gates.
use gerbil_scheme::GerbilRuntime;
use std::path::PathBuf;

pub fn run() -> Result<i32, String> {
    let workspace = std::env::current_dir().map_err(|error| error.to_string())?;
    let prefix =
        std::env::var_os("GERBIL_PATH").map_or_else(|| workspace.join(".gerbil"), PathBuf::from);
    let mut libraries = vec![prefix.join("lib")];
    if let Some(paths) = std::env::var_os("GERBIL_LOADPATH") {
        libraries.extend(std::env::split_paths(&paths));
    }
    let libraries = std::env::join_paths(libraries).map_err(|error| error.to_string())?;
    let mut arguments = std::env::args().skip(1);
    let operation = arguments.next().ok_or("expected build, clean, test or deps")?;
    let sources: Vec<_> = arguments.collect();
    if operation == "deps" && sources.is_empty() {
        let mut prepare = gerbil_scheme_aot_build::gerbil_command("python3");
        prepare
            .args(["-m", "mrr_proof_validation.prepare_gerbil_dependencies"])
            .current_dir(&workspace)
            .env("GERBIL_PATH", &prefix)
            .env("GERBIL_LOADPATH", &libraries)
            .env("PYTHONPATH", workspace.join("proofs/MRRProof/src"));
        gerbil_scheme_aot_build::run_native_process(
            &mut prepare,
            "acquire producer-owned prerequisites",
            true,
        )?;
        let mut command = gerbil_scheme_aot_build::gerbil_command("gxpkg");
        command
            .args(["deps", "--install"])
            .current_dir(&workspace)
            .env("GERBIL_PATH", &prefix)
            .env("GERBIL_LOADPATH", &libraries);
        // Package acquisition/bootstrap remains official GxPkg work. Public
        // build and GxTest operations below are actual owner-local C ABI calls.
        gerbil_scheme_aot_build::run_native_process(
            &mut command,
            "install declared dependencies",
            true,
        )?;
        return Ok(0);
    }
    let runtime = GerbilRuntime::initialize().map_err(|error| error.to_string())?;
    let code = runtime
        .configure_package_paths(
            prefix.to_str().ok_or("package prefix must be UTF-8")?,
            libraries.to_str().ok_or("library paths must be UTF-8")?,
        )
        .map_err(|error| error.to_string())?;
    if code != 0 {
        return Ok(code);
    }
    let directory = workspace.to_str().ok_or("workspace must be UTF-8")?;
    let result = match (operation.as_str(), sources.is_empty()) {
        ("build", true) => runtime
            .build_package(directory)
            .map_err(|error| error.to_string()),
        ("clean", true) => runtime
            .run_package_main(directory, &["clean"])
            .map_err(|error| error.to_string()),
        ("test", false) => sources.iter().try_fold(0, |code, source| {
            if code != 0 {
                return Ok(code);
            }
            runtime
                .test_package(directory, source)
                .map_err(|error| error.to_string())
        }),
        _ => Err("build and clean take no arguments; test requires an explicit nonempty inventory".into()),
    };
    drop(runtime);
    eprintln!("MRR-PACKAGE owner cleanup returned");
    result
}
