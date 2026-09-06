use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate-contracts <output-json>")?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let schema = tiktools_core::contracts::automation_contract_schema();
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&schema)?),
    )?;
    Ok(())
}
