//! Runnable consumer workflow using shared, deterministic external adapter fixtures.
pub mod support;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receipt = support::workflow()?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
