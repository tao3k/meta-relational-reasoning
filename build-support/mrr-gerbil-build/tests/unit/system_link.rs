#[test]
fn package_paths_preserve_spaces_and_unrelated_cargo_commands_refuse() {
    let text=b"cargo:rustc-link-search=native=/opt/package store/lib\ncargo:rustc-link-lib=ssl\ncargo:rustc-link-lib=crypto\n";
    let inputs = super::parse(text).unwrap();
    assert_eq!(
        inputs
            .iter()
            .map(super::CargoDirective::line)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
        std::str::from_utf8(text).unwrap()
    );
    for invalid in [
        b"cargo:rustc-env=RUSTFLAGS=-L/tmp\n".as_slice(),
        b"cargo:rustc-link-search=native=relative\ncargo:rustc-link-lib=ssl\n",
        b"cargo:rustc-link-lib=ssl\n",
        b"cargo:rustc-link-search=native=/tmp\ncargo:rustc-link-lib=ssl -lother\n",
    ] {
        assert!(super::parse(invalid).is_err());
    }
}

#[test]
#[ignore = "requires the actual root build.ss link-inputs receipt"]
fn actual_scheme_package_receipt_uses_declared_crypto_libraries() {
    let path = std::path::PathBuf::from(
        std::env::var_os("MRR_PACKAGE_LINK_RECEIPT").expect("actual Scheme receipt"),
    );
    let inputs = super::read(&path).expect("complete Scheme package output");
    for library in ["ssl", "crypto"] {
        assert!(inputs.iter().any(
            |input| input.kind == super::CargoDirectiveKind::RustcLinkLib && input.value == library
        ));
    }
    for input in inputs
        .iter()
        .filter(|input| input.kind == super::CargoDirectiveKind::RustcLinkSearch)
    {
        assert!(std::path::Path::new(input.value.strip_prefix("native=").unwrap()).is_dir());
    }
}
