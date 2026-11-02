//! Package-owned GxTest host; no Scheme frontend subprocess is launched.
#[path = "../package_cli.rs"]
mod package_cli;

fn main() -> std::process::ExitCode {
    match package_cli::run() {
        Ok(code) => std::process::ExitCode::from(u8::try_from(code).unwrap_or(70)),
        Err(error) => {
            eprintln!("MRR GxTest refused: {error}");
            std::process::ExitCode::from(70)
        }
    }
}
