use crate::com::*;

#[derive(Args)]
#[args(f = food_act, desc = "Add food data.")]
pub struct Food {}
fn food_act(c: &C, _a: Food) -> Res<()> {
    let d = c.block_on(crate::db::Db::open(&c.cfg.db))?;
    print_today(c, &d)?;

    let date = input_date2("When did you eat?")?;
    let foods = c.block_on(crate::db::foods(&d))?;
    loop {
        let f = match select_line(
            "What did you eat? (calories kcal, protein g, fat g, carbohydrates g)",
            &foods,
            |f| f.to_line(),
        )
        .with_help_message("Press ESC to register unknown food")
        .prompt_skippable()?
        {
            Some(f) => foods[f.index].id,
            None => {
                println!("Registering new food...");
                crate::food::reg(c, &d)?
            }
        };

        let amount = CustomType::<f64>::new("Amount")
            .with_help_message("Multiplier")
            .with_default(1.0f64)
            .prompt()?;
        let desc = Text::new("desc").prompt()?;

        c.block_on(crate::db::new_meal(
            &d,
            &t2s(date.as_datetime().to_owned()),
            f,
            amount,
            &desc,
        ))?;

        if !Confirm::new("Add more food?")
            .with_default(false)
            .prompt()?
        {
            break;
        }
    }

    print_today(c, &d)?;
    Ok(())
}

fn print_today(c: &C, d: &crate::db::Db) -> Res<()> {
    let (s, e) = crate::util::day_bounds(chrono::Local::now());
    let t = c.block_on(crate::db::day_totals(d, &s.to_rfc3339(), &e.to_rfc3339()))?;
    println!("{} {}", s.to_rfc3339(), e.to_rfc3339());
    println!("Today's total:");
    println!("{}", to_table(&t.to_lines_today()));
    Ok(())
}
