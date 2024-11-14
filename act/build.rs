use fpr_sh::run;

fn main() -> Result<(), String> {
    run("sh", "sh.rs")?;
    Ok(())
}
