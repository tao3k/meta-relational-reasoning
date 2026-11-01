//! Cargo consumes the canonical Scheme package's pkg-config projection.
use std::io::Read;
use std::path::{Path, PathBuf};

use gerbil_scheme_native_build::{CargoDirective, CargoDirectiveKind};

pub(crate) fn read(path: &Path) -> Result<Vec<CargoDirective>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    parse(&bytes)
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<CargoDirective>, String> {
    if bytes.is_empty() || bytes.len() > 16_384 {
        return Err("invalid package link projection size".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let mut inputs = Vec::new();
    for line in text.lines() {
        let input = match line.split_once('=') {
            Some(("cargo:rustc-link-search", value)) => {
                let path = PathBuf::from(
                    value
                        .strip_prefix("native=")
                        .ok_or("expected native library directory")?,
                );
                if !path.is_absolute() {
                    return Err("package library directory must be absolute".into());
                }
                CargoDirective {
                    kind: CargoDirectiveKind::RustcLinkSearch,
                    value: format!("native={}", path.display()),
                }
            }
            Some(("cargo:rustc-link-lib", name))
                if !name.is_empty()
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c)) =>
            {
                CargoDirective {
                    kind: CargoDirectiveKind::RustcLinkLib,
                    value: name.into(),
                }
            }
            _ => return Err("unexpected package link directive".into()),
        };
        if inputs.len() >= 64 {
            return Err("too many package link inputs".into());
        }
        inputs.push(input);
    }
    if !inputs
        .iter()
        .any(|input| input.kind == CargoDirectiveKind::RustcLinkSearch)
        || !inputs
            .iter()
            .any(|input| input.kind == CargoDirectiveKind::RustcLinkLib)
    {
        return Err("incomplete package link projection".into());
    }
    Ok(inputs)
}
#[cfg(test)]
#[path = "../tests/unit/system_link.rs"]
mod tests;
