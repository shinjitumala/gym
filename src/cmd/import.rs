use crate::com::*;

#[derive(Args)]
#[args(desc = "Fill an empty database from a TOML archive directory.")]
pub struct Import {
    #[arg(desc = "The directory holding the TOML archive, for example ~/g/gym_data/d.")]
    dir: Option<String>,
    #[arg(desc = "A TOML table mapping a note partner name to an exercise name.")]
    map: Option<String>,
}
impl Run<C> for Import {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(import_act(c, a)?)
    }
}
fn import_act(c: &C, a: Import) -> Res<()> {
    let dir = a.dir.ok_or_else(|| {
        "The archive directory is required: 'gym db import --dir ~/g/gym_data/d'.".to_string()
    })?;
    let dir = std::path::PathBuf::from(dir);
    let map = a.map.map(std::path::PathBuf::from);
    if !dir.is_dir() {
        Err(format!("'{}' is not a directory.", dir.display()))?;
    }
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    let present = c.block_on(crate::import::counts(&d))?;
    if present.total() > 0 {
        Err(format!(
            "The database already holds rows in {present:?}; the import needs an empty database created by 'gym db init'."
        ))?;
    }
    let r = c.block_on(crate::import::import(&d, &dir, map.as_deref()))?;
    println!("{}", r.to_lines());
    println!("\nImported: {:#?}", c.block_on(crate::import::counts(&d))?);
    Ok(())
}
