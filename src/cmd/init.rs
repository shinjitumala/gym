use crate::com::*;

#[derive(Args)]
#[args(desc = "Create an empty database at the configured path.")]
pub struct Init {}
impl Run<C> for Init {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(init(c, a)?)
    }
}
fn init(c: &C, _a: Init) -> Res<()> {
    let d = c.block_on(crate::db::Db::create(&c.cfg.db))?;
    println!("Created '{}'.", &c.cfg.db);
    println!("Seeded the effort presets and the four settings.");
    println!("{:#?}", c.block_on(crate::import::counts(&d))?);
    Ok(())
}
