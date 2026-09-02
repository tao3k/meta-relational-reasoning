fn main() {
    for path in [
        "../../gerbil.pkg",
        "../../scheme/grammar/gql-declaration.ss",
        "../../scheme/grammar/native.ss",
        "../../scheme/reasoning/declaration.ss",
        "../../scheme/generated/meta-relational-reasoning__scheme__grammar__native.scm",
        "native/runtime.c",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    mrr_gerbil_native_build::build_native_archive(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}
