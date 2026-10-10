fn main() {
    if std::env::var_os("CARGO_FEATURE_EMBEDDED_RUNTIME").is_none() {
        return;
    }
    for path in [
        "../../gerbil.pkg",
        "../../scheme/grammar/gql-declaration.ss",
        "../../scheme/grammar/parser-authority-receipt-declaration.ss",
        "../../scheme/grammar/native.ss",
        "../../scheme/grammar/native-program.ss",
        "../../scheme/reasoning/declaration.ss",
        "../../scheme/search/enhanced-tree-sitter-query-declaration.ss",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    mrr_gerbil_native_build::build_native_archive(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}
