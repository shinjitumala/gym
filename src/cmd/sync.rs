use crate::com::*;

#[derive(Args)]
#[args(desc = "Copy the database to the configured remote.")]
pub struct Sync {}
impl Run<C> for Sync {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(sync(c, a)?)
    }
}
fn sync(c: &C, _a: Sync) -> Res<()> {
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    let tmp = std::env::temp_dir().join(format!("gym-sync-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let tmp_s = tmp.to_string_lossy().to_string();
    c.block_on(sqlx::query("VACUUM INTO ?1").bind(&tmp_s).execute(&d.pool))?;
    let size = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    println!("Snapshot '{}' ({size} bytes).", tmp.display());
    let out = std::process::Command::new("rsync")
        .arg("-uP")
        .arg(&tmp)
        .arg(&c.cfg.remote)
        .status()
        .map_err(|e| format!("Failed to run 'rsync' because '{e}'."))?;
    let _ = std::fs::remove_file(&tmp);
    if !out.success() {
        Err(format!(
            "'rsync' failed with '{out}'; the snapshot was removed."
        ))?;
    }
    println!("Copied to '{}'.", &c.cfg.remote);
    Ok(())
}
