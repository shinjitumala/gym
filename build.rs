use std::process::Command;

fn main() -> Result<(), String> {
    let s = Command::new("./gen.sh")
        .status()
        .map_err(|e| format!("Failed to execute 'gen.sh' because '{e}'"))?;
    if !s.success() {
        Err(format!("'gen.sh' did not succeed with status '{s}'"))?
    }
    Ok(())
}
