//! An interrupted package must not reuse current-looking SSI intermediates.
use crate::native_archive::run_package_attempt;

#[test]
fn failed_rebuild_invalidates_partial_outputs_before_the_next_attempt() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let prefix = std::env::temp_dir().join(format!("mrr-package-{}-{nonce}", std::process::id()));
    let intermediate = prefix.join("native.ssi");
    run_package_attempt(&prefix, b"SDK-A", || {
        std::fs::write(&intermediate, "successful initial output").unwrap();
        Ok(())
    })
    .unwrap();
    let error = run_package_attempt(&prefix, b"SDK-A", || {
        assert!(
            intermediate.exists(),
            "qualified warm outputs must be retained"
        );
        std::fs::write(&intermediate, "partial newer output").unwrap();
        Err("compiler rejected newer source".to_owned())
    })
    .unwrap_err();
    assert_eq!(error, "compiler rejected newer source");
    assert!(!prefix.join(".mrr-package-complete").exists());
    run_package_attempt(&prefix, b"SDK-A", || {
        assert!(
            !intermediate.exists(),
            "partial SSI must not bypass compilation"
        );
        Ok(())
    })
    .unwrap();
    assert!(prefix.join(".mrr-package-complete").exists());
    std::fs::write(&intermediate, "SDK-A output").unwrap();
    run_package_attempt(&prefix, b"SDK-B", || {
        assert!(
            !intermediate.exists(),
            "a different SDK must not reuse completed outputs"
        );
        Ok(())
    })
    .unwrap();
    std::fs::remove_dir_all(prefix).unwrap();
}
