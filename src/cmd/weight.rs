use crate::com::*;

#[derive(Args)]
#[args(f = weight, desc = "Add weight data.")]
pub struct Weight {}
fn weight(c: &C, _a: Weight) -> Res<()> {
    let date = input_date2("When did you measure?")?;
    let kg = CustomType::<f64>::new("Weight (kg)").prompt()?;
    let bodyfat = CustomType::<f64>::new("Bodyfat (%)")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let desc = Text::new("Note").prompt()?;
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    c.block_on(crate::db::add_weight(
        &d,
        &date.to_string(),
        kg,
        bodyfat,
        &desc,
    ))?;
    Ok(())
}
