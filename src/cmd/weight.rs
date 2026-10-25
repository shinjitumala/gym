use crate::com::*;

#[derive(Args)]
#[args(desc = "Add weight data.")]
pub struct Weight {}
impl Run<C> for Weight {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(weight(c, a)?)
    }
}
fn weight(c: &C, _a: Weight) -> Res<()> {
    let date = input_date2("When did you measure?")?;
    let kg = CustomType::<f64>::new("Weight (kg)").prompt()?;
    let bodyfat = CustomType::<f64>::new("Bodyfat (%)")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let desc = Text::new("Note").prompt()?;
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    c.block_on(d.add_weight(&date.to_string(), kg, bodyfat, &desc))?;
    Ok(())
}
