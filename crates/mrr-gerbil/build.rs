fn main() {
    for path in [
        "../../gerbil.pkg",
        "../../scheme/grammar/gql-declaration.ss",
        "../../scheme/grammar/native.ss",
        "../../scheme/reasoning/declaration.ss",
        "native/runtime.c",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    mrr_gerbil_native_build::build_native_archive(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}
